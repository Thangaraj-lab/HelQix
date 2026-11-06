//! What each stored run was made from (its parameters, its parent run and checkpoint, the model revision), so the engine
//! can freeze and branch it later and the Studio can list a project's runs.
//!
//! Two homes, same contract: PostgreSQL (the default in the API binary: shared by every instance, indexed by project)
//! and a directory of JSON files next to the frames (single machine, no database; used by the tests that do not touch
//! PostgreSQL). The frames themselves live in the frame store either way.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Json as Jsonb};
use uuid::Uuid;

use crate::{battery_run::BatteryRunSpec, error::ApiError, fmu_run::FmuRunSpec, frames::StartRun};

/// What a run was made from.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoredSpec { pub params: StoredParams, pub parent: Option<Parent>, #[serde(default)] pub model: Option<ModelLink>, /** Who started the run (the user's subject, or `service`). */ #[serde(default)] pub owner: Option<String> }

/// The point-mass parameters of a run made by the built-in engine, or the description of a run made by an FMU. Stored as plain JSON of
/// either shape (each refuses the other's fields), so rows written before FMU runs existed still load.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StoredParams { PointMass(StartRun), Fmu(FmuRunSpec), Battery(BatteryRunSpec) }

impl StoredSpec {
    /// The point-mass parameters, for freeze and branch. An FMU cannot restore a saved state, so its runs have none.
    pub fn point_mass(&self) -> Result<&StartRun, ApiError> {
        match &self.params {
            StoredParams::PointMass(p) => Ok(p),
            StoredParams::Fmu(_) | StoredParams::Battery(_) => Err(ApiError::Rejected("this run came from an FMU or a domain pack; they cannot restore a saved state, so it cannot be frozen or branched".into())),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Parent { pub run_id: String, pub index: usize }

/// The committed model revision a run was made from.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ModelLink { pub project_id: String, pub revision: u32 }

/// One row of a project's run list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunEntry { pub run_id: String, pub revision: u32, pub parent_run_id: Option<String>, pub started_at: String }

pub struct SpecStore { files: PathBuf, pool: Option<PgPool> }

fn valid_id(id: &str) -> bool { !id.is_empty() && id.len() <= 80 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') }

const NOT_STARTED_HERE: &str = "this run was not started through the API, so its model is unknown and it cannot be frozen";

impl SpecStore {
    /// `pool` is `Some` to keep the metadata in PostgreSQL, `None` for files under `frames_dir/specs`.
    pub fn new(frames_dir: PathBuf, pool: Option<PgPool>) -> Self { Self { files: frames_dir.join("specs"), pool } }

    pub async fn save(&self, id: &str, spec: &StoredSpec) -> Result<(), ApiError> {
        if !valid_id(id) { return Err(ApiError::BadRequest("bad run id".into())); }
        if let Some(pool) = &self.pool {
            let project = spec.model.as_ref().map(|m| Uuid::parse_str(&m.project_id)).transpose().map_err(|_| ApiError::Internal)?;
            sqlx::query("INSERT INTO helqix_run_specs (run_id, project_id, revision, parent_run_id, parent_index, params) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (run_id) DO NOTHING")
                .bind(id).bind(project).bind(spec.model.as_ref().map(|m| m.revision as i32))
                .bind(spec.parent.as_ref().map(|p| p.run_id.clone())).bind(spec.parent.as_ref().map(|p| p.index as i32))
                .bind(Jsonb(&spec.params)).execute(pool).await?;
            if let Some(owner) = &spec.owner {
                sqlx::query("INSERT INTO helqix_run_owners (run_id, owner) VALUES ($1, $2) ON CONFLICT DO NOTHING").bind(id).bind(owner).execute(pool).await?;
            }
            return Ok(());
        }
        let (files, id, bytes) = (self.files.clone(), id.to_owned(), serde_json::to_vec(spec).map_err(|_| ApiError::Internal)?);
        tokio::task::spawn_blocking(move || -> std::io::Result<()> { std::fs::create_dir_all(&files)?; std::fs::write(files.join(format!("{id}.json")), bytes) })
            .await.map_err(|_| ApiError::Internal)?.map_err(|e| { tracing::error!(error = %e, "could not save the run's parameters"); ApiError::Internal })
    }

    pub async fn load(&self, id: &str) -> Result<StoredSpec, ApiError> {
        if !valid_id(id) { return Err(ApiError::BadRequest("bad run id".into())); }
        if let Some(pool) = &self.pool {
            type Row = (Option<Uuid>, Option<i32>, Option<String>, Option<i32>, Jsonb<StoredParams>);
            let row: Option<Row> = sqlx::query_as("SELECT project_id, revision, parent_run_id, parent_index, params FROM helqix_run_specs WHERE run_id = $1").bind(id).fetch_optional(pool).await?;
            let (project, revision, parent_run, parent_index, params) = row.ok_or_else(|| ApiError::Rejected(NOT_STARTED_HERE.into()))?;
            return Ok(StoredSpec {
                params: params.0,
                parent: parent_run.zip(parent_index).map(|(run_id, i)| Parent { run_id, index: i as usize }),
                model: project.zip(revision).map(|(p, r)| ModelLink { project_id: p.to_string(), revision: r as u32 }),
                owner: None,
            });
        }
        let (files, id) = (self.files.clone(), id.to_owned());
        let bytes = tokio::task::spawn_blocking(move || std::fs::read(files.join(format!("{id}.json")))).await.map_err(|_| ApiError::Internal)?
            .map_err(|_| ApiError::Rejected(NOT_STARTED_HERE.into()))?;
        serde_json::from_slice(&bytes).map_err(|_| ApiError::Internal)
    }

    /// The ids of the runs a user may list: the ones they started and the ones made from a project they own. Without PostgreSQL (run metadata in
    /// files, one machine) a user has none: the files hold no owner.
    pub async fn visible_to(&self, owner: &str) -> Result<std::collections::HashSet<String>, ApiError> {
        let Some(pool) = &self.pool else { return Ok(Default::default()) };
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT run_id FROM helqix_run_owners WHERE owner = $1 UNION SELECT s.run_id FROM helqix_run_specs s JOIN helqix_owners o ON o.resource_id = s.project_id WHERE o.owner = $1").bind(owner).fetch_all(pool).await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// The runs made from a project's revisions, newest first.
    pub async fn project_runs(&self, project_id: &str) -> Result<Vec<RunEntry>, ApiError> {
        if let Some(pool) = &self.pool {
            let pid = Uuid::parse_str(project_id).map_err(|_| ApiError::BadRequest("id must be a UUID".into()))?;
            let rows: Vec<(String, i32, Option<String>, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
                "SELECT run_id, revision, parent_run_id, created_at FROM helqix_run_specs WHERE project_id = $1 AND revision IS NOT NULL ORDER BY created_at DESC, run_id").bind(pid).fetch_all(pool).await?;
            return Ok(rows.into_iter().map(|(run_id, revision, parent_run_id, at)| RunEntry { run_id, revision: revision as u32, parent_run_id, started_at: at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true) }).collect());
        }
        let (files, pid) = (self.files.clone(), project_id.to_owned());
        tokio::task::spawn_blocking(move || {
            let mut out: Vec<(std::time::SystemTime, RunEntry)> = Vec::new();
            let Ok(rd) = std::fs::read_dir(&files) else { return Vec::new() };
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) != Some("json") { continue; }
                let Ok(bytes) = std::fs::read(&path) else { continue };
                let Ok(spec) = serde_json::from_slice::<StoredSpec>(&bytes) else { continue };
                let Some(link) = spec.model.filter(|m| m.project_id == pid) else { continue };
                let Some(id) = path.file_stem().and_then(|x| x.to_str()) else { continue };
                let at = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                out.push((at, RunEntry { run_id: id.to_owned(), revision: link.revision, parent_run_id: spec.parent.map(|p| p.run_id), started_at: chrono::DateTime::<chrono::Utc>::from(at).to_rfc3339_opts(chrono::SecondsFormat::Millis, true) }));
            }
            out.sort_by(|a, b| b.0.cmp(&a.0));
            out.into_iter().map(|(_, e)| e).collect()
        }).await.map_err(|_| ApiError::Internal)
    }
}
