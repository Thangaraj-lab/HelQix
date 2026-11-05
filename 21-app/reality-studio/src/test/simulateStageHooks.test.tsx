import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render } from '@testing-library/react';

// E1 finding: SimulateStage declared a hook after its "no run yet" early return. The first render with no run and
// the next render with a run then used different numbers of hooks, and React threw "Rendered more hooks than
// during the previous render" (a blank screen). A run always arrives after the first render, so this must not throw.

const playback = { t: 0, speed: 1, isPlaying: false, setT: vi.fn(), setDuration: vi.fn(), play: vi.fn(), pause: vi.fn(), togglePlay: vi.fn(), setSpeed: vi.fn() };
let model: any;
vi.mock('../stores', () => ({ useStudio: () => ({ playback }) }));
vi.mock('../pages/studio/ModelContext', () => ({ useModelData: () => model }));

import { SimulateStage } from '../pages/simulate/SimulateStage';

const base = { projectId: 'p', revision: { id: 'r1', revisionNum: 1, parts: {} }, parts: [], connections: [], bindings: [], requirements: [], series: {}, failure: null, loading: false, runChoice: { runs: [], runId: null, runsError: null } };
const run = { id: 'run-1', duration: 2, frames: [{ t: 0, values: {} }, { t: 2, values: {} }], at: () => null, series: () => [] };

afterEach(cleanup);

describe('SimulateStage hook order', () => {
  it('renders when the run arrives after a first render with no run', () => {
    model = { ...base, run: null };
    const view = render(<SimulateStage onInvestigateFailure={() => {}} />);
    model = { ...base, run };
    expect(() => view.rerender(<SimulateStage onInvestigateFailure={() => {}} />)).not.toThrow();
  });
});
