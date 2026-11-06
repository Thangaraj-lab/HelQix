//! Storing the series an external tool produced as a verified frame run: one sample per time, the variables as exact f64, and the state chain
//! that the frame store re-verifies on every read. Shared by every run that does not come from the built-in engine (FMUs, domain packs).
use std::path::Path;

use helqix_core::determinism::DeterminismLevel;
use helqix_frames::{FrameStore, RunIndex, Variable};
use helqix_freeze::{RecordedRun, Sample, chain_link};

use crate::{error::ApiError, frames::map};

/// One recorded variable: name, SI unit and a value per time.
pub(crate) struct Series { pub name: String, pub unit: String, pub values: Vec<f64> }

/// Stores `series` against `time_s` under `run_id`. `genesis` is the chain's first link (a hash of what the run was made from). Every series must
/// have one value per time, and there must be at least one time.
pub(crate) fn store(dir: &Path, run_id: &str, genesis: &str, time_s: &[f64], series: &[Series], determinism: DeterminismLevel) -> Result<RunIndex, ApiError> {
    let rows = time_s.len();
    if rows == 0 { return Err(ApiError::Rejected("the run returned no samples".into())); }
    if series.iter().any(|s| s.values.len() != rows) { return Err(ApiError::Rejected("the run returned series of different lengths".into())); }
    let mut samples = Vec::with_capacity(rows);
    let mut chain: Vec<String> = Vec::with_capacity(rows);
    for (i, t) in time_s.iter().enumerate() {
        let state: Vec<f64> = series.iter().map(|s| s.values[i]).collect();
        let link = chain_link(chain.last().map_or(genesis, String::as_str), i, *t, &state);
        samples.push(Sample { index: i, time_seconds: *t, state });
        chain.push(link);
    }
    let recorded = RecordedRun { genesis: genesis.to_owned(), samples, chain, determinism };
    let variables: Vec<Variable> = series.iter().map(|s| Variable { name: s.name.clone(), unit: s.unit.clone() }).collect();
    FrameStore::open(dir).and_then(|s| s.put(run_id, &recorded, &variables, genesis)).map_err(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("helqix-recorded-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_run_is_stored_and_reads_back_verified() {
        let d = dir("ok");
        let level = DeterminismLevel::tolerance(1e-9).unwrap();
        let s = [Series { name: "v".into(), unit: "V".into(), values: vec![4.0, 3.9, 3.8] }];
        let idx = store(&d, "run-t", "genesis", &[0.0, 1.0, 2.0], &s, level).unwrap();
        assert_eq!(idx.meta.rows, 3);
        let loaded = FrameStore::open(&d).unwrap().load("run-t").unwrap();
        assert_eq!(loaded.frames().len(), 3);
        assert_eq!(loaded.frames()[1].state, vec![3.9]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn no_samples_or_uneven_series_are_refused() {
        let d = dir("bad");
        let level = || DeterminismLevel::tolerance(1e-9).unwrap();
        assert!(store(&d, "r", "g", &[], &[], level()).is_err());
        let uneven = [Series { name: "v".into(), unit: "V".into(), values: vec![1.0] }];
        assert!(store(&d, "r", "g", &[0.0, 1.0], &uneven, level()).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
