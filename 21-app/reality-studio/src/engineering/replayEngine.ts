import type { EngineeringReality } from '../api';
import { captureSnapshot, restoreSnapshot, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import { stepExecution, type ExecutionController } from './executionController';
import type { ExecutableScenario } from './scenarioEngine';

export const REPLAY_ENGINE_SCHEMA = 'helqix.replay-engine.v1' as const;

export interface ReplayFrame { sequence: number; logicalTimeMs: number; stateRevision: number; actionId?: string; snapshotId: string; eventType?: string; state: AuthoritativeEngineeringState; }
export interface ReplayResult { schema: typeof REPLAY_ENGINE_SCHEMA; realityRevision: number; scenarioId: string; initialSnapshotId: string; scenarioVersion?: number; frames: ReplayFrame[]; finalState: AuthoritativeEngineeringState; deterministic: boolean; traceDigest: string; error?: string; }
function digest(value: unknown): string { const serialized = JSON.stringify(value); let hash = 2166136261; for (let index = 0; index < serialized.length; index += 1) { hash ^= serialized.charCodeAt(index); hash = Math.imul(hash, 16777619); } return `fnv1a-${(hash >>> 0).toString(16).padStart(8, '0')}`; }
function replayComparable(result: ReplayResult): unknown { return { realityRevision: result.realityRevision, scenarioId: result.scenarioId, scenarioVersion: result.scenarioVersion, initialSnapshotId: result.initialSnapshotId, frames: result.frames.map((frame) => ({ sequence: frame.sequence, logicalTimeMs: frame.logicalTimeMs, stateRevision: frame.stateRevision, actionId: frame.actionId, eventType: frame.eventType, state: frame.state })) }; }

export function replayExecution(reality: EngineeringReality, scenario: ExecutableScenario, initialSnapshot: StateSnapshot): ReplayResult {
  if (initialSnapshot.state.realityRevision !== reality.revision) throw new Error('Replay snapshot belongs to a different reality revision.');
  const restored = restoreSnapshot(initialSnapshot);
  let controller: ExecutionController = { schema: 'helqix.execution-controller.v1', phase: 'ready', realityRevision: reality.revision, scenarioId: scenario.id, cursor: 0, logicalTimeMs: restored.logicalTimeMs, state: restored, initialSnapshot: captureSnapshot(restored, 'initial', initialSnapshot.id), checkpoints: [initialSnapshot], events: [] };
  const frames: ReplayFrame[] = [{ sequence: 0, logicalTimeMs: restored.logicalTimeMs, stateRevision: restored.stateRevision, snapshotId: initialSnapshot.id, state: structuredClone(restored) }];
  try {
    while (controller.cursor < scenario.actions.length && controller.phase !== 'failed') {
      controller = { ...controller, phase: 'running' };
      const result = stepExecution(reality, scenario, controller);
      controller = result.controller;
      const event = result.event;
      const snapshot = captureSnapshot(controller.state, 'scenario', `replay-${scenario.id}-${frames.length}`);
      frames.push({ sequence: frames.length, logicalTimeMs: controller.logicalTimeMs, stateRevision: controller.state.stateRevision, ...(event?.actionId ? { actionId: event.actionId } : {}), snapshotId: snapshot.id, ...(event?.type ? { eventType: event.type } : {}), state: structuredClone(controller.state) });
    }
    const finalState = structuredClone(controller.state);
    const partial: ReplayResult = { schema: REPLAY_ENGINE_SCHEMA, realityRevision: reality.revision, scenarioId: scenario.id, initialSnapshotId: initialSnapshot.id, scenarioVersion: scenario.version, frames, finalState, deterministic: controller.phase !== 'failed', traceDigest: '' , ...(controller.error ? { error: controller.error } : {}) };
    partial.traceDigest = digest(replayComparable(partial));
    return partial;
  } catch (error) {
    const partial: ReplayResult = { schema: REPLAY_ENGINE_SCHEMA, realityRevision: reality.revision, scenarioId: scenario.id, initialSnapshotId: initialSnapshot.id, scenarioVersion: scenario.version, frames, finalState: restored, deterministic: false, traceDigest: '', error: error instanceof Error ? error.message : 'Replay failed.' };
    partial.traceDigest = digest(replayComparable(partial));
    return partial;
  }
}
export function replayStateAt(result: ReplayResult, sequence: number): AuthoritativeEngineeringState | undefined { if (!Number.isInteger(sequence) || sequence < 0 || sequence >= result.frames.length) return undefined; return structuredClone(result.frames[sequence].state); }
export function replayFrameAtTime(result: ReplayResult, logicalTimeMs: number): ReplayFrame | undefined { if (!Number.isFinite(logicalTimeMs) || logicalTimeMs < 0) return undefined; return [...result.frames].filter((frame) => frame.logicalTimeMs <= logicalTimeMs).pop() ?? result.frames[0]; }
export function verifyReplayDeterminism(first: ReplayResult, second: ReplayResult): { deterministic: boolean; reason: string } { if (first.schema !== REPLAY_ENGINE_SCHEMA || second.schema !== REPLAY_ENGINE_SCHEMA) return { deterministic: false, reason: 'Replay schema mismatch.' }; if (first.realityRevision !== second.realityRevision || first.scenarioId !== second.scenarioId || first.scenarioVersion !== second.scenarioVersion) return { deterministic: false, reason: 'Replay identity mismatch.' }; const left = digest(replayComparable(first)); const right = digest(replayComparable(second)); return left === right && first.traceDigest === second.traceDigest ? { deterministic: first.deterministic && second.deterministic, reason: 'Replay state and event traces match.' } : { deterministic: false, reason: 'Replay state or event traces diverged.' }; }
