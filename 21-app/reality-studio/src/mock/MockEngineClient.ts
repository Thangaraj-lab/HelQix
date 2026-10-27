import {
  EngineClient,
  Project,
  BuildEvent,
  ModelOp,
  ModelRevision,
  Source,
  RunFrame,
  Checkpoint,
  Intervention,
  AssetRef,
  Value,
  RunSummary
} from '../contracts';
import { getMockBuildScriptEvents, MOCK_SOURCES, createMockValue } from './examplePack';

class MockEngineClientImpl implements EngineClient {
  private projects: Map<string, Project> = new Map();
  private stagedBuilds: Map<string, { pid: string; events: BuildEvent[]; ops: ModelOp[]; undos: ModelOp[] }> = new Map();
  private revisions: Map<string, ModelRevision> = new Map();
  private checkpoints: Map<string, Checkpoint> = new Map();
  private runs: RunSummary[] = [];

  constructor() {
    this.restoreState();
  }

  private saveState() {
    try {
      const data = {
        projects: Array.from(this.projects.entries()),
        revisions: Array.from(this.revisions.entries()),
        checkpoints: Array.from(this.checkpoints.entries()),
        runs: this.runs
      };
      localStorage.setItem('helqix_mock_engine_state', JSON.stringify(data));
    } catch {
      // LocalStorage unavailable in non-browser env
    }
  }

  private restoreState() {
    try {
      const raw = localStorage.getItem('helqix_mock_engine_state');
      if (raw) {
        const data = JSON.parse(raw);
        this.projects = new Map(data.projects);
        this.revisions = new Map(data.revisions);
        this.checkpoints = new Map(data.checkpoints);
        this.runs = data.runs ?? [];
      }
    } catch {
      // Clean fallback
    }
  }

  async createProject(): Promise<Project> {
    const id = `prj-${Date.now().toString(36)}`;
    const project: Project = {
      id,
      name: 'Untitled project',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      modelRevisions: []
    };
    this.projects.set(id, project);
    this.saveState();
    return project;
  }

  async listProjects(): Promise<Project[]> {
    this.restoreState();
    return Array.from(this.projects.values()).sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  }

  async renameProject(pid: string, name: string): Promise<Project> {
    this.restoreState();
    const p = this.projects.get(pid);
    if (!p) throw new Error(`Project ${pid} not found`);
    p.name = name.trim() || p.name;
    p.updatedAt = new Date().toISOString();
    this.saveState();
    return p;
  }

  async listRuns(pid: string): Promise<RunSummary[]> {
    this.restoreState();
    return this.runs.filter(r => r.projectId === pid).sort((a, b) => b.startedAt.localeCompare(a.startedAt));
  }

  async getRevision(pid: string, revisionNum: number): Promise<ModelRevision | null> {
    this.restoreState();
    const id = this.projects.get(pid)?.modelRevisions[revisionNum - 1];
    return (id && this.revisions.get(id)) || null;
  }

  async startBuild(
    pid: string,
    input: { prompt: string; attachments?: AssetRef[] } | { scratch: true } | { importAsset: AssetRef }
  ): Promise<{ buildId: string }> {
    const buildId = `build-${pid}-${Date.now().toString(36)}`;
    let initialEvents: BuildEvent[] = [];

    if ('scratch' in input && input.scratch) {
      initialEvents = [
        {
          seq: 1,
          buildId,
          ts: new Date().toISOString(),
          kind: 'step.start',
          step: 'import',
          note: 'Started blank engineering model workspace from scratch.'
        },
        {
          seq: 2,
          buildId,
          ts: new Date().toISOString(),
          kind: 'staged.ready',
          note: 'Empty model draft initialized. Add parts, ports, and connections.'
        }
      ];
    } else {
      initialEvents = getMockBuildScriptEvents(buildId);
    }

    const ops: ModelOp[] = [];
    initialEvents.forEach(e => {
      if (e.op) ops.push(e.op);
    });

    this.stagedBuilds.set(buildId, {
      pid,
      events: initialEvents,
      ops,
      undos: []
    });

    return { buildId };
  }

  async *buildEvents(buildId: string): AsyncIterable<BuildEvent> {
    const staged = this.stagedBuilds.get(buildId);
    if (!staged) return;

    for (const evt of staged.events) {
      // Yield real chronological events with short simulated transmission latency
      yield evt;
    }
  }

  async answerQuestion(buildId: string, qid: string, answer: string | 'default' | 'skip'): Promise<void> {
    const staged = this.stagedBuilds.get(buildId);
    if (!staged) return;

    staged.events.push({
      seq: staged.events.length + 1,
      buildId,
      ts: new Date().toISOString(),
      kind: 'step.done',
      step: 'interpret',
      note: `Question ${qid} answered with: ${answer}`
    });
  }

  async applyOps(buildId: string, ops: ModelOp[]): Promise<BuildEvent[]> {
    const staged = this.stagedBuilds.get(buildId);
    if (!staged) throw new Error(`Build ${buildId} not found`);

    const newEvents: BuildEvent[] = [];
    for (const op of ops) {
      staged.ops.push(op);
      const evt: BuildEvent = {
        seq: staged.events.length + 1,
        buildId,
        ts: new Date().toISOString(),
        kind: 'op.applied',
        op
      };
      staged.events.push(evt);
      newEvents.push(evt);
    }
    return newEvents;
  }

  async undo(buildId: string): Promise<void> {
    const staged = this.stagedBuilds.get(buildId);
    if (!staged || staged.ops.length === 0) return;

    const lastOp = staged.ops.pop();
    if (lastOp) {
      staged.undos.push(lastOp);
      staged.events.push({
        seq: staged.events.length + 1,
        buildId,
        ts: new Date().toISOString(),
        kind: 'step.done',
        note: `Undid operation: ${lastOp.op}`
      });
    }
  }

  async redo(buildId: string): Promise<void> {
    const staged = this.stagedBuilds.get(buildId);
    if (!staged || staged.undos.length === 0) return;

    const opToRedo = staged.undos.pop();
    if (opToRedo) {
      staged.ops.push(opToRedo);
      staged.events.push({
        seq: staged.events.length + 1,
        buildId,
        ts: new Date().toISOString(),
        kind: 'op.applied',
        op: opToRedo
      });
    }
  }

  async commit(buildId: string): Promise<ModelRevision> {
    const staged = this.stagedBuilds.get(buildId);
    if (!staged) throw new Error(`Build ${buildId} not found`);

    const project = this.projects.get(staged.pid);
    const revisionNum = (project?.modelRevisions.length ?? 0) + 1;
    const revId = `r${revisionNum}-${Date.now().toString(36)}`;
    const revision: ModelRevision = {
      id: revId,
      projectId: staged.pid,
      revisionNum,
      committedAt: new Date().toISOString(),
      parts: {},
      ports: {},
      connections: {},
      requirements: {},
      bindings: {}
    };

    // Apply recorded ops into snapshot revision
    for (const op of staged.ops) {
      if (op.op === 'part.add') {
        revision.parts[op.id] = {
          id: op.id,
          name: op.name,
          geometry: op.geometry,
          transform: {
            pos: [createMockValue(0, 'm'), createMockValue(0, 'm'), createMockValue(0, 'm')],
            rot: [createMockValue(0, 'deg'), createMockValue(0, 'deg'), createMockValue(0, 'deg')],
            scale: [1, 1, 1],
            frame: 'world'
          },
          maturity: op.maturity
        };
      } else if (op.op === 'part.place') {
        if (revision.parts[op.id]) {
          revision.parts[op.id].transform = op.transform;
        }
      } else if (op.op === 'port.add') {
        revision.ports[`${op.part}:${op.id}`] = {
          id: op.id,
          partId: op.part,
          kind: op.kind,
          direction: op.direction,
          unit: op.unit
        };
      } else if (op.op === 'connection.add') {
        revision.connections[op.id] = {
          id: op.id,
          from: op.from,
          to: op.to,
          kind: op.kind
        };
      } else if (op.op === 'requirement.add') {
        revision.requirements[op.id] = {
          id: op.id,
          text: op.text,
          expr: op.expr
        };
      } else if (op.op === 'binding.add') {
        revision.bindings[op.binding.id] = op.binding;
      }
    }

    this.revisions.set(revId, revision);
    if (project) {
      project.modelRevisions = [...project.modelRevisions, revId];
      project.activeRevisionId = revId;
      project.updatedAt = revision.committedAt;
    }
    this.saveState();
    return revision;
  }

  async listSources(pid: string): Promise<Source[]> {
    return MOCK_SOURCES;
  }

  async run(pid: string, scenarioId: string, opts?: { seed?: number; revisionNum?: number }): Promise<{ runId: string }> {
    const seed = opts?.seed ?? 8421;
    const runId = `run-${seed}-${Date.now().toString(36)}`;
    this.runs.push({ id: runId, projectId: pid, scenarioId, revisionNum: opts?.revisionNum ?? 1, startedAt: new Date().toISOString() });
    this.saveState();
    return { runId };
  }

  async *runFrames(runId: string): AsyncIterable<RunFrame> {
    // Integer steps: accumulating 0.1 drifts (0.30000000000000004) and breaks bit-identical replay.
    for (let i = 0; i <= 120; i++) {
      yield this.computeFrame(i / 10);
    }
  }

  async getFrame(runId: string, t: number): Promise<RunFrame> {
    return this.computeFrame(t);
  }

  private computeFrame(t: number): RunFrame {
    // Deterministic physics model for Run 1:
    // Angular velocity: constant 314.16 rad/s (3000 RPM)
    // Temperature: rising with exponential Joule heating, crossing 120.0 °C at t=8.42s
    const rpm = 3000.0;
    const radS = (rpm * 2 * Math.PI) / 60;
    
    // Winding Temp: T0 = 25°C, rises to 120.0°C at t=8.42
    // formula: 25 + 95 * (t / 8.42)^1.25
    const tempVal = 25.0 + 95.0 * Math.pow(Math.max(0, t) / 8.42, 1.25);
    const isOverheated = t >= 8.42;

    const values: Record<string, Value> = {
      stator_winding_temperature: createMockValue(
        parseFloat(tempVal.toFixed(1)),
        '°C',
        'simulated',
        'unverified'
      ),
      rotor_angular_velocity: createMockValue(
        parseFloat(radS.toFixed(1)),
        'rad/s',
        'simulated',
        'unverified'
      ),
      phase_rms_current: createMockValue(
        parseFloat((42.0 + (t > 4.2 ? 18.5 : 0)).toFixed(1)),
        'A',
        'simulated',
        'unverified'
      ),
      shaft_torque_load: createMockValue(
        parseFloat((32.0 + (t > 4.2 ? 53.4 : 0)).toFixed(1)),
        'N·m',
        'simulated',
        'unverified'
      )
    };

    return { t: parseFloat(t.toFixed(2)), values };
  }

  async freeze(runId: string, t: number): Promise<Checkpoint> {
    const frame = await this.getFrame(runId, t);
    const cpId = `cp-${Math.floor(t * 100)}`;
    const checkpoint: Checkpoint = {
      id: cpId,
      runId,
      t,
      hash: `0x${Math.floor(t * 987654).toString(16)}`,
      stateSnapshot: frame.values
    };
    this.checkpoints.set(cpId, checkpoint);
    this.saveState();
    return checkpoint;
  }

  async branch(cpId: string, intervention: Intervention): Promise<{ runId: string }> {
    return { runId: `branch-run-${intervention.name.toLowerCase().replace(/\s+/g, '-')}` };
  }
}

export const MockEngineClient = new MockEngineClientImpl();
