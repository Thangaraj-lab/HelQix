export const COMPUTE_FABRIC_SCHEMA = 'helqix.compute-fabric.v1' as const;
export type ComputeKind = 'cpu' | 'gpu' | 'quantum';
export type ComputeJobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface ComputeResource { id: string; kind: ComputeKind; capacity: number; labels: string[]; available: boolean; }
export interface ComputeRequest { id: string; operation: string; kind: ComputeKind; payload: unknown; timeoutMs: number; preferredResourceId?: string; }
export interface ComputeResult { status: 'completed' | 'failed' | 'cancelled' | 'timed_out'; output?: unknown; error?: string; resourceId?: string; startedAt?: number; finishedAt?: number; }
export interface ComputeProvider { id: string; listResources(): Promise<ComputeResource[]> | ComputeResource[]; execute(request: ComputeRequest, resource: ComputeResource, signal: AbortSignal): Promise<ComputeResult>; cancel?(requestId: string): Promise<void>; }

export interface ComputeJob { schema: typeof COMPUTE_FABRIC_SCHEMA; request: ComputeRequest; status: ComputeJobStatus; result?: ComputeResult; }

export function createComputeRequest(operation: string, kind: ComputeKind, payload: unknown, timeoutMs = 300_000): ComputeRequest {
  if (!operation.trim()) throw new Error('Compute operation is required.');
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) throw new Error('Compute timeout must be positive.');
  return { id: `compute-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, operation, kind, payload: structuredClone(payload), timeoutMs };
}

export async function executeCompute(provider: ComputeProvider, request: ComputeRequest): Promise<ComputeJob> {
  const resources = await provider.listResources();
  const resource = request.preferredResourceId
    ? resources.find((item) => item.id === request.preferredResourceId && item.available && item.kind === request.kind)
    : resources.find((item) => item.available && item.kind === request.kind);
  if (!resource) return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'failed', error: `No available ${request.kind} resource satisfies the compute request.` } };
  const controller = new AbortController();
  const startedAt = Date.now();
  const timeout = setTimeout(() => controller.abort(), request.timeoutMs);
  try {
    const result = await provider.execute(request, resource, controller.signal);
    if (controller.signal.aborted) {
      await provider.cancel?.(request.id).catch(() => undefined);
      return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: 'timed_out', error: `Compute request exceeded ${request.timeoutMs}ms.`, resourceId: resource.id, startedAt, finishedAt: Date.now() } };
    }
    return { schema: COMPUTE_FABRIC_SCHEMA, request, status: result.status === 'completed' ? 'completed' : result.status === 'cancelled' ? 'cancelled' : 'failed', result: { ...result, resourceId: result.resourceId ?? resource.id, startedAt: result.startedAt ?? startedAt, finishedAt: result.finishedAt ?? Date.now() } };
  } catch (error) {
    const timedOut = controller.signal.aborted;
    return { schema: COMPUTE_FABRIC_SCHEMA, request, status: 'failed', result: { status: timedOut ? 'timed_out' : 'failed', error: timedOut ? `Compute request exceeded ${request.timeoutMs}ms.` : error instanceof Error ? error.message : 'Compute execution failed.', resourceId: resource.id, startedAt, finishedAt: Date.now() } };
  } finally { clearTimeout(timeout); }
}
