import type { AuthoritativeEngineeringState } from './authoritativeState';
import { setSignal, setStateStatus } from './authoritativeState';
import type { ComputeJob, ComputeResult } from './computeFabric';

export const COMPUTE_INTEGRATION_SCHEMA = 'helqix.compute-integration.v1' as const;

export interface ComputeObservation {
  executionId: string;
  resourceId?: string;
  status: ComputeResult['status'];
  logicalTimeMs: number;
  output: unknown;
  provenance: {
    providerId: string;
    operation: string;
    requestId: string;
    resourceId?: string;
    startedAt?: number;
    finishedAt?: number;
  };
}

export interface ComputeStateBinding {
  componentId: string;
  signal: string;
  unit?: string;
  source?: 'model' | 'solver' | 'adapter' | 'scenario' | 'runtime';
  select?: (output: unknown) => string | number | boolean;
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function defaultValue(output: unknown): string | number | boolean {
  if (typeof output === 'string' || typeof output === 'number' || typeof output === 'boolean') return output;
  throw new Error('Compute output must be scalar or a binding selector must be provided.');
}

export function createComputeObservation(job: ComputeJob, providerId: string): ComputeObservation {
  return {
    executionId: job.request.id,
    resourceId: job.result?.resourceId,
    status: job.result?.status ?? 'failed',
    logicalTimeMs: job.result?.finishedAt ?? Date.now(),
    output: clone(job.result?.output),
    provenance: {
      providerId,
      operation: job.request.operation,
      requestId: job.request.id,
      resourceId: job.result?.resourceId,
      startedAt: job.result?.startedAt,
      finishedAt: job.result?.finishedAt,
    },
  };
}

/**
 * Applies an already-completed compute result to authoritative state.
 * The compute adapter owns execution; this bridge owns only normalization and
 * state binding. It never invents engineering telemetry from an error result.
 */
export function bindComputeResult(
  state: AuthoritativeEngineeringState,
  job: ComputeJob,
  providerId: string,
  bindings: ComputeStateBinding[],
): { state: AuthoritativeEngineeringState; observation: ComputeObservation } {
  const observation = createComputeObservation(job, providerId);
  if (job.status !== 'completed' || job.result?.status !== 'completed') {
    return { state: setStateStatus(state, job.result?.status === 'cancelled' ? 'paused' : 'failed'), observation };
  }

  let next = state;
  for (const binding of bindings) {
    if (!next.components[binding.componentId]) throw new Error(`Unknown compute binding component: ${binding.componentId}`);
    const selected = binding.select ? binding.select(observation.output) : defaultValue(observation.output);
    next = setSignal(next, binding.componentId, binding.signal, selected, binding.unit, binding.source ?? 'adapter');
  }
  return { state: setStateStatus(next, 'completed'), observation };
}

export function validateComputeBinding(binding: ComputeStateBinding): string[] {
  const issues: string[] = [];
  if (!binding.componentId.trim()) issues.push('Compute binding componentId is required.');
  if (!binding.signal.trim()) issues.push('Compute binding signal is required.');
  if (binding.unit !== undefined && !binding.unit.trim()) issues.push('Compute binding unit cannot be blank.');
  return issues;
}
