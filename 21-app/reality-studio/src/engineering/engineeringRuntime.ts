import type { EngineeringReality } from '../api';
import { assertValidState, captureSnapshot, createInitialState, restoreSnapshot, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import { applyBranch, createBranch, type BranchChange, type BranchResult, type EngineeringBranch } from './branchEngine';
import { createScenario, executeScenario, type ExecutableScenario, type ScenarioExecutionOptions, type ScenarioExecutionResult } from './scenarioEngine';
import { createEvidencePackage, verifyEvidencePackage, type EvidencePackage } from './evidenceEngine';
import { createStateFreeze, verifyStateFreeze, type StateFreeze } from './stateFreeze';
import { assertValidReality, diffReality, type RealityDiff } from './engineeringReality';

export const ENGINEERING_RUNTIME_SCHEMA = 'helqix.engineering-runtime.v1' as const;
export interface EngineeringRuntimeContext { schema: typeof ENGINEERING_RUNTIME_SCHEMA; realityRevision: number; state: AuthoritativeEngineeringState; snapshots: StateSnapshot[]; scenarios: ExecutableScenario[]; branches: EngineeringBranch[]; freezes: StateFreeze[]; evidencePackages: EvidencePackage[]; activeScenarioId?: string; activeBranchId?: string; }
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
  const freezeIds = new Set<string>();
  for (const freeze of context.freezes) {
    if (freeze.schema !== 'helqix.state-freeze.v1') throw new Error(`Unsupported state freeze schema: ${freeze.id}.`);
    if (freezeIds.has(freeze.id)) throw new Error(`Duplicate runtime state freeze: ${freeze.id}.`);
    freezeIds.add(freeze.id);
    if (freeze.realityRevision !== reality.revision) throw new Error(`State freeze ${freeze.id} belongs to a different reality revision.`);
    if (freeze.snapshot.stateRevision !== freeze.stateRevision || freeze.snapshot.logicalTimeMs !== freeze.logicalTimeMs) throw new Error(`State freeze ${freeze.id} has inconsistent state identity.`);
    if (!context.snapshots.some((snapshot) => snapshot.stateRevision === freeze.stateRevision && snapshot.state.logicalTimeMs === freeze.logicalTimeMs && snapshot.state.realityRevision === reality.revision)) throw new Error(`State freeze ${freeze.id} has no matching runtime snapshot.`);
    if (!context.scenarios.some((scenario) => scenario.id === freeze.scenarioId && scenario.version === freeze.scenarioVersion)) throw new Error(`State freeze ${freeze.id} references an unregistered scenario version.`);
  }
  const evidenceIds = new Set<string>();
  for (const evidence of context.evidencePackages) {
    if (evidence.schema !== 'helqix.evidence-package.v1') throw new Error(`Unsupported evidence package schema: ${evidence.id}.`);
    if (evidenceIds.has(evidence.id)) throw new Error(`Duplicate runtime evidence package: ${evidence.id}.`);
    evidenceIds.add(evidence.id);
    if (evidence.realityRevision !== reality.revision) throw new Error(`Evidence package ${evidence.id} belongs to a different reality revision.`);
    if (evidence.authoritativeStateRevision > context.state.stateRevision) throw new Error(`Evidence package ${evidence.id} references a future state revision.`);
    if (!context.scenarios.some((scenario) => scenario.id === evidence.scenarioId && scenario.version === evidence.scenarioVersion)) throw new Error(`Evidence package ${evidence.id} references an unregistered scenario version.`);
    const stateArtifact = evidence.artifacts.find((artifact) => artifact.kind === 'state');
    if (!stateArtifact) throw new Error(`Evidence package ${evidence.id} is missing its authoritative state artifact.`);
    if (!context.snapshots.some((snapshot) => snapshot.state.stateRevision === evidence.authoritativeStateRevision && snapshot.state.realityRevision === reality.revision)) throw new Error(`Evidence package ${evidence.id} has no historical runtime snapshot.`);
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
  return { schema: ENGINEERING_RUNTIME_SCHEMA, realityRevision: reality.revision, state, snapshots: [initialSnapshot], scenarios: [], branches: [], freezes: [], evidencePackages: [] };
}

export function syncEngineeringRuntime(context: EngineeringRuntimeContext, reality: EngineeringReality): EngineeringRuntimeContext {
  assertValidReality(reality);
  if (context.realityRevision === reality.revision) { assertRuntimeContext(context, reality); return clone(context); }
  const nextState = createInitialState(reality);
  const snapshot = captureSnapshot(nextState, 'initial', snapshotId('initial', nextState, 0));
  return { schema: ENGINEERING_RUNTIME_SCHEMA, realityRevision: reality.revision, state: nextState, snapshots: [snapshot], scenarios: [], branches: [], freezes: [], evidencePackages: [], activeScenarioId: undefined, activeBranchId: undefined };
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

export function runScenario(context: EngineeringRuntimeContext, reality: EngineeringReality, scenario: ExecutableScenario, options: ScenarioExecutionOptions = {}): { context: EngineeringRuntimeContext; result: ScenarioExecutionResult; freeze: StateFreeze; evidence: EvidencePackage } {
  assertRuntimeContext(context, reality);
  const storedScenario = context.scenarios.find((item) => item.id === scenario.id);
  if (!storedScenario) throw new Error(`Scenario ${scenario.id} is not registered in this runtime.`);
  if (storedScenario.version !== scenario.version) throw new Error(`Scenario ${scenario.id} version mismatch.`);
  if (storedScenario.createdFromRealityRevision !== reality.revision) throw new Error(`Scenario ${scenario.id} belongs to a stale reality revision.`);
  const result = executeScenario(storedScenario, reality, context.state, options);
  const next = clone(context);
  next.state = result.state;
  next.activeScenarioId = storedScenario.id;
  const snapshot = captureSnapshot(result.state, result.status === 'completed' ? 'run-complete' : 'failure', snapshotId(`scenario-${storedScenario.id}`, result.state, next.snapshots.length));
  next.snapshots.push(snapshot);
  const freeze = createStateFreeze(reality, result.state, storedScenario, { deterministic: result.status === 'completed' });
  next.freezes.push(freeze);
  const evidence = createEvidencePackage({ reality, scenario: storedScenario, state: result.state, freeze, deterministic: result.status === 'completed' });
  next.evidencePackages.push(evidence);
  return { context: next, result, freeze, evidence };
}

export function createRuntimeEvidence(context: EngineeringRuntimeContext, reality: EngineeringReality, scenarioId?: string): { context: EngineeringRuntimeContext; evidence: EvidencePackage } {
  assertRuntimeContext(context, reality);
  const selectedScenarioId = scenarioId ?? context.activeScenarioId;
  if (!selectedScenarioId) throw new Error('No active scenario is available for evidence generation.');
  const scenario = context.scenarios.find((item) => item.id === selectedScenarioId);
  if (!scenario) throw new Error(`Unknown scenario for evidence generation: ${selectedScenarioId}.`);
  const existingFreeze = context.freezes.find((item) => item.scenarioId === scenario.id && item.scenarioVersion === scenario.version && item.stateRevision === context.state.stateRevision && item.logicalTimeMs === context.state.logicalTimeMs);
  const freeze = existingFreeze ?? createStateFreeze(reality, context.state, scenario);
  const evidence = createEvidencePackage({ reality, scenario, state: context.state, freeze });
  const next = clone(context);
  if (!existingFreeze) next.freezes.push(freeze);
  const existingIndex = next.evidencePackages.findIndex((item) => item.id === evidence.id);
  if (existingIndex >= 0) next.evidencePackages[existingIndex] = evidence; else next.evidencePackages.push(evidence);
  return { context: next, evidence };
}

export function verifyRuntimeEvidence(context: EngineeringRuntimeContext, reality: EngineeringReality, evidenceId: string): boolean {
  const evidence = context.evidencePackages.find((item) => item.id === evidenceId);
  if (!evidence) return false;
  const scenario = context.scenarios.find((item) => item.id === evidence.scenarioId && item.version === evidence.scenarioVersion);
  if (!scenario) return false;
  const snapshot = context.snapshots.find((item) => item.state.stateRevision === evidence.authoritativeStateRevision && item.state.realityRevision === evidence.realityRevision);
  if (!snapshot) return false;
  const state = restoreSnapshot(snapshot);
  const freezeArtifact = evidence.artifacts.find((artifact) => artifact.kind === 'freeze');
  if (freezeArtifact) {
    const freeze = context.freezes.find((item) => item.id === freezeArtifact.id);
    if (!freeze || freeze.freezeDigest !== freezeArtifact.digest || !verifyStateFreeze(freeze, reality, scenario)) return false;
    if (freeze.stateRevision !== state.stateRevision || freeze.logicalTimeMs !== state.logicalTimeMs) return false;
  }
  return verifyEvidencePackage(evidence, reality, scenario, state);
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
export function runtimeSummary(context: EngineeringRuntimeContext): { schema: typeof ENGINEERING_RUNTIME_SCHEMA; realityRevision: number; stateRevision: number; logicalTimeMs: number; stateStatus: AuthoritativeEngineeringState['status']; componentCount: number; faultedComponentCount: number; eventCount: number; snapshotCount: number; scenarioCount: number; branchCount: number; freezeCount: number; evidencePackageCount: number; generatedAt: string } { const faultedComponentCount = Object.values(context.state.components).filter((component) => component.status === 'faulted' || component.faultIds.length > 0).length; return { schema: ENGINEERING_RUNTIME_SCHEMA, realityRevision: context.realityRevision, stateRevision: context.state.stateRevision, logicalTimeMs: context.state.logicalTimeMs, stateStatus: context.state.status, componentCount: Object.keys(context.state.components).length, faultedComponentCount, eventCount: context.state.events.length, snapshotCount: context.snapshots.length, scenarioCount: context.scenarios.length, branchCount: context.branches.length, freezeCount: context.freezes.length, evidencePackageCount: context.evidencePackages.length, generatedAt: now() }; }
