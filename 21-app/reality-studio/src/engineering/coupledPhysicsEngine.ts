import type { EngineeringReality } from '../api';
import {
  advanceLogicalTime,
  setSignal,
  type AuthoritativeEngineeringState,
} from './authoritativeState';
import type { EnvironmentDefinition } from './environmentFabric';

export const COUPLED_PHYSICS_ENGINE_SCHEMA = 'helqix.coupled-physics-engine.v1' as const;

export interface CoupledMotorParameters {
  componentId: string;
  inertiaKgM2: number;
  resistanceOhm: number;
  torqueConstantNmPerA: number;
  backEmfConstantVPerRadPerS: number;
  viscousDampingNmPerRadPerS: number;
  thermalMassJPerK: number;
  thermalResistanceKPerW: number;
  ambientTemperatureC: number;
  maximumCurrentA?: number;
  maximumTemperatureC?: number;
  loadTorqueNm?: number;
}

export interface CoupledMotorState {
  currentA: number;
  angularVelocityRadPerS: number;
  angleRad: number;
  windingTemperatureC: number;
  electricalPowerW: number;
  copperLossW: number;
  mechanicalPowerW: number;
  loadTorqueNm: number;
}

export interface CoupledPhysicsStepResult {
  schema: typeof COUPLED_PHYSICS_ENGINE_SCHEMA;
  logicalTimeMs: number;
  deltaTimeMs: number;
  componentId: string;
  state: CoupledMotorState;
  nextState: CoupledMotorState;
  authoritativeState: AuthoritativeEngineeringState;
  failures: CoupledPhysicsFailure[];
  energy: {
    electricalJ: number;
    copperLossJ: number;
    mechanicalJ: number;
    thermalStoredJ: number;
  };
}

export interface CoupledPhysicsFailure {
  id: string;
  componentId: string;
  type: 'over_current' | 'over_temperature' | 'invalid_parameter' | 'unstable_step';
  severity: 'warning' | 'error';
  message: string;
  logicalTimeMs: number;
}

function finite(value: number, name: string): void {
  if (!Number.isFinite(value)) throw new Error(`${name} must be finite.`);
}

function positive(value: number, name: string): void {
  finite(value, name);
  if (value <= 0) throw new Error(`${name} must be greater than zero.`);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function propertyNumber(state: AuthoritativeEngineeringState, componentId: string, name: string, fallback: number): number {
  const property = state.components[componentId]?.properties.find((item) => item.name === name);
  return typeof property?.value === 'number' ? property.value : fallback;
}

function signalNumber(state: AuthoritativeEngineeringState, componentId: string, name: string, fallback: number): number {
  const signal = state.components[componentId]?.signals[name];
  return typeof signal?.value === 'number' ? signal.value : fallback;
}

function environmentNumber(environment: EnvironmentDefinition, keys: string[], fallback: number): number {
  for (const key of keys) {
    const value = environment.values.find((item) => item.key === key);
    if (value) return value.value;
  }
  return fallback;
}

export function validateCoupledMotorParameters(parameters: CoupledMotorParameters): void {
  if (!parameters.componentId.trim()) throw new Error('componentId is required.');
  positive(parameters.inertiaKgM2, 'inertiaKgM2');
  positive(parameters.resistanceOhm, 'resistanceOhm');
  positive(parameters.torqueConstantNmPerA, 'torqueConstantNmPerA');
  positive(parameters.backEmfConstantVPerRadPerS, 'backEmfConstantVPerRadPerS');
  finite(parameters.viscousDampingNmPerRadPerS, 'viscousDampingNmPerRadPerS');
  if (parameters.viscousDampingNmPerRadPerS < 0) throw new Error('viscousDampingNmPerRadPerS cannot be negative.');
  positive(parameters.thermalMassJPerK, 'thermalMassJPerK');
  positive(parameters.thermalResistanceKPerW, 'thermalResistanceKPerW');
  finite(parameters.ambientTemperatureC, 'ambientTemperatureC');
  if (parameters.maximumCurrentA !== undefined) positive(parameters.maximumCurrentA, 'maximumCurrentA');
  if (parameters.maximumTemperatureC !== undefined) finite(parameters.maximumTemperatureC, 'maximumTemperatureC');
  if (parameters.loadTorqueNm !== undefined) finite(parameters.loadTorqueNm, 'loadTorqueNm');
}

export function readCoupledMotorState(state: AuthoritativeEngineeringState, parameters: CoupledMotorParameters): CoupledMotorState {
  const component = state.components[parameters.componentId];
  if (!component) throw new Error(`Unknown motor component: ${parameters.componentId}`);
  return {
    currentA: signalNumber(state, parameters.componentId, 'currentA', propertyNumber(state, parameters.componentId, 'currentA', 0)),
    angularVelocityRadPerS: signalNumber(state, parameters.componentId, 'angularVelocityRadPerS', propertyNumber(state, parameters.componentId, 'angularVelocityRadPerS', 0)),
    angleRad: signalNumber(state, parameters.componentId, 'angleRad', propertyNumber(state, parameters.componentId, 'angleRad', 0)),
    windingTemperatureC: signalNumber(state, parameters.componentId, 'windingTemperatureC', propertyNumber(state, parameters.componentId, 'windingTemperatureC', parameters.ambientTemperatureC)),
    electricalPowerW: signalNumber(state, parameters.componentId, 'electricalPowerW', 0),
    copperLossW: signalNumber(state, parameters.componentId, 'copperLossW', 0),
    mechanicalPowerW: signalNumber(state, parameters.componentId, 'mechanicalPowerW', 0),
    loadTorqueNm: signalNumber(state, parameters.componentId, 'loadTorqueNm', parameters.loadTorqueNm ?? 0),
  };
}

export function stepCoupledMotor(
  reality: EngineeringReality,
  state: AuthoritativeEngineeringState,
  environment: EnvironmentDefinition,
  parameters: CoupledMotorParameters,
  deltaTimeMs: number,
): CoupledPhysicsStepResult {
  validateCoupledMotorParameters(parameters);
  positive(deltaTimeMs, 'deltaTimeMs');
  if (state.realityRevision !== reality.revision) throw new Error('Coupled physics state does not match engineering reality revision.');

  const dt = deltaTimeMs / 1000;
  if (dt > 0.1) throw new Error('Coupled physics step is too large; use <= 100 ms for stable deterministic integration.');

  const current = readCoupledMotorState(state, parameters);
  const voltage = signalNumber(state, parameters.componentId, 'voltageV', propertyNumber(state, parameters.componentId, 'voltageV', 0));
  const loadTorque = signalNumber(state, parameters.componentId, 'loadTorqueNm', parameters.loadTorqueNm ?? current.loadTorqueNm);
  const ambient = environmentNumber(environment, ['air.temperature', 'ambient.temperature', 'thermal.ambientTemperature'], parameters.ambientTemperatureC);

  const backEmfV = parameters.backEmfConstantVPerRadPerS * current.angularVelocityRadPerS;
  const electricalCurrentDerivative = (voltage - backEmfV - parameters.resistanceOhm * current.currentA) / parameters.resistanceOhm;
  let nextCurrent = current.currentA + electricalCurrentDerivative * dt;
  if (parameters.maximumCurrentA !== undefined) nextCurrent = clamp(nextCurrent, -parameters.maximumCurrentA, parameters.maximumCurrentA);

  const electromagneticTorque = parameters.torqueConstantNmPerA * nextCurrent;
  const dampingTorque = parameters.viscousDampingNmPerRadPerS * current.angularVelocityRadPerS;
  const angularAcceleration = (electromagneticTorque - dampingTorque - loadTorque) / parameters.inertiaKgM2;
  const nextAngularVelocity = current.angularVelocityRadPerS + angularAcceleration * dt;
  const nextAngle = current.angleRad + nextAngularVelocity * dt;

  const copperLossW = nextCurrent * nextCurrent * parameters.resistanceOhm;
  const electricalPowerW = voltage * nextCurrent;
  const mechanicalPowerW = electromagneticTorque * nextAngularVelocity;
  const coolingW = (current.windingTemperatureC - ambient) / parameters.thermalResistanceKPerW;
  const thermalDerivativeCPerS = (copperLossW - coolingW) / parameters.thermalMassJPerK;
  const nextTemperature = current.windingTemperatureC + thermalDerivativeCPerS * dt;

  const nextState: CoupledMotorState = {
    currentA: nextCurrent,
    angularVelocityRadPerS: nextAngularVelocity,
    angleRad: nextAngle,
    windingTemperatureC: nextTemperature,
    electricalPowerW,
    copperLossW,
    mechanicalPowerW,
    loadTorqueNm: loadTorque,
  };

  for (const [name, value] of Object.entries(nextState)) finite(value, `${parameters.componentId}.${name}`);

  const failures: CoupledPhysicsFailure[] = [];
  if (parameters.maximumCurrentA !== undefined && Math.abs(nextCurrent) >= parameters.maximumCurrentA) {
    failures.push({ id: `${parameters.componentId}:over-current`, componentId: parameters.componentId, type: 'over_current', severity: 'error', message: `Current reached the configured limit of ${parameters.maximumCurrentA} A.`, logicalTimeMs: state.logicalTimeMs + deltaTimeMs });
  }
  if (parameters.maximumTemperatureC !== undefined && nextTemperature >= parameters.maximumTemperatureC) {
    failures.push({ id: `${parameters.componentId}:over-temperature`, componentId: parameters.componentId, type: 'over_temperature', severity: 'error', message: `Winding temperature reached ${parameters.maximumTemperatureC} °C.`, logicalTimeMs: state.logicalTimeMs + deltaTimeMs });
  }
  if (Math.abs(nextAngularVelocity) > 1e9 || Math.abs(nextTemperature) > 1e6) {
    failures.push({ id: `${parameters.componentId}:unstable-step`, componentId: parameters.componentId, type: 'unstable_step', severity: 'error', message: 'Coupled state diverged beyond deterministic stability bounds.', logicalTimeMs: state.logicalTimeMs + deltaTimeMs });
  }

  let nextAuthoritative = advanceLogicalTime(state, state.logicalTimeMs + deltaTimeMs, 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'currentA', nextCurrent, 'A', 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'angularVelocityRadPerS', nextAngularVelocity, 'rad/s', 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'angleRad', nextAngle, 'rad', 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'windingTemperatureC', nextTemperature, '°C', 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'electricalPowerW', electricalPowerW, 'W', 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'copperLossW', copperLossW, 'W', 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'mechanicalPowerW', mechanicalPowerW, 'W', 'solver');
  nextAuthoritative = setSignal(nextAuthoritative, parameters.componentId, 'loadTorqueNm', loadTorque, 'Nm', 'solver');

  return {
    schema: COUPLED_PHYSICS_ENGINE_SCHEMA,
    logicalTimeMs: nextAuthoritative.logicalTimeMs,
    deltaTimeMs,
    componentId: parameters.componentId,
    state: current,
    nextState,
    authoritativeState: nextAuthoritative,
    failures,
    energy: {
      electricalJ: electricalPowerW * dt,
      copperLossJ: copperLossW * dt,
      mechanicalJ: mechanicalPowerW * dt,
      thermalStoredJ: parameters.thermalMassJPerK * (nextTemperature - parameters.ambientTemperatureC),
    },
  };
}

export function runCoupledMotor(
  reality: EngineeringReality,
  initialState: AuthoritativeEngineeringState,
  environment: EnvironmentDefinition,
  parameters: CoupledMotorParameters,
  durationMs: number,
  stepMs = 10,
): CoupledPhysicsStepResult[] {
  positive(durationMs, 'durationMs');
  positive(stepMs, 'stepMs');
  if (stepMs > 100) throw new Error('stepMs must be <= 100 ms.');
  const steps = Math.ceil(durationMs / stepMs);
  const results: CoupledPhysicsStepResult[] = [];
  let state = initialState;
  for (let index = 0; index < steps; index += 1) {
    const remaining = durationMs - index * stepMs;
    const delta = Math.min(stepMs, remaining);
    const result = stepCoupledMotor(reality, state, environment, parameters, delta);
    results.push(result);
    state = result.authoritativeState;
    if (result.failures.some((failure) => failure.severity === 'error')) break;
  }
  return results;
}
