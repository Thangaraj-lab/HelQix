//! K2 engine-logic tests: event order equals op order, undo removes exactly one op, a stored log
//! restores the same build, blocking issues stop a commit, and the scratch path works with no resolver.
use helqix_contracts::{BuildEventKind, Direction, Geometry, Maturity, ModelOp, Origin, PortRef, Provenance, ProvenanceKind, Severity, Status, Value};
use helqix_modelbuild::{BuildError, OpError, StagedBuild};

fn val(v: f64, unit: &str) -> Value { Value { v, unit: unit.into(), origin: Origin::UserStated, status: Status::Unverified, mock: false } }
fn prov() -> Provenance { Provenance { kind: ProvenanceKind::UserStated, source_id: None, as_of: None, note: None } }
fn part(id: &str, maturity: Maturity) -> ModelOp {
    ModelOp::PartAdd { id: id.into(), name: id.to_uppercase(), geometry: Geometry::Primitive { shape: "box".into(), dims: vec![val(0.1, "m")] }, maturity, prov: prov() }
}
fn port(part: &str, id: &str, kind: &str, direction: Direction, unit: &str) -> ModelOp {
    ModelOp::PortAdd { part: part.into(), id: id.into(), kind: kind.into(), direction, unit: Some(unit.into()) }
}
fn conn(id: &str, a: (&str, &str), b: (&str, &str), kind: &str) -> ModelOp {
    ModelOp::ConnectionAdd { id: id.into(), from: PortRef { part: a.0.into(), port: a.1.into() }, to: PortRef { part: b.0.into(), port: b.1.into() }, kind: kind.into() }
}

/// Scratch path B from file 35: three primitive parts, ports with units, one connection.
fn scratch_ops() -> Vec<ModelOp> {
    vec![
        part("base", Maturity::UserDefined), part("body", Maturity::UserDefined), part("spinner", Maturity::UserDefined),
        ModelOp::PartSetParam { id: "body".into(), key: "mass".into(), value: val(2.0, "kg") },
        port("base", "supply", "electrical", Direction::Out, "V"),
        port("body", "supply_in", "electrical", Direction::In, "mV"),
        conn("c1", ("base", "supply"), ("body", "supply_in"), "electrical"),
    ]
}

#[test]
fn applied_events_follow_op_order_exactly() {
    let mut b = StagedBuild::new();
    let ops = scratch_ops();
    let ev = b.apply(&ops).unwrap();
    let applied: Vec<&ModelOp> = ev.iter().filter(|e| e.kind == BuildEventKind::OpApplied).map(|e| e.op.as_ref().unwrap()).collect();
    assert_eq!(applied, ops.iter().collect::<Vec<_>>());
    assert_eq!(b.active_ops(), ops.as_slice());
    assert_eq!(ev.last().unwrap().kind, BuildEventKind::StagedReady, "V to mV is the same quantity: no blocking issue");
}

#[test]
fn undo_removes_exactly_one_op_and_redo_restores_it() {
    let mut b = StagedBuild::new();
    b.apply(&scratch_ops()).unwrap();
    let n = b.active_ops().len();
    let ev = b.undo().unwrap();
    assert_eq!(b.active_ops().len(), n - 1);
    assert_eq!(ev[0].kind, BuildEventKind::OpUndone);
    assert_eq!(ev[0].op.as_ref(), scratch_ops().last());
    assert!(!b.model().connections.contains_key("c1"));
    b.redo().unwrap();
    assert_eq!(b.active_ops(), scratch_ops().as_slice());
    b.undo().unwrap();
    b.apply(&[part("extra", Maturity::UserDefined)]).unwrap();
    assert!(matches!(b.redo(), Err(BuildError::NothingToRedo)), "a new op after undo drops the redo tail");
}

#[test]
fn a_stored_log_restores_the_identical_build() {
    let mut b = StagedBuild::new();
    b.apply(&scratch_ops()).unwrap();
    b.undo().unwrap();
    let restored = StagedBuild::restore(b.ops().to_vec(), b.cursor(), false).unwrap();
    assert_eq!(restored, b);
    assert_eq!(restored.model(), b.model());
    assert_eq!(restored.issues(), b.issues());
}

#[test]
fn invalid_ops_are_rejected_atomically() {
    let mut b = StagedBuild::new();
    b.apply(&scratch_ops()).unwrap();
    let before = b.clone();
    let r = b.apply(&[part("new", Maturity::UserDefined), conn("c2", ("new", "nope"), ("body", "supply_in"), "electrical")]);
    assert_eq!(r, Err(BuildError::Op { index: 1, error: OpError::UnknownPort("new.nope".into()) }));
    assert_eq!(b, before, "nothing from a rejected batch is applied");
    assert!(matches!(b.apply(&[part("body", Maturity::UserDefined)]), Err(BuildError::Op { error: OpError::DuplicateId(_), .. })));
    assert!(matches!(b.apply(&[ModelOp::PartRemove { id: "base".into() }]), Err(BuildError::Op { error: OpError::InUse { .. }, .. })));
}

#[test]
fn a_unit_mismatch_blocks_commit_until_fixed() {
    let mut b = StagedBuild::new();
    let mut ops = scratch_ops();
    ops[5] = port("body", "supply_in", "electrical", Direction::In, "A");
    let ev = b.apply(&ops).unwrap();
    let raised: Vec<_> = ev.iter().filter(|e| e.kind == BuildEventKind::IssueRaised).map(|e| e.issue.clone().unwrap()).collect();
    assert_eq!(raised.len(), 1);
    assert_eq!((raised[0].code.as_str(), raised[0].severity), ("unit.mismatch", Severity::Blocking));
    assert!(raised[0].fix.is_some());
    assert!(!ev.iter().any(|e| e.kind == BuildEventKind::StagedReady));
    assert_eq!(b.commit(), Err(BuildError::BlockingIssues(vec!["unit.mismatch:c1".into()])));
    // Fix: remove the bad connection and the port is irrelevant.
    let ev = b.apply(&[ModelOp::ConnectionRemove { id: "c1".into() }]).unwrap();
    assert!(ev.iter().any(|e| e.kind == BuildEventKind::IssueCleared));
    assert!(ev.iter().any(|e| e.kind == BuildEventKind::StagedReady));
    let sha = b.commit().unwrap();
    assert_eq!(sha.len(), 64);
    assert!(b.is_committed());
    assert_eq!(b.apply(&[part("late", Maturity::UserDefined)]), Err(BuildError::Committed));
}

#[test]
fn direction_kind_unknown_unit_binding_and_placeholder_checks() {
    let mut b = StagedBuild::new();
    b.apply(&[
        part("a", Maturity::Placeholder), part("b", Maturity::UserDefined),
        port("a", "in1", "electrical", Direction::In, "V"), port("b", "out1", "mechanical", Direction::Out, "V"),
        conn("c", ("a", "in1"), ("b", "out1"), "electrical"),
        ModelOp::PartSetParam { id: "b".into(), key: "speed".into(), value: val(1.0, "furlongs") },
        ModelOp::BindingAdd { binding: helqix_contracts::Binding { variable: "omega".into(), node: "ghost".into(), channel: helqix_contracts::Channel::Rotation, axis: None, mapping: helqix_contracts::Mapping::Integrate, units: "rad/s".into(), range: None } },
    ]).unwrap();
    let codes: Vec<(String, Severity)> = b.issues().into_iter().map(|i| (i.code, i.severity)).collect();
    for expected in [("port.direction", Severity::Blocking), ("port.kind", Severity::Blocking), ("unit.unknown", Severity::Blocking), ("binding.node", Severity::Blocking), ("part.placeholder", Severity::Warning)] {
        assert!(codes.iter().any(|(c, s)| c == expected.0 && *s == expected.1), "missing {expected:?} in {codes:?}");
    }
    assert_eq!(codes.iter().filter(|(c, _)| c == "port.direction").count(), 2, "starts at an input and ends at an output");
}

#[test]
fn the_same_ops_give_the_same_model_hash() {
    let mut a = StagedBuild::new();
    let mut b = StagedBuild::new();
    a.apply(&scratch_ops()).unwrap();
    b.apply(&scratch_ops()).unwrap();
    let sha_a = a.commit().unwrap();
    assert_eq!(sha_a, b.commit().unwrap());
    let mut c = StagedBuild::new();
    let mut ops = scratch_ops();
    ops.swap(0, 1);
    c.apply(&ops).unwrap();
    assert_ne!(c.commit().unwrap(), sha_a, "op order is part of the model history");
}
