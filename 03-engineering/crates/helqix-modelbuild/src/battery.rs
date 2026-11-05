//! The translation from a committed model to a battery-cell run plan (E5.1). Pure functions, no I/O.
//!
//! A part with a `battery_cell` behaviour is what the engine runs through the PyBaMM adapter (the battery domain pack: SPM with the
//! Chen2020 parameters). The plan carries the pack's id, the cell's open-circuit voltage, contact resistance and discharge current converted to
//! coherent SI with the unit registry, and builds the adapter's input document for a run time grid. Nothing is guessed: a value in a unit of
//! the wrong dimension, outside the range the pack accepts, or not finite makes the model unrunnable, and the part is named with the reason.
//!
//! The pack is **NotVerified** (published held-out error 71.1 to 77.3 mV against the 46 mV target). That label belongs to every run of this plan; it
//! is attached by the caller from the pack's library entry, not decided here.
use helqix_contracts::{Behavior, Load, ModelOp, Value};
use helqix_core::units::Unit;
use serde_json::{json, Value as Json};

use crate::{lumped::Missing, Model};

/// The domain packs a model may name. One today.
pub const BATTERY_PACKS: [&str; 1] = ["lgm50_spm_chen2020"];

/// Range the pack accepts for the open-circuit voltage (V) and the contact resistance (Ohm): the limits of `helqix_battery.adapter_cli`.
pub const OCV_RANGE_V: (f64, f64) = (2.5, 4.25);
pub const RESISTANCE_RANGE_OHM: (f64, f64) = (0.0, 1.0);
/// A run is at most this many time samples (the solver cost grows with them).
pub const MAX_SAMPLES: usize = 20_000;

#[derive(Debug, Clone, PartialEq)]
pub struct BatteryPlan {
    pub part_id: String,
    pub part_name: String,
    pub pack: String,
    pub initial_ocv_v: f64,
    pub contact_resistance_ohm: f64,
    /// Positive discharges the cell.
    pub current_a: f64,
}

impl BatteryPlan {
    /// The pack's input document for a run of `end_s` seconds on a grid of `step_s`: the time grid and the constant current on it.
    pub fn adapter_input(&self, end_s: f64, step_s: f64) -> Result<Json, String> {
        if !(end_s.is_finite() && step_s.is_finite() && end_s > 0.0 && step_s > 0.0) { return Err("the duration and the step must be positive finite numbers".into()); }
        let n = (end_s / step_s).round();
        if n < 1.0 || n + 1.0 > MAX_SAMPLES as f64 { return Err(format!("a battery run has between 2 and {MAX_SAMPLES} time samples (duration / step)")); }
        let n = n as usize;
        let time: Vec<f64> = (0..=n).map(|i| i as f64 * step_s).collect();
        Ok(json!({
            "schema": "helqix.pybamm-input.v1", "time_s": time, "current_a": vec![self.current_a; n + 1],
            "initial_ocv_v": self.initial_ocv_v, "contact_resistance_ohm": self.contact_resistance_ohm,
        }))
    }
}

fn missing(part: &str, reason: impl Into<String>) -> Missing { Missing { part: part.to_owned(), reason: reason.into() } }

/// One thing wrong with a battery behaviour's values: a stable code (also the code of the blocking issue the model build raises), the field it is about, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem { pub code: &'static str, pub field: &'static str, pub message: String }

/// `value` in coherent SI, when its unit is registered and measures the same quantity as `expect_unit` (for example "V").
fn si(value: &Value, expect_unit: &str) -> Result<f64, (&'static str, String)> {
    let unit = Unit::parse(&value.unit).map_err(|_| ("unit.unknown", format!("unit '{}' is not a registered unit", value.unit)))?;
    let want = Unit::parse(expect_unit).expect("registered");
    if unit.dimension() != want.dimension() { return Err(("behavior.dimension", format!("unit '{}' does not measure the same quantity as {expect_unit}", value.unit))); }
    let x = value.v * unit.scale_to_coherent() / want.scale_to_coherent();
    if value.v.is_finite() && x.is_finite() { Ok(x) } else { Err(("behavior.value", "the value must be a finite number".into())) }
}

/// The SI values of a battery-cell behaviour (voltage V, resistance Ohm, current A), or what is wrong with them. Rules, not guesses.
fn values(b: &Behavior) -> Result<(f64, f64, f64), Vec<Problem>> {
    let Behavior::BatteryCell { initial_ocv, contact_resistance, load, .. } = b else { return Err(Vec::new()) };
    let Load::ConstantCurrent { current } = load;
    let mut bad = Vec::new();
    let mut field = |name: &'static str, r: Result<f64, (&'static str, String)>, range: Option<((f64, f64), &str)>| -> Option<f64> {
        match r {
            Err((code, message)) => { bad.push(Problem { code, field: name, message }); None }
            Ok(x) => {
                if let Some(((lo, hi), unit)) = range {
                    if !(lo..=hi).contains(&x) { bad.push(Problem { code: "behavior.range", field: name, message: format!("{x} {unit} is outside the {lo} to {hi} {unit} the pack accepts") }); return None; }
                }
                Some(x)
            }
        }
    };
    let ocv = field("initialOcv", si(initial_ocv, "V"), Some((OCV_RANGE_V, "V")));
    let r = match contact_resistance { None => Some(0.0), Some(c) => field("contactResistance", si(c, "ohm"), Some((RESISTANCE_RANGE_OHM, "Ohm"))) };
    let i = field("current", si(current, "A"), None);
    match (ocv, r, i) { (Some(o), Some(r), Some(i)) if bad.is_empty() => Ok((o, r, i)), _ => Err(bad) }
}

/// What is wrong with the values of a battery-cell behaviour (empty for other behaviours or good values). The model build raises each as a blocking issue.
pub fn problems(b: &Behavior) -> Vec<Problem> { values(b).err().unwrap_or_default() }

/// `ops` are the active operations of a committed revision, in order. `Ok(None)`: no part has a battery-cell behaviour.
pub fn battery_plan(ops: &[ModelOp]) -> Result<Option<BatteryPlan>, Vec<Missing>> {
    let model = Model::from_ops(ops).map_err(|e| vec![missing("", format!("the model does not replay: {e}"))])?;
    let cells: Vec<(&String, &crate::Part)> = model.parts.iter().filter(|(_, p)| matches!(p.behavior, Some(Behavior::BatteryCell { .. }))).collect();
    let label = |id: &str, name: &str| format!("{name} ({id})");
    match cells.as_slice() {
        [] => Ok(None),
        [(id, part)] => {
            let Some(b @ Behavior::BatteryCell { pack, .. }) = &part.behavior else { unreachable!("filtered on behavior") };
            let who = label(id, &part.name);
            let mut bad = Vec::new();
            if !BATTERY_PACKS.contains(&pack.as_str()) { bad.push(missing(&who, format!("the battery pack '{pack}' is not installed (known: {})", BATTERY_PACKS.join(", ")))); }
            let (ocv, r, i) = match values(b) {
                Ok(v) => v,
                Err(p) => { bad.extend(p.into_iter().map(|p| missing(&who, format!("{}: {}", field_label(p.field), p.message)))); (0.0, 0.0, 0.0) }
            };
            if !bad.is_empty() { bad.sort(); return Err(bad); }
            Ok(Some(BatteryPlan { part_id: (*id).clone(), part_name: part.name.clone(), pack: pack.clone(), initial_ocv_v: ocv, contact_resistance_ohm: r, current_a: i }))
        }
        many => {
            let mut v: Vec<Missing> = many.iter().map(|(id, p)| missing(&label(id, &p.name), "only one part with a battery-cell behaviour can run in a model today")).collect();
            v.sort();
            Err(v)
        }
    }
}

fn field_label(field: &str) -> &str {
    match field { "initialOcv" => "initial open-circuit voltage", "contactResistance" => "contact resistance", "current" => "discharge current", other => other }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_contracts::{Geometry, Maturity, Origin, Provenance, ProvenanceKind, Status};

    fn v(x: f64, unit: &str) -> Value { Value { v: x, unit: unit.into(), origin: Origin::UserStated, status: Status::Unverified, mock: false } }
    fn add(id: &str) -> ModelOp {
        ModelOp::PartAdd { id: id.into(), name: id.to_uppercase(), geometry: Geometry::Primitive { shape: "cylinder".into(), dims: vec![v(21.0, "mm"), v(70.0, "mm")] }, maturity: Maturity::UserDefined, prov: Provenance { kind: ProvenanceKind::UserStated, source_id: None, as_of: None, note: None } }
    }
    fn cell(id: &str, ocv: Value, r: Option<Value>, i: Value) -> ModelOp {
        ModelOp::PartSetBehavior { id: id.into(), behavior: Behavior::BatteryCell { pack: "lgm50_spm_chen2020".into(), initial_ocv: ocv, contact_resistance: r, load: Load::ConstantCurrent { current: i } } }
    }
    fn plan(ops: &[ModelOp]) -> BatteryPlan { battery_plan(ops).unwrap().unwrap() }
    fn reasons(ops: &[ModelOp]) -> Vec<String> { battery_plan(ops).unwrap_err().into_iter().map(|m| m.reason).collect() }

    #[test]
    fn a_model_without_a_battery_cell_is_not_a_battery_model() {
        assert_eq!(battery_plan(&[add("a")]), Ok(None));
        assert_eq!(battery_plan(&[]), Ok(None));
    }

    #[test]
    fn values_are_converted_to_coherent_si() {
        let p = plan(&[add("c"), cell("c", v(4180.0, "mV"), Some(v(0.015805, "ohm")), v(5000.0, "mA"))]);
        assert_eq!((p.part_id.as_str(), p.pack.as_str()), ("c", "lgm50_spm_chen2020"));
        assert!((p.initial_ocv_v - 4.18).abs() < 1e-12 && (p.current_a - 5.0).abs() < 1e-12 && (p.contact_resistance_ohm - 0.015805).abs() < 1e-12, "{p:?}");
    }

    #[test]
    fn no_stated_resistance_means_zero_ohm_not_a_guess_from_the_library() {
        assert_eq!(plan(&[add("c"), cell("c", v(4.18, "V"), None, v(5.0, "A"))]).contact_resistance_ohm, 0.0);
    }

    #[test]
    fn a_wrong_quantity_a_unknown_unit_or_a_non_finite_value_is_named() {
        let r = reasons(&[add("c"), cell("c", v(4.18, "A"), Some(v(0.01, "kg")), v(5.0, "furlongs"))]);
        assert_eq!(r.len(), 3, "{r:?}");
        assert!(r.iter().any(|x| x.contains("open-circuit voltage") && x.contains("same quantity")), "{r:?}");
        assert!(r.iter().any(|x| x.contains("contact resistance") && x.contains("same quantity")), "{r:?}");
        assert!(r.iter().any(|x| x.contains("discharge current") && x.contains("not a registered unit")), "{r:?}");
        assert!(reasons(&[add("c"), cell("c", v(4.18, "V"), None, v(f64::INFINITY, "A"))])[0].contains("finite"));
    }

    #[test]
    fn values_outside_what_the_pack_accepts_are_refused() {
        assert!(reasons(&[add("c"), cell("c", v(5.0, "V"), None, v(1.0, "A"))])[0].contains("outside"));
        assert!(reasons(&[add("c"), cell("c", v(2.4, "V"), None, v(1.0, "A"))])[0].contains("outside"));
        assert!(reasons(&[add("c"), cell("c", v(4.0, "V"), Some(v(2.0, "ohm")), v(1.0, "A"))])[0].contains("outside"));
        assert!(reasons(&[add("c"), cell("c", v(4.0, "V"), Some(v(-0.1, "ohm")), v(1.0, "A"))])[0].contains("outside"));
        assert!(battery_plan(&[add("c"), cell("c", v(4.25, "V"), Some(v(1.0, "ohm")), v(1.0, "A"))]).is_ok(), "the limits themselves are accepted");
    }

    #[test]
    fn an_unknown_pack_is_named() {
        let op = ModelOp::PartSetBehavior { id: "c".into(), behavior: Behavior::BatteryCell { pack: "mystery".into(), initial_ocv: v(4.0, "V"), contact_resistance: None, load: Load::ConstantCurrent { current: v(1.0, "A") } } };
        assert!(reasons(&[add("c"), op])[0].contains("'mystery' is not installed"));
    }

    #[test]
    fn two_battery_parts_are_refused_and_both_named() {
        let err = battery_plan(&[add("a"), add("b"), cell("a", v(4.0, "V"), None, v(1.0, "A")), cell("b", v(4.0, "V"), None, v(1.0, "A"))]).unwrap_err();
        assert_eq!(err.iter().map(|m| m.part.as_str()).collect::<Vec<_>>(), vec!["A (a)", "B (b)"]);
    }

    #[test]
    fn an_fmu_part_is_not_a_battery_model() {
        let fmu = ModelOp::PartSetBehavior { id: "a".into(), behavior: Behavior::Fmu { fmu_sha256: "a3".repeat(32), parameters: Default::default(), outputs: vec!["h".into()] } };
        assert_eq!(battery_plan(&[add("a"), fmu]), Ok(None));
    }

    #[test]
    fn the_adapter_input_is_a_time_grid_with_the_constant_current_on_it() {
        let p = plan(&[add("c"), cell("c", v(4.18, "V"), Some(v(0.0158, "ohm")), v(5.0, "A"))]);
        let d = p.adapter_input(90.0, 30.0).unwrap();
        assert_eq!(d, json!({"schema": "helqix.pybamm-input.v1", "time_s": [0.0, 30.0, 60.0, 90.0], "current_a": [5.0, 5.0, 5.0, 5.0], "initial_ocv_v": 4.18, "contact_resistance_ohm": 0.0158}));
        for bad in [(0.0, 30.0), (90.0, 0.0), (-1.0, 30.0), (f64::NAN, 30.0), (90.0, f64::INFINITY), (10.0, 30.0), (1.0e9, 1.0)] {
            assert!(p.adapter_input(bad.0, bad.1).is_err(), "{bad:?}");
        }
    }
}
