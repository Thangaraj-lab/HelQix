//! Engine/UI contracts from file 35 section 8. JSON field names are camelCase to match the
//! TypeScript side; enum values are the exact strings in the contract (`part.add`, `ai_hypothesis`).
//!
//! Every engineering number is a [`Value`] with unit, origin and status. Nothing here is
//! authoritative by itself: the engine owns state, these are the messages.
//!
//! Additions to the proposal in file 35: `Issue`, `Question`, `Source` and `ModelRevision` fields are
//! spelled out; BuildEvent kinds `op.undone` and `op.redone` exist so undo/redo appear in the log.
use std::collections::BTreeMap;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool { !*b }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Origin { Observed, Simulated, Derived, AiHypothesis, UserStated }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Status { Verified, Unverified }

/// An engineering number. A bare number is never an engineering value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Value {
    pub v: f64,
    pub unit: String,
    pub origin: Origin,
    pub status: Status,
    /// True for anything produced by a mock engine; the UI must show a MOCK tag.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind { UserStated, FromSource, Inferred }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub kind: ProvenanceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Maturity { ValidatedModel, UnvalidatedModel, Placeholder, UserDefined }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Transform { pub pos: [Value; 3], pub rot: [Value; 3], pub scale: [f64; 3], pub frame: String }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Geometry {
    Primitive { shape: String, dims: Vec<Value> },
    #[serde(rename_all = "camelCase")]
    Import { asset_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PortRef { pub part: String, pub port: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Direction { In, Out, Bi }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Channel { Rotation, Translation, Scale, Deformation, Color, Emissive, Visibility, Flow, Vibration }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Mapping { Linear, Table, Integrate }

/// Motion is data: a visual channel bound to a stored state variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub variable: String,
    pub node: String,
    pub channel: Channel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<[f64; 3]>,
    pub mapping: Mapping,
    pub units: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
}

/// One change to a staged model. Every path that builds a model emits these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(tag = "op", deny_unknown_fields)]
pub enum ModelOp {
    #[serde(rename = "part.add")]
    PartAdd { id: String, name: String, geometry: Geometry, maturity: Maturity, prov: Provenance },
    #[serde(rename = "part.setParam")]
    PartSetParam { id: String, key: String, value: Value },
    #[serde(rename = "part.place")]
    PartPlace { id: String, transform: Transform },
    #[serde(rename = "port.add")]
    PortAdd { part: String, id: String, kind: String, direction: Direction, #[serde(default, skip_serializing_if = "Option::is_none")] unit: Option<String> },
    #[serde(rename = "connection.add")]
    ConnectionAdd { id: String, from: PortRef, to: PortRef, kind: String },
    #[serde(rename = "material.assign", rename_all = "camelCase")]
    MaterialAssign { part: String, material_id: String },
    #[serde(rename = "binding.add")]
    BindingAdd { binding: Binding },
    #[serde(rename = "requirement.add")]
    RequirementAdd { id: String, text: String, #[serde(default, skip_serializing_if = "Option::is_none")] expr: Option<String>, prov: Provenance },
    #[serde(rename = "part.remove")]
    PartRemove { id: String },
    #[serde(rename = "connection.remove")]
    ConnectionRemove { id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Severity { Blocking, Warning }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Issue {
    /// Stable id derived from code and subject, so the same problem keeps the same id.
    pub id: String,
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub subject: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Question { pub id: String, pub text: String, #[serde(default, skip_serializing_if = "Option::is_none")] pub default: Option<String> }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum BuildEventKind {
    #[serde(rename = "step.start")] StepStart,
    #[serde(rename = "step.done")] StepDone,
    #[serde(rename = "op.applied")] OpApplied,
    #[serde(rename = "op.undone")] OpUndone,
    #[serde(rename = "op.redone")] OpRedone,
    #[serde(rename = "issue.raised")] IssueRaised,
    #[serde(rename = "issue.cleared")] IssueCleared,
    #[serde(rename = "question.asked")] QuestionAsked,
    #[serde(rename = "staged.ready")] StagedReady,
    #[serde(rename = "committed")] Committed,
    #[serde(rename = "failed")] Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Step { Interpret, Resolve, Import, Place, Connect, Check }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildEvent {
    pub seq: u64,
    pub build_id: String,
    pub ts: String,
    pub kind: BuildEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op: Option<ModelOp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<Issue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<Question>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A committed, authoritative model revision (`r1`, `r2`, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelRevision {
    pub project_id: String,
    pub number: u32,
    pub label: String,
    pub build_id: String,
    pub op_count: u64,
    /// SHA-256 of the canonical JSON of the committed op list.
    pub model_sha256: String,
    pub committed_at: String,
}

/// Stored simulation state at one time index. Scrubbing reads frames; it never computes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunFrame {
    pub run_id: String,
    pub index: u64,
    pub t: Value,
    pub state: BTreeMap<String, Value>,
    /// `Bitwise`, `Tolerance{absolute}` or `Statistical`, as declared by the run.
    pub determinism: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of: Option<String>,
    /// Example data shipped for demos and tests; must be labelled Example in the UI.
    #[serde(default, skip_serializing_if = "is_false")]
    pub example: bool,
}

/// The contract types that get a JSON Schema file, by file stem.
#[cfg(feature = "schema")]
pub fn schemas() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("Value", serde_json::to_value(schemars::schema_for!(Value)).expect("schema")),
        ("ModelOp", serde_json::to_value(schemars::schema_for!(ModelOp)).expect("schema")),
        ("BuildEvent", serde_json::to_value(schemars::schema_for!(BuildEvent)).expect("schema")),
        ("Binding", serde_json::to_value(schemars::schema_for!(Binding)).expect("schema")),
        ("ModelRevision", serde_json::to_value(schemars::schema_for!(ModelRevision)).expect("schema")),
        ("RunFrame", serde_json::to_value(schemars::schema_for!(RunFrame)).expect("schema")),
        ("Source", serde_json::to_value(schemars::schema_for!(Source)).expect("schema")),
        ("Issue", serde_json::to_value(schemars::schema_for!(Issue)).expect("schema")),
    ]
}
