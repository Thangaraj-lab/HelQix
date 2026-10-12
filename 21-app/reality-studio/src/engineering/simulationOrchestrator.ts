import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, StateSnapshot } from './authoritativeState';
import { captureSnapshot } from './authoritativeState';
import type { ExecutableScenario } from './scenarioEngine';

export const SIMULATION_ORCHESTRATOR_SCHEMA = 'helqix.simulation-orchestrator.v1' as const;
export type SimulationStatus = 'planned' | 'queued' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled' | 'timed_out';

export interface SimulationSolver {
  id: string;
  version: string;
  capabilities: string[];
  deterministic: boolean;
}

export interface SimulationRequest {
  id: string;
  realityRevision: number;
  scenarioId: string;
  initialState: AuthoritativeEngineeringState;
  solver: SimulationSolver;
  inputs: Record<string, unknown>;
  timeoutMs: number;
  seed?: number;
}

export interface SimulationObservation {
  logicalTimeMs: number;
  componentId?: string;
  signal?: string;
  value?: unknown;
  unit?: string;
  source: string;
}

export interface SimulationResult {
  status: Exclude<SimulationStatus, 'planned' | 'queued' | 'running' | 'paused'>;
  finalState?: AuthoritativeEngineeringState;
  observations: SimulationObservation[];
  outputs: Record<string, unknown>;
  error?: string;
}

export interface SimulationExecutionContext {
  request: SimulationRequest;
  reality: EngineeringReality;
  scenario: ExecutableScenario;
  status: SimulationStatus;
  startedAt?: number;
  finishedAt?: number;
  logicalTimeMs: number;
  initialSnapshot: StateSnapshot;
  checkpoints: StateSnapshot[];
  observations: SimulationObservation[];
  outputs: Record<string, unknown>;
  result?: SimulationResult;
}

export interface SimulationAdapter {
  prepare(context: SimulationExecutionContext): Promise<void> | void;
  run(context: SimulationExecutionContext, signal: AbortSignal): Promise<SimulationResult>;
  cancel?(context: SimulationExecutionContext): Promise<void> | void;
}

export interface SimulationEvent {
  type: 'planned' | 'queued' | 'started' | 'checkpoint' | 'paused' | 'resumed' | 'completed' | 'failed' | 'cancelled' | 'timed_out';
  at: number;
  logicalTimeMs: number;
  message?: string;
}

export interface SimulationRun {
  schema: typeof SIMULATION_ORCHESTRATOR_SCHEMA;
  request: SimulationRequest;
  status: SimulationStatus;
  context: SimulationExecutionContext;
  events: SimulationEvent[];
}

function now(): number { return Date.now(); }
function clone<T>(value: T): T { return structuredClone(value); }

export function createSimulationRequest(
  reality: EngineeringReality,
  scenario: ExecutableScenario,
  initialState: AuthoritativeEngineeringState,
  solver: SimulationSolver,
  inputs: Record<string, unknown> = {},
  timeoutMs = 300_000,
  seed?: number,
): SimulationRequest {
  if (timeoutMs <= 0 || !Number.isFinite(timeoutMs)) throw new Error('Simulation timeout must be a positive finite number.');
  if (initialState.realityRevision !== reality.revision) throw new Error('Initial state does not match the current engineering reality revision.');
  if (!solver.id.trim() || !solver.version.trim()) throw new Error('Simulation solver id and version are required.');
  return {
    id: `sim-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    realityRevision: reality.revision,
    scenarioId: scenario.id,
    initialState: clone(initialState),
    solver: clone(solver),
    inputs: clone(inputs),
    timeoutMs,
    ...(seed === undefined ? {} : { seed }),
  };
}

export function createSimulationRun(
  reality: EngineeringReality,
  scenario: ExecutableScenario,
  request: SimulationRequest,
): SimulationRun {
  if (request.realityRevision !== reality.revision) throw new Error('Simulation request is based on a stale reality revision.');
  if (request.scenarioId !== scenario.id) throw new Error('Simulation request does not match the supplied scenario.');
  const initialSnapshot = captureSnapshot(request.initialState, 'initial', `simulation-initial-${request.id}`);
  const context: SimulationExecutionContext = {
    request: clone(request), reality: clone(reality), scenario: clone(scenario), status: 'planned',
    logicalTimeMs: request.initialState.logicalTimeMs, initialSnapshot, checkpoints: [initialSnapshot],
    observations: [], outputs: {},
  };
  return { schema: SIMULATION_ORCHESTRATOR_SCHEMA, request: clone(request), status: 'planned', context, events: [{ type: 'planned', at: now(), logicalTimeMs: context.logicalTimeMs }] };
}

function transition(run: SimulationRun, status: SimulationStatus, type: SimulationEvent['type'], message?: string): SimulationRun {
  const at = now();
  return { ...run, status, context: { ...run.context, status, ...(status === 'running' ? { startedAt: run.context.startedAt ?? at } : {}), ...(status === 'completed' || status === 'failed' || status === 'cancelled' || status === 'timed_out' ? { finishedAt: at } : {}) }, events: [...run.events, { type, at, logicalTimeMs: run.context.logicalTimeMs, ...(message ? { message } : {}) }] };
}

export function queueSimulation(run: SimulationRun): SimulationRun {
  if (run.status !== 'planned') throw new Error(`Cannot queue simulation from ${run.status}.`);
  return transition(run, 'queued', 'queued');
}

export function pauseSimulation(run: SimulationRun): SimulationRun {
  if (run.status !== 'running') throw new Error(`Cannot pause simulation from ${run.status}.`);
  return transition(run, 'paused', 'paused');
}

export function resumeSimulation(run: SimulationRun): SimulationRun {
  if (run.status !== 'paused') throw new Error(`Cannot resume simulation from ${run.status}.`);
  return transition(run, 'running', 'resumed');
}

export function checkpointSimulation(run: SimulationRun, reason = 'manual'): SimulationRun {
  if (!['running', 'paused'].includes(run.status)) throw new Error(`Cannot checkpoint simulation from ${run.status}.`);
  const snapshot = captureSnapshot(run.context.request.initialState, 'checkpoint', `simulation-checkpoint-${run.request.id}-${run.context.checkpoints.length}`);
  return { ...run, context: { ...run.context, checkpoints: [...run.context.checkpoints, snapshot] }, events: [...run.events, { type: 'checkpoint', at: now(), logicalTimeMs: snapshot.logicalTimeMs, message: reason }] };
}

export async function executeSimulation(run: SimulationRun, adapter: SimulationAdapter): Promise<SimulationRun> {
  if (!['planned', 'queued'].includes(run.status)) throw new Error(`Cannot execute simulation from ${run.status}.`);
  let active = transition(run, 'running', 'started');
  const controller = new AbortController();
  let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
  try {
    await adapter.prepare(active.context);
    timeoutHandle = setTimeout(() => controller.abort(), active.request.timeoutMs);
    const result = await adapter.run(active.context, controller.signal);
    if (controller.signal.aborted) {
      await adapter.cancel?.(active.context);
      return transition({ ...active, context: { ...active.context, result: { ...result, status: 'timed_out' } } }, 'timed_out', 'timed_out', `Simulation exceeded ${active.request.timeoutMs}ms.`);
    }
    const status: SimulationStatus = result.status === 'completed' ? 'completed' : result.status === 'cancelled' ? 'cancelled' : 'failed';
    const next = { ...active, context: { ...active.context, logicalTimeMs: result.finalState?.logicalTimeMs ?? active.context.logicalTimeMs, observations: [...active.context.observations, ...result.observations], outputs: clone(result.outputs), result } };
    return transition(next, status, status === 'completed' ? 'completed' : status === 'cancelled' ? 'cancelled' : 'failed', result.error);
  } catch (error) {
    const timedOut = controller.signal.aborted;
    await adapter.cancel?.(active.context).catch(() => undefined);
    const message = error instanceof Error ? error.message : 'Simulation execution failed.';
    return transition({ ...active, context: { ...active.context, result: { status: timedOut ? 'timed_out' : 'failed', observations: active.context.observations, outputs: {}, error: timedOut ? `Simulation exceeded ${active.request.timeoutMs}ms.` : message } } }, timedOut ? 'timed_out' : 'failed', timedOut ? 'timed_out' : 'failed', timedOut ? `Simulation exceeded ${active.request.timeoutMs}ms.` : message);
  } finally {
    if (timeoutHandle) clearTimeout(timeoutHandle);
  }
}

export function cancelSimulation(run: SimulationRun): SimulationRun {
  if (!['planned', 'queued', 'running', 'paused'].includes(run.status)) throw new Error(`Cannot cancel simulation from ${run.status}.`);
  return transition(run, 'cancelled', 'cancelled');
}

export function verifySimulationCompatibility(run: SimulationRun, reality: EngineeringReality, scenario: ExecutableScenario): string[] {
  const issues: string[] = [];
  if (run.request.realityRevision !== reality.revision) issues.push(`Reality revision mismatch: simulation=${run.request.realityRevision}, current=${reality.revision}.`);
  if (run.request.scenarioId !== scenario.id) issues.push(`Scenario mismatch: simulation=${run.request.scenarioId}, supplied=${scenario.id}.`);
  if (run.context.request.solver.deterministic && run.request.seed === undefined) issues.push('Deterministic solver requires an explicit seed for reproducible execution.');
  return issues;
}
