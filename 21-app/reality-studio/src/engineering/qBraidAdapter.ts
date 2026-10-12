import type { ComputeProvider, ComputeResource, ComputeRequest, ComputeResult } from './computeFabric';

export const QBRAID_ADAPTER_SCHEMA = 'helqix.qbraid-adapter.v1' as const;

export interface QBraidRuntimeClient {
  listBackends(): Promise<Array<{ id: string; name?: string; kind?: 'simulator' | 'qpu'; available?: boolean; qubits?: number }>>;
  submit(request: { backendId: string; operation: string; payload: unknown; timeoutMs: number }): Promise<{ jobId: string }>;
  status(jobId: string): Promise<{ status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled'; error?: string }>;
  result(jobId: string): Promise<unknown>;
  cancel?(jobId: string): Promise<void>;
}

export interface QBraidAdapterOptions {
  client: QBraidRuntimeClient;
  backendIds?: string[];
  pollIntervalMs?: number;
}

/**
 * Provider-neutral HELQIX boundary for qBraid Runtime.
 * Authentication, endpoint selection and provider credentials stay outside
 * this module. The adapter only translates HELQIX compute jobs to the
 * qBraid runtime lifecycle and normalizes the result back to HELQIX.
 */
export class QBraidAdapter implements ComputeProvider {
  readonly id = 'qbraid-runtime';
  private readonly client: QBraidRuntimeClient;
  private readonly backendIds?: Set<string>;
  private readonly pollIntervalMs: number;
  private readonly activeJobs = new Map<string, string>();

  constructor(options: QBraidAdapterOptions) {
    this.client = options.client;
    this.backendIds = options.backendIds ? new Set(options.backendIds) : undefined;
    this.pollIntervalMs = Math.max(25, options.pollIntervalMs ?? 250);
  }

  async listResources(): Promise<ComputeResource[]> {
    const backends = await this.client.listBackends();
    return backends
      .filter((backend) => !this.backendIds || this.backendIds.has(backend.id))
      .map((backend) => ({
        id: backend.id,
        kind: 'gpu' as const,
        capacity: backend.qubits ?? 1,
        labels: ['quantum', 'qbraid', backend.kind ?? 'unknown', backend.name ?? backend.id],
        available: backend.available !== false,
      }));
  }

  async execute(request: ComputeRequest, resource: ComputeResource, signal: AbortSignal): Promise<ComputeResult> {
    const startedAt = Date.now();
    if (signal.aborted) return { status: 'cancelled', resourceId: resource.id, startedAt, finishedAt: Date.now() };

    const submitted = await this.client.submit({
      backendId: resource.id,
      operation: request.operation,
      payload: structuredClone(request.payload),
      timeoutMs: request.timeoutMs,
    });
    this.activeJobs.set(request.id, submitted.jobId);

    try {
      while (true) {
        if (signal.aborted) {
          await this.client.cancel?.(submitted.jobId).catch(() => undefined);
          return { status: 'cancelled', resourceId: resource.id, startedAt, finishedAt: Date.now() };
        }
        const state = await this.client.status(submitted.jobId);
        if (state.status === 'completed') {
          return { status: 'completed', output: await this.client.result(submitted.jobId), resourceId: resource.id, startedAt, finishedAt: Date.now() };
        }
        if (state.status === 'failed') return { status: 'failed', error: state.error ?? 'qBraid job failed.', resourceId: resource.id, startedAt, finishedAt: Date.now() };
        if (state.status === 'cancelled') return { status: 'cancelled', error: state.error, resourceId: resource.id, startedAt, finishedAt: Date.now() };
        await new Promise<void>((resolve) => setTimeout(resolve, this.pollIntervalMs));
      }
    } finally {
      this.activeJobs.delete(request.id);
    }
  }

  async cancel(requestId: string): Promise<void> {
    const jobId = this.activeJobs.get(requestId);
    if (jobId) await this.client.cancel?.(jobId);
  }
}

export function createQBraidAdapter(options: QBraidAdapterOptions): QBraidAdapter {
  return new QBraidAdapter(options);
}
