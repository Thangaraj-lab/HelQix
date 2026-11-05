import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { fmuPartOf } from '../pages/simulate/fmuRun';

// Simulate for a model whose part has an FMU behaviour (E4.4c): the run uses the FMU's own default time span, and says what ran.
const SHA = 'a3'.repeat(32);
const runFmuRevision = vi.fn();
const describeFmu = vi.fn();
vi.mock('../engine', () => ({ engine: { runFmuRevision: (...a: unknown[]) => runFmuRevision(...a), describeFmu: (...a: unknown[]) => describeFmu(...a) }, engineMode: 'real' }));

import { FmuRunForm } from '../pages/simulate/FmuRunForm';

const revision = (behavior: unknown) => ({ id: 'r2', projectId: 'p1', revisionNum: 2, parts: { ball: { id: 'ball', name: 'Ball', behavior }, box: { id: 'box', name: 'Box' } } }) as never;
const behavior = { kind: 'fmu', fmuSha256: SHA, outputs: ['h'] };

beforeEach(() => {
  describeFmu.mockResolvedValue({ sha256: SHA, modelName: 'BouncingBall', defaultExperiment: { startTime: 0, stopTime: 3, stepSize: 0.01 }, variables: [] });
  runFmuRevision.mockResolvedValue({ runId: 'run-abc', rows: 301, fmuSha256: SHA, determinism: 'Tolerance(1e-12)' });
});
afterEach(() => { cleanup(); vi.clearAllMocks(); });

describe('which part runs', () => {
  it('finds the part with an FMU behaviour, or none', () => {
    expect(fmuPartOf(revision(behavior))?.id).toBe('ball');
    expect(fmuPartOf(revision(undefined))).toBeNull();
    expect(fmuPartOf(null)).toBeNull();
  });
});

describe('FmuRunForm', () => {
  it('starts from the FMU default time span, runs the revision, and reports what ran', async () => {
    const onRan = vi.fn();
    render(<FmuRunForm revision={revision(behavior)} onRan={onRan} />);
    await waitFor(() => expect((screen.getByLabelText('Duration (s)') as HTMLInputElement).value).toBe('3'));
    expect((screen.getByLabelText('Step (s)') as HTMLInputElement).value).toBe('0.01');
    fireEvent.click(screen.getByRole('button', { name: 'Run my model' }));
    await waitFor(() => expect(onRan).toHaveBeenCalledWith('run-abc'));
    expect(runFmuRevision).toHaveBeenCalledWith('p1', 2, { endSeconds: 3, stepSeconds: 0.01 });
    const note = screen.getByText(/301 frames/).textContent ?? '';
    expect(note).toMatch(/BouncingBall/);
    expect(note).toMatch(/Tolerance/);
  });

  it('refuses a bad time span without asking the engine', async () => {
    render(<FmuRunForm revision={revision(behavior)} onRan={vi.fn()} />);
    await waitFor(() => expect((screen.getByLabelText('Duration (s)') as HTMLInputElement).value).toBe('3'));
    fireEvent.change(screen.getByLabelText('Step (s)'), { target: { value: '0' } });
    expect((screen.getByRole('button', { name: 'Run my model' }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByLabelText('Step (s)'), { target: { value: '0.01' } });
    fireEvent.change(screen.getByLabelText('Duration (s)'), { target: { value: 'abc' } });
    expect((screen.getByRole('button', { name: 'Run my model' }) as HTMLButtonElement).disabled).toBe(true);
    expect(runFmuRevision).not.toHaveBeenCalled();
  });

  it('shows the reason the engine gives when the run is refused', async () => {
    runFmuRevision.mockRejectedValue(new Error('The engine answered 422: parameter e: the FMU declares unit 1 (dimension [0,0,0,0,0,0,0]) but the model gave dimension [1,0,0,0,0,0,0]'));
    render(<FmuRunForm revision={revision(behavior)} onRan={vi.fn()} />);
    await waitFor(() => expect((screen.getByLabelText('Duration (s)') as HTMLInputElement).value).toBe('3'));
    fireEvent.click(screen.getByRole('button', { name: 'Run my model' }));
    expect((await screen.findByRole('alert')).textContent).toContain('dimension');
  });

  it('a late description does not overwrite what the user already typed', async () => {
    let release: (d: unknown) => void = () => undefined;
    describeFmu.mockImplementation(() => new Promise(r => { release = r; }));
    render(<FmuRunForm revision={revision(behavior)} onRan={vi.fn()} />);
    fireEvent.change(screen.getByLabelText('Duration (s)'), { target: { value: '7' } });
    release({ sha256: SHA, modelName: 'BouncingBall', defaultExperiment: { startTime: 0, stopTime: 3, stepSize: 0.02 }, variables: [] });
    await waitFor(() => expect((screen.getByLabelText('Step (s)') as HTMLInputElement).value).toBe('0.02'));
    expect((screen.getByLabelText('Duration (s)') as HTMLInputElement).value).toBe('7');
  });
});
