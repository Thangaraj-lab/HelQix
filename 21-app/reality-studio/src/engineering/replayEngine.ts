import type { EngineeringReality } from '../api';
import { captureSnapshot, restoreSnapshot, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
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
  state: AuthoritativeEngineeringState;
}

export interface ReplayResult {
  schema: typeof REPLAY_ENGINE_SCHEMA;
  realityRevision: number;
  scenarioId: string;
  frames: ReplayFrame[];
  finalState: AuthoritativeEngineeringState;
  deterministic: boolean;
  error?: string;
}

function replayComparable(result: ReplayResult): unknown {
  return {
    realityRevision: result.realityRevision,
    scenarioId: result.scenarioId,
    frames: result.frames.map((frame) => ({
      sequence: frame.sequence,
      logicalTimeMs: frame.logicalTimeMs,
      stateRevision: frame.stateRevision,
      actionId: frame.actionId,
      eventType: frame.eventType,
      state: frame.state,
    })),
  };
}

export function replayExecution(reality: EngineeringReality, scenario: ExecutableScenario, initialSnapshot: StateSnapshot): ReplayResult {
  const restored = restoreSnapshot(initialSnapshot);
  let controller: ExecutionController = {
    schema: 'helqix.execution-controller.v1', phase: 'ready', realityRevision: reality.revision,
    scenarioId: scenario.id, cursor: 0, logicalTimeMs: restored.logicalTimeMs, state: restored,
    initialSnapshot: captureSnapshot(restored, 'initial', initialSnapshot.id), checkpoints: [initialSnapshot], events: [],
  };
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
    return { schema: REPLAY_ENGINE_SCHEMA, realityRevision: reality.revision, scenarioId: scenario.id, frames, finalState: structuredClone(controller.state), deterministic: controller.phase !== 'failed', ...(controller.error ? { error: controller.error } : {}) };
  } catch (error) {
    return { schema: REPLAY_ENGINE_SCHEMA, realityRevision: reality.revision, scenarioId: scenario.id, frames, finalState: restored, deterministic: false, error: error instanceof Error ? error.message : 'Replay failed.' };
  }
}

export function replayStateAt(result: ReplayResult, sequence: number): AuthoritativeEngineeringState | undefined {
  if (!Number.isInteger(sequence) || sequence < 0 || sequence >= result.frames.length) return undefined;
  return structuredClone(result.frames[sequence].state);
}

export function replayFrameAtTime(result: ReplayResult, logicalTimeMs: number): ReplayFrame | undefined {
  if (!Number.isFinite(logicalTimeMs) || logicalTimeMs < 0) return undefined;
  return [...result.frames].filter((frame) => frame.logicalTimeMs <= logicalTimeMs).pop() ?? result.frames[0];
}

export function verifyReplayDeterminism(first: ReplayResult, second: ReplayResult): { deterministic: boolean; reason: string } {
  if (first.schema !== REPLAY_ENGINE_SCHEMA || second.schema !== REPLAY_ENGINE_SCHEMA) return { deterministic: false, reason: 'Replay schema mismatch.' };
  if (first.realityRevision !== second.realityRevision || first.scenarioId !== second.scenarioId) return { deterministic: false, reason: 'Replay identity mismatch.' };
  const left = JSON.stringify(replayComparable(first));
  const right = JSON.stringify(replayComparable(second));
  return left === right
    ? { deterministic: first.deterministic && second.deterministic, reason: 'Replay state and event traces match.' }
    : { deterministic: false, reason: 'Replay state or event traces diverged.' };
}
