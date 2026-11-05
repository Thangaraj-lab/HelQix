//! Which run a committed model gets: the built-in point mass, an FMU, or a battery cell (E5.1). A model has at most one behaviour: the engine runs
//! one thing per model today, and a model that names two (an FMU part and a battery part, or two parts of one kind) is refused with each part named.
use helqix_contracts::ModelOp;

use crate::{battery::{battery_plan, BatteryPlan}, fmu::{fmu_plan, FmuPlan}, lumped::Missing, Model};

#[derive(Debug, Clone, PartialEq)]
pub enum RunPlan {
    /// No part has a behaviour: run the parts as one lumped body.
    PointMass,
    Fmu(FmuPlan),
    Battery(BatteryPlan),
}

/// `ops` are the active operations of a committed revision, in order.
pub fn run_plan(ops: &[ModelOp]) -> Result<RunPlan, Vec<Missing>> {
    let model = Model::from_ops(ops).map_err(|e| vec![Missing { part: String::new(), reason: format!("the model does not replay: {e}") }])?;
    let with: Vec<(&String, &crate::Part)> = model.parts.iter().filter(|(_, p)| p.behavior.is_some()).collect();
    if with.len() > 1 {
        let mut v: Vec<Missing> = with.iter().map(|(id, p)| Missing { part: format!("{} ({id})", p.name), reason: "only one part with a behaviour (an FMU or a battery cell) can run in a model today".into() }).collect();
        v.sort();
        return Err(v);
    }
    if let Some(plan) = fmu_plan(ops)? { return Ok(RunPlan::Fmu(plan)); }
    if let Some(plan) = battery_plan(ops)? { return Ok(RunPlan::Battery(plan)); }
    Ok(RunPlan::PointMass)
}

#[cfg(test)]
mod tests {
    use super::*;
    use helqix_contracts::{Behavior, Geometry, Load, Maturity, Origin, Provenance, ProvenanceKind, Status, Value};

    fn v(x: f64, unit: &str) -> Value { Value { v: x, unit: unit.into(), origin: Origin::UserStated, status: Status::Unverified, mock: false } }
    fn add(id: &str) -> ModelOp {
        ModelOp::PartAdd { id: id.into(), name: id.to_uppercase(), geometry: Geometry::Primitive { shape: "box".into(), dims: vec![v(100.0, "mm")] }, maturity: Maturity::UserDefined, prov: Provenance { kind: ProvenanceKind::UserStated, source_id: None, as_of: None, note: None } }
    }
    fn fmu(id: &str) -> ModelOp { ModelOp::PartSetBehavior { id: id.into(), behavior: Behavior::Fmu { fmu_sha256: "a3".repeat(32), parameters: Default::default(), outputs: vec!["h".into()] } } }
    fn cell(id: &str) -> ModelOp {
        ModelOp::PartSetBehavior { id: id.into(), behavior: Behavior::BatteryCell { pack: "lgm50_spm_chen2020".into(), initial_ocv: v(4.18, "V"), contact_resistance: None, load: Load::ConstantCurrent { current: v(5.0, "A") } } }
    }

    #[test]
    fn no_behaviour_is_a_point_mass() {
        assert_eq!(run_plan(&[add("a")]), Ok(RunPlan::PointMass));
        assert_eq!(run_plan(&[]), Ok(RunPlan::PointMass));
    }

    #[test]
    fn each_kind_of_behaviour_gets_its_own_plan() {
        assert!(matches!(run_plan(&[add("a"), fmu("a")]), Ok(RunPlan::Fmu(_))));
        assert!(matches!(run_plan(&[add("a"), cell("a")]), Ok(RunPlan::Battery(_))));
    }

    #[test]
    fn two_behaviours_are_refused_whatever_their_kinds_and_both_parts_are_named() {
        for ops in [vec![add("a"), add("b"), fmu("a"), cell("b")], vec![add("a"), add("b"), cell("a"), cell("b")], vec![add("a"), add("b"), fmu("a"), fmu("b")]] {
            let err = run_plan(&ops).unwrap_err();
            assert_eq!(err.iter().map(|m| m.part.as_str()).collect::<Vec<_>>(), vec!["A (a)", "B (b)"], "{ops:?}");
            assert!(err[0].reason.contains("only one"), "{err:?}");
        }
    }

    #[test]
    fn the_plan_of_the_one_behaviour_still_refuses_bad_values() {
        let bad = ModelOp::PartSetBehavior { id: "a".into(), behavior: Behavior::BatteryCell { pack: "lgm50_spm_chen2020".into(), initial_ocv: v(9.0, "V"), contact_resistance: None, load: Load::ConstantCurrent { current: v(5.0, "A") } } };
        assert!(run_plan(&[add("a"), bad]).unwrap_err()[0].reason.contains("outside"));
    }
}
