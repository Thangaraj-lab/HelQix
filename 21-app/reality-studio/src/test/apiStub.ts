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
      const body = raw ? JSON.parse(raw) : undefined;
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
