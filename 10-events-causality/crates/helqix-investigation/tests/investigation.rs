//! Phase 3 tests: failure definition, event log, click-through contract, replay link.
use helqix_causal::CandidateMethod;
use helqix_core::id::Identifier;
use helqix_freeze::{detect_failure, freeze, load_freeze, run, save_freeze, Comparator as FreezeComparator, FreezeTrigger, PointMassSpec, RunSpec, StateLimit};
use helqix_investigation::{investigate, Comparator, InvestigationInput, NodeSeries, Relation, Requirement, StateReference};

fn id(v: &str) -> Identifier { Identifier::parse(v).unwrap() }
const METHOD: CandidateMethod = CandidateMethod { min_relative_change: 0.05, onset_fraction: 0.1, path_limit: 64 };

fn synthetic() -> InvestigationInput {
    InvestigationInput {
        times_seconds: vec![0., 1., 2., 3., 4., 5., 6., 7.],
        nodes: vec![
            NodeSeries { id: id("load"), values: vec![1., 1., 2., 2., 2., 2., 2., 2.] },
            NodeSeries { id: id("current"), values: vec![0., 0., 0., 0., 3., 3., 3., 3.] },
            NodeSeries { id: id("temperature"), values: vec![25., 25., 25., 25., 25., 25., 90., 95.] },
        ],
        relations: vec![
            Relation { cause: id("load"), effect: id("current"), relation: id("eq_current"), parameters: vec![("gain".into(), 1.5)] },
            Relation { cause: id("current"), effect: id("temperature"), relation: id("eq_heat"), parameters: vec![("resistance".into(), 0.2)] },
        ],
        requirement: Requirement { id: id("r_temp"), node: id("temperature"), comparator: Comparator::AtMost, threshold: 80.0 },
        state_reference: None,
        method: METHOD,
    }
}

#[test]
fn failure_comes_from_the_requirement_not_from_code() {
    let inv = investigate(&synthetic()).unwrap();
    let f = inv.failure().expect("temperature exceeds 80");
    assert_eq!(f.requirement, id("r_temp"));
    assert_eq!(f.index, 6);
    assert_eq!(f.time_seconds, 6.0);
    assert_eq!(f.observed_value, 90.0);
    assert_eq!(f.threshold, 80.0);
}

#[test]
fn no_failure_no_chain() {
    let mut input = synthetic();
    input.requirement.threshold = 1000.0;
    let inv = investigate(&input).unwrap();
    assert!(inv.failure().is_none());
    assert!(inv.chains().is_empty());
}

#[test]
fn every_node_on_the_chain_has_the_full_click_through_detail() {
    let inv = investigate(&synthetic()).unwrap();
    let chain = &inv.chains()[0];
    assert_eq!(chain.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["load", "current", "temperature"]);
    for node in &chain.nodes {
        let d = inv.node_detail(&node.id).expect("detail for every chain node");
        assert_eq!(d.signal, node.id);
        assert!(d.value_at_failure.is_finite() && d.previous_value.is_finite() && d.baseline_value.is_finite());
        assert_eq!(d.grade, "candidate (structural + temporal)");
        assert!(!d.related_events.is_empty(), "node {} should have an onset event", node.id.as_str());
        assert!(d.branch_experiments.is_empty());
    }
    let current = inv.node_detail(&id("current")).unwrap();
    assert_eq!(current.relation, Some(id("eq_current")));
    assert_eq!(current.parameters, vec![("gain".to_owned(), 1.5)]);
    assert_eq!(current.previous_value, 3.0);
    assert_eq!(current.value_at_failure, 3.0);
    let root = inv.node_detail(&id("load")).unwrap();
    assert_eq!(root.relation, None);
}

#[test]
fn the_event_log_is_ordered_and_ends_with_the_failure() {
    let inv = investigate(&synthetic()).unwrap();
    let events = inv.events().events();
    assert!(events.windows(2).all(|w| w[0].at <= w[1].at));
    assert_eq!(events.last().unwrap().event_type, id("failure"));
}

#[test]
fn unknown_nodes_and_mismatched_lengths_are_rejected() {
    let mut bad = synthetic();
    bad.relations[0].cause = id("ghost");
    assert!(investigate(&bad).is_err());
    let mut short = synthetic();
    short.nodes[0].values.pop();
    assert!(investigate(&short).is_err());
}

fn reference(initial_state: Vec<f64>) -> RunSpec {
    RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 3.0, step_seconds: 0.001, initial_state, seeds: vec![],
    }
}

fn point_mass_input(recorded: &helqix_freeze::RecordedRun, state_reference: Option<StateReference>) -> InvestigationInput {
    let series = |k: usize| recorded.samples.iter().map(|s| s.state[k]).collect::<Vec<_>>();
    InvestigationInput {
        times_seconds: recorded.samples.iter().map(|s| s.time_seconds).collect(),
        nodes: vec![NodeSeries { id: id("velocity_x"), values: series(3) }, NodeSeries { id: id("velocity_y"), values: series(4) }, NodeSeries { id: id("height"), values: series(1) }],
        relations: vec![
            Relation { cause: id("velocity_x"), effect: id("velocity_y"), relation: id("quadratic_drag"), parameters: vec![("drag_coefficient".into(), 0.47), ("reference_area_m2".into(), 0.01), ("air_density_kg_m3".into(), 1.225), ("mass_kg".into(), 0.5)] },
            Relation { cause: id("velocity_y"), effect: id("height"), relation: id("kinematics_y"), parameters: vec![] },
        ],
        requirement: Requirement { id: id("stay_above_ground"), node: id("height"), comparator: Comparator::AtLeast, threshold: 0.0 },
        state_reference,
        method: METHOD,
    }
}

/// KNOWN LIMITATION (documented, not hidden): for a body launched upward, height starts changing
/// at once because the vertical velocity is already non-zero, while that velocity only starts
/// *changing* later. The onset rule ("a cause changes before its effect") does not fit an
/// integrated quantity driven by a cause with a non-zero initial value, so no chain is found.
#[test]
fn known_limitation_launched_body_yields_no_candidate_chain() {
    let spec = reference(vec![0.0, 0.0, 0.0, 12.0, 9.0, 0.0]);
    let inv = investigate(&point_mass_input(&run(&spec).unwrap(), None)).unwrap();
    assert!(inv.failure().is_some());
    assert!(inv.chains().is_empty());
}

/// Reference scenario on the core runtime (Phase 2): a body dropped from 20 m hits the ground;
/// the state reference opens the exact frozen state.
#[test]
fn replay_linked_reference_scenario() {
    let spec = RunSpec {
        model: PointMassSpec { mass_kg: 0.5, gravity_m_s2: [0.0, -9.80665, 0.0], air_density_kg_m3: 1.225, wind_m_s: [0.0; 3], applied_force_n: [0.0; 3], drag_coefficient: 0.47, reference_area_m2: 0.01 },
        start_seconds: 0.0, end_seconds: 3.0, step_seconds: 0.001, initial_state: vec![0.0, 20.0, 0.0, 3.0, 0.0, 0.0], seeds: vec![],
    };
    let recorded = run(&spec).unwrap();
    let limit = StateLimit { name: "ground".into(), variable: 1, comparator: FreezeComparator::AtLeast, threshold: 0.0 };
    let failure = detect_failure(&recorded, &limit).unwrap();
    let frozen = freeze(&spec, &recorded, failure.index, FreezeTrigger::Failure(failure.clone())).unwrap();
    let dir = std::env::temp_dir().join(format!("helqix-inv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("freeze.json");
    save_freeze(&frozen, &path).unwrap();

    let input = point_mass_input(&recorded, Some(StateReference { freeze_sha256: frozen.sha256(), index: failure.index }));
    let inv = investigate(&input).unwrap();
    assert_eq!(inv.failure().unwrap().index, failure.index);
    let chain = &inv.chains()[0];
    assert_eq!(chain.nodes.last().unwrap().id, id("height"));
    assert!(chain.nodes.iter().any(|n| n.id == id("velocity_y")));
    let reference = inv.node_detail(&id("height")).unwrap().state_reference.clone().unwrap();
    let opened = load_freeze(&path).unwrap();
    assert_eq!(opened.sha256(), reference.freeze_sha256);
    assert_eq!(opened.logical_time().0, reference.index);
    std::fs::remove_dir_all(&dir).unwrap();
}
