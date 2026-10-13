import type { RuntimeEntityState } from './visualStateBinding';

export interface EngineeringVisualMotion {
  rotationRad: number;
  translationMeters: number;
  thermalIntensity: number;
  temperatureCelsius: number;
  temperatureLimitCelsius: number;
  hasMotionSignal: boolean;
  hasThermalSignal: boolean;
}

function normalize(name: string): string { return name.toLowerCase().replace(/[\s_-]/g, ''); }
const RPM_ALIASES = new Set(['rpm', 'rotationalspeed', 'rotationsperminute', 'shaftspeed']);
const VELOCITY_ALIASES = new Set(['velocityms', 'speedms', 'linearvelocity']);
const TEMPERATURE_ALIASES = new Set(['temperaturec', 'temperature', 'tempc', 'celltemperature', 'windingtemperaturec']);
const TEMPERATURE_LIMIT_ALIASES = new Set(['temperaturelimitc', 'temperaturelimit', 'maxtemperaturec', 'maximumtemperaturec']);

function numberByAlias(entity: RuntimeEntityState, aliases: Set<string>): number | undefined {
  const values = [...Object.entries(entity.signals), ...Object.entries(entity.propertyValues)];
  for (const [name, value] of values) if (aliases.has(normalize(name)) && typeof value === 'number' && Number.isFinite(value)) return value;
  return undefined;
}

function eventIsAlias(name: string, aliases: Set<string>): boolean { return aliases.has(normalize(name)); }

/** Integrates piecewise-constant RPM over authoritative signal events. */
function integratedRotationRad(entity: RuntimeEntityState, fallbackRpm?: number): number {
  const targetTime = Math.max(0, entity.logicalTimeMs);
  const events = entity.motionEvents.filter((event) => eventIsAlias(event.name, RPM_ALIASES) && event.logicalTimeMs <= targetTime).sort((a, b) => a.logicalTimeMs - b.logicalTimeMs);
  let angle = 0;
  let currentRpm = 0;
  let cursorMs = 0;
  for (const event of events) {
    const eventTime = Math.max(cursorMs, event.logicalTimeMs);
    const deltaSeconds = (eventTime - cursorMs) / 1000;
    angle += deltaSeconds * currentRpm * (Math.PI * 2 / 60);
    currentRpm = event.value;
    cursorMs = eventTime;
  }
  if (cursorMs < targetTime) angle += ((targetTime - cursorMs) / 1000) * currentRpm * (Math.PI * 2 / 60);
  if (!events.length && fallbackRpm !== undefined) angle = targetTime / 1000 * fallbackRpm * (Math.PI * 2 / 60);
  return angle;
}

export function engineeringVisualMotion(entity: RuntimeEntityState): EngineeringVisualMotion {
  const rpm = numberByAlias(entity, RPM_ALIASES);
  const velocity = numberByAlias(entity, VELOCITY_ALIASES);
  const temperature = numberByAlias(entity, TEMPERATURE_ALIASES);
  const temperatureLimit = numberByAlias(entity, TEMPERATURE_LIMIT_ALIASES) ?? Math.max(100, temperature ?? 100);
  const seconds = Math.max(0, entity.logicalTimeMs) / 1000;
  const hasVelocityHistory = entity.motionEvents.some((event) => eventIsAlias(event.name, VELOCITY_ALIASES));
  const translationMeters = velocity === undefined ? 0 : seconds * velocity;
  const hasThermalSignal = temperature !== undefined;

  return {
    rotationRad: integratedRotationRad(entity, rpm),
    translationMeters,
    thermalIntensity: temperature === undefined ? 0 : Math.max(0, Math.min(1, (temperature - 20) / Math.max(1, temperatureLimit - 20))),
    temperatureCelsius: temperature ?? 20,
    temperatureLimitCelsius: temperatureLimit,
    hasMotionSignal: rpm !== undefined || velocity !== undefined || hasVelocityHistory,
    hasThermalSignal,
  };
}
