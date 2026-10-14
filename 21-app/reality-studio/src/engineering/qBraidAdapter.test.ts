import { describe, expect, it } from 'vitest';
import { QBraidAdapter } from './qBraidAdapter';

describe('QBraidAdapter', () => {
  it('discovers quantum resources through the runtime boundary', async () => {
    const adapter = new QBraidAdapter({
      client: {
        async listBackends() { return [{ id: 'sim-1', kind: 'simulator', qubits: 32, available: true }]; },
        async submit() { return { jobId: 'job-1' }; },
        async status() { return { status: 'completed' }; },
        async result() { return { value: 42 }; },
      },
    });
    await expect(adapter.listResources()).resolves.toEqual([{ id: 'sim-1', kind: 'quantum', capacity: 32, labels: ['quantum', 'qbraid', 'simulator', 'sim-1'], available: true }]);
  });

  it('rejects classical requests at the adapter boundary', async () => {
    const adapter = new QBraidAdapter({ client: {
      async listBackends() { return []; },
      async submit() { return { jobId: 'unused' }; },
      async status() { return { status: 'failed' as const }; },
      async result() { return null; },
    } });
    const result = await adapter.execute({ id: 'request-1', kind: 'cpu', operation: 'solve', payload: {}, timeoutMs: 1000 }, { id: 'sim-1', kind: 'cpu', capacity: 1, labels: [], available: true }, new AbortController().signal);
    expect(result.status).toBe('failed');
    expect(result.error).toContain('only quantum');
  });

  it('submits, polls, returns results, and clears active job state', async () => {
    const cancelled: string[] = [];
    const adapter = new QBraidAdapter({ client: {
      async listBackends() { return []; },
      async submit() { return { jobId: 'job-2' }; },
      async status() { return { status: 'completed' as const }; },
      async result() { return { optimum: 0.91 }; },
      async cancel(jobId: string) { cancelled.push(jobId); },
    } });
    const result = await adapter.execute({ id: 'request-2', kind: 'quantum', operation: 'qaoa', payload: { layers: 2 }, timeoutMs: 1000 }, { id: 'qpu-1', kind: 'quantum', capacity: 20, labels: [], available: true }, new AbortController().signal);
    expect(result).toMatchObject({ status: 'completed', resourceId: 'qpu-1', output: { optimum: 0.91 } });
    expect(cancelled).toEqual([]);
    await expect(adapter.cancel('request-2')).resolves.toBeUndefined();
    expect(cancelled).toEqual([]);
  });
});
