import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { RunFrame } from '../contracts';

// E1 finding: the replay check always asked the engine for the mock run ("mock-run-1"). Against the real engine that
// is a 404, the evidence package silently lost every run item, and Verify said "No run to replay" for a run that exists.
// It also asked for every frame one by one (2001 requests for a 2 s run), so a real run is checked on an even sample.

const calls: { runId: string; t: number }[] = [];
let tamper: ((f: RunFrame) => RunFrame) | null = null;
vi.mock('../engine', () => ({
  engine: {
    getFrame: async (runId: string, t: number) => {
      calls.push({ runId, t });
      const f = frames.find(x => x.t === t)!;
      return tamper ? tamper(f) : f;
    }
  }
}));

import { replayCheck } from '../app/evidence';

const make = (n: number): RunFrame[] => Array.from({ length: n }, (_, i) => ({ t: i / 10, values: { x: { v: i, unit: 'm', origin: 'simulated', status: 'unverified' } } })) as RunFrame[];
let frames: RunFrame[] = [];

beforeEach(() => { calls.length = 0; tamper = null; });

describe('replayCheck', () => {
  it('asks the engine about the run it was given, not the mock run', async () => {
    frames = make(10);
    await replayCheck('run-abc', frames, 'real');
    expect(new Set(calls.map(c => c.runId))).toEqual(new Set(['run-abc']));
  });
  it('mock engine: re-reads every frame', async () => {
    frames = make(120);
    const r = await replayCheck('mock-run-1', frames, 'mock');
    expect(calls.length).toBe(120);
    expect(r).toMatchObject({ matches: true, frames: 120, checked: 120 });
  });
  it('real engine: re-reads an even sample that includes the first and the last frame', async () => {
    frames = make(2001);
    const r = await replayCheck('run-abc', frames, 'real');
    expect(calls.length).toBe(50);
    expect(calls[0].t).toBe(frames[0].t);
    expect(calls[calls.length - 1].t).toBe(frames[2000].t);
    expect(r).toMatchObject({ matches: true, frames: 2001, checked: 50 });
  });
  it('a frame that differs on re-read makes the check fail', async () => {
    frames = make(2001);
    tamper = f => ({ ...f, values: { x: { ...f.values.x, v: f.values.x.v + 1 } } }) as RunFrame;
    const r = await replayCheck('run-abc', frames, 'real');
    expect(r.matches).toBe(false);
  });
});
