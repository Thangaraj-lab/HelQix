import { describe, expect, it } from 'vitest';
import { foldRevision, fromWireEvent, fromWireIssue, fromWireProject, toWireOp } from '../engine/wire';
import { ModelOp } from '../contracts';

// Shapes below are copied from the Rust serde definitions in 15-sdk/platform/contracts and the
// requests used in 22-api/helqix-api/tests/builds_api.rs.

describe('UI to wire', () => {
  it('connections use part and port, not partId and portId', () => {
    const op: ModelOp = { op: 'connection.add', id: 'c1', from: { partId: 'a', portId: 'out' }, to: { partId: 'b', portId: 'in' }, kind: 'electrical' };
    expect(toWireOp(op)).toEqual({ op: 'connection.add', id: 'c1', from: { part: 'a', port: 'out' }, to: { part: 'b', port: 'in' }, kind: 'electrical' });
  });

  it('a binding drops its UI id, sends an axis vector, and integrates a rotation', () => {
    const w = toWireOp({ op: 'binding.add', binding: { id: 'ui-only', variable: 'w', node: 'm', channel: 'rotation', axis: 'y', units: 'rad/s', range: [0, 9] } }) as { binding: Record<string, unknown> };
    expect(w.binding).toEqual({ variable: 'w', node: 'm', channel: 'rotation', axis: [0, 1, 0], mapping: 'integrate', units: 'rad/s', range: [0, 9] });
    expect('id' in w.binding).toBe(false); // the API rejects unknown fields
  });

  it('a colour binding maps linearly and has no axis', () => {
    const w = toWireOp({ op: 'binding.add', binding: { id: 'x', variable: 't', node: 'm', channel: 'color', units: 'C' } }) as { binding: Record<string, unknown> };
    expect(w.binding.mapping).toBe('linear');
    expect('axis' in w.binding).toBe(false);
  });
});

describe('wire to UI', () => {
  it('maps an issue: the first subject is the part, and no Fix op is invented', () => {
    const i = fromWireIssue({ id: 'unit.mismatch:c1', severity: 'blocking', code: 'unit.mismatch', message: 'V to A', subject: ['base', 'body'], fix: 'convert one side' });
    expect(i).toEqual({ id: 'unit.mismatch:c1', partId: 'base', message: 'V to A', severity: 'blocking' });
    expect('fixOp' in i).toBe(false);
  });

  it('maps a build event with an op', () => {
    const e = fromWireEvent({ seq: 3, buildId: 'b', ts: 't', kind: 'op.applied', op: { op: 'connection.add', id: 'c', from: { part: 'a', port: 'p' }, to: { part: 'b', port: 'q' }, kind: 'k' } });
    expect(e.op).toMatchObject({ op: 'connection.add', from: { partId: 'a', portId: 'p' } });
  });

  it('folds a revision from its ops', () => {
    const v = (x: number) => ({ v: x, unit: 'm', origin: 'user_stated', status: 'unverified' });
    const rev = foldRevision({ number: 2, projectId: 'p', committedAt: 'now' }, [
      { op: 'part.add', id: 'a', name: 'A', geometry: { kind: 'primitive', shape: 'box', dims: [v(1)] }, maturity: 'user_defined', prov: { kind: 'user_stated' } },
      { op: 'part.place', id: 'a', transform: { pos: [v(1), v(2), v(3)], rot: [v(0), v(0), v(0)], scale: [1, 1, 1], frame: 'world' } },
      { op: 'port.add', part: 'a', id: 'out', kind: 'electrical', direction: 'out', unit: 'V' },
      { op: 'requirement.add', id: 'R1', text: 't below 100', expr: 't < 100', prov: { kind: 'user_stated' } },
      { op: 'binding.add', binding: { variable: 't', node: 'a', channel: 'color', mapping: 'linear', units: 'C', range: [0, 100] } }
    ]);
    expect(rev.id).toBe('r2');
    expect(rev.parts.a.transform.pos[2].v).toBe(3);
    expect(rev.ports['a:out']).toMatchObject({ partId: 'a', direction: 'out' });
    expect(rev.requirements.R1.expr).toBe('t < 100');
    expect(Object.values(rev.bindings)[0]).toMatchObject({ node: 'a', variable: 't', channel: 'color' });
  });

  it('names a project from its payload, with a fallback', () => {
    expect(fromWireProject({ id: 'x', payload: { name: 'Rig' }, created_at: 'c', updated_at: 'u' }, 2)).toMatchObject({ name: 'Rig', modelRevisions: ['r1', 'r2'], activeRevisionId: 'r2' });
    expect(fromWireProject({ id: 'x', payload: {}, created_at: 'c', updated_at: 'u' }, 0).name).toBe('Untitled project');
  });
});
