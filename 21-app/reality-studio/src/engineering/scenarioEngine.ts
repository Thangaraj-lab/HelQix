import type { EngineeringReality } from '../api';
import {
  addFault,
  advanceLogicalTime,
  setComponentProperty,
  setComponentStatus,
  type AuthoritativeEngineeringState,
} from './authoritativeState';

export const SCENARIO_SCHEMA = 'helqix.executable-scenario.v1' as const;

export type ScenarioActionType =
  | 'set_property'
  | 'set_status'
  | 'advance_time'
  | 'set_environment'
  | 'inject_fault';

export interface ScenarioAction {
  id: string;
  type: ScenarioActionType;
  atMs: number;
  componentId?: string;
  propertyName?: string;
  value?: string | number | boolean;
  unit?: string;
  status?: 'idle' | 'ready' | 'running' | 'paused' | 'completed' | 'failed' | 'faulted';
  environmentKey?: string;
  faultId?: string;
}

export interface ExecutableScenario {
  schema: typeof SCENARIO_SCHEMA;
  id: string;
  name: string;
  description: string;
  actions: ScenarioAction[];
  createdFromRealityRevision: number;
}

export interface ScenarioValidationIssue {
  actionId: string;
  message: string;
}

export interface ScenarioExecutionEvent {
  actionId: string;
  logicalTimeMs: number;
  type: ScenarioActionType;
  componentId?: string;
  outcome: 'applied' | 'failed';
  message?: string;
}

export interface ScenarioExecutionResult {
  scenarioId: string;
  state: AuthoritativeEngineeringState;
  events: ScenarioExecutionEvent[];
  status: 'completed' | 'failed';
}

export function validateScenario(scenario: ExecutableScenario, reality: EngineeringReality): ScenarioValidationIssue[] {
  const issues: ScenarioValidationIssue[] = [];
  const components = new Set(reality.components.map((component) => component.id));
  let previousTime = 0;
  for (const action of scenario.actions) {
    if (!action.id.trim()) issues.push({ actionId: action.id, message: 'Action ID is required.' });
    if (!Number.isFinite(action.atMs) || action.atMs < 0) issues.push({ actionId: action.id, message: 'Action time must be a non-negative finite number.' });
    if (action.atMs < previousTime) issues.push({ actionId: action.id, message: 'Actions must be ordered by non-decreasing logical time.' });
    previousTime = action.atMs;
    if (action.componentId && !components.has(action.componentId)) issues.push({ actionId: action.id, message: `Unknown component: ${action.componentId}` });
    if (action.type === 'set_property' && (!action.componentId || !action.propertyName || action.value === undefined)) {
      issues.push({ actionId: action.id, message: 'set_property requires componentId, propertyName, and value.' });
    }
    if (action.type === 'set_status' && (!action.componentId || !action.status)) issues.push({ actionId: action.id, message: 'set_status requires componentId and status.' });
    if (action.type === 'set_environment' && (!action.environmentKey || action.value === undefined)) issues.push({ actionId: action.id, message: 'set_environment requires environmentKey and value.' });
    if (action.type === 'inject_fault' && (!action.componentId || !action.faultId)) issues.push({ actionId: action.id, message: 'inject_fault requires componentId and faultId.' });
    if (action.type === 'advance_time' && action.atMs < previousTime) issues.push({ actionId: action.id, message: 'advance_time cannot move logical time backwards.' });
  }
  return issues;
}

export function sortActions(actions: ScenarioAction[]): ScenarioAction[] {
  return [...actions].sort((a, b) => a.atMs - b.atMs || a.id.localeCompare(b.id));
}

/** Execute declared actions against authoritative state. It does not fabricate solver or physics results. */
export function executeScenario(
  scenario: ExecutableScenario,
  reality: EngineeringReality,
  initialState: AuthoritativeEngineeringState,
): ScenarioExecutionResult {
  const issues = validateScenario(scenario, reality);
  if (issues.length) {
    return {
      scenarioId: scenario.id,
      state: initialState,
      events: issues.map((issue) => ({ actionId: issue.actionId, logicalTimeMs: initialState.logicalTimeMs, type: 'advance_time', outcome: 'failed', message: issue.message })),
      status: 'failed',
    };
  }

  let state = initialState;
  const events: ScenarioExecutionEvent[] = [];
  for (const action of sortActions(scenario.actions)) {
    try {
      if (action.atMs < state.logicalTimeMs) throw new Error('Action time is behind current logical time.');
      state = advanceLogicalTime(state, action.atMs);
      switch (action.type) {
        case 'set_property':
          state = setComponentProperty(state, action.componentId!, action.propertyName!, action.value!, action.unit, action.id);
          break;
        case 'set_status':
          state = setComponentStatus(state, action.componentId!, action.status!, action.id);
          break;
        case 'advance_time':
          break;
        case 'set_environment':
          state = { ...state, environment: { values: { ...state.environment.values, [action.environmentKey!]: action.value! } }, stateRevision: state.stateRevision + 1 };
          break;
        case 'inject_fault':
          state = addFault(state, action.componentId!, action.faultId!, action.id);
          break;
      }
      events.push({ actionId: action.id, logicalTimeMs: state.logicalTimeMs, type: action.type, ...(action.componentId ? { componentId: action.componentId } : {}), outcome: 'applied' });
    } catch (error) {
      events.push({ actionId: action.id, logicalTimeMs: state.logicalTimeMs, type: action.type, ...(action.componentId ? { componentId: action.componentId } : {}), outcome: 'failed', message: error instanceof Error ? error.message : 'Action failed.' });
      return { scenarioId: scenario.id, state: { ...state, status: 'failed' }, events, status: 'failed' };
    }
  }
  return { scenarioId: scenario.id, state: { ...state, status: 'completed' }, events, status: 'completed' };
}

export function createScenario(name: string, reality: EngineeringReality, actions: ScenarioAction[] = []): ExecutableScenario {
  return {
    schema: SCENARIO_SCHEMA,
    id: `scenario-${Date.now()}`,
    name: name.trim() || 'Untitled scenario',
    description: '',
    actions: sortActions(actions),
    createdFromRealityRevision: reality.revision,
  };
}
