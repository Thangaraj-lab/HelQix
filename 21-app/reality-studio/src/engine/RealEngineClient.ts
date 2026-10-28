import {
  PointMassRunParams,
  AssetRef,
  BuildEvent,
  Checkpoint,
  EngineClient,
  Intervention,
  ModelOp,
  ModelRevision,
  Project,
  RunFrame,
  RunSummary,
  Source,
  StoredRunInfo
} from '../contracts';
import { foldRevision, fromWireEvent, fromWireFrame, fromWireProject, fromWireSource, toWireOp } from './wire';

/** Something the real engine does not offer yet. The UI shows this as a plain statement, never a fake result. */
export class EngineUnavailable extends Error {
  constructor(public feature: string, message: string) {
    super(message);
    this.name = 'EngineUnavailable';
  }
}

/** The HELQIX API (22-api) through the dev proxy, which adds the API key on the server side. */
export class RealEngineClient implements EngineClient {
  constructor(private base: string = '/api') {}

  private async call<T>(method: string, path: string, body?: unknown, headers: Record<string, string> = {}): Promise<{ data: T; etag: string | null }> {
    const res = await fetch(`${this.base}${path}`, {
      method,
      headers: { ...(body !== undefined ? { 'content-type': 'application/json' } : {}), ...headers },
      body: body !== undefined ? JSON.stringify(body) : undefined
    });
    const text = await res.text();
    if (!res.ok) {
      let detail = text;
      try { const j = JSON.parse(text); detail = typeof j.error === 'string' ? j.error : (j.error?.message ?? j.message ?? text); } catch { /* plain text */ }
      throw new Error(`The engine answered ${res.status}: ${String(detail).slice(0, 300) || res.statusText}`);
    }
    return { data: (text ? JSON.parse(text) : null) as T, etag: res.headers.get('etag') };
  }

  async createProject(): Promise<Project> {
    const { data } = await this.call<{ data: any }>('POST', '/v1/projects', { payload: { name: 'Untitled project' } });
    return fromWireProject(data.data, 0);
  }

  async listProjects(): Promise<Project[]> {
    const out: Project[] = [];
    let after: string | null = null;
    do {
      const { data } = await this.call<{ data: any[]; next_cursor: string | null }>('GET', `/v1/projects?limit=100${after ? `&after=${after}` : ''}`);
      const counts = await Promise.all(data.data.map(r => this.call<{ revisions: unknown[] }>('GET', `/v1/projects/${r.id}/revisions`).then(x => x.data.revisions.length).catch(() => 0)));
      data.data.forEach((r, i) => out.push(fromWireProject(r, counts[i])));
      after = data.next_cursor;
    } while (after);
    return out.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  }

  async renameProject(pid: string, name: string): Promise<Project> {
    const current = await this.call<{ data: any }>('GET', `/v1/projects/${pid}`);
    const payload = { ...current.data.data.payload, name: name.trim() || current.data.data.payload?.name || 'Untitled project' };
    const { data } = await this.call<{ data: any }>('PUT', `/v1/projects/${pid}`, { payload }, { 'if-match': current.etag ?? `"${current.data.data.version}"` });
    return fromWireProject(data.data, 0);
  }

  async listStoredRuns(): Promise<StoredRunInfo[]> {
    const { data } = await this.call<{ runs: any[] }>('GET', '/v1/runs');
    return data.runs.map(r => ({ id: r.runId, rows: r.rows, determinism: r.determinism, variables: r.variables }));
  }

  async startPointMassRun(p: PointMassRunParams): Promise<{ runId: string }> {
    const { data } = await this.call<{ runId: string }>('POST', '/v1/runs', p);
    return { runId: data.runId };
  }

  async runRevision(pid: string, revisionNum: number, p: Omit<PointMassRunParams, 'massKg' | 'referenceAreaM2'>): Promise<{ runId: string; massKg: number; areaM2: number; parts: number }> {
    const { data } = await this.call<{ runId: string; massKg: number; areaM2: number; parts: number }>('POST', `/v1/projects/${pid}/revisions/${revisionNum}/run`, p);
    return data;
  }

  async listRuns(pid: string): Promise<RunSummary[]> {
    const { data } = await this.call<{ runs: { runId: string; revision: number; startedAt: string }[] }>('GET', `/v1/projects/${pid}/runs`);
    return data.runs.map(r => ({ id: r.runId, projectId: pid, scenarioId: 'lumped-body', revisionNum: r.revision, startedAt: r.startedAt }));
  }

  async getRevision(pid: string, revisionNum: number): Promise<ModelRevision | null> {
    try {
      const { data } = await this.call<{ revision: any; ops: any[] }>('GET', `/v1/projects/${pid}/revisions/${revisionNum}`);
      return foldRevision(data.revision, data.ops);
    } catch (e) {
      if (e instanceof Error && / 404:/.test(e.message)) return null;
      throw e;
    }
  }

  async startBuild(pid: string, input: { prompt: string; attachments?: AssetRef[] } | { scratch: true } | { importAsset: AssetRef }): Promise<{ buildId: string }> {
    if ('prompt' in input) return this.interpretIntoBuild(pid, input.prompt);
    if ('importAsset' in input) throw new EngineUnavailable('import', 'Importing a file as a whole model is not available in the real engine yet.');
    const { data } = await this.call<{ buildId: string }>('POST', `/v1/projects/${pid}/builds`);
    return { buildId: data.buildId };
  }

  /**
   * A local open-source model proposes parts for the description; the API validates them. They are applied as
   * ordinary operations in a new build, labelled as AI hypotheses, so the engineer edits and commits them.
   */
  private async interpretIntoBuild(pid: string, prompt: string): Promise<{ buildId: string }> {
    let proposal: { model: string; parts: { name: string; shape: 'box' | 'cylinder' | 'sphere' | 'plate'; dimsMm: number[]; positionM: number[] }[] };
    try {
      proposal = (await this.call<typeof proposal>('POST', '/v1/interpret', { prompt })).data;
    } catch (e) {
      if (e instanceof Error && / 503:/.test(e.message)) throw new EngineUnavailable('interpret', 'No local language model is connected. Start Ollama and the API with HELQIX_OLLAMA_MODEL, or start from scratch and define the parts yourself.');
      throw e;
    }
    const { data } = await this.call<{ buildId: string }>('POST', `/v1/projects/${pid}/builds`);
    const hyp = (v: number, unit: string) => ({ v, unit, origin: 'ai_hypothesis' as const, status: 'unverified' as const });
    const ops: ModelOp[] = proposal.parts.flatMap((p, i): ModelOp[] => {
      const id = `part-ai-${i + 1}`;
      return [
        { op: 'part.add', id, name: p.name, geometry: { kind: 'primitive', shape: p.shape, dims: p.dimsMm.map(d => hyp(d, 'mm')) }, maturity: 'placeholder', prov: { kind: 'inferred', note: `Proposed by ${proposal.model} from your description; sizes are placeholders` } },
        { op: 'part.place', id, transform: { pos: [hyp(p.positionM[0], 'm'), hyp(p.positionM[1], 'm'), hyp(p.positionM[2], 'm')], rot: [hyp(0, 'deg'), hyp(0, 'deg'), hyp(0, 'deg')], scale: [1, 1, 1], frame: 'world' } }
      ];
    });
    await this.applyOps(data.buildId, ops);
    return { buildId: data.buildId };
  }

  /** Events already stored for the build. The stream stays open for new ones, so this reads until it goes quiet. */
  async *buildEvents(buildId: string): AsyncIterable<BuildEvent> {
    const ctl = new AbortController();
    const res = await fetch(`${this.base}/v1/builds/${buildId}/events`, { signal: ctl.signal, headers: { accept: 'text/event-stream' } });
    if (!res.ok || !res.body) throw new Error(`The engine answered ${res.status} for the build's events.`);
    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let buf = '';
    let idle: ReturnType<typeof setTimeout> | undefined;
    const arm = () => { clearTimeout(idle); idle = setTimeout(() => ctl.abort(), 700); };
    arm();
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        arm();
        buf += decoder.decode(value, { stream: true });
        let cut: number;
        while ((cut = buf.indexOf('\n\n')) >= 0) {
          const block = buf.slice(0, cut);
          buf = buf.slice(cut + 2);
          const data = block.split('\n').filter(l => l.startsWith('data:')).map(l => l.slice(5).trim()).join('');
          if (data) yield fromWireEvent(JSON.parse(data));
        }
      }
    } catch (e) {
      if ((e as Error).name !== 'AbortError') throw e;
    } finally {
      clearTimeout(idle);
    }
  }

  async answerQuestion(): Promise<void> {
    throw new EngineUnavailable('questions', 'Questions are not asked by the real engine yet.');
  }

  async applyOps(buildId: string, ops: ModelOp[]): Promise<BuildEvent[]> {
    const { data } = await this.call<{ events: any[] }>('POST', `/v1/builds/${buildId}/ops`, { ops: ops.map(toWireOp) });
    return data.events.map(fromWireEvent);
  }

  async undo(buildId: string): Promise<BuildEvent[]> {
    const { data } = await this.call<{ events: any[] }>('POST', `/v1/builds/${buildId}/undo`);
    return data.events.map(fromWireEvent);
  }

  async redo(buildId: string): Promise<BuildEvent[]> {
    const { data } = await this.call<{ events: any[] }>('POST', `/v1/builds/${buildId}/redo`);
    return data.events.map(fromWireEvent);
  }

  async commit(buildId: string): Promise<ModelRevision> {
    const { data } = await this.call<any>('POST', `/v1/builds/${buildId}/commit`);
    const full = await this.getRevision(data.projectId, data.number);
    if (!full) throw new Error('The revision was committed but could not be read back.');
    return full;
  }

  async listSources(): Promise<Source[]> {
    const { data } = await this.call<{ sources: any[] }>('GET', '/v1/sources');
    return data.sources.map(fromWireSource);
  }

  async run(): Promise<{ runId: string }> {
    throw new EngineUnavailable('run', 'The real engine has no solver connected to the Studio yet, so it cannot start a run.');
  }

  async *runFrames(runId: string): AsyncIterable<RunFrame> {
    let from: number | null = null;
    do {
      const { data } = await this.call<{ frames: any[]; next: number | null }>('GET', `/v1/runs/${runId}/frames${from !== null ? `?from=${from}` : ''}`);
      for (const f of data.frames) yield fromWireFrame(f);
      from = data.next;
    } while (from !== null);
  }

  async getFrame(runId: string, t: number): Promise<RunFrame> {
    const { data } = await this.call<{ frame: any }>('GET', `/v1/runs/${runId}/frame?t=${encodeURIComponent(t)}`);
    return fromWireFrame(data.frame);
  }

  async freeze(runId: string, t: number): Promise<Checkpoint> {
    const { data } = await this.call<{ checkpointId: string; runId: string; t: number; hash: string; state: Record<string, number> }>('POST', `/v1/runs/${runId}/freeze`, { t });
    const units: Record<string, string> = { x: 'm', y: 'm', z: 'm', vx: 'm/s', vy: 'm/s', vz: 'm/s' };
    const snapshot = Object.fromEntries(Object.entries(data.state).map(([k, v]) => [k, { v, unit: units[k] ?? '', origin: 'simulated' as const, status: 'unverified' as const }]));
    return { id: data.checkpointId, runId: data.runId, t: data.t, hash: data.hash, stateSnapshot: snapshot };
  }

  async branch(cp: string, intervention: Intervention): Promise<{ runId: string }> {
    const overrides = Object.fromEntries(Object.entries(intervention.paramOverrides).map(([k, val]) => [k, val.v]));
    const { data } = await this.call<{ runId: string }>('POST', '/v1/branches', { checkpointId: cp, overrides });
    return { runId: data.runId };
  }
}
