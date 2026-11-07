import type { PackBehavior, PackInfo, PackRunResult } from '../domainpacks';
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

/** What makes a part do something when the model runs. An FMU is named by the SHA-256 of its file, never by a path. */
export type FmuBehavior = { kind: 'fmu'; fmuSha256: string; parameters?: Record<string, Value>; outputs: string[] };
/** A behaviour added by a domain pack (see src/domainpacks). */
export type Behavior = FmuBehavior | PackBehavior;

export type ModelOp =
  | { op: 'part.add'; id: string; name: string; geometry: GeometryKind; maturity: Maturity; prov: Provenance }
  | { op: 'part.setParam'; id: string; key: string; value: Value }
  | { op: 'part.place'; id: string; transform: Transform }
  | { op: 'port.add'; part: string; id: string; kind: string; direction: 'in' | 'out' | 'bi'; unit?: string }
  | { op: 'connection.add'; id: string; from: PortRef; to: PortRef; kind: string }
  | { op: 'material.assign'; part: string; materialId: string }
  | { op: 'binding.add'; binding: Binding }
  | { op: 'requirement.add'; id: string; text: string; expr?: string; prov: Provenance }
  | { op: 'part.setBehavior'; id: string; behavior: Behavior }
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

export interface PointMassRunParams {
  massKg: number;
  dragCoefficient: number;
  referenceAreaM2: number;
  endSeconds: number;
  /** x, y, z (m) and vx, vy, vz (m/s) */
  initialState: [number, number, number, number, number, number];
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
  parts: Record<string, { id: string; name: string; geometry: GeometryKind; transform: Transform; maturity: Maturity; params?: Record<string, Value>; behavior?: Behavior }>;
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

/** An FMU installed in the operator's library, named by the SHA-256 of its file. */
export interface FmuInfo { sha256: string; bytes: number }

export interface FmuVariable {
  name: string;
  causality: string;
  variability: string;
  type: string;
  /** The unit the FMU declares ('' when it declares none). */
  unit: string;
  description: string;
  /** Exponents of the unit's dimension (length, mass, time, current, temperature, substance, luminous intensity); null when the unit could not be read. */
  dimension: number[] | null;
  siFactor: number | null;
  start: number | null;
  unitProblem: string | null;
}

/** What an FMU contains, read from its model description without running it. */
export interface FmuDescription {
  sha256: string;
  bytes: number;
  modelName: string;
  fmiVersion: string;
  defaultExperiment: { startTime: number | null; stopTime: number | null; stepSize: number | null };
  variables: FmuVariable[];
}

/** The requirement a signed evidence package states the outcome of (the comparison that must hold at every recorded sample). */
export interface EvidenceRequirement {
  name: string;
  variable: string;
  /** atLeast >=, atMost <=, above >, below < */
  comparator: 'atLeast' | 'atMost' | 'above' | 'below';
  threshold: number;
}

/** Which key signs evidence on the server, so the person who receives a package knows what to trust. */
export interface EvidenceKeyInfo {
  publicKey: string;
  keyId: string;
  signer: string;
  keyKind: 'test' | 'production';
  note: string;
}

/** The key that signed a package, as the server sent it beside the file (the key the package names, not whatever key the service holds later). */
export type EvidenceSigner = Pick<EvidenceKeyInfo, 'publicKey' | 'keyId' | 'signer' | 'keyKind'>;

export interface EvidenceDownload { blob: Blob; filename: string; signedBy: EvidenceSigner | null }

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
  /** Real engine only: run the point-mass model and store its frames. Absent on the mock engine. */
  startPointMassRun?(p: PointMassRunParams): Promise<{ runId: string }>;
  /** Real engine only: run a committed revision as one lumped body (the engine computes mass and area from the parts). */
  runRevision?(pid: string, revisionNum: number, p: Omit<PointMassRunParams, 'massKg' | 'referenceAreaM2'>): Promise<{ runId: string; massKg: number; areaM2: number; parts: number }>;
  /** Real engine only: the FMUs the operator installed. */
  listFmus?(): Promise<FmuInfo[]>;
  /** Real engine only: what one installed FMU contains. */
  describeFmu?(sha256: string): Promise<FmuDescription>;
  /** Real engine only: run a committed revision whose part has an FMU behaviour, through FMPy. */
  runFmuRevision?(pid: string, revisionNum: number, p: { endSeconds: number; stepSeconds: number }): Promise<{ runId: string; rows: number; fmuSha256: string; determinism: string }>;
  /** Real engine only: the domain packs installed on the server, with their label and published error. */
  listPacks?(): Promise<PackInfo[]>;
  /** Real engine only: run a committed revision whose part comes from a domain pack. */
  runPackRevision?(pid: string, revisionNum: number, p: { endSeconds: number; stepSeconds: number }): Promise<PackRunResult>;
  /** Real engine only: a signed evidence package (a ZIP) for a stored run, built and signed on the server. */
  exportEvidence?(runId: string, requirement: EvidenceRequirement, subject?: string): Promise<EvidenceDownload>;
  /** Real engine only: the public key that signs evidence, and whether it is a test key. */
  evidenceKey?(): Promise<EvidenceKeyInfo>;
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
