import { describe, expect, it } from 'vitest';
import { DENSITY_KEY, lumpedModel, volumeM3 } from '../app/lumped';
import { ModelRevision, Value } from '../contracts';

const v = (x: number, unit = 'mm'): Value => ({ v: x, unit, origin: 'user_stated', status: 'unverified' });
const part = (id: string, shape: 'box' | 'cylinder' | 'sphere' | 'plate', dims: number[], pos: [number, number, number], density?: number) => ({
  id, name: id, maturity: 'user_defined' as const,
  geometry: { kind: 'primitive' as const, shape, dims: dims.map(d => v(d)) },
  transform: { pos: pos.map(p => v(p, 'm')) as [Value, Value, Value], rot: [v(0, 'deg'), v(0, 'deg'), v(0, 'deg')] as [Value, Value, Value], scale: [1, 1, 1] as [number, number, number], frame: 'world' },
  params: density === undefined ? undefined : { [DENSITY_KEY]: v(density, 'kg/m3') }
});
const rev = (...parts: ReturnType<typeof part>[]): ModelRevision => ({ id: 'r1', projectId: 'p', revisionNum: 1, committedAt: '', parts: Object.fromEntries(parts.map(p => [p.id, p])), ports: {}, connections: {}, requirements: {}, bindings: {} });

describe('lumped model from parts', () => {
  it('computes volumes of the primitives', () => {
    expect(volumeM3('box', [v(100), v(200), v(300)])).toBeCloseTo(0.006, 12);
    expect(volumeM3('cylinder', [v(100), v(200), v(100)])).toBeCloseTo(Math.PI * 0.05 ** 2 * 0.2, 12);
    expect(volumeM3('sphere', [v(100)])).toBeCloseTo((4 / 3) * Math.PI * 0.05 ** 3, 12);
    expect(volumeM3('box', [v(100), v(0), v(300)])).toBeNull();
    expect(volumeM3('torus', [v(1)])).toBeNull();
  });

  it('sums mass and takes the footprint of the bounding box', () => {
    const r = lumpedModel(rev(part('a', 'box', [100, 100, 100], [0, 0, 0], 1000), part('b', 'box', [100, 100, 100], [0.5, 0, 0], 2000)));
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.massKg).toBeCloseTo(0.001 * 1000 + 0.001 * 2000, 9);
      expect(r.areaM2).toBeCloseTo(0.6 * 0.1, 9); // x from -0.05 to 0.55, z 0.1 deep
    }
  });

  it('never invents a number: a part without density makes the model unrunnable and is named', () => {
    const r = lumpedModel(rev(part('a', 'box', [100, 100, 100], [0, 0, 0], 1000), part('b', 'box', [100, 100, 100], [0.5, 0, 0])));
    expect(r).toEqual({ ok: false, missing: [{ id: 'b', name: 'b', reason: 'no density stated' }] });
    expect(lumpedModel(rev())).toMatchObject({ ok: false });
  });

  it('refuses imported geometry because its volume is unknown', () => {
    const imp = { ...part('a', 'box', [1, 1, 1], [0, 0, 0], 1000), geometry: { kind: 'import' as const, assetId: 'x' } };
    expect(lumpedModel(rev(imp as never))).toMatchObject({ ok: false, missing: [{ reason: 'imported geometry has no known volume' }] });
  });
});
