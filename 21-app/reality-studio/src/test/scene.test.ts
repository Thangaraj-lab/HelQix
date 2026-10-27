import { describe, expect, it } from 'vitest';
import { Binding } from '../contracts';
import { integralAtTime, stateFor, valueAtTime, VISUAL_SPEED_SCALE } from '../scene/bindings';
import { parseRequirement, violates } from '../app/requirements';

const series = { w: [[0, 10], [1, 10], [2, 30]] as [number, number][], temp: [[0, 25], [10, 125]] as [number, number][] };
const spin: Binding = { id: 'b1', variable: 'w', node: 'motor', channel: 'rotation', axis: 'y', units: 'rad/s' };
const heat: Binding = { id: 'b2', variable: 'temp', node: 'motor', channel: 'color', units: 'C', range: [25, 125] };

describe('motion is data (bindings)', () => {
  it('interpolates stored samples linearly', () => {
    expect(valueAtTime(series.temp, 5)).toBe(75);
    expect(valueAtTime(undefined, 5)).toBeNull();
  });

  it('integrates the stored variable, so the angle at t is a pure function of t', () => {
    expect(integralAtTime(series.w, 1)).toBeCloseTo(10);
    expect(integralAtTime(series.w, 2)).toBeCloseTo(30);
    const a = integralAtTime(series.w, 1.5);
    integralAtTime(series.w, 0.2);
    expect(integralAtTime(series.w, 1.5)).toBe(a);
  });

  it('rotates only the bound part, by the integrated angle at the stated visual scale', () => {
    const s = stateFor('motor', [spin], series, 2);
    expect(s.rotation?.axis).toBe('y');
    expect(s.rotation?.angle).toBeCloseTo(30 / VISUAL_SPEED_SCALE);
    expect(stateFor('pump', [spin], series, 2).rotation).toBeNull();
  });

  it('a part with no binding never moves, heats or glows', () => {
    const s = stateFor('motor', [], series, 5);
    expect(s).toEqual({ rotation: null, heat: null, glow: null, shake: null, slide: null, visible: true });
  });

  it('heat is the position inside the bound range, and overshoots past the limit', () => {
    expect(stateFor('motor', [heat], series, 5).heat).toBeCloseTo(0.5);
    expect(stateFor('motor', [heat], series, 10).heat).toBeCloseTo(1);
  });
});

describe('requirements', () => {
  it('parses an expression and finds a breach', () => {
    const r = parseRequirement('R1', 'limit', 'temp < 100', [heat])!;
    expect(r).toMatchObject({ variable: 'temp', op: '<', limit: 100, unit: 'C' });
    expect(violates(r, 99)).toBe(false);
    expect(violates(r, 100)).toBe(true);
  });

  it('refuses an expression it cannot evaluate rather than guessing', () => {
    expect(parseRequirement('R1', 'x', 'temp is high', [])).toBeNull();
    expect(parseRequirement('R1', 'x', undefined, [])).toBeNull();
  });
});
