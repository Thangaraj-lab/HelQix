//! Frame files in an S3-compatible object store (SeaweedFS, Apache-2.0, is the one this project uses).
//!
//! The frame store stays a local directory, which is a cache; the object store is the durable copy that every API
//! instance shares. Files are content-addressed (`frames/<blake3>.parquet`) or small run indexes (`runs/<id>.json`),
//! so a copy can never go stale: a name that exists has the right bytes. Reads still re-verify the BLAKE3 address and
//! the state chain (the frame store does that on every load), so a corrupted or swapped object is caught, not served.
//!
//! - after a run is stored: `push` uploads its index and frames if the object store does not have them;
//! - before a run is read: `ensure` downloads them if the local cache does not have them;
//! - before the run list: `sync_indexes` downloads indexes this instance has not seen.
use std::{path::{Path, PathBuf}, sync::Arc};

use futures_util::TryStreamExt;
use helqix_frames::RunIndex;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, path::Path as ObjectPath};

use crate::error::ApiError;

pub struct ObjectMirror { store: Arc<dyn ObjectStore> }

fn valid_id(id: &str) -> bool { !id.is_empty() && id.len() <= 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') }
fn valid_hash(h: &str) -> bool { h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()) }

fn unavailable(what: &str, e: impl std::fmt::Display) -> ApiError {
    tracing::error!(error = %e, "object store: {what}");
    ApiError::Unavailable
}

impl ObjectMirror {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self { Self { store } }

    /// `HELQIX_S3_ENDPOINT`, `HELQIX_S3_BUCKET`, `HELQIX_S3_ACCESS_KEY_ID` and `HELQIX_S3_SECRET_ACCESS_KEY` together turn it on
    /// (`HELQIX_S3_REGION` defaults to us-east-1; `HELQIX_S3_ALLOW_HTTP=1` permits plain HTTP for a local store). None turns it off;
    /// a partial setting is an error.
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let get = |n: &str| std::env::var(n).ok();
        let (endpoint, bucket, key, secret) = (get("HELQIX_S3_ENDPOINT"), get("HELQIX_S3_BUCKET"), get("HELQIX_S3_ACCESS_KEY_ID"), get("HELQIX_S3_SECRET_ACCESS_KEY"));
        match (endpoint, bucket, key, secret) {
            (None, None, None, None) => Ok(None),
            (Some(endpoint), Some(bucket), Some(key), Some(secret)) => {
                let store = object_store::aws::AmazonS3Builder::new()
                    .with_endpoint(endpoint).with_bucket_name(bucket).with_access_key_id(key).with_secret_access_key(secret)
                    .with_region(get("HELQIX_S3_REGION").unwrap_or_else(|| "us-east-1".into()))
                    .with_allow_http(get("HELQIX_S3_ALLOW_HTTP").is_some_and(|v| v == "1"))
                    .with_virtual_hosted_style_request(false)
                    .build()?;
                Ok(Some(Self::new(Arc::new(store))))
            }
            _ => anyhow::bail!("set all of HELQIX_S3_ENDPOINT, HELQIX_S3_BUCKET, HELQIX_S3_ACCESS_KEY_ID and HELQIX_S3_SECRET_ACCESS_KEY, or none"),
        }
    }

    async fn put_if_absent(&self, key: &str, bytes: Vec<u8>) -> Result<(), ApiError> {
        let path = ObjectPath::from(key);
        match self.store.head(&path).await {
            Ok(_) => Ok(()),
            Err(object_store::Error::NotFound { .. }) => self.store.put(&path, PutPayload::from(bytes)).await.map(|_| ()).map_err(|e| unavailable("upload", e)),
            Err(e) => Err(unavailable("head", e)),
        }
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, ApiError> {
        match self.store.get(&ObjectPath::from(key)).await {
            Ok(r) => Ok(Some(r.bytes().await.map_err(|e| unavailable("download", e))?.to_vec())),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(unavailable("download", e)),
        }
    }

    /// Uploads a stored run's index and frame file.
    pub async fn push(&self, dir: &Path, run_id: &str) -> Result<(), ApiError> {
        if !valid_id(run_id) { return Err(ApiError::BadRequest("bad run id".into())); }
        let index_path = dir.join("runs").join(format!("{run_id}.json"));
        let index_bytes = tokio::fs::read(&index_path).await.map_err(|_| ApiError::NotFound)?;
        let index: RunIndex = serde_json::from_slice(&index_bytes).map_err(|_| ApiError::Internal)?;
        if !valid_hash(&index.frames_blake3) { return Err(ApiError::Internal); }
        let frames = tokio::fs::read(dir.join("frames").join(format!("{}.parquet", index.frames_blake3))).await.map_err(|_| ApiError::NotFound)?;
        self.put_if_absent(&format!("frames/{}.parquet", index.frames_blake3), frames).await?;
        self.put_if_absent(&format!("runs/{run_id}.json"), index_bytes).await
    }

    /// Makes sure the local cache holds a run's index and frame file, downloading them when it does not. A run the object
    /// store does not have is left alone (the caller's own load then answers not found).
    pub async fn ensure(&self, dir: &Path, run_id: &str) -> Result<(), ApiError> {
        if !valid_id(run_id) { return Ok(()); }
        let index_path = dir.join("runs").join(format!("{run_id}.json"));
        let index_bytes = match tokio::fs::read(&index_path).await {
            Ok(b) => b,
            Err(_) => {
                let Some(b) = self.get(&format!("runs/{run_id}.json")).await? else { return Ok(()) };
                write_atomic(&index_path, &b).await?;
                b
            }
        };
        let Ok(index) = serde_json::from_slice::<RunIndex>(&index_bytes) else { return Ok(()) };
        if !valid_hash(&index.frames_blake3) { return Ok(()); }
        let frames_path = dir.join("frames").join(format!("{}.parquet", index.frames_blake3));
        if tokio::fs::metadata(&frames_path).await.is_err() {
            if let Some(b) = self.get(&format!("frames/{}.parquet", index.frames_blake3)).await? { write_atomic(&frames_path, &b).await?; }
        }
        Ok(())
    }

    /// Downloads every run index the local cache has not seen.
    pub async fn sync_indexes(&self, dir: &Path) -> Result<(), ApiError> {
        let prefix = ObjectPath::from("runs");
        let metas: Vec<_> = self.store.list(Some(&prefix)).try_collect().await.map_err(|e| unavailable("list", e))?;
        for meta in metas {
            let Some(name) = meta.location.filename() else { continue };
            let Some(id) = name.strip_suffix(".json") else { continue };
            if valid_id(id) && tokio::fs::metadata(dir.join("runs").join(name)).await.is_err() { self.ensure(dir, id).await?; }
        }
        Ok(())
    }
}

async fn write_atomic(path: &PathBuf, bytes: &[u8]) -> Result<(), ApiError> {
    let tmp = path.with_extension("part");
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|_| ApiError::Internal)?; }
    tokio::fs::write(&tmp, bytes).await.map_err(|_| ApiError::Internal)?;
    tokio::fs::rename(&tmp, path).await.map_err(|_| ApiError::Internal)
}
