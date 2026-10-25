import type { EngineeringReality } from '../api';
import {
  addFault,
  advanceLogicalTime,
  assertValidState,
  clearFault,
  setComponentProperty,
  setComponentStatus,
  setEnvironmentValue,
  setSignal,
  setStateStatus,
  type AuthoritativeEngineeringState,
  type RuntimeStatus,
  type ScalarValue,
} from './authoritativeState';
import { assertValidReality } from './engineeringReality';

export const SCENARIO_SCHEMA = 'helqix.executable-scenario.v1' as const;
export const SCENARIO_RESULT_SCHEMA = 'helqix.scenario-result.v1' as const;

export type ScenarioActionType =
  | 'set_property'
  | 'set_status'
  | 'advance_time'
  | 'set_environment'
  | 'inject_fault'
  | 'clear_fault'
  | 'set_signal'
  | 'checkpoint';

export interface ScenarioAction {
  id: string;
  type: ScenarioActionType;
  atMs: number;
  order?: number;
  componentId?: string;
  propertyName?: string;
  value?: ScalarValue;
  unit?: string;
  status?: RuntimeStatus;
  environmentKey?: string;
  faultId?: string;
  signalName?: string;
  signalSource?: 'model' | 'solver' | 'adapter' | 'scenario' | 'runtime';
  checkpointId?: string;
  enabled?: boolean;
  metadata?: Record<string, string | number | boolean>;
}

export interface ExecutableScenario {
  schema: typeof SCENARIO_SCHEMA;
  id: string;
  name: string;
  description: string;
  version: number;
  actions: ScenarioAction[];
  createdFromRealityRevision: number;
  metadata: {
    createdAt: string;
    updatedAt: string;
    author?: string;
    tags: string[];
  };
}

export interface ScenarioValidationIssue {
  actionId?: string;
  path: string;
  code: 'invalid_schema' | 'invalid_time' | 'duplicate_action' | 'unknown_component' | 'missing_argument' | 'invalid_value' | 'invalid_order';
  message: string;
  severity: 'error' | 'warning';
}

export interface ScenarioExecutionEvent {
  sequence: number;
  actionId: string;
  logicalTimeMs: number;
  type: ScenarioActionType;
  componentId?: string;
  outcome: 'applied' | 'failed' | 'skipped';
  message?: string;
  stateRevisionBefore: number;
  stateRevisionAfter: number;
}

export interface ScenarioCheckpoint {
  id: string;
  actionId: string;
  logicalTimeMs: number;
  stateRevision: number;
}

export interface ScenarioExecutionResult {
  schema: typeof SCENARIO_RESULT_SCHEMA;
  scenarioId: string;
  scenarioVersion: number;
  realityRevision: number;
  state: AuthoritativeEngineeringState;
  events: ScenarioExecutionEvent[];
  checkpoints: ScenarioCheckpoint[];
  status: 'completed' | 'failed' | 'cancelled';
  failedActionId?: string;
  startedAt: string;
  completedAt: string;
}

export interface ScenarioExecutionOptions {
  stopOnError?: boolean;
  validateBeforeExecution?: boolean;
  maxActions?: number;
  onEvent?: (event: ScenarioExecutionEvent, state: AuthoritativeEngineeringState) => void;
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function now(): string {
  return new Date().toISOString();
}

export function sortActions(actions: ScenarioAction[]): ScenarioAction[] {
  return [...actions]
    .filter((action) => action.enabled !== false)
    .sort((a, b) => a.atMs - b.atMs || (a.order ?? 0) - (b.order ?? 0) || a.id.localeCompare(b.id));
}

export function validateScenario(
  scenario: ExecutableScenario,
  reality: EngineeringReality,
): ScenarioValidationIssue[] {
  const issues: ScenarioValidationIssue[] = [];
  if (scenario.schema !== SCENARIO_SCHEMA) issues.push({ path: 'schema', code: 'invalid_schema', message: `Expected ${SCENARIO_SCHEMA}.`, severity: 'error' });
  if (!scenario.id.trim()) issues.push({ path: 'id', code: 'missing_argument', message: 'Scenario id is required.', severity: 'error' });
  if (!scenario.name.trim()) issues.push({ path: 'name', code: 'missing_argument', message: 'Scenario name is required.', severity: 'error' });
  if (!Number.isInteger(scenario.version) || scenario.version < 1) issues.push({ path: 'version', code: 'invalid_value', message: 'Scenario version must be a positive integer.', severity: 'error' });
  if (scenario.createdFromRealityRevision !== reality.revision) issues.push({ path: 'createdFromRealityRevision', code: 'invalid_value', message: `Scenario was created from reality revision ${scenario.createdFromRealityRevision}, current reality is ${reality.revision}.`, severity: 'error' });

  const componentIds = new Set(reality.components.map((component) => component.id));
  const actionIds = new Set<string>();
  let previousTime = -Infinity;
  for (const action of scenario.actions) {
    if (!action.id.trim()) issues.push({ actionId: action.id, path: 'actions.id', code: 'missing_argument', message: 'Action id is required.', severity: 'error' });
    if (actionIds.has(action.id)) issues.push({ actionId: action.id, path: `actions.${action.id}`, code: 'duplicate_action', message: 'Action ids must be unique.', severity: 'error' });
    actionIds.add(action.id);
    if (!Number.isFinite(action.atMs) || action.atMs < 0) issues.push({ actionId: action.id, path: `actions.${action.id}.atMs`, code: 'invalid_time', message: 'Action time must be a non-negative finite number.', severity: 'error' });
    if (action.atMs < previousTime) issues.push({ actionId: action.id, path: `actions.${action.id}.atMs`, code: 'invalid_order', message: 'Actions must be ordered by non-decreasing logical time.', severity: 'error' });
    previousTime = Math.max(previousTime, action.atMs);
    if (action.componentId && !componentIds.has(action.componentId)) issues.push({ actionId: action.id, path: `actions.${action.id}.componentId`, code: 'unknown_component', message: `Unknown component ${action.componentId}.`, severity: 'error' });
    switch (action.type) {
      case 'set_property':
        if (!action.componentId || !action.propertyName || action.value === undefined) issues.push({ actionId: action.id, path: `actions.${action.id}`, code: 'missing_argument', message: 'set_property requires componentId, propertyName, and value.', severity: 'error' });
        break;
      case 'set_status':
        if (!action.componentId || !action.status) issues.push({ actionId: action.id, path: `actions.${action.id}`, code: 'missing_argument', message: 'set_status requires componentId and status.', severity: 'error' });
        break;
      case 'set_environment':
        if (!action.environmentKey || action.value === undefined) issues.push({ actionId: action.id, path: `actions.${action.id}`, code: 'missing_argument', message: 'set_environment requires environmentKey and value.', severity: 'error' });
        break;
      case 'inject_fault':
      case 'clear_fault':
        if (!action.componentId || !action.faultId) issues.push({ actionId: action.id, path: `actions.${action.id}`, code: 'missing_argument', message: `${action.type} requires componentId and faultId.`, severity: 'error' });
        break;
      case 'set_signal':
        if (!action.componentId || !action.signalName || action.value === undefined || !action.signalSource) issues.push({ actionId: action.id, path: `actions.${action.id}`, code: 'missing_argument', message: 'set_signal requires componentId, signalName, value, and signalSource.', severity: 'error' });
        break;
      case 'checkpoint':
      case 'advance_time':
        break;
    }
  }
  return issues;
}

function executeAction(
  state: AuthoritativeEngineeringState,
  action: ScenarioAction,
): AuthoritativeEngineeringState {
  let next = advanceLogicalTime(state, action.atMs, 'scenario', action.id);
  switch (action.type) {
    case 'set_property':
      return setComponentProperty(next, action.componentId!, action.propertyName!, action.value!, action.unit, action.id, 'scenario');
    case 'set_status':
      return setComponentStatus(next, action.componentId!, action.status!, action.id, 'scenario');
    case 'advance_time':
      return next;
    case 'set_environment':
      return setEnvironmentValue(next, action.environmentKey!, action.value!, 'scenario');
    case 'inject_fault':
      return addFault(next, action.componentId!, action.faultId!, action.id, 'scenario');
    case 'clear_fault':
      return clearFault(next, action.componentId!, action.faultId!);
    case 'set_signal':
      return setSignal(next, action.componentId!, action.signalName!, action.value!, action.unit, action.signalSource!);
    case 'checkpoint':
      return next;
  }
}

export function executeScenario(
  scenario: ExecutableScenario,
  reality: EngineeringReality,
  initialState: AuthoritativeEngineeringState,
  options: ScenarioExecutionOptions = {},
): ScenarioExecutionResult {
  const startedAt = now();
  const stopOnError = options.stopOnError ?? true;
  const maxActions = options.maxActions ?? 10_000;
  const validation = options.validateBeforeExecution === false ? [] : validateScenario(scenario, reality);
  if (validation.some((issue) => issue.severity === 'error')) {
    return {
      schema: SCENARIO_RESULT_SCHEMA,
      scenarioId: scenario.id,
      scenarioVersion: scenario.version,
      realityRevision: reality.revision,
      state: setStateStatus(clone(initialState), 'failed'),
      events: validation.map((issue, index) => ({ sequence: index + 1, actionId: issue.actionId ?? 'validation', logicalTimeMs: initialState.logicalTimeMs, type: 'advance_time', outcome: 'failed', message: issue.message, stateRevisionBefore: initialState.stateRevision, stateRevisionAfter: initialState.stateRevision })),
      checkpoints: [],
      status: 'failed',
      startedAt,
      completedAt: now(),
    };
  }
  assertValidReality(reality);
  assertValidState(initialState, reality);
  const actions = sortActions(scenario.actions);
  if (actions.length > maxActions) throw new Error(`Scenario exceeds the execution limit of ${maxActions} actions.`);

  let state = clone(initialState);
  state.metadata.scenarioId = scenario.id;
  state.status = 'running';
  const events: ScenarioExecutionEvent[] = [];
  const checkpoints: ScenarioCheckpoint[] = [];
  let failedActionId: string | undefined;

  for (const action of actions) {
    const beforeRevision = state.stateRevision;
    try {
      state = executeAction(state, action);
      const event: ScenarioExecutionEvent = {
        sequence: events.length + 1,
        actionId: action.id,
        logicalTimeMs: state.logicalTimeMs,
        type: action.type,
        ...(action.componentId ? { componentId: action.componentId } : {}),
        outcome: 'applied',
        stateRevisionBefore: beforeRevision,
        stateRevisionAfter: state.stateRevision,
      };
      events.push(event);
      if (action.type === 'checkpoint') checkpoints.push({ id: action.checkpointId || `checkpoint-${action.id}`, actionId: action.id, logicalTimeMs: state.logicalTimeMs, stateRevision: state.stateRevision });
      options.onEvent?.(event, state);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Action failed.';
      failedActionId = action.id;
      const event: ScenarioExecutionEvent = {
        sequence: events.length + 1,
        actionId: action.id,
        logicalTimeMs: state.logicalTimeMs,
        type: action.type,
        ...(action.componentId ? { componentId: action.componentId } : {}),
        outcome: 'failed',
        message,
        stateRevisionBefore: beforeRevision,
        stateRevisionAfter: state.stateRevision,
      };
      events.push(event);
      options.onEvent?.(event, state);
      state = setStateStatus(state, 'failed');
      if (stopOnError) break;
    }
  }

  if (!failedActionId) state = setStateStatus(state, 'completed');
  return {
    schema: SCENARIO_RESULT_SCHEMA,
    scenarioId: scenario.id,
    scenarioVersion: scenario.version,
    realityRevision: reality.revision,
    state,
    events,
    checkpoints,
    status: failedActionId ? 'failed' : 'completed',
    ...(failedActionId ? { failedActionId } : {}),
    startedAt,
    completedAt: now(),
  };
}

export function createScenario(
  name: string,
  reality: EngineeringReality,
  actions: ScenarioAction[] = [],
  description = '',
): ExecutableScenario {
  const timestamp = now();
  return {
    schema: SCENARIO_SCHEMA,
    id: `scenario-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name: name.trim() || 'Untitled scenario',
    description: description.trim(),
    version: 1,
    actions: sortActions(clone(actions)),
    createdFromRealityRevision: reality.revision,
    metadata: { createdAt: timestamp, updatedAt: timestamp, tags: [] },
  };
}

export function appendScenarioAction(
  scenario: ExecutableScenario,
  action: ScenarioAction,
): ExecutableScenario {
  const next = clone(scenario);
  next.actions.push(clone(action));
  next.actions = sortActions(next.actions);
  next.version += 1;
  next.metadata.updatedAt = now();
  return next;
}

export function removeScenarioAction(scenario: ExecutableScenario, actionId: string): ExecutableScenario {
  const next = clone(scenario);
  next.actions = next.actions.filter((action) => action.id !== actionId);
  next.version += 1;
  next.metadata.updatedAt = now();
  return next;
}

export function scenarioDurationMs(scenario: ExecutableScenario): number {
  return scenario.actions.reduce((maximum, action) => Math.max(maximum, action.atMs), 0);
}
