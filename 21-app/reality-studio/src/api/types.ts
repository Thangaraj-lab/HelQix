/**
 * HELQIX domain view models (frontend mirror of backend contracts).
 * --------------------------------------------------------------------------
 * These types describe the SHAPE of data the Layer 22 API is expected to
 * return. They are NOT a second source of truth: every value rendered from
 * them originates from an API response. The frontend never synthesizes
 * engineering state, physics results, failures, causal edges, verification
 * verdicts, or evidence — see `src/api/client.ts` for the integration
 * boundary and capability availability model.
 */

/* ---------- Identity & time primitives (canonical, string-serialized) ---- */

export type ProjectId = string;
export type RealityId = string;
export type ExecutionId = string;
export type StateId = string;
export type SnapshotId = string;
export type EventId = string;
export type FailureId = string;
export type BranchId = string;
export type ComponentId = string;
export type MetricId = string;
export type RequirementId = string;
export type CandidateId = string;

/** Logical time as serialized by Layer 07 (ns since simulation epoch). */
export interface LogicalTime {
  readonly nanos: string; // decimal string — safe beyond Number precision
}

export interface Quantity {
  readonly value: number;
  readonly unit: string;
}

/* ---------- Projects / Realities ----------------------------------------- */

export type EngineeringDomain = "aerospace" | "automotive" | "robotics" | "energy" | "generic";

export interface ProjectSummary {
  readonly id: ProjectId;
  readonly name: string;
  readonly description: string;
  readonly domain: EngineeringDomain;
  readonly lastActivity: LogicalTime;
  readonly status: "draft" | "active" | "archived";
}

export interface CreateProjectRequest {
  readonly name: string;
  readonly description: string;
  readonly domain: EngineeringDomain;
}

export interface RealitySummary {
  readonly id: RealityId;
  readonly projectId: ProjectId;
  readonly name: string;
  readonly status: "unconstructed" | "ready" | "running" | "paused" | "faulted";
}

/* ---------- Engineering model (Layer 03) --------------------------------- */

export interface ModelComponent {
  readonly id: ComponentId;
  readonly parentId: ComponentId | null;
  readonly name: string;
  readonly kind: string; // generic component taxonomy from Layer 01/03
  readonly properties: ReadonlyArray<PropertyEntry>;
}

export interface PropertyEntry {
  readonly name: string;
  readonly quantity: Quantity | null;
  readonly unitSymbol: string | null;
}

export interface ModelInterface {
  readonly id: string;
  readonly sourceComponentId: ComponentId;
  readonly targetComponentId: ComponentId;
  readonly kind: string;
}

export interface EngineeringModel {
  readonly projectId: ProjectId;
  readonly components: ReadonlyArray<ModelComponent>;
  readonly interfaces: ReadonlyArray<ModelInterface>;
  readonly validationDiagnostics: ReadonlyArray<string>;
}

/* ---------- Intent (Layer 03 intent contracts) --------------------------- */

export interface IntentStatement {
  readonly projectId: ProjectId;
  readonly text: string;
  readonly objectives: ReadonlyArray<string>;
  readonly constraints: ReadonlyArray<string>;
  readonly targets: ReadonlyArray<string>;
  readonly requirements: ReadonlyArray<RequirementRef>;
}

export interface RequirementRef {
  readonly id: RequirementId;
  readonly summary: string;
}

/* ---------- State (Layer 04) --------------------------------------------- */

export interface StatePropertyValue {
  readonly componentId: ComponentId;
  readonly componentName: string;
  readonly property: string;
  readonly quantity: Quantity;
}

export interface AuthoritativeState {
  readonly stateId: StateId;
  readonly realityId: RealityId;
  readonly time: LogicalTime;
  readonly values: ReadonlyArray<StatePropertyValue>;
}

export interface StateSnapshotRef {
  readonly snapshotId: SnapshotId;
  readonly stateId: StateId;
  readonly time: LogicalTime;
  readonly label: string;
}

/* ---------- Execution (Layer 05) ----------------------------------------- */

export type ExecutionStatus =
  | "idle"
  | "ready"
  | "running"
  | "paused"
  | "stepping"
  | "completed"
  | "stopped"
  | "faulted"
  | "failed";

export interface ExecutionSummary {
  readonly id: ExecutionId;
  readonly realityId: RealityId;
  readonly status: ExecutionStatus;
  readonly currentTime: LogicalTime;
  readonly branchId: BranchId | null;
}

export interface ActionDescriptor {
  readonly kind: string; // e.g. "ParameterChange" — supplied BY THE BACKEND
  readonly label: string;
  readonly parameterSchema: ReadonlyArray<ActionParameterSpec>;
}

export interface ActionParameterSpec {
  readonly name: string;
  readonly type: "number" | "string" | "boolean";
  readonly unit?: string;
  readonly required: boolean;
}

export interface ExecuteActionRequest {
  readonly executionId: ExecutionId;
  readonly targetComponentId: ComponentId;
  readonly actionKind: string;
  readonly parameters: Readonly<Record<string, string | number | boolean>>;
}

export interface ExecuteActionResponse {
  readonly accepted: boolean;
  readonly eventId: EventId | null;
  readonly message: string;
}

/* ---------- Events / Timeline (Layers 10 + 07) --------------------------- */

export type TimelineItemType = "action" | "state_change" | "event" | "failure" | "snapshot";

export interface TimelineItem {
  readonly id: string;
  readonly executionId: ExecutionId;
  readonly type: TimelineItemType;
  readonly time: LogicalTime;
  readonly label: string;
  readonly severity: "info" | "warning" | "failure";
  readonly referenceId: string | null; // event/failure/snapshot id
}

/* ---------- Observation (Layer 08) --------------------------------------- */

export interface MetricDescriptor {
  readonly id: MetricId;
  readonly name: string;
  readonly unit: string;
  readonly componentId: ComponentId | null;
  /** Constraint limit rendered on charts — comes from Layer 03 constraints via API. */
  readonly limit: { readonly max: number; readonly constraintId: string } | null;
}

export interface MetricSample {
  readonly time: LogicalTime;
  readonly value: number;
}

export interface MetricSeries {
  readonly metricId: MetricId;
  readonly samples: ReadonlyArray<MetricSample>;
}

/* ---------- Failure (Layer 09) ------------------------------------------- */

export interface FailureRecord {
  readonly id: FailureId;
  readonly executionId: ExecutionId;
  readonly componentId: ComponentId | null;
  readonly componentName: string;
  readonly time: LogicalTime;
  readonly severity: "info" | "warning" | "critical";
  readonly condition: string;
  readonly stateAtFailure: StateId;
}

/* ---------- Causality (Layer 10) ----------------------------------------- */

export interface CausalNode {
  readonly id: string;
  readonly label: string;
  readonly kind: "cause" | "intermediate" | "effect" | "failure" | "root";
  readonly componentId: ComponentId | null;
  readonly propertyName: string | null;
  readonly valueChange: string | null;
  readonly time: LogicalTime | null;
  readonly evidenceRefs: ReadonlyArray<string>;
}

export interface CausalEdge {
  readonly from: string;
  readonly to: string;
  readonly relation: string;
}

export interface CausalGraphView {
  readonly failureId: FailureId;
  readonly nodes: ReadonlyArray<CausalNode>;
  readonly edges: ReadonlyArray<CausalEdge>;
  readonly rootCauseIds: ReadonlyArray<string>;
}

/* ---------- Branching (Layer 11) ----------------------------------------- */

export interface BranchNode {
  readonly id: BranchId;
  readonly parentBranchId: BranchId | null;
  readonly originExecutionId: ExecutionId;
  readonly originStateId: StateId;
  readonly originTime: LogicalTime;
  readonly modification: string;
  readonly status: "pending" | "running" | "completed" | "faulted" | "abandoned";
  readonly createdAt: LogicalTime;
}

export interface CreateBranchRequest {
  readonly projectId: ProjectId;
  readonly originExecutionId: ExecutionId;
  readonly originStateId: StateId;
  readonly modifications: ReadonlyArray<{
    readonly componentId: ComponentId;
    readonly property: string;
    readonly newValue: string;
  }>;
}

export interface ComparisonTable {
  readonly leftLabel: string;
  readonly rightLabel: string;
  readonly rows: ReadonlyArray<{
    readonly label: string;
    readonly left: string;
    readonly right: string;
    readonly delta: string | null;
  }>;
}

/* ---------- Search (Layer 12) -------------------------------------------- */

export interface SearchDefinition {
  readonly variables: ReadonlyArray<{ readonly name: string; readonly min: number; readonly max: number; readonly unit: string }>;
  readonly objective: { readonly sense: "minimize" | "maximize"; readonly expression: string };
  readonly constraints: ReadonlyArray<string>;
}

export interface SearchRunStatus {
  readonly runId: string;
  readonly state: "queued" | "running" | "completed" | "failed";
  readonly progress: number; // 0..1 reported by backend
  readonly evaluated: number;
}

export interface CandidateRow {
  readonly id: CandidateId;
  readonly rank: number;
  readonly parameters: Readonly<Record<string, string>>;
  readonly objectiveValue: string;
  readonly feasible: boolean;
  readonly status: "evaluating" | "verified" | "violated";
}

/* ---------- Verification / Evidence (Layer 13) --------------------------- */

export type Verdict = "pass" | "fail" | "inconclusive" | "not_run";

export interface VerificationCheck {
  readonly requirementId: RequirementId;
  readonly requirementText: string;
  readonly verdict: Verdict;
  readonly evidence: {
    readonly executionId: ExecutionId | null;
    readonly time: LogicalTime | null;
    readonly stateId: StateId | null;
    readonly detail: string;
  };
}

export interface EvidenceRecord {
  readonly id: string;
  readonly kind: "execution_record" | "state_history" | "snapshot" | "provenance" | "verification" | "branch_lineage";
  readonly summary: string;
  readonly refs: ReadonlyArray<string>;
}
