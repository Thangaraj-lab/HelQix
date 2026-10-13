import type { RuntimeEntityState } from './visualStateBinding';

export interface EngineeringVisualMotion {
  /** Engineering rotation in radians at the state's logical time. */
  rotationRad: number;
  /** Engineering translation in meters derived from velocity and logical time. */
  translationMeters: number;
  /** Normalized thermal intensity. 0 means no thermal signal is available. */
  thermalIntensity: number;
  hasMotionSignal: boolean;
  hasThermalSignal: boolean;
}

function normalize(name: string): string {
  return name.toLowerCase().replace(/[\s_-]/g, '');
}

function numberByAlias(entity: RuntimeEntityState, aliases: string[]): number | undefined {
  const wanted = new Set(aliases.map(normalize));
  const values = [...Object.entries(entity.signals), ...Object.entries(entity.propertyValues)];
  for (const [name, value] of values) {
    if (wanted.has(normalize(name)) && typeof value === 'number' && Number.isFinite(value)) return value;
  }
  return undefined;
}

export function engineeringVisualMotion(entity: RuntimeEntityState): EngineeringVisualMotion {
  const rpm = numberByAlias(entity, ['rpm', 'rotational_speed', 'rotations_per_minute', 'shaft_speed']);
  const velocity = numberByAlias(entity, ['velocity_m_s', 'speed_m_s', 'linear_velocity']);
  const temperature = numberByAlias(entity, ['temperature_c', 'temperature', 'temp_c', 'cell_temperature']);
  const seconds = Math.max(0, entity.logicalTimeMs) / 1000;

  return {
    rotationRad: rpm === undefined ? 0 : seconds * rpm * (Math.PI * 2 / 60),
    translationMeters: velocity === undefined ? 0 : seconds * velocity,
    thermalIntensity: temperature === undefined ? 0 : Math.max(0, Math.min(1, (temperature - 20) / 100)),
    hasMotionSignal: rpm !== undefined || velocity !== undefined,
    hasThermalSignal: temperature !== undefined,
  };
}
