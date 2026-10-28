// A stand-in for 22-api that is as strict as the real one about what it accepts (serde
// deny_unknown_fields) and answers in the real response shapes. Used by the client tests, and by
// hand:  node src/test/apiStub.ts 8090   (then HELQIX_API_URL=http://127.0.0.1:8090 VITE_ENGINE=real)
import http from 'node:http';
import type { AddressInfo } from 'node:net';

const ALLOWED_OP_KEYS: Record<string, string[]> = {
  'part.add': ['op', 'id', 'name', 'geometry', 'maturity', 'prov'],
  'part.place': ['op', 'id', 'transform'],
  'port.add': ['op', 'part', 'id', 'kind', 'direction', 'unit'],
  'connection.add': ['op', 'id', 'from', 'to', 'kind'],
  'binding.add': ['op', 'binding'],
  'requirement.add': ['op', 'id', 'text', 'expr', 'prov']
};
const ALLOWED_BINDING_KEYS = ['variable', 'node', 'channel', 'axis', 'mapping', 'units', 'range'];

const projects = new Map<string, any>();
const builds = new Map<string, { pid: string; ops: any[]; events: any[] }>();
const revisions = new Map<string, any[]>();
export const seen = { authorization: false };

// A stored drop run: a point mass falling from 20 m, y(t) = 20 - g t^2 / 2, sampled every 10 ms.
const DROP_VARS = ['x', 'y', 'z', 'vx', 'vy', 'vz'].map(name => ({ name, unit: name.startsWith('v') ? 'm/s' : 'm' }));
const val = (v: number, unit: string) => ({ v, unit, origin: 'simulated', status: 'unverified' });
const DROP = Array.from({ length: 101 }, (_, i) => {
  const t = i / 100;
  return { runId: 'reference-drop', index: i, t: val(t, 's'), state: { x: val(3 * t, 'm'), y: val(20 - 0.5 * 9.80665 * t * t, 'm'), z: val(0, 'm'), vx: val(3, 'm/s'), vy: val(-9.80665 * t, 'm/s'), vz: val(0, 'm/s') }, determinism: 'Bitwise', stateSha256: 'c'.repeat(64) };
});

const send = (res: http.ServerResponse, status: number, body: unknown, headers: Record<string, string> = {}) => {
  res.writeHead(status, { 'content-type': 'application/json', ...headers });
  res.end(JSON.stringify(body));
};

export async function startStub(port = 0): Promise<{ server: http.Server; base: string }> {
  const server = http.createServer((req, res) => {
    let raw = '';
    req.on('data', c => (raw += c));
    req.on('end', () => {
      if (req.headers.authorization) seen.authorization = true;
      const url = new URL(req.url!, 'http://x');
      const body: any = raw ? JSON.parse(raw) : undefined;
      const m = (re: RegExp) => re.exec(url.pathname);
      let r: RegExpExecArray | null;
      if (req.method === 'POST' && url.pathname === '/v1/projects') {
        const id = crypto.randomUUID();
        const rec = { id, kind: 'projects', project_id: null, payload: body.payload, version: 1, created_at: 'c', updated_at: 'u' };
        projects.set(id, rec);
        return send(res, 201, { data: rec }, { etag: '"1"' });
      }
      if (req.method === 'GET' && url.pathname === '/v1/projects') return send(res, 200, { data: [...projects.values()], next_cursor: null });
      if ((r = m(/^\/v1\/projects\/([^/]+)$/)) && req.method === 'GET') return send(res, 200, { data: projects.get(r[1]) }, { etag: `"${projects.get(r[1]).version}"` });
      if ((r = m(/^\/v1\/projects\/([^/]+)$/)) && req.method === 'PUT') {
        if (req.headers['if-match'] !== `"${projects.get(r[1]).version}"`) return send(res, 412, { error: 'precondition' });
        const rec = { ...projects.get(r[1]), payload: body.payload, version: projects.get(r[1]).version + 1 };
        projects.set(r[1], rec);
        return send(res, 200, { data: rec });
      }
      if ((r = m(/^\/v1\/projects\/([^/]+)\/builds$/)) && req.method === 'POST') {
        const id = crypto.randomUUID();
        builds.set(id, { pid: r[1], ops: [], events: [] });
        return send(res, 201, { buildId: id, projectId: r[1], status: 'staged' });
      }
      if ((r = m(/^\/v1\/builds\/([^/]+)\/ops$/)) && req.method === 'POST') {
        const b = builds.get(r[1])!;
        for (const op of body.ops) {
          const allowed = ALLOWED_OP_KEYS[op.op];
          if (!allowed) return send(res, 422, { error: `unknown op ${op.op}` });
          const extra = Object.keys(op).filter(k => !allowed.includes(k));
          if (extra.length) return send(res, 422, { error: `unknown field(s) ${extra.join(',')} in ${op.op}` });
          if (op.op === 'binding.add') {
            const bx = Object.keys(op.binding).filter(k => !ALLOWED_BINDING_KEYS.includes(k));
            if (bx.length) return send(res, 422, { error: `unknown binding field(s) ${bx.join(',')}` });
          }
          if (op.op === 'connection.add' && !(op.from.part && op.from.port)) return send(res, 422, { error: 'connection needs part and port' });
        }
        b.ops.push(...body.ops);
        const events: any[] = body.ops.map((op: any, i: number) => ({ seq: b.events.length + i + 1, buildId: r![1], ts: 't', kind: 'op.applied', op }));
        events.push({ seq: b.events.length + events.length + 1, buildId: r[1], ts: 't', kind: 'staged.ready', note: `${b.ops.length} ops, no blocking issues` });
        b.events.push(...events);
        return send(res, 200, { events, cursor: b.ops.length, issues: [] });
      }
      if ((r = m(/^\/v1\/builds\/([^/]+)\/commit$/)) && req.method === 'POST') {
        const b = builds.get(r[1])!;
        if (b.ops.length === 0) return send(res, 409, { error: 'state conflict: nothing to commit' });
        const list = revisions.get(b.pid) ?? [];
        list.push(b.ops);
        revisions.set(b.pid, list);
        return send(res, 201, { projectId: b.pid, number: list.length, label: `r${list.length}`, buildId: r[1], opCount: b.ops.length, modelSha256: 'a'.repeat(64), committedAt: 'now' });
      }
      if ((r = m(/^\/v1\/builds\/([^/]+)\/events$/)) && req.method === 'GET') {
        res.writeHead(200, { 'content-type': 'text/event-stream' });
        for (const e of builds.get(r[1])!.events) res.write(`id: ${e.seq}\nevent: ${e.kind}\ndata: ${JSON.stringify(e)}\n\n`);
        return; // stays open, like the real stream; the client must stop on its own
      }
      if ((r = m(/^\/v1\/projects\/([^/]+)\/revisions$/)) && req.method === 'GET') {
        const list = revisions.get(r[1]) ?? [];
        return send(res, 200, { revisions: list.map((_, i) => ({ projectId: r![1], number: i + 1, label: `r${i + 1}`, buildId: 'b', opCount: 1, modelSha256: 'a'.repeat(64), committedAt: 'now' })) });
      }
      if ((r = m(/^\/v1\/projects\/([^/]+)\/revisions\/(\d+)$/)) && req.method === 'GET') {
        const ops = (revisions.get(r[1]) ?? [])[Number(r[2]) - 1];
        if (!ops) return send(res, 404, { error: 'resource not found' });
        return send(res, 200, { revision: { projectId: r[1], number: Number(r[2]), label: `r${r[2]}`, buildId: 'b', opCount: ops.length, modelSha256: 'a'.repeat(64), committedAt: 'now' }, ops });
      }
      if (req.method === 'GET' && url.pathname === '/v1/runs') return send(res, 200, { runs: [{ runId: 'reference-drop', rows: DROP.length, determinism: 'Bitwise', variables: DROP_VARS, framesBlake3: 'b'.repeat(64) }] });
      if ((r = m(/^\/v1\/runs\/([^/]+)\/frames$/)) && req.method === 'GET') {
        if (r[1] !== 'reference-drop') return send(res, 404, { error: 'resource not found' });
        const from = Number(url.searchParams.get('from') ?? 0), limit = Number(url.searchParams.get('limit') ?? 1000);
        const frames = DROP.filter(f => f.index >= from).slice(0, limit);
        const last = frames[frames.length - 1];
        return send(res, 200, { runId: r[1], framesBlake3: 'b'.repeat(64), rows: DROP.length, determinism: 'Bitwise', frames, next: last && last.index + 1 < DROP.length ? last.index + 1 : null });
      }
      if ((r = m(/^\/v1\/runs\/([^/]+)\/frame$/)) && req.method === 'GET') {
        const t = Number(url.searchParams.get('t'));
        const f = DROP.find(x => Math.abs(x.t.v - t) < 1e-9);
        return f ? send(res, 200, { framesBlake3: 'b'.repeat(64), frame: f }) : send(res, 422, { error: 'rejected: time outside the run' });
      }
      if ((r = m(/^\/v1\/runs\/([^/]+)\/freeze$/)) && req.method === 'POST') {
        const keys = Object.keys(body ?? {});
        if (keys.length !== 1 || keys[0] !== 't' || typeof body.t !== 'number') return send(res, 422, { error: 'unknown field' });
        const f = DROP.find(x => Math.abs(x.t.v - body.t) < 1e-9) ?? DROP[0];
        return send(res, 200, { checkpointId: `${r[1]}@${f.index}`, runId: r[1], index: f.index, t: f.t.v, hash: 'c'.repeat(64), state: { x: 0, y: 20, z: 0, vx: 3, vy: 0, vz: 0 } });
      }
      if (req.method === 'POST' && url.pathname === '/v1/branches') {
        const keys = Object.keys(body ?? {}).sort().join();
        if (keys !== 'checkpointId,overrides' || typeof body.overrides?.referenceAreaM2 !== 'number') return send(res, 422, { error: 'unknown field' });
        return send(res, 201, { runId: 'br-stub', parentRunId: 'reference-drop', index: 0, rows: DROP.length, framesBlake3: 'd'.repeat(64) });
      }
      if (req.method === 'GET' && url.pathname === '/v1/sources') return send(res, 200, { sources: [], note: 'No sources connected. Parts will be placeholders or defined from scratch.' });
      return send(res, 404, { error: 'resource not found' });
    });
  });
  await new Promise<void>(ok => server.listen(port, '127.0.0.1', ok));
  return { server, base: `http://127.0.0.1:${(server.address() as AddressInfo).port}` };
}

if (process.argv[1]?.endsWith('apiStub.ts')) {
  startStub(Number(process.argv[2] ?? 8090)).then(({ base }) => console.log(`API stub listening on ${base}`));
}
