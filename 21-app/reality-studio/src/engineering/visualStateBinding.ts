import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, RuntimeStatus } from './authoritativeState';
import { assertValidState } from './authoritativeState';
import type { RuntimeEntity } from '../visualRuntime';

export interface RuntimeMotionEvent {
  logicalTimeMs: number;
  name: string;
  value: number;
  source: 'model' | 'solver' | 'adapter' | 'scenario' | 'runtime';
}

export interface RuntimeEntityState extends RuntimeEntity {
  status: RuntimeStatus | 'unknown';
  propertyValues: Record<string, string | number | boolean>;
  faults: string[];
  signals: Record<string, string | number | boolean>;
  selected: boolean;
  stateRevision: number;
  logicalTimeMs: number;
  motionEvents: RuntimeMotionEvent[];
}

export interface VisualStateBindingResult {
  realityRevision: number;
  stateRevision: number;
  logicalTimeMs: number;
  entities: RuntimeEntityState[];
  environment: AuthoritativeEngineeringState['environment'];
  stateStatus: AuthoritativeEngineeringState['status'];
}

function clone<T>(value: T): T { return structuredClone(value); }
function normalize(name: string): string { return name.toLowerCase().replace(/[\s_-]/g, ''); }
const RPM_ALIASES = new Set(['rpm', 'rotationalspeed', 'rotationsperminute', 'shaftspeed']);
const VELOCITY_ALIASES = new Set(['velocityms', 'speedms', 'linearvelocity']);

/** Projects authoritative engineering state into UI-safe runtime entities. No physics or failure is inferred. */
export function bindAuthoritativeStateToReality(
  reality: EngineeringReality,
  state: AuthoritativeEngineeringState,
  selectedComponentId?: string | null,
): VisualStateBindingResult {
  assertValidState(state, reality);
  const runtimeByComponent = new Map(Object.values(state.components).map((component) => [component.componentId, component]));
  const motionByComponent = new Map<string, RuntimeMotionEvent[]>();
  for (const event of state.events) {
    if (event.type !== 'signal_updated' || !event.componentId || typeof event.after !== 'number' || !Number.isFinite(event.after)) continue;
    const name = normalize(event.name ?? '');
    if (!RPM_ALIASES.has(name) && !VELOCITY_ALIASES.has(name)) continue;
    const events = motionByComponent.get(event.componentId) ?? [];
    events.push({ logicalTimeMs: event.logicalTimeMs, name: event.name ?? '', value: event.after, source: event.source });
    motionByComponent.set(event.componentId, events);
  }
  for (const events of motionByComponent.values()) events.sort((a, b) => a.logicalTimeMs - b.logicalTimeMs);

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
      motionEvents: clone(motionByComponent.get(component.id) ?? []),
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

export function runtimeEntityById(binding: VisualStateBindingResult, componentId: string): RuntimeEntityState | undefined { return binding.entities.find((entity) => entity.id === componentId); }

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

export function stateBindingRevision(binding: VisualStateBindingResult): string { return `R${binding.realityRevision} · S${binding.stateRevision} · T+${(binding.logicalTimeMs / 1000).toFixed(3)}s`; }
