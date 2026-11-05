// @vitest-environment node
import http from 'node:http';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { RealEngineClient } from '../engine/RealEngineClient';

// A tiny server with the answers of the real API for the FMU routes (shapes copied from 22-api/helqix-api/tests/fmu_run_api.rs).
let server: http.Server;
let base = '';
const calls: { method: string; url: string; body: string }[] = [];
const SHA = 'a3'.repeat(32);

beforeAll(async () => {
  server = http.createServer((req, res) => {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      calls.push({ method: req.method ?? '', url: req.url ?? '', body });
      const send = (status: number, j: unknown) => { res.writeHead(status, { 'content-type': 'application/json' }); res.end(JSON.stringify(j)); };
      if (req.url === '/v1/fmus') return send(200, { fmus: [{ sha256: SHA, bytes: 218148 }] });
      if (req.url === `/v1/fmus/${SHA}`) return send(200, { sha256: SHA, bytes: 218148, modelName: 'BouncingBall', fmiVersion: '2.0', defaultExperiment: { startTime: 0, stopTime: 3, stepSize: 0.01 },
        variables: [{ name: 'g', causality: 'parameter', variability: 'fixed', type: 'Real', unit: 'm/s2', description: '', dimension: [1, 0, -2, 0, 0, 0, 0], siFactor: 1, start: -9.81, unitProblem: null }] });
      if (req.url?.startsWith('/v1/fmus/')) return send(422, { error: { code: 'rejected', message: 'the FMU is not in the FMU library; ask the operator to install it' } });
      if (req.url === '/v1/projects/p1/revisions/2/run') return send(201, { runId: 'run-abc', rows: 301, framesBlake3: 'x', determinism: 'Tolerance', fmuSha256: SHA, parts: 1 });
      send(404, { error: { code: 'not_found', message: 'resource not found' } });
    });
  });
  await new Promise<void>(r => server.listen(0, '127.0.0.1', r));
  base = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
});
afterAll(() => { server.closeAllConnections?.(); server.close(); });

describe('the FMU routes through the real client', () => {
  it('lists the library', async () => {
    expect(await new RealEngineClient(base).listFmus()).toEqual([{ sha256: SHA, bytes: 218148 }]);
  });
  it('describes an FMU', async () => {
    const d = await new RealEngineClient(base).describeFmu(SHA);
    expect(d.modelName).toBe('BouncingBall');
    expect(d.variables[0]).toMatchObject({ name: 'g', causality: 'parameter', unit: 'm/s2', start: -9.81, dimension: [1, 0, -2, 0, 0, 0, 0] });
  });
  it('says why an FMU was refused', async () => {
    await expect(new RealEngineClient(base).describeFmu('b4'.repeat(32))).rejects.toThrow(/not in the FMU library/);
  });
  it('runs a revision with only the time span (no point-mass fields)', async () => {
    const r = await new RealEngineClient(base).runFmuRevision('p1', 2, { endSeconds: 3, stepSeconds: 0.01 });
    expect(r).toEqual({ runId: 'run-abc', rows: 301, fmuSha256: SHA, determinism: 'Tolerance' });
    const sent = calls.find(c => c.url.endsWith('/run'))!;
    expect(JSON.parse(sent.body)).toEqual({ endSeconds: 3, stepSeconds: 0.01 });
  });
});
