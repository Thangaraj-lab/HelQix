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
  Value
} from '../contracts';
import { getMockBuildScriptEvents, MOCK_SOURCES, createMockValue } from './examplePack';

class MockEngineClientImpl implements EngineClient {
  private projects: Map<string, Project> = new Map();
  private stagedBuilds: Map<string, { events: BuildEvent[]; ops: ModelOp[]; undos: ModelOp[] }> = new Map();
  private revisions: Map<string, ModelRevision> = new Map();
  private checkpoints: Map<string, Checkpoint> = new Map();

  constructor() {
    this.restoreState();
  }

  private saveState() {
    try {
      const data = {
        projects: Array.from(this.projects.entries()),
        revisions: Array.from(this.revisions.entries()),
        checkpoints: Array.from(this.checkpoints.entries())
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

    const revId = `r1-${Date.now().toString(36)}`;
    const revision: ModelRevision = {
      id: revId,
      projectId: buildId.split('-')[1] || 'default-prj',
      revisionNum: 1,
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
        revision.ports[op.id] = {
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
    this.saveState();
    return revision;
  }

  async listSources(pid: string): Promise<Source[]> {
    return MOCK_SOURCES;
  }

  async run(pid: string, scenarioId: string, opts?: { seed?: number }): Promise<{ runId: string }> {
    const seed = opts?.seed ?? 8421;
    const runId = `run-${seed}-${Date.now().toString(36)}`;
    return { runId };
  }

  async *runFrames(runId: string): AsyncIterable<RunFrame> {
    for (let t = 0; t <= 12.0; t += 0.1) {
      yield this.computeFrame(t);
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
        isOverheated ? 'unverified' : 'verified'
      ),
      rotor_angular_velocity: createMockValue(
        parseFloat(radS.toFixed(1)),
        'rad/s',
        'simulated',
        'verified'
      ),
      phase_rms_current: createMockValue(
        parseFloat((42.0 + (t > 4.2 ? 18.5 : 0)).toFixed(1)),
        'A',
        'simulated',
        'verified'
      ),
      shaft_torque_load: createMockValue(
        parseFloat((32.0 + (t > 4.2 ? 53.4 : 0)).toFixed(1)),
        'N·m',
        'simulated',
        'verified'
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
