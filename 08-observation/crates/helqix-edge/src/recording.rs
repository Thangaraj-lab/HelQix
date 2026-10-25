//! A recording keeps the raw lines exactly as received, with their receive times, plus the sensor
//! specs and ingest settings. It is content-addressed (SHA-256 of its bytes) and re-feeding it
//! gives the same accepted values and the same rejections.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ingest::{IngestConfig, Ingestor, SensorSpec};

pub const SCHEMA: &str = "helqix.edge-recording.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry { pub receive_time_s: f64, pub line: String }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recording { pub schema: String, pub sensors: Vec<SensorSpec>, pub config: IngestConfig, pub entries: Vec<Entry> }

impl Recording {
    pub fn new(sensors: Vec<SensorSpec>, config: IngestConfig) -> Self { Self { schema: SCHEMA.into(), sensors, config, entries: Vec::new() } }
    pub fn push(&mut self, receive_time_s: f64, line: &str) { self.entries.push(Entry { receive_time_s, line: line.into() }); }
    pub fn to_bytes(&self) -> Vec<u8> { serde_json::to_vec_pretty(self).expect("recording serializes") }
    pub fn sha256(&self) -> String { format!("{:x}", Sha256::digest(self.to_bytes())) }
    pub fn from_bytes(bytes: &[u8], expected_sha256: Option<&str>) -> Result<Self, String> {
        if let Some(h) = expected_sha256 { if format!("{:x}", Sha256::digest(bytes)) != h { return Err("recording bytes do not match the expected SHA-256".into()); } }
        let r: Self = serde_json::from_slice(bytes).map_err(|e| format!("not a recording: {e}"))?;
        if r.schema != SCHEMA { return Err(format!("schema must be {SCHEMA}")); }
        Ok(r)
    }
    /// Re-feeds every line, in recorded order, into a fresh ingestor.
    pub fn replay(&self) -> Ingestor {
        let mut ing = Ingestor::new(self.sensors.clone(), self.config);
        for e in &self.entries { let _ = ing.ingest_line(&e.line, e.receive_time_s); }
        ing
    }
}
