import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';

// The Behaviour panel of a part (E4.4c): choose an installed FMU, see its parameters and outputs, attach it.
const SHA = 'a3'.repeat(32);
const v = (o: Record<string, unknown>) => ({ causality: 'local', variability: 'continuous', type: 'Real', unit: '', description: '', dimension: [0, 0, 0, 0, 0, 0, 0], siFactor: 1, start: null, unitProblem: null, ...o });
const ball = {
  sha256: SHA, bytes: 1, modelName: 'BouncingBall', fmiVersion: '2.0', defaultExperiment: { startTime: 0, stopTime: 3, stepSize: 0.01 },
  variables: [v({ name: 'time', causality: 'independent' }), v({ name: 'h', causality: 'output', unit: 'm', start: 1 }), v({ name: 'v', causality: 'output', unit: 'm/s', start: 0 }),
    v({ name: 'g', causality: 'parameter', variability: 'fixed', unit: 'm/s2', start: -9.81 }), v({ name: 'e', causality: 'parameter', variability: 'tunable', start: 0.7 })]
};
const listFmus = vi.fn();
const describeFmu = vi.fn();
vi.mock('../engine', () => ({ engine: { listFmus: (...a: unknown[]) => listFmus(...a), describeFmu: (...a: unknown[]) => describeFmu(...a) }, engineMode: 'real' }));

import { FmuBehaviorPanel } from '../pages/define/FmuBehaviorPanel';

beforeEach(() => { listFmus.mockResolvedValue([{ sha256: SHA, bytes: 218148 }]); describeFmu.mockResolvedValue(ball); });
afterEach(() => { cleanup(); vi.clearAllMocks(); });

async function choose() {
  await screen.findByRole('option', { name: /a3a3a3a3/ });
  fireEvent.change(screen.getByLabelText('FMU from the library'), { target: { value: SHA } });
  await screen.findByText(/BouncingBall/);
}

describe('FmuBehaviorPanel', () => {
  it('lists the library, shows what the chosen FMU offers, and attaches it with its outputs ticked', async () => {
    const onApply = vi.fn();
    render(<FmuBehaviorPanel partId="ball" locked={false} onApply={onApply} />);
    await choose();
    expect(screen.getByLabelText('Parameter g')).toBeTruthy();
    expect(screen.getByLabelText('Parameter e')).toBeTruthy();
    expect((screen.getByLabelText('Record h') as HTMLInputElement).checked).toBe(true);
    expect((screen.getByLabelText('Record v') as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Attach FMU' }));
    expect(onApply).toHaveBeenCalledWith({ op: 'part.setBehavior', id: 'ball', behavior: { kind: 'fmu', fmuSha256: SHA, outputs: ['h', 'v'] } });
  });

  it('sends a parameter only when the user changed it, with the FMU unit', async () => {
    const onApply = vi.fn();
    render(<FmuBehaviorPanel partId="ball" locked={false} onApply={onApply} />);
    await choose();
    fireEvent.change(screen.getByLabelText('Parameter g'), { target: { value: '-1.62' } });
    fireEvent.click(screen.getByLabelText('Record v'));
    fireEvent.click(screen.getByRole('button', { name: 'Attach FMU' }));
    expect(onApply).toHaveBeenCalledWith({ op: 'part.setBehavior', id: 'ball', behavior: { kind: 'fmu', fmuSha256: SHA, parameters: { g: { v: -1.62, unit: 'm/s2', origin: 'user_stated', status: 'unverified' } }, outputs: ['h'] } });
  });

  it('says what is wrong instead of attaching', async () => {
    const onApply = vi.fn();
    render(<FmuBehaviorPanel partId="ball" locked={false} onApply={onApply} />);
    await choose();
    fireEvent.change(screen.getByLabelText('Parameter g'), { target: { value: 'fast' } });
    fireEvent.click(screen.getByRole('button', { name: 'Attach FMU' }));
    expect((await screen.findByRole('alert')).textContent).toContain('g must be a number');
    expect(onApply).not.toHaveBeenCalled();
  });

  it('an empty library says an operator installs FMUs and they cannot be uploaded', async () => {
    listFmus.mockResolvedValue([]);
    render(<FmuBehaviorPanel partId="ball" locked={false} onApply={vi.fn()} />);
    expect((await screen.findByText(/No FMU is installed/)).textContent).toMatch(/operator/);
  });

  it('shows why the engine refused to describe an FMU', async () => {
    describeFmu.mockRejectedValue(new Error('The engine answered 422: the FMU is not in the FMU library'));
    render(<FmuBehaviorPanel partId="ball" locked={false} onApply={vi.fn()} />);
    await screen.findByRole('option', { name: /a3a3a3a3/ });
    fireEvent.change(screen.getByLabelText('FMU from the library'), { target: { value: SHA } });
    expect((await screen.findByRole('alert')).textContent).toContain('not in the FMU library');
  });

  it('a committed model cannot change its behaviour, and the current one is shown', async () => {
    render(<FmuBehaviorPanel partId="ball" locked current={{ kind: 'fmu', fmuSha256: SHA, outputs: ['h', 'v'] }} onApply={vi.fn()} />);
    await waitFor(() => expect(screen.getByText(/h, v/)).toBeTruthy());
    expect(screen.queryByRole('button', { name: 'Attach FMU' })).toBeNull();
    expect(screen.getByText(/committed/i)).toBeTruthy();
  });

  it('an answer for an FMU the user has already moved away from does not replace the current one', async () => {
    const other = 'b4'.repeat(32);
    listFmus.mockResolvedValue([{ sha256: SHA, bytes: 1 }, { sha256: other, bytes: 1 }]);
    let release: (d: unknown) => void = () => undefined;
    describeFmu.mockImplementation((s: string) => s === SHA ? new Promise(r => { release = r; }) : Promise.resolve({ ...ball, sha256: other, modelName: 'Other' }));
    render(<FmuBehaviorPanel partId="ball" locked={false} onApply={vi.fn()} />);
    await screen.findByRole('option', { name: /a3a3a3a3/ });
    fireEvent.change(screen.getByLabelText('FMU from the library'), { target: { value: SHA } });
    fireEvent.change(screen.getByLabelText('FMU from the library'), { target: { value: other } });
    await screen.findByText(/Other/);
    release(ball);
    await new Promise(r => setTimeout(r, 30));
    expect(screen.queryByText(/BouncingBall/)).toBeNull();
    expect(screen.getByText(/Other/)).toBeTruthy();
  });
});
