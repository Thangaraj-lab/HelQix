import type { EngineeringReality } from '../api';
import { captureSnapshot, restoreSnapshot, validateState, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import { stepExecution, type ExecutionController } from './executionController';
import type { ExecutableScenario } from './scenarioEngine';

export const REPLAY_ENGINE_SCHEMA = 'helqix.replay-engine.v1' as const;

export interface ReplayFrame {
  sequence: number;
  logicalTimeMs: number;
  stateRevision: number;
  actionId?: string;
  snapshotId: string;
  eventType?: string;
  eventOutcome?: 'applied' | 'failed' | 'skipped';
  stateDigest: string;
  state: AuthoritativeEngineeringState;
}
export interface ReplayResult {
  schema: typeof REPLAY_ENGINE_SCHEMA;
  realityRevision: number;
  scenarioId: string;
  initialSnapshotId: string;
  scenarioVersion?: number;
  frames: ReplayFrame[];
  finalState: AuthoritativeEngineeringState;
  deterministic: boolean;
  traceDigest: string;
  error?: string;
}

function clone<T>(value: T): T { return structuredClone(value); }
function canonical(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value) ?? 'undefined';
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  const record = value as Record<string, unknown>;
  return `{${Object.keys(record).sort().map((key) => `${JSON.stringify(key)}:${canonical(record[key])}`).join(',')}}`;
}
function digest(value: unknown): string {
  const serialized = canonical(value);
  let hash = 2166136261;
  for (let index = 0; index < serialized.length; index += 1) { hash ^= serialized.charCodeAt(index); hash = Math.imul(hash, 16777619); }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}
function stateDigest(state: AuthoritativeEngineeringState): string {
  const comparable = clone(state);
  comparable.metadata.createdAt = '';
  comparable.metadata.updatedAt = '';
  return digest(comparable);
}
function replayComparable(result: ReplayResult): unknown {
  return {
    schema: result.schema, realityRevision: result.realityRevision, scenarioId: result.scenarioId,
    initialSnapshotId: result.initialSnapshotId, scenarioVersion: result.scenarioVersion,
    frames: result.frames.map((frame) => ({ sequence: frame.sequence, logicalTimeMs: frame.logicalTimeMs, stateRevision: frame.stateRevision, actionId: frame.actionId, eventType: frame.eventType, eventOutcome: frame.eventOutcome, stateDigest: frame.stateDigest })),
    finalStateDigest: stateDigest(result.finalState), deterministic: result.deterministic, error: result.error,
  };
}

export function replayExecution(reality: EngineeringReality, scenario: ExecutableScenario, initialSnapshot: StateSnapshot): ReplayResult {
  if (initialSnapshot.schema !== 'helqix.state-snapshot.v1') throw new Error('Replay requires a supported state snapshot.');
  if (initialSnapshot.state.realityRevision !== reality.revision) throw new Error('Replay snapshot belongs to a different reality revision.');
  if (initialSnapshot.stateRevision !== initialSnapshot.state.stateRevision || initialSnapshot.logicalTimeMs !== initialSnapshot.state.logicalTimeMs) throw new Error('Replay snapshot identity does not match its authoritative state.');
  const validation = validateState(initialSnapshot.state, reality);
  if (!validation.valid) throw new Error(`Replay snapshot is invalid: ${validation.issues.filter((issue) => issue.severity === 'error').map((issue) => issue.message).join('; ')}`);

  const restored = clone(initialSnapshot.state);
  let controller: ExecutionController = {
    schema: 'helqix.execution-controller.v1', phase: 'ready', realityRevision: reality.revision,
    scenarioId: scenario.id, cursor: 0, logicalTimeMs: restored.logicalTimeMs, state: restored,
    initialSnapshot: clone(initialSnapshot), checkpoints: [clone(initialSnapshot)], events: [],
  };
  const frames: ReplayFrame[] = [{ sequence: 0, logicalTimeMs: restored.logicalTimeMs, stateRevision: restored.stateRevision, snapshotId: initialSnapshot.id, stateDigest: stateDigest(restored), state: clone(restored) }];
  let error: string | undefined;
  try {
    while (controller.cursor < scenario.actions.length && controller.phase !== 'failed') {
      controller = { ...controller, phase: 'running' };
      const result = stepExecution(reality, scenario, controller);
      controller = result.controller;
      const event = result.event;
      const snapshot = captureSnapshot(controller.state, 'scenario', `replay-${scenario.id}-${frames.length}`);
      frames.push({ sequence: frames.length, logicalTimeMs: controller.logicalTimeMs, stateRevision: controller.state.stateRevision, ...(event?.actionId ? { actionId: event.actionId } : {}), snapshotId: snapshot.id, ...(event?.type ? { eventType: event.type } : {}), ...(event?.outcome ? { eventOutcome: event.outcome } : {}), stateDigest: stateDigest(controller.state), state: clone(controller.state) });
      if (controller.phase === 'failed') { error = controller.error ?? 'Replay execution failed.'; break; }
    }
  } catch (cause) { error = cause instanceof Error ? cause.message : 'Replay failed.'; }
  const finalState = clone(controller.state);
  const result: ReplayResult = { schema: REPLAY_ENGINE_SCHEMA, realityRevision: reality.revision, scenarioId: scenario.id, initialSnapshotId: initialSnapshot.id, scenarioVersion: scenario.version, frames, finalState, deterministic: error === undefined, traceDigest: '', ...(error ? { error } : {}) };
  result.traceDigest = digest(replayComparable(result));
  return result;
}

export function replayStateAt(result: ReplayResult, sequence: number): AuthoritativeEngineeringState | undefined { if (!Number.isInteger(sequence) || sequence < 0 || sequence >= result.frames.length) return undefined; return clone(result.frames[sequence].state); }
export function replayFrameAtTime(result: ReplayResult, logicalTimeMs: number): ReplayFrame | undefined { if (!Number.isFinite(logicalTimeMs) || logicalTimeMs < 0) return undefined; return [...result.frames].filter((frame) => frame.logicalTimeMs <= logicalTimeMs).sort((a, b) => a.logicalTimeMs - b.logicalTimeMs || a.sequence - b.sequence).at(-1) ?? result.frames[0]; }
export function verifyReplayDeterminism(first: ReplayResult, second: ReplayResult): { deterministic: boolean; reason: string } {
  if (first.schema !== REPLAY_ENGINE_SCHEMA || second.schema !== REPLAY_ENGINE_SCHEMA) return { deterministic: false, reason: 'Replay schema mismatch.' };
  if (first.realityRevision !== second.realityRevision || first.scenarioId !== second.scenarioId || first.scenarioVersion !== second.scenarioVersion || first.initialSnapshotId !== second.initialSnapshotId) return { deterministic: false, reason: 'Replay identity mismatch.' };
  if (first.frames.length !== second.frames.length) return { deterministic: false, reason: 'Replay frame count differs.' };
  for (let index = 0; index < first.frames.length; index += 1) {
    const left = first.frames[index]; const right = second.frames[index];
    if (left.sequence !== right.sequence || left.logicalTimeMs !== right.logicalTimeMs || left.stateRevision !== right.stateRevision || left.actionId !== right.actionId || left.eventType !== right.eventType || left.eventOutcome !== right.eventOutcome || left.stateDigest !== right.stateDigest) return { deterministic: false, reason: `Replay diverged at frame ${index}.` };
  }
  if (first.traceDigest !== second.traceDigest) return { deterministic: false, reason: 'Replay trace digest mismatch.' };
  return { deterministic: first.deterministic && second.deterministic, reason: first.deterministic && second.deterministic ? 'Replay state and event traces match.' : 'Replay runs match, but one or both runs failed.' };
}
