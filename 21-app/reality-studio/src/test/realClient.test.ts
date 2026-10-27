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

  it('does not pretend: no interpreter, no solver, no branching', async () => {
    const c = new RealEngineClient(base);
    await expect(c.startBuild('p', { prompt: 'a pump' })).rejects.toBeInstanceOf(EngineUnavailable);
    await expect(c.run()).rejects.toBeInstanceOf(EngineUnavailable);
    await expect(c.freeze()).rejects.toBeInstanceOf(EngineUnavailable);
    expect(await c.listRuns()).toEqual([]);
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
