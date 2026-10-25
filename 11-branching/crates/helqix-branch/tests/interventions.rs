//! Phase 4 required tests (docs/plan/16): isolation, determinism, intervention verdicts,
//! common driver, compare alignment, budget, dose-response.
use helqix_branch::intervention::{
    compare_runs, dose_response, intervention_test, Budget, ExperimentError, Experiments, Intervention, Parameter, RequirementStatus, Verdict,
};
use helqix_core::id::Identifier;
use helqix_freeze::{freeze, run, Comparator, FreezeTrigger, PointMassSpec, RunSpec, StateLimit};

fn id(v: &str) -> Identifier { Identifier::parse(v).unwrap() }

/// Body dropped from 20 m with a small horizontal velocity; it hits the ground near t = 2 s.
fn spec() -> RunSpec {
    RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 4.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    }
}
fn ground() -> StateLimit { StateLimit { name: "ground".into(), variable: 1, comparator: Comparator::AtLeast, threshold: 0.0 } }
fn budget() -> Budget { Budget { max_branches: 20, max_total_steps: 200_000 } }
const TOL_S: f64 = 0.01;

#[test]
fn a_branch_never_changes_its_parent() {
    let s = spec();
    let parent = run(&s).unwrap();
    let before = parent.chain.clone();
    let frozen = freeze(&s, &parent, 500, FreezeTrigger::TimeMarker(0.5)).unwrap();
    let mut ex = Experiments::new(budget());
    ex.fork(id("b1"), &frozen, Intervention::Parameter { parameter: Parameter::DragCoefficient, value: 1.2 }).unwrap();
    assert_eq!(parent.chain, before);
    assert_eq!(run(&s).unwrap().chain, before);
}

#[test]
fn a_branch_records_its_lineage_and_starts_from_the_frozen_state() {
    let s = spec();
    let parent = run(&s).unwrap();
    let frozen = freeze(&s, &parent, 500, FreezeTrigger::TimeMarker(0.5)).unwrap();
    let mut ex = Experiments::new(budget());
    let b = ex.fork(id("b1"), &frozen, Intervention::Parameter { parameter: Parameter::DragCoefficient, value: 1.2 }).unwrap();
    assert_eq!(b.parent_freeze_sha256, frozen.sha256());
    assert_eq!(b.fork_index, 500);
    assert_eq!(b.run.samples[0].state, parent.samples[500].state);
}

#[test]
fn the_same_intervention_twice_gives_identical_results() {
    let s = spec();
    let frozen = freeze(&s, &run(&s).unwrap(), 300, FreezeTrigger::Manual).unwrap();
    let mut ex = Experiments::new(budget());
    let a = ex.fork(id("a"), &frozen, Intervention::State { variable: 1, value: 25.0 }).unwrap().run.chain.clone();
    let b = ex.fork(id("b"), &frozen, Intervention::State { variable: 1, value: 25.0 }).unwrap().run.chain.clone();
    assert_eq!(a, b);
}

#[test]
fn intervening_on_the_true_cause_supports_it_and_on_a_non_cause_does_not() {
    let s = spec();
    let parent = run(&s).unwrap();
    let frozen = freeze(&s, &parent, 0, FreezeTrigger::Manual).unwrap();
    let mut ex = Experiments::new(budget());
    // True cause of reaching the ground by t: the height the body falls from.
    let cause = intervention_test(&mut ex, &parent, &frozen, &ground(), id("raise_height"), Intervention::State { variable: 1, value: 30.0 }, TOL_S).unwrap();
    assert_eq!(cause.verdict, Verdict::InterventionSupported);
    assert_eq!(cause.grade, "model-causal");
    // Not a cause: the out-of-plane position.
    let non_cause = intervention_test(&mut ex, &parent, &frozen, &ground(), id("move_z"), Intervention::State { variable: 2, value: 5.0 }, TOL_S).unwrap();
    assert_eq!(non_cause.verdict, Verdict::NotSupported);
}

#[test]
fn intervening_on_an_effect_of_a_shared_driver_is_not_credited() {
    // Horizontal position is an effect of the horizontal velocity, which shares the drag coupling
    // with the vertical motion; changing the position itself must not change when the body lands.
    let s = spec();
    let parent = run(&s).unwrap();
    let frozen = freeze(&s, &parent, 0, FreezeTrigger::Manual).unwrap();
    let mut ex = Experiments::new(budget());
    let r = intervention_test(&mut ex, &parent, &frozen, &ground(), id("move_x"), Intervention::State { variable: 0, value: 100.0 }, TOL_S).unwrap();
    assert_eq!(r.verdict, Verdict::NotSupported);
}

#[test]
fn compare_aligns_only_identical_times_and_reports_requirement_status() {
    let s = spec();
    let parent = run(&s).unwrap();
    let mut coarse = spec();
    coarse.step_seconds = 0.002;
    coarse.end_seconds = 3.0;
    let other = run(&coarse).unwrap();
    let report = compare_runs(&parent, &other, &[(1, "height".to_owned()), (9, "missing".to_owned())], &ground());
    assert!(report.aligned_samples > 0);
    assert!(report.unmatched_parent > 0);
    assert_eq!(report.missing_variables, vec!["missing".to_owned()]);
    assert!(matches!(report.parent_status, RequirementStatus::Violated { .. }));
    assert!(report.alignment_rule.contains("identical"));
}

#[test]
fn compare_of_a_run_with_itself_has_no_divergence() {
    let parent = run(&spec()).unwrap();
    let report = compare_runs(&parent, &parent, &[(1, "height".to_owned())], &ground());
    assert!(report.first_divergence_seconds.is_none());
    assert_eq!(report.max_abs_delta[0].1, 0.0);
}

#[test]
fn branch_budget_is_enforced() {
    let s = spec();
    let frozen = freeze(&s, &run(&s).unwrap(), 0, FreezeTrigger::Manual).unwrap();
    let mut ex = Experiments::new(Budget { max_branches: 2, max_total_steps: 1_000_000 });
    ex.fork(id("a"), &frozen, Intervention::Parameter { parameter: Parameter::MassKg, value: 0.6 }).unwrap();
    ex.fork(id("b"), &frozen, Intervention::Parameter { parameter: Parameter::MassKg, value: 0.7 }).unwrap();
    assert!(matches!(ex.fork(id("c"), &frozen, Intervention::Parameter { parameter: Parameter::MassKg, value: 0.8 }), Err(ExperimentError::BudgetExceeded { .. })));
    let mut tight = Experiments::new(Budget { max_branches: 10, max_total_steps: 100 });
    assert!(matches!(tight.fork(id("a"), &frozen, Intervention::Parameter { parameter: Parameter::MassKg, value: 0.6 }), Err(ExperimentError::BudgetExceeded { .. })));
}

#[test]
fn dose_response_detects_monotonic_and_non_monotonic_effects() {
    let s = spec();
    let parent = run(&s).unwrap();
    let frozen = freeze(&s, &parent, 0, FreezeTrigger::Manual).unwrap();
    let mut ex = Experiments::new(budget());
    let levels = [22.0, 25.0, 28.0].map(|h| Intervention::State { variable: 1, value: h });
    let up = dose_response(&mut ex, &frozen, &ground(), "dose", &levels).unwrap();
    assert!(up.monotonic, "{:?}", up.failure_times_seconds);
    let shuffled = [25.0, 22.0, 28.0].map(|h| Intervention::State { variable: 1, value: h });
    let mixed = dose_response(&mut ex, &frozen, &ground(), "mixed", &shuffled).unwrap();
    assert!(!mixed.monotonic);
}

#[test]
fn invalid_interventions_are_rejected() {
    let s = spec();
    let frozen = freeze(&s, &run(&s).unwrap(), 0, FreezeTrigger::Manual).unwrap();
    let mut ex = Experiments::new(budget());
    assert!(ex.fork(id("a"), &frozen, Intervention::State { variable: 99, value: 1.0 }).is_err());
    assert!(ex.fork(id("b"), &frozen, Intervention::Parameter { parameter: Parameter::MassKg, value: f64::NAN }).is_err());
    assert!(ex.fork(id("b2"), &frozen, Intervention::Parameter { parameter: Parameter::MassKg, value: -1.0 }).is_err());
}
