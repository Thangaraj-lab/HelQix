import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, StateSnapshot } from './authoritativeState';
import { captureSnapshot } from './authoritativeState';
import type { ExecutableScenario } from './scenarioEngine';
import type { EnvironmentDefinition } from './environmentFabric';
import { createEnvironment } from './environmentFabric';
import { runCoupledMotor, type CoupledMotorParameters } from './coupledPhysicsEngine';

export const SIMULATION_ORCHESTRATOR_SCHEMA = 'helqix.simulation-orchestrator.v1' as const;
export type SimulationStatus = 'planned' | 'queued' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled' | 'timed_out';

export interface SimulationRequest { id: string; realityRevision: number; scenarioId: string; solver: { id: string; version: string; deterministic: boolean; seed?: number }; timeoutMs: number; checkpointIntervalMs?: number; }
export interface SimulationObservation { logicalTimeMs: number; componentId?: string; signal?: string; value: unknown; unit?: string; source: string; }
export interface SimulationResult { status: 'completed' | 'failed' | 'cancelled' | 'timed_out'; finalState?: AuthoritativeEngineeringState; observations: SimulationObservation[]; outputs: Record<string, unknown>; error?: string; }
export interface SimulationContext { reality: EngineeringReality; scenario: ExecutableScenario; state: AuthoritativeEngineeringState; logicalTimeMs: number; observations: SimulationObservation[]; outputs: Record<string, unknown>; checkpoints: StateSnapshot[]; environment?: EnvironmentDefinition; result?: SimulationResult; }
export interface SimulationAdapter { prepare(context: SimulationContext): Promise<void>; run(context: SimulationContext, signal: AbortSignal): Promise<SimulationResult>; cancel?(context: SimulationContext): Promise<void> | void; pause?(context: SimulationContext): Promise<void> | void; resume?(context: SimulationContext): Promise<void> | void; }
export interface SimulationRun { schema: typeof SIMULATION_ORCHESTRATOR_SCHEMA; request: SimulationRequest; status: SimulationStatus; context: SimulationContext; error?: string; updatedAt: string; }

function clone<T>(value: T): T { return structuredClone(value); }
function now(): string { return new Date().toISOString(); }

export function createSimulationRun(reality: EngineeringReality, scenario: ExecutableScenario, state: AuthoritativeEngineeringState, request: Omit<SimulationRequest, 'realityRevision' | 'scenarioId'>, environment?: EnvironmentDefinition): SimulationRun {
  if (state.realityRevision !== reality.revision) throw new Error('Simulation state does not match the current engineering reality revision.');
  if (!Number.isFinite(request.timeoutMs) || request.timeoutMs <= 0) throw new Error('Simulation timeout must be a positive finite number.');
  return { schema: SIMULATION_ORCHESTRATOR_SCHEMA, request: { ...request, realityRevision: reality.revision, scenarioId: scenario.id }, status: 'planned', context: { reality: clone(reality), scenario: clone(scenario), state: clone(state), logicalTimeMs: state.logicalTimeMs, observations: [], outputs: {}, checkpoints: [], ...(environment ? { environment: clone(environment) } : {}) }, updatedAt: now() };
}

function transition(run: SimulationRun, status: SimulationStatus, error?: string): SimulationRun { return { ...run, status, ...(error ? { error } : { error: undefined }), updatedAt: now() }; }

export function queueSimulation(run: SimulationRun): SimulationRun { if (run.status !== 'planned') throw new Error(`Cannot queue simulation from ${run.status}.`); return transition(run, 'queued'); }

export async function executeSimulation(run: SimulationRun, adapter: SimulationAdapter): Promise<SimulationRun> {
  if (!['planned', 'queued'].includes(run.status)) throw new Error(`Cannot execute simulation from ${run.status}.`);
  const active = transition(run, 'running'); const controller = new AbortController(); let timeoutHandle: ReturnType<typeof setTimeout> | undefined; let timedOut = false; let cancelRequested = false;
  const cancelAdapter = async () => { if (cancelRequested) return; cancelRequested = true; try { await Promise.resolve(adapter.cancel?.(active.context)); } catch { /* Preserve primary failure. */ } };
  try {
    const work = (async () => { await adapter.prepare(active.context); if (controller.signal.aborted) throw new Error('Simulation aborted during preparation.'); return adapter.run(active.context, controller.signal); })();
    const deadline = new Promise<never>((_, reject) => { timeoutHandle = setTimeout(() => { timedOut = true; controller.abort(); reject(new Error(`Simulation exceeded ${active.request.timeoutMs}ms.`)); }, active.request.timeoutMs); });
    const result = await Promise.race([work, deadline]);
    if (controller.signal.aborted) throw new Error(`Simulation exceeded ${active.request.timeoutMs}ms.`);
    const status: SimulationStatus = result.status === 'completed' ? 'completed' : result.status === 'cancelled' ? 'cancelled' : 'failed';
    const next: SimulationRun = { ...active, context: { ...active.context, logicalTimeMs: result.finalState?.logicalTimeMs ?? active.context.logicalTimeMs, state: result.finalState ? clone(result.finalState) : active.context.state, observations: [...active.context.observations, ...result.observations], outputs: clone(result.outputs), result: clone(result) } };
    return transition(next, status, result.error);
  } catch (error) {
    const message = timedOut ? `Simulation exceeded ${active.request.timeoutMs}ms.` : error instanceof Error ? error.message : 'Simulation execution failed.';
    controller.abort(); await cancelAdapter(); const result: SimulationResult = { status: timedOut ? 'timed_out' : 'failed', observations: [...active.context.observations], outputs: {}, error: message };
    return transition({ ...active, context: { ...active.context, result } }, result.status, message);
  } finally { if (timeoutHandle !== undefined) clearTimeout(timeoutHandle); }
}

export function cancelSimulation(run: SimulationRun): SimulationRun { if (!['planned', 'queued', 'running', 'paused'].includes(run.status)) throw new Error(`Cannot cancel simulation from ${run.status}.`); return transition(run, 'cancelled'); }
export function verifySimulationCompatibility(reality: EngineeringReality, run: SimulationRun): string[] { const errors: string[] = []; if (run.schema !== SIMULATION_ORCHESTRATOR_SCHEMA) errors.push('Unsupported simulation run schema.'); if (run.request.realityRevision !== reality.revision) errors.push(`Simulation targets reality revision ${run.request.realityRevision}, current revision is ${reality.revision}.`); if (run.context.state.realityRevision !== reality.revision) errors.push('Simulation authoritative state is stale.'); if (run.request.scenarioId !== run.context.scenario.id) errors.push('Simulation scenario identity is inconsistent.'); if (run.context.environment && run.context.environment.schema !== 'helqix.environment-fabric.v1') errors.push('Simulation environment schema is unsupported.'); return errors; }

export interface CoupledMotorSimulationAdapterOptions { parameters: CoupledMotorParameters; durationMs: number; stepMs?: number; environment?: EnvironmentDefinition; checkpointEverySteps?: number; }

export function createCoupledMotorSimulationAdapter(options: CoupledMotorSimulationAdapterOptions): SimulationAdapter {
  if (!Number.isFinite(options.durationMs) || options.durationMs <= 0) throw new Error('Coupled simulation duration must be positive.');
  const stepMs = options.stepMs ?? 10;
  if (!Number.isFinite(stepMs) || stepMs <= 0 || stepMs > 100) throw new Error('Coupled simulation step must be > 0 and <= 100 ms.');
  const checkpointEvery = Math.max(1, Math.floor(options.checkpointEverySteps ?? 100));

  return {
    async prepare(context) {
      if (context.state.realityRevision !== context.reality.revision) throw new Error('Simulation state/reality revision mismatch.');
      if (options.environment) context.environment = clone(options.environment);
      if (!context.environment) context.environment = createEnvironment(`sim-${context.requestId ?? 'runtime'}`, '1', 'controlled', [], [], 'global', context.state.logicalTimeMs);
    },
    async run(context, signal) {
      if (!context.environment) throw new Error('Coupled physics requires an environment definition.');
      const totalSteps = Math.ceil(options.durationMs / stepMs);
      const observations: SimulationObservation[] = [];
      let state = clone(context.state);
      let completedSteps = 0;
      for (let index = 0; index < totalSteps; index += 1) {
        if (signal.aborted) return { status: 'cancelled', finalState: state, observations, outputs: { completedSteps, totalSteps } };
        const remaining = options.durationMs - index * stepMs;
        const delta = Math.min(stepMs, remaining);
        const [result] = runCoupledMotor(context.reality, state, context.environment, options.parameters, delta, delta);
        state = result.authoritativeState;
        completedSteps += 1;
        observations.push(
          { logicalTimeMs: result.logicalTimeMs, componentId: result.componentId, signal: 'currentA', value: result.nextState.currentA, unit: 'A', source: 'coupled-physics' },
          { logicalTimeMs: result.logicalTimeMs, componentId: result.componentId, signal: 'angularVelocityRadPerS', value: result.nextState.angularVelocityRadPerS, unit: 'rad/s', source: 'coupled-physics' },
          { logicalTimeMs: result.logicalTimeMs, componentId: result.componentId, signal: 'windingTemperatureC', value: result.nextState.windingTemperatureC, unit: '°C', source: 'coupled-physics' },
        );
        if (completedSteps % checkpointEvery === 0 || result.failures.length) context.checkpoints.push(captureSnapshot(state, result.failures.length ? 'failure' : 'checkpoint'));
        if (result.failures.some((failure) => failure.severity === 'error')) {
          return { status: 'failed', finalState: state, observations, outputs: { completedSteps, totalSteps, failures: result.failures, energy: result.energy }, error: result.failures.map((failure) => failure.message).join('; ') };
        }
      }
      return { status: 'completed', finalState: state, observations, outputs: { completedSteps, totalSteps } };
    },
  };
}
