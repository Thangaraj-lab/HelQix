import type { EngineeringReality } from '../api';
import { addFault, assertValidState, captureSnapshot, setStateStatus, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import { CausalityEngine, type CausalFailure } from './causalityEngine';
import { runCoupledMotor, type CoupledMotorParameters, type CoupledPhysicsFailure } from './coupledPhysicsEngine';
import { createEnvironment, type EnvironmentDefinition } from './environmentFabric';
import { ObservationFabric, type ObservationAnomaly, type ObservationFrame } from './observationFabric';
import { DeterministicSimulationClock } from './simulationClock';

export const DETERMINISTIC_ENGINEERING_SIMULATION_SCHEMA = 'helqix.deterministic-engineering-simulation.v1' as const;

export interface DeterministicSimulationOptions {
  durationMs: number;
  fixedStepMs: number;
  checkpointEverySteps?: number;
  environment?: EnvironmentDefinition;
  motorParameters?: readonly CoupledMotorParameters[];
  stopOnFailure?: boolean;
}

export interface DeterministicSimulationFailure {
  failure: CoupledPhysicsFailure;
  causal?: CausalFailure;
}

export interface DeterministicSimulationResult {
  schema: typeof DETERMINISTIC_ENGINEERING_SIMULATION_SCHEMA;
  status: 'completed' | 'failed';
  initialState: AuthoritativeEngineeringState;
  finalState: AuthoritativeEngineeringState;
  clock: ReturnType<DeterministicEngineeringSimulationEngine['clockSnapshot']>;
  observations: ObservationFrame[];
  failures: DeterministicSimulationFailure[];
  checkpoints: StateSnapshot[];
  energy: { electricalJ: number; copperLossJ: number; mechanicalJ: number };
}

function clone<T>(value: T): T { return structuredClone(value); }
function finite(value: number, label: string): void { if (!Number.isFinite(value)) throw new Error(`${label} must be finite.`); }
function positive(value: number, label: string): void { finite(value, label); if (value <= 0) throw new Error(`${label} must be greater than zero.`); }

function defaultEnvironment(timeMs: number): EnvironmentDefinition {
  return createEnvironment(
    'helqix-default-environment',
    '1',
    'controlled',
    [],
    [{ key: 'air.temperature', value: 25, unit: '°C', timestampMs: timeMs, source: { id: 'helqix-default', kind: 'simulation', label: 'HELQIX default', confidence: 1 } }],
    'global',
    timeMs,
  );
}

function anomalyForFailure(failure: CoupledPhysicsFailure): ObservationAnomaly {
  const kind = failure.type === 'over_current' || failure.type === 'over_temperature' ? 'above_limit' : 'invalid';
  return { id: failure.id, probeId: `physics:${failure.componentId}`, logicalTimeMs: failure.logicalTimeMs, kind, message: failure.message, severity: failure.severity === 'error' ? 'critical' : 'warning' };
}

export class DeterministicEngineeringSimulationEngine {
  private readonly clock: DeterministicSimulationClock;
  private readonly observation = new ObservationFabric();
  private readonly causality = new CausalityEngine();

  constructor(
    private readonly reality: EngineeringReality,
    private state: AuthoritativeEngineeringState,
    private readonly options: DeterministicSimulationOptions,
  ) {
    positive(options.durationMs, 'durationMs');
    positive(options.fixedStepMs, 'fixedStepMs');
    if (options.fixedStepMs > 100) throw new Error('fixedStepMs must be <= 100 ms for coupled engineering physics.');
    if (!Number.isInteger(options.durationMs / options.fixedStepMs)) throw new Error('durationMs must align to fixedStepMs.');
    if (state.realityRevision !== reality.revision) throw new Error('Simulation state does not match engineering reality revision.');
    assertValidState(state, reality);
    this.clock = new DeterministicSimulationClock({ startTimeMs: state.logicalTimeMs, fixedStepMs: options.fixedStepMs, maxTimeMs: state.logicalTimeMs + options.durationMs });
    for (const motor of options.motorParameters ?? []) this.registerMotorObservationProbes(motor);
    for (const motor of options.motorParameters ?? []) {
      this.causality.addNode({ id: `component:${motor.componentId}`, kind: 'component', label: motor.componentId, reference: motor.componentId });
      this.causality.addNode({ id: `signal:${motor.componentId}:currentA`, kind: 'signal', label: `${motor.componentId} current`, reference: `${motor.componentId}.currentA` });
      this.causality.addNode({ id: `signal:${motor.componentId}:temperature`, kind: 'signal', label: `${motor.componentId} temperature`, reference: `${motor.componentId}.windingTemperatureC` });
      this.causality.addEdge({ from: `signal:${motor.componentId}:currentA`, to: `signal:${motor.componentId}:temperature`, relation: 'causes', confidence: 0.95, evidenceIds: ['coupled-physics'] });
    }
  }

  clockSnapshot() { return this.clock.snapshot(); }
  currentState(): AuthoritativeEngineeringState { return clone(this.state); }

  run(): DeterministicSimulationResult {
    const initialState = clone(this.state);
    const checkpoints: StateSnapshot[] = [captureSnapshot(initialState, 'initial', `deterministic-initial-${initialState.stateRevision}-${initialState.logicalTimeMs}`)];
    const observations: ObservationFrame[] = [];
    const failures: DeterministicSimulationFailure[] = [];
    const energy = { electricalJ: 0, copperLossJ: 0, mechanicalJ: 0 };
    const environment = this.options.environment ?? defaultEnvironment(initialState.logicalTimeMs);
    const motors = [...(this.options.motorParameters ?? [])];
    this.state = setStateStatus(this.state, 'running');
    const totalSteps = this.options.durationMs / this.options.fixedStepMs;
    const checkpointEvery = Math.max(1, Math.floor(this.options.checkpointEverySteps ?? 100));

    for (let stepIndex = 0; stepIndex < totalSteps; stepIndex += 1) {
      const clockState = this.clock.step();
      let stepState = this.state;
      for (const motor of motors) {
        const result = runCoupledMotor(this.reality, stepState, environment, motor, this.options.fixedStepMs, this.options.fixedStepMs)[0];
        stepState = result.authoritativeState;
        energy.electricalJ += result.energy.electricalJ;
        energy.copperLossJ += result.energy.copperLossJ;
        energy.mechanicalJ += result.energy.mechanicalJ;
        for (const failure of result.failures) {
          stepState = addFault(stepState, failure.componentId, failure.id, undefined, 'solver');
          const anomaly = anomalyForFailure(failure);
          const nodeCandidates = [`component:${failure.componentId}`, `signal:${failure.componentId}:currentA`, `signal:${failure.componentId}:temperature`];
          const causal = this.causality.ingestAnomaly(anomaly, nodeCandidates);
          failures.push({ failure, causal });
        }
      }
      this.state = stepState;
      const frame = this.observation.observeState(this.state);
      observations.push(frame);
      for (const anomaly of frame.anomalies) {
        const causal = this.causality.ingestAnomaly(anomaly, [`component:${anomaly.probeId.split(':')[1] ?? 'unknown'}`].filter(id => id !== 'component:unknown'));
        const existing = failures.find(item => item.failure.id === anomaly.id);
        if (!existing) failures.push({ failure: { id: anomaly.id, componentId: anomaly.probeId.split(':')[1] ?? 'unknown', type: 'unstable_step', severity: 'error', message: anomaly.message, logicalTimeMs: anomaly.logicalTimeMs }, causal });
      }
      if (stepIndex === 0 || (stepIndex + 1) % checkpointEvery === 0 || failures.length) checkpoints.push(captureSnapshot(this.state, failures.length ? 'failure' : 'checkpoint', `deterministic-${this.state.stateRevision}-${clockState.logicalTimeMs}-${checkpoints.length}`));
      if (failures.length && this.options.stopOnFailure !== false) {
        this.state = setStateStatus(this.state, 'faulted');
        return { schema: DETERMINISTIC_ENGINEERING_SIMULATION_SCHEMA, status: 'failed', initialState, finalState: clone(this.state), clock: this.clock.snapshot(), observations, failures, checkpoints, energy };
      }
    }

    this.state = setStateStatus(this.state, 'completed');
    assertValidState(this.state, this.reality);
    return { schema: DETERMINISTIC_ENGINEERING_SIMULATION_SCHEMA, status: 'completed', initialState, finalState: clone(this.state), clock: this.clock.snapshot(), observations, failures, checkpoints, energy };
  }

  causalityGraph() { return this.causality.graph(); }

  private registerMotorObservationProbes(parameters: CoupledMotorParameters): void {
    this.observation.registerProbe({ id: `current:${parameters.componentId}`, name: `${parameters.componentId} current`, source: 'state', componentId: parameters.componentId, signalName: 'currentA', unit: 'A', ...(parameters.maximumCurrentA !== undefined ? { max: parameters.maximumCurrentA } : {}), enabled: true });
    this.observation.registerProbe({ id: `temperature:${parameters.componentId}`, name: `${parameters.componentId} winding temperature`, source: 'state', componentId: parameters.componentId, signalName: 'windingTemperatureC', unit: '°C', ...(parameters.maximumTemperatureC !== undefined ? { max: parameters.maximumTemperatureC } : {}), enabled: true });
  }
}
