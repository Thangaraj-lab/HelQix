import type { EngineeringReality } from '../api';
import { assertValidState, captureSnapshot, createInitialState, restoreSnapshot, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import { applyBranch, createBranch, type BranchChange, type BranchResult, type EngineeringBranch } from './branchEngine';
import { createScenario, executeScenario, type ExecutableScenario, type ScenarioExecutionOptions, type ScenarioExecutionResult } from './scenarioEngine';
import { assertValidReality, diffReality, type RealityDiff } from './engineeringReality';

export const ENGINEERING_RUNTIME_SCHEMA = 'helqix.engineering-runtime.v1' as const;
export interface EngineeringRuntimeContext { schema: typeof ENGINEERING_RUNTIME_SCHEMA; realityRevision: number; state: AuthoritativeEngineeringState; snapshots: StateSnapshot[]; scenarios: ExecutableScenario[]; branches: EngineeringBranch[]; activeScenarioId?: string; activeBranchId?: string; }
export interface RuntimeCheckpoint { snapshot: StateSnapshot; stateRevision: number; logicalTimeMs: number; label: string; }
export interface RuntimeSnapshotReplayResult { sourceSnapshotId: string; state: AuthoritativeEngineeringState; replayedEventCount: number; targetLogicalTimeMs: number; }
function clone<T>(value: T): T { return structuredClone(value); }
function now(): string { return new Date().toISOString(); }
function snapshotId(prefix: string, state: AuthoritativeEngineeringState, sequence: number): string { return `${prefix}-${state.realityRevision}-${state.stateRevision}-${state.logicalTimeMs}-${sequence}`; }

function assertRuntimeContext(context: EngineeringRuntimeContext, reality: EngineeringReality): void {
  if (context.schema !== ENGINEERING_RUNTIME_SCHEMA) throw new Error('Unsupported engineering runtime schema.');
  if (context.realityRevision !== reality.revision) throw new Error(`Runtime is based on reality revision ${context.realityRevision}; current reality is ${reality.revision}.`);
  assertValidState(context.state, reality);
  const snapshotIds = new Set<string>();
  for (const snapshot of context.snapshots) {
    if (snapshot.schema !== 'helqix.state-snapshot.v1') throw new Error(`Unsupported runtime snapshot schema: ${snapshot.id}.`);
    if (snapshotIds.has(snapshot.id)) throw new Error(`Duplicate runtime snapshot: ${snapshot.id}.`);
    snapshotIds.add(snapshot.id);
    if (snapshot.state.realityRevision !== reality.revision) throw new Error(`Snapshot ${snapshot.id} belongs to a different reality revision.`);
    if (snapshot.stateRevision !== snapshot.state.stateRevision || snapshot.logicalTimeMs !== snapshot.state.logicalTimeMs) throw new Error(`Snapshot ${snapshot.id} has inconsistent state identity.`);
  }
  for (const scenario of context.scenarios) {
    if (!scenario.id.trim() || !Number.isInteger(scenario.version) || scenario.version < 1) throw new Error('Runtime contains an invalid scenario identity.');
    if (scenario.createdFromRealityRevision !== reality.revision) throw new Error(`Scenario ${scenario.id} belongs to reality revision ${scenario.createdFromRealityRevision}, current reality is ${reality.revision}.`);
  }
  for (const branch of context.branches) {
    if (!branch.id.trim() || branch.baseRealityRevision !== reality.revision) throw new Error(`Runtime contains a branch that does not belong to the current reality revision: ${branch.id}.`);
  }
  if (context.activeScenarioId && !context.scenarios.some((scenario) => scenario.id === context.activeScenarioId)) throw new Error(`Unknown active scenario: ${context.activeScenarioId}.`);
  if (context.activeBranchId && !context.branches.some((branch) => branch.id === context.activeBranchId)) throw new Error(`Unknown active branch: ${context.activeBranchId}.`);
}

export function createEngineeringRuntime(reality: EngineeringReality): EngineeringRuntimeContext {
  assertValidReality(reality);
  const state = createInitialState(reality);
  const initialSnapshot = captureSnapshot(state, 'initial', snapshotId('initial', state, 0));
  return { schema: ENGINEERING_RUNTIME_SCHEMA, realityRevision: reality.revision, state, snapshots: [initialSnapshot], scenarios: [], branches: [] };
}

export function syncEngineeringRuntime(context: EngineeringRuntimeContext, reality: EngineeringReality): EngineeringRuntimeContext {
  assertValidReality(reality);
  if (context.realityRevision === reality.revision) { assertRuntimeContext(context, reality); return clone(context); }
  const nextState = createInitialState(reality);
  const snapshot = captureSnapshot(nextState, 'initial', snapshotId('initial', nextState, 0));
  return { schema: ENGINEERING_RUNTIME_SCHEMA, realityRevision: reality.revision, state: nextState, snapshots: [snapshot], scenarios: [], branches: [], activeScenarioId: undefined, activeBranchId: undefined };
}

export function saveCheckpoint(context: EngineeringRuntimeContext, label: string, reason: StateSnapshot['reason'] = 'checkpoint'): EngineeringRuntimeContext {
  const normalizedLabel = label.trim() || 'checkpoint';
  const snapshot = captureSnapshot(context.state, reason, snapshotId(normalizedLabel, context.state, context.snapshots.length));
  const next = { ...clone(context), snapshots: [...context.snapshots, snapshot] };
  if (next.state.realityRevision !== next.realityRevision) throw new Error('Checkpoint state does not belong to the runtime reality revision.');
  if (next.snapshots.some((item, index) => next.snapshots.findIndex((candidate) => candidate.id === item.id) !== index)) throw new Error(`Duplicate checkpoint snapshot: ${snapshot.id}.`);
  return next;
}

export function restoreCheckpoint(context: EngineeringRuntimeContext, snapshotIdValue: string): EngineeringRuntimeContext {
  const snapshot = context.snapshots.find((item) => item.id === snapshotIdValue);
  if (!snapshot) throw new Error(`Unknown state snapshot: ${snapshotIdValue}`);
  if (snapshot.state.realityRevision !== context.realityRevision) throw new Error(`Snapshot ${snapshot.id} belongs to a different reality revision.`);
  const restored = restoreSnapshot(snapshot);
  const next = { ...clone(context), state: restored };
  next.activeScenarioId = next.scenarios.find((scenario) => scenario.id === restored.metadata.scenarioId)?.id;
  return next;
}

export function addScenario(context: EngineeringRuntimeContext, reality: EngineeringReality, name: string, actions: ExecutableScenario['actions'] = [], description = ''): { context: EngineeringRuntimeContext; scenario: ExecutableScenario } {
  assertRuntimeContext(context, reality);
  const scenario = createScenario(name, reality, actions, description);
  const next = clone(context);
  next.scenarios.push(scenario);
  next.activeScenarioId = scenario.id;
  return { context: next, scenario };
}

export function runScenario(context: EngineeringRuntimeContext, reality: EngineeringReality, scenario: ExecutableScenario, options: ScenarioExecutionOptions = {}): { context: EngineeringRuntimeContext; result: ScenarioExecutionResult } {
  assertRuntimeContext(context, reality);
  const storedScenario = context.scenarios.find((item) => item.id === scenario.id);
  if (!storedScenario) throw new Error(`Scenario ${scenario.id} is not registered in this runtime.`);
  if (storedScenario.version !== scenario.version) throw new Error(`Scenario ${scenario.id} version mismatch.`);
  if (storedScenario.createdFromRealityRevision !== reality.revision) throw new Error(`Scenario ${scenario.id} belongs to a stale reality revision.`);
  const result = executeScenario(storedScenario, reality, context.state, options);
  const next = clone(context);
  next.state = result.state;
  next.activeScenarioId = storedScenario.id;
  next.snapshots.push(captureSnapshot(result.state, result.status === 'completed' ? 'run-complete' : 'failure', snapshotId(`scenario-${storedScenario.id}`, result.state, next.snapshots.length)));
  return { context: next, result };
}

export function createRuntimeBranch(context: EngineeringRuntimeContext, reality: EngineeringReality, name: string, changes: BranchChange[], description = ''): { context: EngineeringRuntimeContext; branch: EngineeringBranch } {
  assertRuntimeContext(context, reality);
  const branch = createBranch(reality, name, description, changes, context.state.stateRevision);
  const next = clone(context);
  next.branches.push(branch);
  next.activeBranchId = branch.id;
  return { context: next, branch };
}

export function executeRuntimeBranch(context: EngineeringRuntimeContext, reality: EngineeringReality, branch: EngineeringBranch): { context: EngineeringRuntimeContext; result: BranchResult } {
  assertRuntimeContext(context, reality);
  const registeredBranch = context.branches.find((item) => item.id === branch.id);
  if (!registeredBranch) throw new Error(`Branch ${branch.id} is not registered in this runtime.`);
  if (JSON.stringify(registeredBranch) !== JSON.stringify(branch)) throw new Error(`Branch ${branch.id} does not match the registered runtime definition.`);
  if (branch.baseStateRevision !== undefined && branch.baseStateRevision !== context.state.stateRevision) throw new Error(`Branch ${branch.id} is based on state revision ${branch.baseStateRevision}, current state is ${context.state.stateRevision}.`);
  const result = applyBranch(branch, reality, context.state);
  const next = clone(context);
  next.activeBranchId = branch.id;
  if (result.status === 'completed') { next.state = result.resultingState; next.snapshots.push(result.branchSnapshot); }
  return { context: next, result };
}

export function replayToSnapshot(context: EngineeringRuntimeContext, snapshotIdValue: string): RuntimeSnapshotReplayResult {
  const snapshot = context.snapshots.find((item) => item.id === snapshotIdValue);
  if (!snapshot) throw new Error(`Unknown state snapshot: ${snapshotIdValue}`);
  if (snapshot.state.realityRevision !== context.realityRevision) throw new Error(`Snapshot ${snapshot.id} belongs to a different reality revision.`);
  const state = restoreSnapshot(snapshot);
  return { sourceSnapshotId: snapshot.id, state, replayedEventCount: state.events.length, targetLogicalTimeMs: state.logicalTimeMs };
}

export function listCheckpoints(context: EngineeringRuntimeContext): RuntimeCheckpoint[] { return context.snapshots.map((snapshot) => ({ snapshot, stateRevision: snapshot.stateRevision, logicalTimeMs: snapshot.logicalTimeMs, label: snapshot.reason })); }
export function compareReality(context: EngineeringRuntimeContext, before: EngineeringReality, after: EngineeringReality): RealityDiff { assertValidReality(before); assertValidReality(after); if (context.realityRevision !== after.revision) throw new Error(`Runtime is at reality revision ${context.realityRevision}; comparison target is ${after.revision}.`); return diffReality(before, after); }
export function runtimeSummary(context: EngineeringRuntimeContext): { schema: typeof ENGINEERING_RUNTIME_SCHEMA; realityRevision: number; stateRevision: number; logicalTimeMs: number; stateStatus: AuthoritativeEngineeringState['status']; componentCount: number; faultedComponentCount: number; eventCount: number; snapshotCount: number; scenarioCount: number; branchCount: number; generatedAt: string } { const faultedComponentCount = Object.values(context.state.components).filter((component) => component.status === 'faulted' || component.faultIds.length > 0).length; return { schema: ENGINEERING_RUNTIME_SCHEMA, realityRevision: context.realityRevision, stateRevision: context.state.stateRevision, logicalTimeMs: context.state.logicalTimeMs, stateStatus: context.state.status, componentCount: Object.keys(context.state.components).length, faultedComponentCount, eventCount: context.state.events.length, snapshotCount: context.snapshots.length, scenarioCount: context.scenarios.length, branchCount: context.branches.length, generatedAt: now() }; }
