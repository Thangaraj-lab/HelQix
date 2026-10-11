import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, ComponentRuntimeState } from './authoritativeState';
import { assertValidState } from './authoritativeState';
import type { RuntimeEntity } from '../visualRuntime';

export interface RuntimeEntityState extends RuntimeEntity {
  status: ComponentRuntimeState['status'];
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

/**
 * Projects authoritative engineering state into UI-safe runtime entities.
 * No physics, telemetry, geometry, health, or failure is inferred here.
 * Missing runtime data remains explicit as missing/unknown.
 */
export function bindAuthoritativeStateToReality(
  reality: EngineeringReality,
  state: AuthoritativeEngineeringState,
  selectedComponentId?: string | null,
): VisualStateBindingResult {
  assertValidState(state, reality);
  const runtimeByComponent = new Map(state.components.map((component) => [component.componentId, component]));

  const entities: RuntimeEntityState[] = reality.components.map((component) => {
    const runtime = runtimeByComponent.get(component.id);
    return {
      id: component.id,
      name: component.name,
      componentType: component.componentType,
      status: runtime?.status ?? 'unknown',
      propertyValues: Object.fromEntries((runtime?.properties ?? []).map((property) => [property.name, property.value])),
      faults: [...(runtime?.faults ?? [])],
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

export function runtimeEntityById(
  binding: VisualStateBindingResult,
  componentId: string,
): RuntimeEntityState | undefined {
  return binding.entities.find((entity) => entity.id === componentId);
}

export function runtimeStatusLabel(status: ComponentRuntimeState['status']): string {
  switch (status) {
    case 'running': return 'RUNNING';
    case 'paused': return 'PAUSED';
    case 'stopped': return 'STOPPED';
    case 'failed': return 'FAILED';
    case 'completed': return 'COMPLETED';
    case 'idle': return 'IDLE';
    default: return 'UNKNOWN';
  }
}

export function runtimeStatusHealth(status: ComponentRuntimeState['status']): 'unknown' | 'nominal' | 'warning' | 'critical' | 'failed' {
  switch (status) {
    case 'failed': return 'failed';
    case 'running':
    case 'completed': return 'nominal';
    case 'paused': return 'warning';
    case 'stopped':
    case 'idle': return 'unknown';
    default: return 'unknown';
  }
}

export function stateBindingRevision(binding: VisualStateBindingResult): string {
  return `R${binding.realityRevision} · S${binding.stateRevision} · T+${(binding.logicalTimeMs / 1000).toFixed(3)}s`;
}
