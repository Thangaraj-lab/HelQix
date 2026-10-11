import type { EngineeringReality, RealityComponent, RealityProperty } from '../api';

export const AUTHORITATIVE_STATE_SCHEMA = 'helqix.authoritative-state.v1' as const;

export type RuntimeStatus = 'idle' | 'ready' | 'running' | 'paused' | 'completed' | 'failed' | 'faulted';

export interface RuntimeProperty {
  name: string;
  value: string | number | boolean;
  unit?: string;
  origin: RealityProperty['origin'] | 'runtime';
}

export interface ComponentRuntimeState {
  componentId: string;
  status: RuntimeStatus;
  properties: RuntimeProperty[];
  signals: Record<string, number | string | boolean>;
  faultIds: string[];
  lastActionId?: string;
}

export interface EngineeringEnvironmentState {
  values: Record<string, number | string | boolean>;
}

export interface AuthoritativeEngineeringState {
  schema: typeof AUTHORITATIVE_STATE_SCHEMA;
  realityRevision: number;
  stateRevision: number;
  logicalTimeMs: number;
  status: RuntimeStatus;
  components: Record<string, ComponentRuntimeState>;
  environment: EngineeringEnvironmentState;
  sourceScenarioId?: string;
  sourceExecutionId?: string;
}

export interface StateSnapshot {
  schema: 'helqix.state-snapshot.v1';
  id: string;
  capturedAt: string;
  state: AuthoritativeEngineeringState;
}

export interface StateValidationIssue {
  path: string;
  message: string;
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function componentState(component: RealityComponent): ComponentRuntimeState {
  return {
    componentId: component.id,
    status: 'ready',
    properties: component.properties.map((property) => ({
      name: property.name,
      value: property.value,
      ...(property.unit ? { unit: property.unit } : {}),
      origin: property.origin,
    })),
    signals: {},
    faultIds: [],
  };
}

/** Build the runtime state from the persisted engineering reality without inventing physics. */
export function createInitialState(reality: EngineeringReality): AuthoritativeEngineeringState {
  const components: Record<string, ComponentRuntimeState> = {};
  for (const component of reality.components) components[component.id] = componentState(component);
  return {
    schema: AUTHORITATIVE_STATE_SCHEMA,
    realityRevision: reality.revision,
    stateRevision: 1,
    logicalTimeMs: 0,
    status: 'ready',
    components,
    environment: { values: {} },
  };
}

export function cloneState(state: AuthoritativeEngineeringState): AuthoritativeEngineeringState {
  return clone(state);
}

export function setComponentStatus(
  state: AuthoritativeEngineeringState,
  componentId: string,
  status: RuntimeStatus,
  actionId?: string,
): AuthoritativeEngineeringState {
  const component = state.components[componentId];
  if (!component) throw new Error(`Unknown component: ${componentId}`);
  const next = clone(state);
  next.components[componentId].status = status;
  if (actionId) next.components[componentId].lastActionId = actionId;
  next.stateRevision += 1;
  return next;
}

export function setComponentProperty(
  state: AuthoritativeEngineeringState,
  componentId: string,
  name: string,
  value: string | number | boolean,
  unit?: string,
  actionId?: string,
): AuthoritativeEngineeringState {
  const component = state.components[componentId];
  if (!component) throw new Error(`Unknown component: ${componentId}`);
  const next = clone(state);
  const existing = next.components[componentId].properties.find((property) => property.name === name);
  if (existing) {
    existing.value = value;
    if (unit) existing.unit = unit;
    existing.origin = 'runtime';
  } else {
    next.components[componentId].properties.push({ name, value, ...(unit ? { unit } : {}), origin: 'runtime' });
  }
  if (actionId) next.components[componentId].lastActionId = actionId;
  next.stateRevision += 1;
  return next;
}

export function setEnvironmentValue(
  state: AuthoritativeEngineeringState,
  name: string,
  value: number | string | boolean,
): AuthoritativeEngineeringState {
  const next = clone(state);
  next.environment.values[name] = value;
  next.stateRevision += 1;
  return next;
}

export function advanceLogicalTime(state: AuthoritativeEngineeringState, logicalTimeMs: number): AuthoritativeEngineeringState {
  if (!Number.isFinite(logicalTimeMs) || logicalTimeMs < state.logicalTimeMs) {
    throw new Error('Logical time must be finite and monotonic.');
  }
  const next = clone(state);
  next.logicalTimeMs = logicalTimeMs;
  next.stateRevision += 1;
  return next;
}

export function addFault(state: AuthoritativeEngineeringState, componentId: string, faultId: string, actionId?: string): AuthoritativeEngineeringState {
  const component = state.components[componentId];
  if (!component) throw new Error(`Unknown component: ${componentId}`);
  const next = clone(state);
  if (!next.components[componentId].faultIds.includes(faultId)) next.components[componentId].faultIds.push(faultId);
  next.components[componentId].status = 'faulted';
  if (actionId) next.components[componentId].lastActionId = actionId;
  next.stateRevision += 1;
  return next;
}

export function captureSnapshot(state: AuthoritativeEngineeringState, id = `snapshot-${state.stateRevision}`): StateSnapshot {
  return {
    schema: 'helqix.state-snapshot.v1',
    id,
    capturedAt: new Date().toISOString(),
    state: clone(state),
  };
}

export function restoreSnapshot(snapshot: StateSnapshot): AuthoritativeEngineeringState {
  if (snapshot.schema !== 'helqix.state-snapshot.v1') throw new Error(`Unsupported snapshot schema: ${snapshot.schema}`);
  return clone(snapshot.state);
}

export function validateState(state: AuthoritativeEngineeringState, reality: EngineeringReality): StateValidationIssue[] {
  const issues: StateValidationIssue[] = [];
  if (state.schema !== AUTHORITATIVE_STATE_SCHEMA) issues.push({ path: 'schema', message: 'Unsupported authoritative-state schema.' });
  if (state.realityRevision !== reality.revision) issues.push({ path: 'realityRevision', message: `State revision ${state.realityRevision} does not match reality revision ${reality.revision}.` });
  const knownIds = new Set(reality.components.map((component) => component.id));
  for (const componentId of Object.keys(state.components)) {
    if (!knownIds.has(componentId)) issues.push({ path: `components.${componentId}`, message: 'State references a component absent from the authoritative reality.' });
  }
  for (const component of reality.components) {
    if (!state.components[component.id]) issues.push({ path: `components.${component.id}`, message: 'Reality component has no runtime state.' });
  }
  if (!Number.isFinite(state.logicalTimeMs) || state.logicalTimeMs < 0) issues.push({ path: 'logicalTimeMs', message: 'Logical time must be a non-negative finite number.' });
  return issues;
}
