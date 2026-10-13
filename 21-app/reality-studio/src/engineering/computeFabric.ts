export const COMPUTE_FABRIC_SCHEMA = 'helqix.compute-fabric.v1' as const;
export type ComputeKind = 'cpu' | 'gpu' | 'quantum';
export type ComputeJobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface ComputeResource { id: string; kind: ComputeKind; capacity: number; labels: string[]; available: boolean; }
export interface ComputeRequest { id: string; operation: string; kind: ComputeKind; payload: unknown; timeoutMs: number; preferredResourceId?: string; }
export interface ComputeResult { status: 'completed' | 'failed' | 'cancelled' | 'timed_out'; output?: unknown; error?: string; resourceId?: string; startedAt?: number; finishedAt?: number; }
export interface ComputeProvider { id: string; listResources(): Promise<ComputeResource[]> | ComputeResource[]; execute(request: ComputeRequest, resource: ComputeResource, signal: AbortSignal): Promise<ComputeResult>; cancel?(requestId: string): Promise<void>; }
export interface ComputeJob { schema: typeof COMPUTE_FABRIC_SCHEMA; request: ComputeRequest; status: ComputeJobStatus; result?: ComputeResult; }

function clone<T>(value: T): T { return structuredClone(value); }
function normalizeId(value: string, label: string): string { const id = value.trim(); if (!id) throw new Error(`${label} is required.`); return id; }

export function createComputeRequest(operation: string, kind: ComputeKind, payload: unknown, timeoutMs = 300_000): ComputeRequest {
  const normalizedOperation = normalizeId(operation, 'Compute operation');
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) throw new Error('Compute timeout must be positive.');
  return { id: `compute-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, operation: normalizedOperation, kind, payload: clone(payload), timeoutMs };
}

function selectResource(resources: ComputeResource[], request: ComputeRequest): ComputeResource | undefined {
  const usable = resources.filter((resource) => resource.available && resource.kind === request.kind && resource.capacity > 0 && resource.id.trim());
  if (request.preferredResourceId) {
    const preferred = usable.find((resource) => resource.id === request.preferredResourceId);
    if (preferred) return preferred;
  }
  return [...usable].sort((a, b) => b.capacity - a.capacity || a.id.localeCompare(b.id))[0];
}

export async function executeCompute(provider: ComputeProvider, request: ComputeRequest): Promise<ComputeJob> {
  const requestId = normalizeId(request.id, 'Compute request id');
  if (!request.operation.trim()) return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'failed', error: 'Compute operation is required.' } };
  if (!Number.isFinite(request.timeoutMs) || request.timeoutMs <= 0) return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'failed', error: 'Compute timeout must be positive.' } };
  const resources = await provider.listResources();
  if (!Array.isArray(resources)) return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'failed', error: 'Compute provider returned an invalid resource list.' } };
  const resource = selectResource(resources, request);
  if (!resource) return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'failed', error: `No available ${request.kind} resource satisfies the compute request.` } };

  const controller = new AbortController();
  const startedAt = Date.now();
  let timedOut = false;
  let settled = false;
  const timeout = setTimeout(() => {
    if (settled) return;
    timedOut = true;
    controller.abort();
    if (provider.cancel) void Promise.resolve(provider.cancel(requestId)).catch(() => undefined);
  }, request.timeoutMs);

  try {
    const result = await provider.execute({ ...request, id: requestId, payload: clone(request.payload) }, clone(resource), controller.signal);
    const finishedAt = Date.now();
    settled = true;
    if (!result || typeof result !== 'object') return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'failed', error: 'Compute provider returned an invalid result.', resourceId: resource.id, startedAt, finishedAt } };
    if (timedOut || controller.signal.aborted) return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'timed_out', error: `Compute request exceeded ${request.timeoutMs}ms.`, resourceId: resource.id, startedAt, finishedAt } };
    const status: ComputeJobStatus = result.status === 'completed' ? 'completed' : result.status === 'cancelled' ? 'cancelled' : 'failed';
    return { schema: COMPUTE_FABRIC_SCHEMA, request, status, result: { ...clone(result), status: result.status, resourceId: result.resourceId ?? resource.id, startedAt: result.startedAt ?? startedAt, finishedAt: result.finishedAt ?? finishedAt } };
  } catch (error) {
    const finishedAt = Date.now();
    settled = true;
    return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: timedOut ? 'timed_out' : 'failed', error: timedOut ? `Compute request exceeded ${request.timeoutMs}ms.` : error instanceof Error ? error.message : 'Compute execution failed.', resourceId: resource.id, startedAt, finishedAt } };
  } finally {
    settled = true;
    clearTimeout(timeout);
  }
}
