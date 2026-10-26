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
  AssetRef
} from '../contracts';

export class RealEngineClient implements EngineClient {
  private endpoint: string;

  constructor(endpoint: string = 'http://localhost:8080') {
    this.endpoint = endpoint;
  }

  async createProject(): Promise<Project> {
    throw new Error(`RealEngineClient: Connection to Rust backend (${this.endpoint}) not yet implemented. Crate 22-api is in development.`);
  }

  async startBuild(pid: string, input: any): Promise<{ buildId: string }> {
    throw new Error('RealEngineClient: startBuild not yet connected to live engine crate.');
  }

  async *buildEvents(buildId: string): AsyncIterable<BuildEvent> {
    throw new Error('RealEngineClient: buildEvents stream not yet connected to live engine crate.');
  }

  async answerQuestion(buildId: string, qid: string, answer: string | 'default' | 'skip'): Promise<void> {
    throw new Error('RealEngineClient: answerQuestion not implemented on live backend.');
  }

  async applyOps(buildId: string, ops: ModelOp[]): Promise<BuildEvent[]> {
    throw new Error('RealEngineClient: applyOps not implemented on live backend.');
  }

  async undo(buildId: string): Promise<void> {
    throw new Error('RealEngineClient: undo not implemented on live backend.');
  }

  async redo(buildId: string): Promise<void> {
    throw new Error('RealEngineClient: redo not implemented on live backend.');
  }

  async commit(buildId: string): Promise<ModelRevision> {
    throw new Error('RealEngineClient: commit not implemented on live backend.');
  }

  async listSources(pid: string): Promise<Source[]> {
    throw new Error('RealEngineClient: listSources not implemented on live backend.');
  }

  async run(pid: string, scenarioId: string, opts?: { seed?: number }): Promise<{ runId: string }> {
    throw new Error('RealEngineClient: run not implemented on live backend.');
  }

  async *runFrames(runId: string): AsyncIterable<RunFrame> {
    throw new Error('RealEngineClient: runFrames stream not implemented on live backend.');
  }

  async getFrame(runId: string, t: number): Promise<RunFrame> {
    throw new Error('RealEngineClient: getFrame not implemented on live backend.');
  }

  async freeze(runId: string, t: number): Promise<Checkpoint> {
    throw new Error('RealEngineClient: freeze not implemented on live backend.');
  }

  async branch(cp: string, intervention: Intervention): Promise<{ runId: string }> {
    throw new Error('RealEngineClient: branch not implemented on live backend.');
  }
}
