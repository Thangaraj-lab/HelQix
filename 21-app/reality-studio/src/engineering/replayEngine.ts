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

/** Replays a scenario from a captured initial state without mutating the supplied controller. */
export function replayExecution(
  reality: EngineeringReality,
  scenario: ExecutableScenario,
  initialSnapshot: StateSnapshot,
): ReplayResult {
  const restored = restoreSnapshot(initialSnapshot);
  let controller: ExecutionController = {
    schema: 'helqix.execution-controller.v1',
    phase: 'ready',
    realityRevision: reality.revision,
    scenarioId: scenario.id,
    cursor: 0,
    logicalTimeMs: restored.logicalTimeMs,
    state: restored,
    initialSnapshot: captureSnapshot(restored, 'initial', initialSnapshot.id),
    checkpoints: [initialSnapshot],
    events: [],
  };
  const frames: ReplayFrame[] = [{ sequence: 0, logicalTimeMs: restored.logicalTimeMs, stateRevision: restored.stateRevision, snapshotId: initialSnapshot.id }];
  try {
    while (controller.cursor < scenario.actions.length && controller.phase !== 'failed') {
      controller = { ...controller, phase: 'running' };
      const result = stepExecution(reality, scenario, controller);
      controller = result.controller;
      const event = result.event;
      const snapshot = captureSnapshot(controller.state, 'replay-frame', `replay-${scenario.id}-${frames.length}`);
      frames.push({ sequence: frames.length, logicalTimeMs: controller.logicalTimeMs, stateRevision: controller.state.stateRevision, ...(event?.actionId ? { actionId: event.actionId } : {}), snapshotId: snapshot.id, ...(event?.type ? { eventType: event.type } : {}) });
    }
    return { schema: REPLAY_ENGINE_SCHEMA, realityRevision: reality.revision, scenarioId: scenario.id, frames, finalState: restoreSnapshot(captureSnapshot(controller.state, 'replay-final', `replay-final-${scenario.id}`)), deterministic: controller.phase !== 'failed', ...(controller.error ? { error: controller.error } : {}) };
  } catch (error) {
    return { schema: REPLAY_ENGINE_SCHEMA, realityRevision: reality.revision, scenarioId: scenario.id, frames, finalState: restored, deterministic: false, error: error instanceof Error ? error.message : 'Replay failed.' };
  }
}

export function replayStateAt(result: ReplayResult, sequence: number): AuthoritativeEngineeringState | undefined {
  if (!Number.isInteger(sequence) || sequence < 0 || sequence >= result.frames.length) return undefined;
  if (sequence === result.frames.length - 1) return result.finalState;
  return undefined;
}
