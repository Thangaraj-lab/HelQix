import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { buildCellOp, cellPartOf, packNotice } from '../domainpacks/battery/batteryCell';

// The battery domain pack in the Studio (E5.3): the Define panel, the Simulate run form, and the label that never goes missing.
const PACK = { id: 'lgm50_spm_chen2020', label: 'NotVerified', contextOfUse: 'Predict the terminal voltage of an LG M50 21700 cell during constant-current discharge at 25 C', heldOutRmseMv: [73.3, 71.1, 77.3], thresholdMv: 46, calibratedContactResistanceOhm: 0.015805, validatedModelIdentitySha256: 'a'.repeat(64), reportDate: '2026-10-25' };
const listPacks = vi.fn();
const runPackRevision = vi.fn();
vi.mock('../engine', () => ({ engine: { listPacks: (...a: unknown[]) => listPacks(...a), runPackRevision: (...a: unknown[]) => runPackRevision(...a) }, engineMode: 'real' }));

import { BatteryPanel } from '../domainpacks/battery/BatteryPanel';
import { BatteryRunForm } from '../domainpacks/battery/BatteryRunForm';
import { PackVerifyNote } from '../domainpacks';

const cellBehavior = { kind: 'battery_cell', pack: PACK.id, initialOcv: { v: 4.18, unit: 'V' }, load: { kind: 'constant_current', current: { v: 5, unit: 'A' } } };
const revision = (behavior: unknown) => ({ id: 'r1', projectId: 'p1', revisionNum: 1, parts: { cell: { id: 'cell', name: 'Cell', behavior }, box: { id: 'box', name: 'Box' } } }) as never;

beforeEach(() => { listPacks.mockResolvedValue([PACK]); runPackRevision.mockResolvedValue({ runId: 'run-b', rows: 21, determinism: 'Tolerance(4.2e-6)', pack: PACK, modelIdentitySha256: 'b'.repeat(64) }); });
afterEach(() => { cleanup(); vi.clearAllMocks(); });

describe('the cell op built from the form', () => {
  const ok = { ocv: '4.18', resistance: '0.0158', current: '5' };
  it('states every value with its unit and omits an empty resistance', () => {
    expect(buildCellOp('cell', 'lgm50_spm_chen2020', ok)).toEqual({ op: { op: 'part.setBehavior', id: 'cell', behavior: { kind: 'battery_cell', pack: 'lgm50_spm_chen2020',
      initialOcv: { v: 4.18, unit: 'V', origin: 'user_stated', status: 'unverified' }, contactResistance: { v: 0.0158, unit: 'ohm', origin: 'user_stated', status: 'unverified' },
      load: { kind: 'constant_current', current: { v: 5, unit: 'A', origin: 'user_stated', status: 'unverified' } } } } });
    const r = buildCellOp('cell', 'p', { ...ok, resistance: '' });
    expect('op' in r && r.op.op === 'part.setBehavior' && r.op.behavior.kind === 'battery_cell' && 'contactResistance' in r.op.behavior).toBe(false);
  });
  it('refuses what the pack would refuse, and says which value', () => {
    expect(buildCellOp('c', 'p', { ...ok, ocv: '5' })).toEqual({ error: 'Open-circuit voltage must be between 2.5 and 4.25 V.' });
    expect(buildCellOp('c', 'p', { ...ok, ocv: 'x' })).toEqual({ error: 'Open-circuit voltage must be a number.' });
    expect(buildCellOp('c', 'p', { ...ok, resistance: '2' })).toEqual({ error: 'Contact resistance must be between 0 and 1 Ohm.' });
    expect(buildCellOp('c', 'p', { ...ok, current: '' })).toEqual({ error: 'Discharge current must be a number.' });
  });
  it('finds the part that is a cell, and the notice says NotVerified with the published error', () => {
    expect(cellPartOf(revision(cellBehavior))?.id).toBe('cell');
    expect(cellPartOf(revision({ kind: 'fmu' }))).toBeNull();
    const n = packNotice(PACK);
    expect(n).toMatch(/NotVerified/);
    expect(n).toMatch(/71\.1/);
    expect(n).toMatch(/77\.3/);
    expect(n).toMatch(/46/);
  });
});

describe('BatteryPanel', () => {
  it('shows the label first, offers the calibrated resistance without applying it, and attaches the cell', async () => {
    const onApply = vi.fn();
    render(<BatteryPanel partId="cell" locked={false} onApply={onApply} />);
    expect((await screen.findByText(/NotVerified/)).textContent).toMatch(/71\.1/);
    fireEvent.change(screen.getByLabelText('Open-circuit voltage (V)'), { target: { value: '4.18' } });
    fireEvent.change(screen.getByLabelText('Discharge current (A)'), { target: { value: '5' } });
    expect((screen.getByLabelText('Contact resistance (Ohm)') as HTMLInputElement).value).toBe('');
    fireEvent.click(screen.getByRole('button', { name: /Use the calibrated value/ }));
    expect((screen.getByLabelText('Contact resistance (Ohm)') as HTMLInputElement).value).toBe('0.015805');
    fireEvent.click(screen.getByRole('button', { name: 'Make this part a battery cell' }));
    expect(onApply).toHaveBeenCalledTimes(1);
    expect(onApply.mock.calls[0][0].behavior).toMatchObject({ kind: 'battery_cell', pack: PACK.id });
  });
  it('says what is wrong instead of attaching', async () => {
    const onApply = vi.fn();
    render(<BatteryPanel partId="cell" locked={false} onApply={onApply} />);
    await screen.findByText(/NotVerified/);
    fireEvent.change(screen.getByLabelText('Open-circuit voltage (V)'), { target: { value: '9' } });
    fireEvent.change(screen.getByLabelText('Discharge current (A)'), { target: { value: '5' } });
    fireEvent.click(screen.getByRole('button', { name: 'Make this part a battery cell' }));
    expect((await screen.findByRole('alert')).textContent).toContain('between 2.5 and 4.25 V');
    expect(onApply).not.toHaveBeenCalled();
  });
  it('without a pack on the server it says so and offers nothing', async () => {
    listPacks.mockResolvedValue([]);
    render(<BatteryPanel partId="cell" locked={false} onApply={vi.fn()} />);
    expect(await screen.findByText(/No battery pack is installed/)).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Make this part a battery cell' })).toBeNull();
  });
});

describe('BatteryRunForm', () => {
  it('shows the NotVerified notice before and after the run, and reports what ran', async () => {
    const onRan = vi.fn();
    render(<BatteryRunForm revision={revision(cellBehavior)} onRan={onRan} />);
    expect((await screen.findByText(/NotVerified/)).textContent).toMatch(/46/);
    fireEvent.click(screen.getByRole('button', { name: 'Run my model' }));
    await waitFor(() => expect(onRan).toHaveBeenCalledWith('run-b'));
    expect(runPackRevision).toHaveBeenCalledWith('p1', 1, { endSeconds: 600, stepSeconds: 30 });
    expect(screen.getAllByText(/NotVerified/).length).toBeGreaterThan(0);
  });
  it('refuses a bad time span without asking the engine and shows the engine reason otherwise', async () => {
    render(<BatteryRunForm revision={revision(cellBehavior)} onRan={vi.fn()} />);
    await screen.findByText(/NotVerified/);
    fireEvent.change(screen.getByLabelText('Step (s)'), { target: { value: '0' } });
    expect((screen.getByRole('button', { name: 'Run my model' }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByLabelText('Step (s)'), { target: { value: '30' } });
    runPackRevision.mockRejectedValue(new Error('The engine answered 503: service unavailable'));
    fireEvent.click(screen.getByRole('button', { name: 'Run my model' }));
    expect((await screen.findByRole('alert')).textContent).toContain('unavailable');
  });
  it('renders nothing for a model that has no cell', async () => {
    const { container } = render(<BatteryRunForm revision={revision(undefined)} onRan={vi.fn()} />);
    expect(container.textContent).toBe('');
  });
});

describe('PackVerifyNote', () => {
  it('says NotVerified with the published error for a pack model, and nothing for any other model', async () => {
    const { container, rerender } = render(<PackVerifyNote revision={revision(cellBehavior)} />);
    expect((await screen.findByRole('note')).textContent).toMatch(/NotVerified.*71\.1 to 77\.3 mV/);
    rerender(<PackVerifyNote revision={revision(undefined)} />);
    expect(container.textContent).toBe('');
  });
  it('when the server cannot say, it does not pretend: it says the results are unvalidated', async () => {
    listPacks.mockRejectedValue(new Error('down'));
    render(<PackVerifyNote revision={revision(cellBehavior)} />);
    expect((await screen.findByRole('note')).textContent).toMatch(/unvalidated/);
  });
});
