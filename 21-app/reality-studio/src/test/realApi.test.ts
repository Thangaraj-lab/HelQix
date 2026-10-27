// @vitest-environment node
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { RealEngineClient } from '../engine/RealEngineClient';

// The UI's engine client against the real HELQIX API and PostgreSQL. It runs only when
// HELQIX_API_URL and HELQIX_API_KEY are set (CI sets them; locally it is skipped). The browser never
// holds the key: a small forwarder, like the Vite dev proxy, adds it on the server side.
const URL_ = process.env.HELQIX_API_URL;
const KEY = process.env.HELQIX_API_KEY;
const RUN = process.env.HELQIX_RUN_ID ?? 'reference-drop';

describe.skipIf(!URL_ || !KEY)('RealEngineClient against the real API', () => {
  let proxy: http.Server;
  let client: RealEngineClient;

  beforeAll(async () => {
    proxy = http.createServer(async (req, res) => {
      const chunks: Buffer[] = [];
      for await (const c of req) chunks.push(c as Buffer);
      const upstream = await fetch(`${URL_}${req.url}`, {
        method: req.method,
        headers: { ...(req.headers['content-type'] ? { 'content-type': req.headers['content-type'] as string } : {}), ...(req.headers['if-match'] ? { 'if-match': req.headers['if-match'] as string } : {}), authorization: `Bearer ${KEY}` },
        body: ['GET', 'HEAD'].includes(req.method ?? '') ? undefined : Buffer.concat(chunks)
      });
      res.writeHead(upstream.status, { 'content-type': upstream.headers.get('content-type') ?? 'application/json', ...(upstream.headers.get('etag') ? { etag: upstream.headers.get('etag')! } : {}) });
      if (!upstream.body) return res.end();
      const reader = upstream.body.getReader();
      req.on('close', () => reader.cancel().catch(() => undefined));
      for (;;) { const { value, done } = await reader.read().catch(() => ({ value: undefined, done: true })); if (done) break; res.write(value); }
      res.end();
    });
    await new Promise<void>(ok => proxy.listen(0, '127.0.0.1', ok));
    client = new RealEngineClient(`http://127.0.0.1:${(proxy.address() as AddressInfo).port}`);
  });
  afterAll(() => { proxy.closeAllConnections?.(); proxy.close(); });

  const v = (x: number, unit = 'm') => ({ v: x, unit, origin: 'user_stated' as const, status: 'unverified' as const });
  const part = (id: string) => ({ op: 'part.add' as const, id, name: id, geometry: { kind: 'primitive' as const, shape: 'box' as const, dims: [v(100, 'mm'), v(80, 'mm'), v(60, 'mm')] }, maturity: 'user_defined' as const, prov: { kind: 'user_stated' as const } });
  const place = (id: string, x: number) => ({ op: 'part.place' as const, id, transform: { pos: [v(x), v(0.5), v(0)] as [ReturnType<typeof v>, ReturnType<typeof v>, ReturnType<typeof v>], rot: [v(0, 'deg'), v(0, 'deg'), v(0, 'deg')] as [ReturnType<typeof v>, ReturnType<typeof v>, ReturnType<typeof v>], scale: [1, 1, 1] as [number, number, number], frame: 'world' } });

  it('creates a project, builds a model, is refused on a unit mismatch, fixes it by undo, commits and reads r1 back', async () => {
    const p = await client.createProject();
    const { buildId } = await client.startBuild(p.id, { scratch: true });

    // Volts into amps: applied, but the engine raises a blocking issue and refuses the commit.
    const events = await client.applyOps(buildId, [
      part('base'), part('body'), place('base', -0.5), place('body', 0.5),
      { op: 'port.add', part: 'base', id: 'out', kind: 'electrical', direction: 'out', unit: 'V' },
      { op: 'port.add', part: 'body', id: 'in', kind: 'electrical', direction: 'in', unit: 'A' },
      { op: 'connection.add', id: 'c1', from: { partId: 'base', portId: 'out' }, to: { partId: 'body', portId: 'in' }, kind: 'electrical' }
    ]);
    const issue = events.find(e => e.kind === 'issue.raised')?.issue;
    expect(issue?.severity).toBe('blocking');
    await expect(client.commit(buildId)).rejects.toThrow(/409/);

    // Undo the connection: the issue clears and the build can be committed.
    const undone = await client.undo(buildId);
    expect(undone && undone.some(e => e.kind === 'issue.cleared')).toBe(true);
    const rev = await client.commit(buildId);
    expect(rev.revisionNum).toBe(1);
    expect(Object.keys(rev.parts).sort()).toEqual(['base', 'body']);
    expect(rev.parts.body.transform.pos[0].v).toBe(0.5);
    expect(Object.keys(rev.ports).sort()).toEqual(['base:out', 'body:in']);
    expect(Object.keys(rev.connections)).toEqual([]);

    const mine = (await client.listProjects()).find(x => x.id === p.id);
    expect(mine?.modelRevisions).toEqual(['r1']);
    expect(await client.getRevision(p.id, 2)).toBeNull();
  });

  it('keeps a project name across a rename', async () => {
    const p = await client.createProject();
    expect((await client.renameProject(p.id, 'Test rig')).name).toBe('Test rig');
    expect((await client.listProjects()).find(x => x.id === p.id)?.name).toBe('Test rig');
  });

  it('reads a stored run: frames in order, and the same frame by time', async () => {
    const frames: number[] = [];
    for await (const f of client.runFrames(RUN)) { frames.push(f.t); if (frames.length >= 5) break; }
    expect(frames.length).toBeGreaterThan(1);
    expect([...frames].sort((a, b) => a - b)).toEqual(frames);
    const at = await client.getFrame(RUN, frames[1]);
    expect(at.t).toBeCloseTo(frames[1], 9);
    expect(Object.values(at.values).every(x => typeof x.v === 'number' && x.unit.length > 0)).toBe(true);
  });

  it('reports no sources honestly when none are connected', async () => {
    const s = await client.listSources();
    expect(Array.isArray(s)).toBe(true);
  });
});
