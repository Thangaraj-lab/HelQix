//! Staged model builds (file 35 section 6). A model is the replay of an ordered list of
//! [`ModelOp`]s. A [`StagedBuild`] keeps that list with an undo/redo cursor, rejects ops that
//! reference things that do not exist, derives [`Issue`]s with deterministic checks, and commits
//! only when no blocking Issue is left. The caller stores the op list and the cursor; restoring
//! from them gives the identical build (a browser refresh loses nothing).
//!
//! Checks are rules, not judgement: port direction, port kind, unit dimension (helqix-core unit
//! registry), unknown units, references to missing parts, placeholder parts.
use std::collections::{BTreeMap, BTreeSet};

use helqix_contracts::{BuildEventKind, Direction, Issue, Maturity, ModelOp, PortRef, Severity, Value};
use helqix_core::units::Unit;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq)]
pub struct Port { pub kind: String, pub direction: Direction, pub unit: Option<String> }

#[derive(Debug, Clone, PartialEq)]
pub struct Part { pub name: String, pub maturity: Maturity, pub params: BTreeMap<String, Value>, pub placed: bool, pub material: Option<String>, pub ports: BTreeMap<String, Port> }

#[derive(Debug, Clone, PartialEq)]
pub struct Connection { pub from: PortRef, pub to: PortRef, pub kind: String }

/// The model a list of ops describes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model {
    pub parts: BTreeMap<String, Part>,
    pub connections: BTreeMap<String, Connection>,
    pub bindings: Vec<helqix_contracts::Binding>,
    pub requirements: BTreeSet<String>,
}

/// Why an op cannot be applied at all (it is not added to the log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpError {
    DuplicateId(String),
    UnknownPart(String),
    UnknownConnection(String),
    UnknownPort(String),
    InUse { part: String, connection: String },
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "id {id} already exists"),
            Self::UnknownPart(id) => write!(f, "no part {id}"),
            Self::UnknownConnection(id) => write!(f, "no connection {id}"),
            Self::UnknownPort(p) => write!(f, "no port {p}"),
            Self::InUse { part, connection } => write!(f, "part {part} is used by connection {connection}; remove the connection first"),
        }
    }
}

impl Model {
    pub fn apply(&mut self, op: &ModelOp) -> Result<(), OpError> {
        fn part_mut<'m>(m: &'m mut Model, id: &str) -> Result<&'m mut Part, OpError> { m.parts.get_mut(id).ok_or_else(|| OpError::UnknownPart(id.to_owned())) }
        match op {
            ModelOp::PartAdd { id, name, maturity, .. } => {
                if self.parts.contains_key(id) { return Err(OpError::DuplicateId(id.clone())); }
                self.parts.insert(id.clone(), Part { name: name.clone(), maturity: *maturity, params: BTreeMap::new(), placed: false, material: None, ports: BTreeMap::new() });
            }
            ModelOp::PartSetParam { id, key, value } => { part_mut(self, id)?.params.insert(key.clone(), value.clone()); }
            ModelOp::PartPlace { id, .. } => part_mut(self, id)?.placed = true,
            ModelOp::PortAdd { part, id, kind, direction, unit } => {
                let p = part_mut(self, part)?;
                if p.ports.contains_key(id) { return Err(OpError::DuplicateId(format!("{part}.{id}"))); }
                p.ports.insert(id.clone(), Port { kind: kind.clone(), direction: *direction, unit: unit.clone() });
            }
            ModelOp::ConnectionAdd { id, from, to, kind } => {
                if self.connections.contains_key(id) { return Err(OpError::DuplicateId(id.clone())); }
                for r in [from, to] {
                    let p = self.parts.get(&r.part).ok_or_else(|| OpError::UnknownPart(r.part.clone()))?;
                    if !p.ports.contains_key(&r.port) { return Err(OpError::UnknownPort(format!("{}.{}", r.part, r.port))); }
                }
                self.connections.insert(id.clone(), Connection { from: from.clone(), to: to.clone(), kind: kind.clone() });
            }
            ModelOp::MaterialAssign { part, material_id } => part_mut(self, part)?.material = Some(material_id.clone()),
            ModelOp::BindingAdd { binding } => self.bindings.push(binding.clone()),
            ModelOp::RequirementAdd { id, .. } => { if !self.requirements.insert(id.clone()) { return Err(OpError::DuplicateId(id.clone())); } }
            ModelOp::PartRemove { id } => {
                if let Some((cid, _)) = self.connections.iter().find(|(_, c)| c.from.part == *id || c.to.part == *id) {
                    return Err(OpError::InUse { part: id.clone(), connection: cid.clone() });
                }
                self.parts.remove(id).ok_or_else(|| OpError::UnknownPart(id.clone()))?;
            }
            ModelOp::ConnectionRemove { id } => { self.connections.remove(id).ok_or_else(|| OpError::UnknownConnection(id.clone()))?; }
        }
        Ok(())
    }

    pub fn from_ops<'a>(ops: impl IntoIterator<Item = &'a ModelOp>) -> Result<Self, OpError> {
        let mut m = Self::default();
        for op in ops { m.apply(op)?; }
        Ok(m)
    }

    /// Deterministic checks, sorted by issue id.
    pub fn issues(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        let mut push = |severity, code: &str, subject: Vec<String>, message: String, fix: Option<&str>| {
            out.push(Issue { id: format!("{code}:{}", subject.join(",")), severity, code: code.into(), message, subject, fix: fix.map(str::to_owned) });
        };
        let unit_ok = |u: &str| Unit::parse(u).is_ok();
        for (pid, p) in &self.parts {
            if p.maturity == Maturity::Placeholder { push(Severity::Warning, "part.placeholder", vec![pid.clone()], format!("{} has no model (PLACEHOLDER MODEL)", p.name), Some("import a model or define the part")); }
            for (k, v) in &p.params {
                if !unit_ok(&v.unit) { push(Severity::Blocking, "unit.unknown", vec![format!("{pid}.{k}")], format!("unit '{}' of {}.{k} is not a registered unit", v.unit, p.name), Some("use a registered unit symbol")); }
            }
            for (port_id, port) in &p.ports {
                if let Some(u) = &port.unit { if !unit_ok(u) { push(Severity::Blocking, "unit.unknown", vec![format!("{pid}.{port_id}")], format!("unit '{u}' of port {pid}.{port_id} is not a registered unit"), Some("use a registered unit symbol")); } }
            }
        }
        for (cid, c) in &self.connections {
            let (Some(a), Some(b)) = (self.port(&c.from), self.port(&c.to)) else {
                push(Severity::Blocking, "connection.dangling", vec![cid.clone()], format!("connection {cid} refers to a missing port"), Some("remove the connection"));
                continue;
            };
            if a.direction == Direction::In { push(Severity::Blocking, "port.direction", vec![cid.clone()], format!("connection {cid} starts at an input port"), Some("swap the ends")); }
            if b.direction == Direction::Out { push(Severity::Blocking, "port.direction", vec![cid.clone()], format!("connection {cid} ends at an output port"), Some("swap the ends")); }
            if a.kind != c.kind || b.kind != c.kind { push(Severity::Blocking, "port.kind", vec![cid.clone()], format!("connection {cid} is {} but joins {} and {} ports", c.kind, a.kind, b.kind), Some("connect ports of the same kind")); }
            if let (Some(ua), Some(ub)) = (&a.unit, &b.unit) {
                if let (Ok(x), Ok(y)) = (Unit::parse(ua), Unit::parse(ub)) {
                    if x.dimension() != y.dimension() {
                        push(Severity::Blocking, "unit.mismatch", vec![cid.clone()], format!("connection {cid} joins {ua} and {ub}, which measure different quantities"), Some("change one port's unit or connect a compatible port"));
                    }
                }
            }
        }
        for b in &self.bindings {
            if !self.parts.contains_key(&b.node) { push(Severity::Blocking, "binding.node", vec![b.node.clone(), b.variable.clone()], format!("binding of {} targets missing part {}", b.variable, b.node), Some("bind to an existing part")); }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    fn port(&self, r: &PortRef) -> Option<&Port> { self.parts.get(&r.part)?.ports.get(&r.port) }
}

/// One event the build emitted; the caller adds sequence number, build id and timestamp.
#[derive(Debug, Clone, PartialEq)]
pub struct Emitted { pub kind: BuildEventKind, pub op: Option<ModelOp>, pub issue: Option<Issue>, pub note: Option<String> }

impl Emitted {
    fn new(kind: BuildEventKind) -> Self { Self { kind, op: None, issue: None, note: None } }
    fn op(kind: BuildEventKind, op: ModelOp) -> Self { Self { op: Some(op), ..Self::new(kind) } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError { Op { index: usize, error: OpError }, NothingToUndo, NothingToRedo, Committed, Empty, BlockingIssues(Vec<String>) }

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Op { index, error } => write!(f, "op {index} rejected: {error}"),
            Self::NothingToUndo => write!(f, "nothing to undo"),
            Self::NothingToRedo => write!(f, "nothing to redo"),
            Self::Committed => write!(f, "build is already committed"),
            Self::Empty => write!(f, "an empty build cannot be committed"),
            Self::BlockingIssues(ids) => write!(f, "blocking issues: {}", ids.join(", ")),
        }
    }
}

/// The staged op log. `ops[..cursor]` are active; `ops[cursor..]` can be redone.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StagedBuild { ops: Vec<ModelOp>, cursor: usize, committed: bool }

impl StagedBuild {
    pub fn new() -> Self { Self::default() }
    /// Rebuilds from a stored log. Fails if the stored ops do not replay.
    pub fn restore(ops: Vec<ModelOp>, cursor: usize, committed: bool) -> Result<Self, BuildError> {
        if cursor > ops.len() { return Err(BuildError::Op { index: cursor, error: OpError::UnknownConnection("cursor beyond log".into()) }); }
        Model::from_ops(&ops[..cursor]).map_err(|error| BuildError::Op { index: 0, error })?;
        Ok(Self { ops, cursor, committed })
    }
    pub fn ops(&self) -> &[ModelOp] { &self.ops }
    pub fn active_ops(&self) -> &[ModelOp] { &self.ops[..self.cursor] }
    pub fn cursor(&self) -> usize { self.cursor }
    pub fn is_committed(&self) -> bool { self.committed }
    pub fn model(&self) -> Model { Model::from_ops(self.active_ops()).expect("active ops always replay") }
    pub fn issues(&self) -> Vec<Issue> { self.model().issues() }

    /// Applies all ops or none. Drops any redo tail.
    pub fn apply(&mut self, new_ops: &[ModelOp]) -> Result<Vec<Emitted>, BuildError> {
        if self.committed { return Err(BuildError::Committed); }
        let before = self.issues();
        let mut model = self.model();
        for (index, op) in new_ops.iter().enumerate() { model.apply(op).map_err(|error| BuildError::Op { index, error })?; }
        self.ops.truncate(self.cursor);
        self.ops.extend_from_slice(new_ops);
        self.cursor = self.ops.len();
        let mut ev: Vec<Emitted> = new_ops.iter().map(|op| Emitted::op(BuildEventKind::OpApplied, op.clone())).collect();
        ev.extend(self.issue_diff(&before));
        Ok(ev)
    }

    /// Steps back exactly one op.
    pub fn undo(&mut self) -> Result<Vec<Emitted>, BuildError> {
        if self.committed { return Err(BuildError::Committed); }
        if self.cursor == 0 { return Err(BuildError::NothingToUndo); }
        let before = self.issues();
        self.cursor -= 1;
        let mut ev = vec![Emitted::op(BuildEventKind::OpUndone, self.ops[self.cursor].clone())];
        ev.extend(self.issue_diff(&before));
        Ok(ev)
    }

    pub fn redo(&mut self) -> Result<Vec<Emitted>, BuildError> {
        if self.committed { return Err(BuildError::Committed); }
        if self.cursor == self.ops.len() { return Err(BuildError::NothingToRedo); }
        let before = self.issues();
        self.cursor += 1;
        let mut ev = vec![Emitted::op(BuildEventKind::OpRedone, self.ops[self.cursor - 1].clone())];
        ev.extend(self.issue_diff(&before));
        Ok(ev)
    }

    /// Marks the build committed and returns the SHA-256 of the canonical JSON of the active ops.
    pub fn commit(&mut self) -> Result<String, BuildError> {
        if self.committed { return Err(BuildError::Committed); }
        if self.cursor == 0 { return Err(BuildError::Empty); }
        let blocking: Vec<String> = self.issues().into_iter().filter(|i| i.severity == Severity::Blocking).map(|i| i.id).collect();
        if !blocking.is_empty() { return Err(BuildError::BlockingIssues(blocking)); }
        self.committed = true;
        Ok(model_sha256(self.active_ops()))
    }

    fn issue_diff(&self, before: &[Issue]) -> Vec<Emitted> {
        let after = self.issues();
        let mut ev = Vec::new();
        for i in before { if !after.iter().any(|a| a.id == i.id) { ev.push(Emitted { issue: Some(i.clone()), ..Emitted::new(BuildEventKind::IssueCleared) }); } }
        for i in &after { if !before.iter().any(|b| b.id == i.id) { ev.push(Emitted { issue: Some(i.clone()), ..Emitted::new(BuildEventKind::IssueRaised) }); } }
        if self.cursor > 0 && !after.iter().any(|i| i.severity == Severity::Blocking) {
            ev.push(Emitted { note: Some(format!("{} ops, no blocking issues", self.cursor)), ..Emitted::new(BuildEventKind::StagedReady) });
        }
        ev
    }
}

pub fn model_sha256(ops: &[ModelOp]) -> String { format!("{:x}", Sha256::digest(serde_json::to_vec(ops).expect("ops serialize"))) }
