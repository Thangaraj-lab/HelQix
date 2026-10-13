import type { AuthoritativeEngineeringState } from './authoritativeState';

export const OBSERVATION_FABRIC_SCHEMA = 'helqix.observation-fabric.v1' as const;

export type ObservationSource = 'state' | 'physics' | 'environment' | 'sensor' | 'external';
export type ObservationSeverity = 'info' | 'warning' | 'critical';

export interface ObservationProbe {
  id: string;
  name: string;
  source: ObservationSource;
  componentId?: string;
  signalName?: string;
  environmentKey?: string;
  unit?: string;
  min?: number;
  max?: number;
  enabled: boolean;
}

export interface ObservationSample {
  probeId: string;
  logicalTimeMs: number;
  value: string | number | boolean;
  unit?: string;
  severity: ObservationSeverity;
  source: ObservationSource;
}

export interface ObservationAnomaly {
  id: string;
  probeId: string;
  logicalTimeMs: number;
  kind: 'above_limit' | 'below_limit' | 'invalid' | 'missing' | 'state_fault';
  message: string;
  severity: ObservationSeverity;
  observedValue?: string | number | boolean;
}

export interface ObservationFrame {
  schema: typeof OBSERVATION_FABRIC_SCHEMA;
  logicalTimeMs: number;
  samples: readonly ObservationSample[];
  anomalies: readonly ObservationAnomaly[];
}

function finite(value: number, label: string): void { if (!Number.isFinite(value)) throw new Error(`${label} must be finite.`); }
function clone<T>(value: T): T { return structuredClone(value); }

export class ObservationFabric {
  private readonly probes = new Map<string, ObservationProbe>();
  private readonly samples: ObservationSample[] = [];
  private readonly anomalies: ObservationAnomaly[] = [];

  registerProbe(probe: ObservationProbe): void {
    if (!probe.id.trim() || !probe.name.trim()) throw new Error('Observation probe id and name are required.');
    if (this.probes.has(probe.id)) throw new Error(`Observation probe already exists: ${probe.id}`);
    if (probe.min !== undefined) finite(probe.min, `${probe.id}.min`);
    if (probe.max !== undefined) finite(probe.max, `${probe.id}.max`);
    if (probe.min !== undefined && probe.max !== undefined && probe.max < probe.min) throw new Error(`Probe ${probe.id} has an invalid range.`);
    this.probes.set(probe.id, clone(probe));
  }

  removeProbe(probeId: string): void { this.probes.delete(probeId); }
  listProbes(): ObservationProbe[] { return [...this.probes.values()].map(clone); }

  observeState(state: AuthoritativeEngineeringState): ObservationFrame {
    const frameSamples: ObservationSample[] = [];
    const frameAnomalies: ObservationAnomaly[] = [];
    for (const probe of this.probes.values()) {
      if (!probe.enabled) continue;
      let value: string | number | boolean | undefined;
      if (probe.source === 'state' && probe.componentId && probe.signalName) value = state.components[probe.componentId]?.signals[probe.signalName]?.value;
      else if (probe.source === 'state' && probe.componentId && probe.signalName === undefined) value = state.components[probe.componentId]?.status;
      else if (probe.source === 'environment' && probe.environmentKey) value = state.environment.values[probe.environmentKey];
      if (value === undefined) {
        frameAnomalies.push({ id: `${probe.id}:${state.logicalTimeMs}:missing`, probeId: probe.id, logicalTimeMs: state.logicalTimeMs, kind: 'missing', message: `No observation value available for probe ${probe.name}.`, severity: 'warning' });
        continue;
      }
      let severity: ObservationSeverity = 'info';
      if (typeof value === 'number') {
        if (!Number.isFinite(value)) {
          frameAnomalies.push({ id: `${probe.id}:${state.logicalTimeMs}:invalid`, probeId: probe.id, logicalTimeMs: state.logicalTimeMs, kind: 'invalid', message: `Probe ${probe.name} produced a non-finite value.`, severity: 'critical', observedValue: value });
          continue;
        }
        if (probe.max !== undefined && value > probe.max) {
          severity = 'critical';
          frameAnomalies.push({ id: `${probe.id}:${state.logicalTimeMs}:above`, probeId: probe.id, logicalTimeMs: state.logicalTimeMs, kind: 'above_limit', message: `${probe.name} exceeded its maximum limit.`, severity, observedValue: value });
        } else if (probe.min !== undefined && value < probe.min) {
          severity = 'critical';
          frameAnomalies.push({ id: `${probe.id}:${state.logicalTimeMs}:below`, probeId: probe.id, logicalTimeMs: state.logicalTimeMs, kind: 'below_limit', message: `${probe.name} fell below its minimum limit.`, severity, observedValue: value });
        }
      }
      const sample: ObservationSample = { probeId: probe.id, logicalTimeMs: state.logicalTimeMs, value, ...(probe.unit ? { unit: probe.unit } : {}), severity, source: probe.source };
      frameSamples.push(sample);
      this.samples.push(clone(sample));
    }
    this.anomalies.push(...frameAnomalies.map(clone));
    return { schema: OBSERVATION_FABRIC_SCHEMA, logicalTimeMs: state.logicalTimeMs, samples: frameSamples, anomalies: frameAnomalies };
  }

  record(sample: ObservationSample): void { finite(sample.logicalTimeMs, 'Observation time'); this.samples.push(clone(sample)); }
  history(probeId?: string): ObservationSample[] { return this.samples.filter((sample) => !probeId || sample.probeId === probeId).map(clone); }
  anomaliesHistory(probeId?: string): ObservationAnomaly[] { return this.anomalies.filter((anomaly) => !probeId || anomaly.probeId === probeId).map(clone); }
}
