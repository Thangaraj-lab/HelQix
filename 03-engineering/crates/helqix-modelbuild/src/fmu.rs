//! The translation from a committed model to an FMU run plan (E4.2). Pure functions, no I/O.
//!
//! A part with an FMU behaviour (`part.setBehavior`) is what the engine runs through the FMI adapter. The plan carries the
//! FMU's SHA-256 (never a path: the caller finds the file by hash), the parameters converted to coherent SI with the unit
//! registry, the dimension each parameter measures, and the output variables to record. Nothing is guessed:
//! an unknown unit, a non-finite number, a bad time span, or more than one FMU part (no co-simulation master yet) makes the
//! model unrunnable, and each part is named with the reason.
//!
//! The plan says what dimension each parameter has. The FMI tool (`parameters_si`) reads the unit the FMU declares for that
//! variable, refuses the run when it measures another dimension, and converts the SI value to the FMU's unit (E4.3a).
//!
//! A model with no behaviour is not an error: [`fmu_plan`] returns `Ok(None)` and the caller uses `lumped::lumped_body`.
use std::collections::BTreeMap;

use helqix_contracts::{Behavior, ModelOp, Value};
use helqix_core::units::Unit;
use serde_json::{json, Value as Json};

use crate::{lumped::Missing, Model};

/// One parameter converted to coherent SI.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedParam {
    pub value_si: f64,
    /// Exponents of the dimension (length, mass, time, ...), as in the unit registry; all zero is dimensionless.
    pub dimension: [i8; 7],
    /// What the user stated, kept for the record.
    pub stated: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FmuPlan {
    pub part_id: String,
    pub part_name: String,
    pub fmu_sha256: String,
    pub parameters: BTreeMap<String, PlannedParam>,
    pub outputs: Vec<String>,
}

impl FmuPlan {
    /// The document for the FMI adapter's `run` (`parameters_si`, `start_time`, `stop_time`, `step_size`). Each parameter carries its SI value and its
    /// dimension, which the FMI tool checks against the unit the FMU declares and converts to it. It never names a file:
    /// the adapter fixes the FMU path itself and records the file's hash.
    pub fn adapter_input(&self, start_s: f64, stop_s: f64, step_s: f64) -> Result<Json, String> {
        if !(start_s.is_finite() && stop_s.is_finite() && step_s.is_finite()) { return Err("start, stop and step must be finite numbers".into()); }
        if !(stop_s > start_s) { return Err("the stop time must be after the start time".into()); }
        if !(step_s > 0.0) { return Err("the step must be positive".into()); }
        let params: serde_json::Map<String, Json> = self.parameters.iter().map(|(k, p)| (k.clone(), json!({"value_si": p.value_si, "dimension": p.dimension}))).collect();
        Ok(json!({ "parameters_si": params, "start_time": start_s, "stop_time": stop_s, "step_size": step_s }))
    }
}

fn missing(part: &str, reason: impl Into<String>) -> Missing { Missing { part: part.to_owned(), reason: reason.into() } }

/// `ops` are the active operations of a committed revision, in order. `Ok(None)`: no part has a behaviour.
pub fn fmu_plan(ops: &[ModelOp]) -> Result<Option<FmuPlan>, Vec<Missing>> {
    let model = Model::from_ops(ops).map_err(|e| vec![missing("", format!("the model does not replay: {e}"))])?;
    let with: Vec<(&String, &crate::Part)> = model.parts.iter().filter(|(_, p)| p.behavior.is_some()).collect();
    let label = |id: &str, name: &str| format!("{name} ({id})");
    match with.as_slice() {
        [] => Ok(None),
        [(id, part)] => {
            let Some(Behavior::Fmu { fmu_sha256, parameters, outputs }) = &part.behavior else { unreachable!("filtered on behavior") };
            let (mut planned, mut bad) = (BTreeMap::new(), Vec::new());
            for (name, stated) in parameters {
                let who = label(id, &part.name);
                let unit = match Unit::parse(&stated.unit) {
                    Ok(u) => u,
                    Err(_) => { bad.push(missing(&who, format!("parameter {name}: unit '{}' is not a registered unit", stated.unit))); continue; }
                };
                let value_si = stated.v * unit.scale_to_coherent();
                if !(stated.v.is_finite() && value_si.is_finite()) { bad.push(missing(&who, format!("parameter {name}: the value must be a finite number"))); continue; }
                planned.insert(name.clone(), PlannedParam { value_si, dimension: unit.dimension().exponents(), stated: stated.clone() });
            }
            if !bad.is_empty() { bad.sort(); return Err(bad); }
            Ok(Some(FmuPlan { part_id: (*id).clone(), part_name: part.name.clone(), fmu_sha256: fmu_sha256.clone(), parameters: planned, outputs: outputs.clone() }))
        }
        many => {
            let mut v: Vec<Missing> = many.iter().map(|(id, p)| missing(&label(id, &p.name), "only one part with an FMU behaviour can run in a model today (no co-simulation of several FMUs yet)")).collect();
            v.sort();
            Err(v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_contracts::{Geometry, Maturity, Origin, Provenance, ProvenanceKind, Status};

    const SHA: &str = "a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3";
    fn v(x: f64, unit: &str) -> Value { Value { v: x, unit: unit.into(), origin: Origin::UserStated, status: Status::Unverified, mock: false } }
    fn add(id: &str) -> ModelOp {
        ModelOp::PartAdd { id: id.into(), name: id.to_uppercase(), geometry: Geometry::Primitive { shape: "box".into(), dims: vec![v(100.0, "mm")] }, maturity: Maturity::UserDefined, prov: Provenance { kind: ProvenanceKind::UserStated, source_id: None, as_of: None, note: None } }
    }
    fn fmu(id: &str, params: &[(&str, f64, &str)], outputs: &[&str]) -> ModelOp {
        ModelOp::PartSetBehavior { id: id.into(), behavior: Behavior::Fmu { fmu_sha256: SHA.into(), parameters: params.iter().map(|(k, x, u)| ((*k).to_owned(), v(*x, u))).collect(), outputs: outputs.iter().map(|o| (*o).to_owned()).collect() } }
    }

    #[test]
    fn a_model_with_no_behaviour_is_not_an_fmu_model() {
        assert_eq!(fmu_plan(&[add("a")]), Ok(None));
        assert_eq!(fmu_plan(&[]), Ok(None));
    }

    #[test]
    fn parameters_are_converted_to_coherent_si_and_keep_what_was_stated() {
        let plan = fmu_plan(&[add("ball"), fmu("ball", &[("radius", 250.0, "mm"), ("m", 2.0, "kg"), ("e", 0.7, "one")], &["h", "v"])]).unwrap().unwrap();
        assert_eq!((plan.part_id.as_str(), plan.part_name.as_str(), plan.fmu_sha256.as_str()), ("ball", "BALL", SHA));
        assert_eq!(plan.outputs, vec!["h".to_owned(), "v".to_owned()]);
        assert!((plan.parameters["radius"].value_si - 0.25).abs() < 1e-12);
        assert_eq!(plan.parameters["radius"].dimension, [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(plan.parameters["radius"].stated, v(250.0, "mm"));
        assert_eq!(plan.parameters["m"].value_si, 2.0);
        assert_eq!(plan.parameters["m"].dimension, [0, 1, 0, 0, 0, 0, 0]);
        assert_eq!((plan.parameters["e"].value_si, plan.parameters["e"].dimension), (0.7, [0; 7]));
    }

    #[test]
    fn the_latest_behaviour_wins() {
        let plan = fmu_plan(&[add("ball"), fmu("ball", &[("e", 0.5, "one")], &["h"]), fmu("ball", &[("e", 0.9, "one")], &["v"])]).unwrap().unwrap();
        assert_eq!((plan.parameters["e"].value_si, plan.outputs), (0.9, vec!["v".to_owned()]));
    }

    #[test]
    fn a_removed_part_no_longer_makes_an_fmu_model() {
        assert_eq!(fmu_plan(&[add("ball"), fmu("ball", &[], &["h"]), ModelOp::PartRemove { id: "ball".into() }]), Ok(None));
    }

    #[test]
    fn two_fmu_parts_are_refused_and_both_are_named() {
        let err = fmu_plan(&[add("a"), add("b"), fmu("a", &[], &["x"]), fmu("b", &[], &["x"])]).unwrap_err();
        assert_eq!(err.iter().map(|m| m.part.as_str()).collect::<Vec<_>>(), vec!["A (a)", "B (b)"]);
        assert!(err[0].reason.contains("only one"), "{err:?}");
    }

    #[test]
    fn an_unknown_unit_or_a_non_finite_value_is_named_never_guessed() {
        let err = fmu_plan(&[add("ball"), fmu("ball", &[("radius", 1.0, "furlongs")], &["h"])]).unwrap_err();
        assert_eq!(err, vec![Missing { part: "BALL (ball)".into(), reason: "parameter radius: unit 'furlongs' is not a registered unit".into() }]);
        let err = fmu_plan(&[add("ball"), ModelOp::PartSetBehavior { id: "ball".into(), behavior: Behavior::Fmu { fmu_sha256: SHA.into(), parameters: [("k".to_owned(), v(f64::INFINITY, "kg"))].into(), outputs: vec!["h".into()] } }]).unwrap_err();
        assert!(err[0].reason.contains("finite"), "{err:?}");
        let err = fmu_plan(&[add("ball"), fmu("ball", &[("k", f64::MAX, "km")], &["h"])]).unwrap_err();
        assert!(err[0].reason.contains("finite"), "a value that overflows when converted to SI is refused: {err:?}");
    }

    #[test]
    fn ops_that_do_not_replay_are_refused() {
        let err = fmu_plan(&[fmu("ghost", &[], &["h"])]).unwrap_err();
        assert!(err[0].reason.contains("does not replay"), "{err:?}");
    }

    #[test]
    fn the_adapter_input_has_si_numbers_times_and_no_file_name() {
        let plan = fmu_plan(&[add("ball"), fmu("ball", &[("radius", 250.0, "mm")], &["h"])]).unwrap().unwrap();
        let doc = plan.adapter_input(0.0, 3.0, 0.1).unwrap();
        assert_eq!(doc, json!({"parameters_si": {"radius": {"value_si": 0.25, "dimension": [1, 0, 0, 0, 0, 0, 0]}}, "start_time": 0.0, "stop_time": 3.0, "step_size": 0.1}));
        for bad in [(0.0, 0.0, 0.1), (1.0, 0.5, 0.1), (0.0, 1.0, 0.0), (0.0, 1.0, -1.0), (0.0, f64::NAN, 0.1), (f64::NEG_INFINITY, 1.0, 0.1)] {
            assert!(plan.adapter_input(bad.0, bad.1, bad.2).is_err(), "{bad:?}");
        }
    }
}
