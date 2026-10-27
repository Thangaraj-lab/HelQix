// HELQIX Engineering Protocol Contracts (Section 9)
// Shared interface contract between Engine and Frontend Studio

export type Origin = 'observed' | 'simulated' | 'derived' | 'ai_hypothesis' | 'user_stated';
export type Status = 'verified' | 'unverified';

export interface Value {
  v: number;
  unit: string;
  origin: Origin;
  status: Status;
  mock?: boolean;
}

export interface Provenance {
  kind: 'user_stated' | 'from_source' | 'inferred';
  sourceId?: string;
  asOf?: string;
  note?: string;
}

export type Maturity = 'validated_model' | 'unvalidated_model' | 'placeholder' | 'user_defined';

export interface Transform {
  pos: [Value, Value, Value];
  rot: [Value, Value, Value];
  scale: [number, number, number];
  frame: string;
}

export interface PortRef {
  partId: string;
  portId: string;
}

export type BindingChannel =
  | 'rotation'
  | 'translation'
  | 'scale'
  | 'deformation'
  | 'color'
  | 'emissive'
  | 'visibility'
  | 'flow'
  | 'vibration';

export interface Binding {
  id: string;
  variable: string;
  node: string;
  channel: BindingChannel;
  axis?: 'x' | 'y' | 'z';
  mapping?: string;
  units: string;
  range?: [number, number];
}

export type GeometryKind =
  | { kind: 'primitive'; shape: 'box' | 'cylinder' | 'sphere' | 'plate'; dims: Value[] }
  | { kind: 'import'; assetId: string };

export type ModelOp =
  | { op: 'part.add'; id: string; name: string; geometry: GeometryKind; maturity: Maturity; prov: Provenance }
  | { op: 'part.setParam'; id: string; key: string; value: Value }
  | { op: 'part.place'; id: string; transform: Transform }
  | { op: 'port.add'; part: string; id: string; kind: string; direction: 'in' | 'out' | 'bi'; unit?: string }
  | { op: 'connection.add'; id: string; from: PortRef; to: PortRef; kind: string }
  | { op: 'material.assign'; part: string; materialId: string }
  | { op: 'binding.add'; binding: Binding }
  | { op: 'requirement.add'; id: string; text: string; expr?: string; prov: Provenance }
  | { op: 'part.remove'; id: string }
  | { op: 'connection.remove'; id: string };

export interface Issue {
  id: string;
  partId?: string;
  portId?: string;
  message: string;
  severity: 'blocking' | 'warning';
  fixOp?: ModelOp;
}

export interface Question {
  id: string;
  text: string;
  defaultVal?: string;
  choices?: string[];
}

export interface BuildEvent {
  seq: number;
  buildId: string;
  ts: string;
  kind:
    | 'op.undone'
    | 'op.redone'
    | 'step.start'
    | 'step.done'
    | 'op.applied'
    | 'issue.raised'
    | 'issue.cleared'
    | 'question.asked'
    | 'staged.ready'
    | 'committed'
    | 'failed';
  step?: 'interpret' | 'resolve' | 'import' | 'place' | 'connect' | 'check';
  op?: ModelOp;
  issue?: Issue;
  question?: Question;
  note?: string;
}

/** A recorded run the engine can serve frames for (not tied to a model; the user binds model parts to its variables). */
export interface StoredRunInfo {
  id: string;
  rows: number;
  determinism: string;
  variables: { name: string; unit: string }[];
}

export interface RunSummary {
  id: string;
  projectId: string;
  scenarioId: string;
  revisionNum: number;
  startedAt: string;
}

export interface Project {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  activeRevisionId?: string;
  modelRevisions: string[];
}

export interface ModelRevision {
  id: string;
  projectId: string;
  revisionNum: number;
  committedAt: string;
  parts: Record<string, { id: string; name: string; geometry: GeometryKind; transform: Transform; maturity: Maturity }>;
  ports: Record<string, { id: string; partId: string; kind: string; direction: 'in' | 'out' | 'bi'; unit?: string }>;
  connections: Record<string, { id: string; from: PortRef; to: PortRef; kind: string }>;
  requirements: Record<string, { id: string; text: string; expr?: string }>;
  bindings: Record<string, Binding>;
}

export interface Source {
  id: string;
  name: string;
  kind: 'connected_tool' | 'cad_repository' | 'catalog' | 'manual';
  status: 'connected' | 'syncing' | 'unreachable';
  lastSync?: string;
}

export interface AssetRef {
  id: string;
  name: string;
  sizeBytes: number;
  mimeType: string;
  uri: string;
}

export interface RunFrame {
  t: number;
  values: Record<string, Value>;
  transforms?: Record<string, Transform>;
}

export interface Checkpoint {
  id: string;
  runId: string;
  t: number;
  hash: string;
  stateSnapshot: Record<string, Value>;
}

export interface Intervention {
  id: string;
  name: string;
  checkpointId: string;
  paramOverrides: Record<string, Value>;
}

export interface EngineClient {
  createProject(): Promise<Project>;
  /** Home's project list (GET /v1/projects). Not in file 35 section 8; added for the Home screen. */
  listProjects(): Promise<Project[]>;
  /** Rename a project (inline rename in the Studio header). */
  renameProject(pid: string, name: string): Promise<Project>;
  /** Recorded runs available to simulate against. */
  listStoredRuns(): Promise<StoredRunInfo[]>;
  /** Runs started for a project, newest first. Not in file 35 section 8; added for Home's recent runs. */
  listRuns(pid: string): Promise<RunSummary[]>;
  /** A committed revision of a project (r1 is revisionNum 1), or null when it does not exist. */
  getRevision(pid: string, revisionNum: number): Promise<ModelRevision | null>;
  startBuild(
    pid: string,
    input: { prompt: string; attachments?: AssetRef[] } | { scratch: true } | { importAsset: AssetRef }
  ): Promise<{ buildId: string }>;
  buildEvents(buildId: string): AsyncIterable<BuildEvent>;
  answerQuestion(buildId: string, qid: string, answer: string | 'default' | 'skip'): Promise<void>;
  applyOps(buildId: string, ops: ModelOp[]): Promise<BuildEvent[]>;
  /** The real engine returns the events the undo produced; the mock returns nothing and the UI pops locally. */
  undo(buildId: string): Promise<BuildEvent[] | void>;
  redo(buildId: string): Promise<BuildEvent[] | void>;
  commit(buildId: string): Promise<ModelRevision>;
  listSources(pid: string): Promise<Source[]>;
  run(pid: string, scenarioId: string, opts?: { seed?: number; revisionNum?: number }): Promise<{ runId: string }>;
  runFrames(runId: string): AsyncIterable<RunFrame>;
  getFrame(runId: string, t: number): Promise<RunFrame>;
  freeze(runId: string, t: number): Promise<Checkpoint>;
  branch(cp: string, intervention: Intervention): Promise<{ runId: string }>;
}
