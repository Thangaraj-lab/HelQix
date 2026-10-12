import type { EngineeringReality } from '../api';
import { captureSnapshot, restoreSnapshot, setStateStatus, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import { executeScenario, sortActions, type ExecutableScenario, type ScenarioAction, type ScenarioExecutionEvent } from './scenarioEngine';

export const EXECUTION_CONTROLLER_SCHEMA = 'helqix.execution-controller.v1' as const;
export type ExecutionPhase = 'idle' | 'ready' | 'running' | 'paused' | 'completed' | 'failed';

export interface ExecutionController {
  schema: typeof EXECUTION_CONTROLLER_SCHEMA;
  phase: ExecutionPhase;
  realityRevision: number;
  scenarioId?: string;
  cursor: number;
  logicalTimeMs: number;
  state: AuthoritativeEngineeringState;
  initialSnapshot: StateSnapshot;
  checkpoints: StateSnapshot[];
  events: ScenarioExecutionEvent[];
  error?: string;
}

export interface ExecutionStepResult { controller: ExecutionController; action?: ScenarioAction; event?: ScenarioExecutionEvent; }
function clone<T>(value: T): T { return structuredClone(value); }

export function createExecutionController(reality: EngineeringReality, scenario: ExecutableScenario, initialState: AuthoritativeEngineeringState): ExecutionController {
  const initialSnapshot = captureSnapshot(initialState, 'initial', `execution-initial-${scenario.id}`);
  return { schema: EXECUTION_CONTROLLER_SCHEMA, phase: 'ready', realityRevision: reality.revision, scenarioId: scenario.id, cursor: 0, logicalTimeMs: initialState.logicalTimeMs, state: clone(initialState), initialSnapshot, checkpoints: [initialSnapshot], events: [] };
}

export function startExecution(controller: ExecutionController): ExecutionController {
  if (controller.phase === 'completed' || controller.phase === 'failed') return controller;
  return { ...clone(controller), phase: 'running', error: undefined };
}
export function pauseExecution(controller: ExecutionController): ExecutionController {
  return controller.phase === 'running' ? { ...clone(controller), phase: 'paused' } : controller;
}
export function resetExecution(controller: ExecutionController): ExecutionController {
  const restored = restoreSnapshot(controller.initialSnapshot);
  return { ...clone(controller), phase: 'ready', cursor: 0, logicalTimeMs: restored.logicalTimeMs, state: restored, checkpoints: [controller.initialSnapshot], events: [], error: undefined };
}

export function stepExecution(reality: EngineeringReality, scenario: ExecutableScenario, controller: ExecutionController): ExecutionStepResult {
  const actions = sortActions(scenario.actions);
  if (controller.cursor >= actions.length) return { controller: { ...clone(controller), phase: 'completed' } };
  const action = actions[controller.cursor];
  const before = controller.state;
  try {
    const result = executeScenario({ ...scenario, actions: [action] }, reality, before, { stopOnError: true, validateBeforeExecution: false });
    if (result.status === 'failed') {
      return { action, controller: { ...clone(controller), phase: 'failed', state: setStateStatus(result.state, 'failed'), logicalTimeMs: result.state.logicalTimeMs, events: [...controller.events, ...result.events], error: result.events.find((event) => event.outcome === 'failed')?.message ?? 'Scenario action failed.' } };
    }
    const after = result.state;
    const event: ScenarioExecutionEvent = { sequence: controller.events.length + 1, actionId: action.id, logicalTimeMs: after.logicalTimeMs, type: action.type, ...(action.componentId ? { componentId: action.componentId } : {}), outcome: 'applied', stateRevisionBefore: before.stateRevision, stateRevisionAfter: after.stateRevision };
    const checkpoints = [...controller.checkpoints];
    if (action.type === 'checkpoint') checkpoints.push(captureSnapshot(after, 'checkpoint', action.checkpointId || `checkpoint-${action.id}`));
    const cursor = controller.cursor + 1;
    return { action, event, controller: { ...clone(controller), phase: cursor >= actions.length ? 'completed' : controller.phase === 'running' ? 'running' : 'paused', cursor, logicalTimeMs: after.logicalTimeMs, state: after, checkpoints, events: [...controller.events, event], error: undefined } };
  } catch (error) {
    return { action, controller: { ...clone(controller), phase: 'failed', state: setStateStatus(before, 'failed'), error: error instanceof Error ? error.message : 'Execution step failed.' } };
  }
}

export function runToCompletion(reality: EngineeringReality, scenario: ExecutableScenario, controller: ExecutionController): ExecutionController {
  let next = startExecution(controller);
  while (next.phase === 'running') next = stepExecution(reality, scenario, next).controller;
  return next;
}

export function restoreExecutionCheckpoint(controller: ExecutionController, snapshotId: string): ExecutionController {
  const snapshot = controller.checkpoints.find((item) => item.id === snapshotId);
  if (!snapshot) throw new Error(`Unknown execution checkpoint: ${snapshotId}`);
  const state = restoreSnapshot(snapshot);
  const lastActionId = [...state.events].reverse().find((event) => event.actionId)?.actionId;
  const restoredEventIndex = lastActionId ? controller.events.findIndex((event) => event.actionId === lastActionId) : -1;
  const cursor = restoredEventIndex >= 0 ? restoredEventIndex + 1 : 0;
  return { ...clone(controller), phase: 'paused', cursor, logicalTimeMs: state.logicalTimeMs, state, error: undefined };
}
