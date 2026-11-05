import { describe, expect, it } from 'vitest';
import { buildBehaviorOp, outputCandidates, parameterCandidates, paramUnit } from '../pages/define/fmuBehavior';
import type { FmuDescription, FmuVariable } from '../contracts';

const v = (o: Partial<FmuVariable> & { name: string }): FmuVariable => ({ causality: 'local', variability: 'continuous', type: 'Real', unit: '', description: '', dimension: [0, 0, 0, 0, 0, 0, 0], siFactor: 1, start: null, unitProblem: null, ...o });
const ball: FmuDescription = {
  sha256: 'a3'.repeat(32), bytes: 1, modelName: 'BouncingBall', fmiVersion: '2.0', defaultExperiment: { startTime: 0, stopTime: 3, stepSize: 0.01 },
  variables: [
    v({ name: 'time', causality: 'independent' }),
    v({ name: 'h', causality: 'output', unit: 'm', start: 1 }),
    v({ name: 'der(h)', unit: 'm/s' }),
    v({ name: 'v', causality: 'output', unit: 'm/s', start: 0 }),
    v({ name: 'g', causality: 'parameter', variability: 'fixed', unit: 'm/s2', start: -9.81 }),
    v({ name: 'e', causality: 'parameter', variability: 'tunable', start: 0.7 }),
    v({ name: 'v_min', variability: 'constant', unit: 'm/s', start: 0.1 }),
    v({ name: 'flag', causality: 'parameter', variability: 'fixed', type: 'Boolean' }),
  ]
};

describe('what an FMU offers a part', () => {
  it('parameters are the settable real parameters, with their start value and unit', () => {
    expect(parameterCandidates(ball).map(p => [p.name, p.unit, p.start])).toEqual([['g', 'm/s2', -9.81], ['e', 'one', 0.7]]);
  });
  it('outputs come first, then other recordable variables; time and constants are not offered', () => {
    expect(outputCandidates(ball).map(o => [o.name, o.isOutput])).toEqual([['h', true], ['v', true], ['der(h)', false]]);
  });
  it('a dimensionless FMU variable uses the registry symbol "one"', () => {
    expect(paramUnit(v({ name: 'e', unit: '' }))).toBe('one');
    expect(paramUnit(v({ name: 'g', unit: 'm/s2' }))).toBe('m/s2');
  });
});

describe('the behaviour op built from the form', () => {
  const sha = 'a3'.repeat(32);
  it('keeps only the parameters the user changed, with unit and user-stated origin', () => {
    const r = buildBehaviorOp('ball', sha, ball, { g: '-1.62', e: '0.7' }, ['h', 'v']);
    expect(r).toEqual({ op: { op: 'part.setBehavior', id: 'ball', behavior: { kind: 'fmu', fmuSha256: sha, parameters: { g: { v: -1.62, unit: 'm/s2', origin: 'user_stated', status: 'unverified' } }, outputs: ['h', 'v'] } } });
  });
  it('no changed parameter means no parameters at all', () => {
    const r = buildBehaviorOp('ball', sha, ball, {}, ['h']);
    expect('op' in r && r.op.op === 'part.setBehavior' && r.op.behavior.parameters).toEqual(undefined);
  });
  it('refuses no output, a non-number, and a name the FMU does not have', () => {
    expect(buildBehaviorOp('ball', sha, ball, {}, [])).toEqual({ error: 'Choose at least one output to record.' });
    expect(buildBehaviorOp('ball', sha, ball, { g: 'fast' }, ['h'])).toEqual({ error: 'g must be a number.' });
    expect(buildBehaviorOp('ball', sha, ball, { g: '' }, ['h'])).toMatchObject({ op: expect.anything() });
    expect(buildBehaviorOp('ball', sha, ball, { nope: '1' }, ['h'])).toEqual({ error: 'The FMU has no settable parameter nope.' });
    expect(buildBehaviorOp('ball', sha, ball, {}, ['nope'])).toEqual({ error: 'The FMU has no variable nope to record.' });
  });
});
