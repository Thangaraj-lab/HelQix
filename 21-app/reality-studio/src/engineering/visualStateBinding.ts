import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, RuntimeStatus } from './authoritativeState';
import { assertValidState } from './authoritativeState';
import type { RuntimeEntity } from '../visualRuntime';

export interface RuntimeEntityState extends RuntimeEntity {
  status: RuntimeStatus | 'unknown';
  propertyValues: Record<string, string | number | boolean>;
  faults: string[];
  signals: Record<string, string | number | boolean>;
  selected: boolean;
  stateRevision: number;
  logicalTimeMs: number;
}

export interface VisualStateBindingResult {
  realityRevision: number;
  stateRevision: number;
  logicalTimeMs: number;
  entities: RuntimeEntityState[];
  environment: AuthoritativeEngineeringState['environment'];
  stateStatus: AuthoritativeEngineeringState['status'];
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

/** Projects authoritative engineering state into UI-safe runtime entities. No physics or failure is inferred. */
export function bindAuthoritativeStateToReality(
  reality: EngineeringReality,
  state: AuthoritativeEngineeringState,
  selectedComponentId?: string | null,
): VisualStateBindingResult {
  assertValidState(state, reality);
  const runtimeByComponent = new Map(Object.values(state.components).map((component) => [component.componentId, component]));

  const entities: RuntimeEntityState[] = reality.components.map((component) => {
    const runtime = runtimeByComponent.get(component.id);
    return {
      id: component.id,
      name: component.name,
      componentType: component.componentType,
      status: runtime?.status ?? 'unknown',
      propertyValues: Object.fromEntries((runtime?.properties ?? []).map((property) => [property.name, property.value])),
      faults: [...(runtime?.faultIds ?? [])],
      signals: Object.fromEntries(Object.entries(runtime?.signals ?? {}).map(([name, signal]) => [name, signal.value])),
      selected: selectedComponentId === component.id,
      stateRevision: state.stateRevision,
      logicalTimeMs: state.logicalTimeMs,
    };
  });

  return {
    realityRevision: reality.revision,
    stateRevision: state.stateRevision,
    logicalTimeMs: state.logicalTimeMs,
    entities,
    environment: clone(state.environment),
    stateStatus: state.status,
  };
}

export function runtimeEntityById(binding: VisualStateBindingResult, componentId: string): RuntimeEntityState | undefined {
  return binding.entities.find((entity) => entity.id === componentId);
}

export function runtimeStatusLabel(status: RuntimeEntityState['status']): string {
  switch (status) {
    case 'running': return 'RUNNING';
    case 'paused': return 'PAUSED';
    case 'failed': return 'FAILED';
    case 'faulted': return 'FAULTED';
    case 'completed': return 'COMPLETED';
    case 'ready': return 'READY';
    case 'idle': return 'IDLE';
    case 'unknown': return 'UNKNOWN';
  }
}

export function runtimeStatusHealth(status: RuntimeEntityState['status']): 'unknown' | 'nominal' | 'warning' | 'critical' | 'failed' {
  switch (status) {
    case 'failed': return 'failed';
    case 'faulted': return 'critical';
    case 'running':
    case 'completed': return 'nominal';
    case 'paused': return 'warning';
    case 'ready':
    case 'idle':
    case 'unknown': return 'unknown';
  }
}

export function stateBindingRevision(binding: VisualStateBindingResult): string {
  return `R${binding.realityRevision} · S${binding.stateRevision} · T+${(binding.logicalTimeMs / 1000).toFixed(3)}s`;
}
