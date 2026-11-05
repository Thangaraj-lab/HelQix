import { describe, expect, it } from 'vitest';
import { runForModel } from '../pages/studio/runVisibility';

// E1 finding: in the real engine a run of a model with no bindings was hidden behind "The selected stored run
// could not be read." A stored run is data the engine produced; bindings only decide what moves in the 3D view.

const run = { id: 'run-1' };

describe('runForModel', () => {
  it('real engine: shows the stored run even when the model has no bindings', () => {
    expect(runForModel(run, 0, 'real')).toBe(run);
  });
  it('real engine: shows the stored run when the model has bindings', () => {
    expect(runForModel(run, 2, 'real')).toBe(run);
  });
  it('mock engine: the scripted run is meaningless without bindings, so it is hidden', () => {
    expect(runForModel(run, 0, 'mock')).toBeNull();
  });
  it('mock engine: shows the scripted run when the model has bindings', () => {
    expect(runForModel(run, 1, 'mock')).toBe(run);
  });
  it('no run stays no run', () => {
    expect(runForModel(null, 3, 'real')).toBeNull();
  });
});
