/**
 * HELQIX Visual Runtime — Visual State Specification
 * Layer 14: 14-visual-runtime
 *
 * Defines the visual projection of authoritative engineering states.
 * This represents presentation-tier state and never mutates underlying physics.
 */

export interface Vector3D {
  x: number;
  y: number;
  z: number;
}

export interface Quaternion4D {
  x: number;
  y: number;
  z: number;
  w: number;
}

export type HealthStatus = 'unknown' | 'nominal' | 'warning' | 'critical' | 'failed';

export interface ThermalVisualState {
  temperatureCelsius: number;
  temperatureLimitCelsius: number;
  normalizedHeatRatio: number; // 0.0 (ambient) to 1.0+ (critical)
  thermalGlowColor?: string;
}

export interface MechanicalVisualState {
  stressMegaPascals?: number;
  rpm?: number;
  vibrationAmplitude?: number;
  deflection?: Vector3D;
}

export interface ElectricalVisualState {
  voltageVolts?: number;
  currentAmperes?: number;
  powerWatts?: number;
  stateOfChargePercent?: number;
}

export interface ComponentVisualAttributes {
  visible: boolean;
  wireframe: boolean;
  highlighted: boolean;
  selected: boolean;
  health: HealthStatus;
  opacity: number;
  colorOverride?: string;
  thermal?: ThermalVisualState;
  mechanical?: MechanicalVisualState;
  electrical?: ElectricalVisualState;
}

export interface SpatialSnapshot {
  entityId: string;
  position: Vector3D;
  rotation: Quaternion4D;
  scale: Vector3D;
  linearVelocity?: Vector3D;
  angularVelocity?: Vector3D;
}

export interface EngineeringVisualSnapshot {
  realityId: string;
  timestampSeconds: number;
  timestepIndex: number;
  entities: Record<string, {
    spatial: SpatialSnapshot;
    attributes: ComponentVisualAttributes;
    telemetry: Record<string, number | string | boolean>;
  }>;
  environment: {
    ambientTemperatureCelsius?: number;
    windVelocity?: Vector3D;
    gravityAcceleration?: Vector3D;
  };
}
