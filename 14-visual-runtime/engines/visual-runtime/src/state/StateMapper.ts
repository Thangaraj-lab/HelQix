/**
 * HELQIX Visual Runtime — StateMapper
 * Layer 14: 14-visual-runtime
 *
 * Maps raw engineering state and telemetry records into visual presentation state.
 * Evaluates operational thresholds into health alerts (nominal, warning, critical, failed).
 */

import { EngineeringVisualSnapshot, ComponentVisualAttributes, HealthStatus } from './VisualState';

export interface RawCoreSnapshot {
  reality_id: string;
  timestamp_seconds: number;
  timestep_index: number;
  components: Record<string, {
    position?: [number, number, number];
    rotation?: [number, number, number, number];
    scale?: [number, number, number];
    properties?: Record<string, number | string | boolean>;
    health?: string;
  }>;
  environment?: {
    ambient_temp_c?: number;
    wind_velocity?: [number, number, number];
  };
}

function requireFinite(value: unknown, label: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    throw new TypeError(`Snapshot ${label} must be a finite number.`);
  }
  return value;
}

function requireVector<const T extends number>(
  value: unknown,
  length: T,
  label: string,
  fallback: readonly number[],
): number[] {
  const candidate = value === undefined ? fallback : value;
  if (!Array.isArray(candidate) || candidate.length !== length) {
    throw new TypeError(`Snapshot ${label} must contain exactly ${length} numeric values.`);
  }
  return candidate.map((item, index) => requireFinite(item, `${label}[${index}]`));
}

export class StateMapper {
  public static mapFromCoreSnapshot(raw: RawCoreSnapshot): EngineeringVisualSnapshot {
    if (!raw || typeof raw !== 'object' || typeof raw.reality_id !== 'string' || !raw.reality_id.trim()) {
      throw new TypeError('Snapshot reality_id must be a non-empty string.');
    }
    const timestampSeconds = requireFinite(raw.timestamp_seconds, 'timestamp_seconds');
    if (!Number.isInteger(raw.timestep_index) || raw.timestep_index < 0) {
      throw new TypeError('Snapshot timestep_index must be a non-negative integer.');
    }
    if (!raw.components || typeof raw.components !== 'object' || Array.isArray(raw.components)) {
      throw new TypeError('Snapshot components must be an object keyed by stable component IDs.');
    }
    const entities: EngineeringVisualSnapshot['entities'] = {};

    for (const [id, comp] of Object.entries(raw.components)) {
      if (!id.trim() || !comp || typeof comp !== 'object') {
        throw new TypeError('Snapshot components require non-empty IDs and component records.');
      }
      const pos = requireVector(comp.position, 3, `components.${id}.position`, [0, 0, 0]);
      const rot = requireVector(comp.rotation, 4, `components.${id}.rotation`, [0, 0, 0, 1]);
      const rotationMagnitude = Math.hypot(...rot);
      if (!Number.isFinite(rotationMagnitude) || rotationMagnitude <= Number.EPSILON) {
        throw new TypeError(`Snapshot components.${id}.rotation must be a non-zero quaternion.`);
      }
      const normalizedRotation = rot.map((value) => value / rotationMagnitude);
      const scl = requireVector(comp.scale, 3, `components.${id}.scale`, [1, 1, 1]);
      const props = comp.properties || {};
      if (typeof props !== 'object' || Array.isArray(props)) {
        throw new TypeError(`Snapshot components.${id}.properties must be an object.`);
      }
      for (const [name, value] of Object.entries(props)) {
        if (typeof value === 'number' && !Number.isFinite(value)) {
          throw new TypeError(`Snapshot components.${id}.properties.${name} must be finite.`);
        }
        if (!['number', 'string', 'boolean'].includes(typeof value)) {
          throw new TypeError(`Snapshot components.${id}.properties.${name} must be a scalar value.`);
        }
      }

      let health: HealthStatus = 'unknown';
      if (comp.health !== undefined) {
        if (!['unknown', 'nominal', 'warning', 'critical', 'failed'].includes(comp.health)) {
          throw new TypeError(`Snapshot components.${id}.health contains an unsupported status.`);
        }
        health = comp.health as HealthStatus;
      }

      const temp = props.temperature_celsius;
      const tempLimit = props.temperature_limit_celsius;
      let thermal: ComponentVisualAttributes['thermal'];
      if (typeof temp === 'number' && typeof tempLimit === 'number') {
        if (tempLimit <= 0) throw new TypeError(`Snapshot components.${id}.properties.temperature_limit_celsius must be positive.`);
        const heatRatio = requireFinite(temp / tempLimit, `components.${id}.thermal_ratio`);
        thermal = {
          temperatureCelsius: temp,
          temperatureLimitCelsius: tempLimit,
          normalizedHeatRatio: heatRatio
        };
        if (health !== 'failed') {
          if (heatRatio >= 1.0) health = 'critical';
          else if (heatRatio >= 0.85 && health !== 'critical') health = 'warning';
          else if (health === 'unknown') health = 'nominal';
        }
      }

      const attributes: ComponentVisualAttributes = {
        visible: true,
        wireframe: false,
        highlighted: false,
        selected: false,
        health,
        opacity: 1.0,
        ...(thermal ? { thermal } : {}),
        electrical: {
          voltageVolts: typeof props.voltage_v === 'number' ? props.voltage_v : undefined,
          currentAmperes: typeof props.current_a === 'number' ? props.current_a : undefined,
          powerWatts: typeof props.power_w === 'number' ? props.power_w : undefined,
          stateOfChargePercent: typeof props.soc_percent === 'number' ? props.soc_percent : undefined
        },
        mechanical: {
          rpm: typeof props.rpm === 'number' ? props.rpm : undefined,
          stressMegaPascals: typeof props.stress_mpa === 'number' ? props.stress_mpa : undefined
        }
      };

      entities[id] = {
        spatial: {
          entityId: id,
          position: { x: pos[0], y: pos[1], z: pos[2] },
          rotation: { x: normalizedRotation[0], y: normalizedRotation[1], z: normalizedRotation[2], w: normalizedRotation[3] },
          scale: { x: scl[0], y: scl[1], z: scl[2] }
        },
        attributes,
        telemetry: props
      };
    }

    const windVelocity = raw.environment?.wind_velocity === undefined
      ? undefined
      : requireVector(raw.environment.wind_velocity, 3, 'environment.wind_velocity', [0, 0, 0]);

    return {
      realityId: raw.reality_id,
      timestampSeconds,
      timestepIndex: raw.timestep_index,
      entities,
      environment: {
        ...(raw.environment?.ambient_temp_c === undefined ? {} : { ambientTemperatureCelsius: requireFinite(raw.environment.ambient_temp_c, 'environment.ambient_temp_c') }),
        ...(windVelocity ? { windVelocity: { x: windVelocity[0], y: windVelocity[1], z: windVelocity[2] } } : {})
      }
    };
  }
}
