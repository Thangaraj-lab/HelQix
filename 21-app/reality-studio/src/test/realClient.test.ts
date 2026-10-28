// @vitest-environment node
import http from 'node:http';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { RealEngineClient, EngineUnavailable } from '../engine/RealEngineClient';

// The stub in ./apiStub.ts refuses what the real API refuses. The real API is exercised in CI.
import { startStub, seen } from './apiStub';

let server: http.Server;
let base = '';
beforeAll(async () => { ({ server, base } = await startStub()); });
afterAll(() => { server.closeAllConnections?.(); server.close(); });

const v = (x: number, unit = 'm') => ({ v: x, unit, origin: 'user_stated' as const, status: 'unverified' as const });

describe('RealEngineClient against the API wire format', () => {
  it('builds, commits and reads back a model with a binding and a requirement', async () => {
    const c = new RealEngineClient(base);
    const project = await c.createProject();
    expect(project.name).toBe('Untitled project');
    const { buildId } = await c.startBuild(project.id, { scratch: true });

    const events = await c.applyOps(buildId, [
      { op: 'part.add', id: 'motor', name: 'Motor', geometry: { kind: 'primitive', shape: 'cylinder', dims: [v(90, 'mm'), v(140, 'mm')] }, maturity: 'user_defined', prov: { kind: 'user_stated' } },
      { op: 'part.place', id: 'motor', transform: { pos: [v(0), v(0.5), v(0)], rot: [v(0, 'deg'), v(0, 'deg'), v(0, 'deg')], scale: [1, 1, 1], frame: 'world' } },
      { op: 'part.add', id: 'pump', name: 'Pump', geometry: { kind: 'primitive', shape: 'box', dims: [v(100, 'mm')] }, maturity: 'user_defined', prov: { kind: 'user_stated' } },
      { op: 'port.add', part: 'motor', id: 'shaft', kind: 'mechanical_torque', direction: 'out', unit: 'N·m' },
      { op: 'port.add', part: 'pump', id: 'shaft', kind: 'mechanical_torque', direction: 'in', unit: 'N·m' },
      { op: 'connection.add', id: 'c1', from: { partId: 'motor', portId: 'shaft' }, to: { partId: 'pump', portId: 'shaft' }, kind: 'mechanical_torque' },
      { op: 'requirement.add', id: 'R1', text: 'temperature below 100', expr: 'temp < 100', prov: { kind: 'user_stated' } },
      { op: 'binding.add', binding: { id: 'ui-id', variable: 'w', node: 'motor', channel: 'rotation', axis: 'y', units: 'rad/s' } }
    ]);
    expect(events.some(e => e.kind === 'staged.ready')).toBe(true);

    const rev = await c.commit(buildId);
    expect(rev.revisionNum).toBe(1);
    expect(Object.keys(rev.parts)).toEqual(['motor', 'pump']);
    expect(rev.connections.c1.from).toEqual({ partId: 'motor', portId: 'shaft' });
    expect(rev.ports['motor:shaft'].direction).toBe('out');
    expect(Object.values(rev.bindings)[0]).toMatchObject({ channel: 'rotation', axis: 'y', node: 'motor' });

    const projects = await c.listProjects();
    expect(projects.find(p => p.id === project.id)?.modelRevisions).toEqual(['r1']);
    expect(await c.getRevision(project.id, 9)).toBeNull();
  });

  it('reads stored build events and stops by itself when the stream goes quiet', async () => {
    const c = new RealEngineClient(base);
    const p = await c.createProject();
    const { buildId } = await c.startBuild(p.id, { scratch: true });
    await c.applyOps(buildId, [{ op: 'part.add', id: 'a', name: 'A', geometry: { kind: 'primitive', shape: 'box', dims: [v(1)] }, maturity: 'user_defined', prov: { kind: 'user_stated' } }]);
    const seen: string[] = [];
    for await (const e of c.buildEvents(buildId)) seen.push(e.kind);
    expect(seen).toEqual(['op.applied', 'staged.ready']);
  });

  it('renames a project using its version (If-Match)', async () => {
    const c = new RealEngineClient(base);
    const p = await c.createProject();
    expect((await c.renameProject(p.id, ' Test rig ')).name).toBe('Test rig');
  });

  it('refuses to commit nothing, and says why', async () => {
    const c = new RealEngineClient(base);
    const p = await c.createProject();
    const { buildId } = await c.startBuild(p.id, { scratch: true });
    await expect(c.commit(buildId)).rejects.toThrow(/409/);
  });

  it('does not pretend: no interpreter and no scenario runner', async () => {
    const c = new RealEngineClient(base);
    await expect(c.startBuild('p', { prompt: 'a pump, no-model' })).rejects.toBeInstanceOf(EngineUnavailable);
    await expect(c.run()).rejects.toBeInstanceOf(EngineUnavailable);
  });

  it('freezes a stored run and branches it with the wire format the API accepts', async () => {
    const c = new RealEngineClient(base);
    const cp = await c.freeze('reference-drop', 0);
    expect(cp.id).toMatch(/^reference-drop@/);
    expect(cp.stateSnapshot.y).toMatchObject({ v: 20, unit: 'm', origin: 'simulated', status: 'unverified' });
    const ov = { v: 0.3, unit: '', origin: 'user_stated' as const, status: 'unverified' as const };
    const br = await c.branch(cp.id, { id: '', name: 'b', checkpointId: cp.id, paramOverrides: { referenceAreaM2: ov } });
    expect(br.runId).toBe('br-stub');
  });

  it('turns a description into AI-labelled parts in a new build', async () => {
    const c = new RealEngineClient(base);
    const p = await c.createProject();
    const { buildId } = await c.startBuild(p.id, { prompt: 'a pump pushing water' });
    const seen: any[] = [];
    for await (const e of c.buildEvents(buildId)) { if (e.op) seen.push(e.op); }
    const add = seen.find(o => o.op === 'part.add');
    expect(add.name).toBe('Pump');
    expect(add.maturity).toBe('placeholder');
    expect(add.geometry.dims[0]).toMatchObject({ v: 80, unit: 'mm', origin: 'ai_hypothesis', status: 'unverified' });
  });

  it('runs a committed revision and lists the project runs the engine linked to it', async () => {
    const c = new RealEngineClient(base);
    const r = await c.runRevision('p1', 1, { dragCoefficient: 0.8, endSeconds: 0.2, initialState: [0, 10, 0, 0, 0, 0] });
    expect(r).toMatchObject({ runId: 'run-stubmodel', massKg: 2, parts: 2 });
    await expect(c.runRevision('nodensity', 1, { dragCoefficient: 0.8, endSeconds: 0.2, initialState: [0, 10, 0, 0, 0, 0] })).rejects.toThrow(/no density stated/);
    expect(await c.listRuns('p1')).toEqual([{ id: 'run-stubmodel', projectId: 'p1', scenarioId: 'lumped-body', revisionNum: 1, startedAt: '2026-10-28T10:00:00.000Z' }]);
  });

  it('never sends the API key itself (the dev proxy adds it)', () => {
    expect(seen.authorization).toBe(false);
  });
});

describe('stored runs through the real client', () => {
  it('lists stored runs with their variables and units', async () => {
    const c = new RealEngineClient(base);
    const runs = await c.listStoredRuns();
    expect(runs[0]).toMatchObject({ id: 'reference-drop', determinism: 'Bitwise' });
    expect(runs[0].variables).toContainEqual({ name: 'y', unit: 'm' });
  });

  it('reads every frame across pages, in order, with units and origin', async () => {
    const c = new RealEngineClient(base);
    const ts: number[] = [];
    for await (const f of c.runFrames('reference-drop')) {
      ts.push(f.t);
      expect(f.values.y.unit).toBe('m');
      expect(f.values.y.origin).toBe('simulated');
    }
    expect(ts).toHaveLength(101);
    expect([...ts].sort((a, b) => a - b)).toEqual(ts);
  });
});
