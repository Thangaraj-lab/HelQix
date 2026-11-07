import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { EvidenceKeyInfo, EvidenceRequirement } from '../contracts';
import type { Requirement } from '../app/requirements';
import { exportEvidence, toEvidenceRequirement, verifierCommand } from '../app/evidenceExport';
import { EvidenceExportPanel, whyNotExportable } from '../pages/evidence/EvidenceExportPanel';
import { RealEngineClient } from '../engine/RealEngineClient';

// E8.3: the Studio asks the server for a signed evidence package and saves it. It never signs, never holds a key, and says what the signature is
// (a test key, until the founder decides custody) and what it is not (proof the physics is right).
const req = (over: Partial<Requirement> = {}): Requirement => ({ id: 'R1', text: 'The body stays above ground', variable: 'y', op: '>=', limit: 0, unit: 'm', ...over });
const testKey: EvidenceKeyInfo = { publicKey: 'ab'.repeat(32), keyId: '0123456789abcdef', signer: 'helqix-test-signer', keyKind: 'test', note: 'A test key.' };
const zip = () => new Blob([new Uint8Array([0x50, 0x4b, 3, 4])], { type: 'application/zip' });

describe('mapping a requirement to what is signed', () => {
  it('keeps strict comparisons strict (a value that only equals the limit breaks a strict requirement)', () => {
    const cmp = (op: Requirement['op']) => toEvidenceRequirement(req({ op })).comparator;
    expect([cmp('>='), cmp('<='), cmp('>'), cmp('<')]).toEqual(['atLeast', 'atMost', 'above', 'below']);
  });
  it('sends the variable and the limit as stated, with a one-line name of at most 120 characters', () => {
    const r = toEvidenceRequirement(req({ text: 'a\nlong   text ' + 'x'.repeat(300), limit: -2.5, variable: 'vy' }));
    expect(r).toMatchObject({ variable: 'vy', threshold: -2.5 });
    expect(r.name.length).toBeLessThanOrEqual(120);
    expect(r.name).not.toMatch(/[\n\r]/);
  });
});

describe('exporting', () => {
  it('asks for the package, saves the file the server returned, and reads which key signed it', async () => {
    const save = vi.fn();
    const client = { exportEvidence: vi.fn(async () => ({ blob: zip(), filename: 'evidence-run-1.zip' })), evidenceKey: vi.fn(async () => testKey) };
    const out = await exportEvidence(client, 'run-1', req({ op: '>' }), save);
    expect(client.exportEvidence).toHaveBeenCalledWith('run-1', expect.objectContaining({ comparator: 'above', variable: 'y', threshold: 0 }));
    expect(save).toHaveBeenCalledTimes(1);
    expect(save.mock.calls[0][1]).toBe('evidence-run-1.zip');
    expect(out).toMatchObject({ filename: 'evidence-run-1.zip', bytes: 4, key: testKey, keyError: null });
  });
  it('still saves the package when the key cannot be read, and says so', async () => {
    const save = vi.fn();
    const out = await exportEvidence({ exportEvidence: async () => ({ blob: zip(), filename: 'e.zip' }), evidenceKey: async () => { throw new Error('key unavailable'); } }, 'run-1', req(), save);
    expect(save).toHaveBeenCalledTimes(1);
    expect(out.key).toBeNull();
    expect(out.keyError).toBe('key unavailable');
  });
  it('saves nothing when the server refuses', async () => {
    const save = vi.fn();
    await expect(exportEvidence({ exportEvidence: async () => { throw new Error('The engine answered 422: this run has no variable "y"'); } }, 'run-1', req(), save)).rejects.toThrow(/422/);
    expect(save).not.toHaveBeenCalled();
  });
  it('names the folder of the run in the verifier command', () => {
    expect(verifierCommand('run-abc')).toBe('helqix-evidence-verify evidence-run-abc --trusted-key-file trusted-public-key.txt --attestation evidence-run-abc.intoto.dsse.json');
    expect(verifierCommand(null)).toContain('evidence-<run>');
  });
});

describe('the export panel', () => {
  afterEach(cleanup);
  const real = { exportEvidence: async () => ({ blob: zip(), filename: 'evidence-run-1.zip' }), evidenceKey: async () => testKey };

  it('says why export is not possible instead of pretending', () => {
    expect(whyNotExportable({ client: {}, runId: 'run-1', requirements: [req()], engineIsReal: false })).toMatch(/real engine/);
    expect(whyNotExportable({ client: real, runId: null, requirements: [req()], engineIsReal: true })).toMatch(/Run a simulation/);
    expect(whyNotExportable({ client: real, runId: 'run-1', requirements: [], engineIsReal: true })).toMatch(/requirement/);
    expect(whyNotExportable({ client: real, runId: 'run-1', requirements: [req()], engineIsReal: true })).toBeNull();
  });

  it('keeps the button off with the reason on screen when export is not possible', () => {
    render(<EvidenceExportPanel client={{}} runId="run-1" requirements={[req()]} engineIsReal={false} />);
    expect((screen.getByRole('button', { name: 'Export evidence' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/real engine/)).toBeTruthy();
    expect(screen.getByText('Not signed yet.')).toBeTruthy();
  });

  it('exports, then shows who signed, that it is a test key, and the command that checks it offline', async () => {
    const save = vi.fn();
    render(<EvidenceExportPanel client={real} runId="run-1" requirements={[req()]} engineIsReal save={save} />);
    fireEvent.click(screen.getByRole('button', { name: 'Export evidence' }));
    expect(await screen.findByText(/Signed by helqix-test-signer, key 0123456789abcdef/)).toBeTruthy();
    expect(screen.getByRole('note').textContent).toMatch(/test key/);
    expect(screen.getByText(/does not mean|not correct physics/i)).toBeTruthy();
    expect(screen.getByText(/Saved evidence-run-1\.zip/)).toBeTruthy();
    expect(save).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/helqix-evidence-verify evidence-run-1 /)).toBeTruthy();
    expect(screen.queryByText('Not signed yet.')).toBeNull();
  });

  it('shows the server\'s refusal and does not claim a signature', async () => {
    const save = vi.fn();
    const client = { exportEvidence: async () => { throw new Error('The engine answered 422: Evidence export covers runs of the built-in engine.'); }, evidenceKey: async () => testKey };
    render(<EvidenceExportPanel client={client} runId="run-1" requirements={[req()]} engineIsReal save={save} />);
    fireEvent.click(screen.getByRole('button', { name: 'Export evidence' }));
    expect((await screen.findByRole('alert')).textContent).toMatch(/built-in engine/);
    expect(save).not.toHaveBeenCalled();
    expect(screen.getByText('Not signed yet.')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Export evidence' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('lets the person choose which requirement the package states, and exports that one', async () => {
    const seen: EvidenceRequirement[] = [];
    const client = { exportEvidence: async (_id: string, r: EvidenceRequirement) => { seen.push(r); return { blob: zip(), filename: 'e.zip' }; }, evidenceKey: async () => testKey };
    render(<EvidenceExportPanel client={client} runId="run-1" requirements={[req(), req({ id: 'R2', variable: 'vx', op: '<', limit: 3, unit: 'm/s', text: 'Speed stays low' })]} engineIsReal save={() => undefined} />);
    fireEvent.change(screen.getByLabelText(/Requirement to state/), { target: { value: 'R2' } });
    fireEvent.click(screen.getByRole('button', { name: 'Export evidence' }));
    await waitFor(() => expect(seen).toHaveLength(1));
    expect(seen[0]).toMatchObject({ variable: 'vx', comparator: 'below', threshold: 3 });
  });

  it('cannot be double-submitted while the server is signing', async () => {
    let release: () => void = () => undefined;
    const client = { exportEvidence: vi.fn(() => new Promise<{ blob: Blob; filename: string }>(res => { release = () => res({ blob: zip(), filename: 'e.zip' }); })), evidenceKey: async () => testKey };
    render(<EvidenceExportPanel client={client} runId="run-1" requirements={[req()]} engineIsReal save={() => undefined} />);
    fireEvent.click(screen.getByRole('button', { name: 'Export evidence' }));
    const busy = await screen.findByRole('button', { name: 'Signing' });
    expect((busy as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(busy);
    expect(client.exportEvidence).toHaveBeenCalledTimes(1);
    release();
    await screen.findByText(/Signed by/);
  });
});

describe('the real engine client', () => {
  const original = globalThis.fetch;
  afterEach(() => { globalThis.fetch = original; });

  it('posts the requirement with the user\'s token and returns the ZIP with the server\'s file name', async () => {
    const seen: { url: string; auth: string | null; body: unknown }[] = [];
    globalThis.fetch = vi.fn(async (url: unknown, init?: RequestInit) => {
      seen.push({ url: String(url), auth: new Headers(init?.headers).get('authorization'), body: JSON.parse(String(init?.body)) });
      return new Response(new Uint8Array([1, 2, 3]), { status: 200, headers: { 'content-type': 'application/zip', 'content-disposition': 'attachment; filename="evidence-run-9.zip"' } });
    }) as unknown as typeof fetch;
    const out = await new RealEngineClient('/api', async () => 'tok-1').exportEvidence('run-9', { name: 'n', variable: 'y', comparator: 'atLeast', threshold: 0 }, 'my subject');
    expect(seen[0]).toMatchObject({ url: '/api/v1/runs/run-9/evidence', auth: 'Bearer tok-1', body: { requirement: { variable: 'y' }, subject: 'my subject' } });
    expect(out.filename).toBe('evidence-run-9.zip');
    expect(out.blob.size).toBe(3);
  });

  it('turns a refusal into words, tells the session about a 401, and never returns a file', async () => {
    globalThis.fetch = vi.fn(async () => new Response(JSON.stringify({ error: { message: 'Evidence export covers runs of the built-in engine.' } }), { status: 422 })) as unknown as typeof fetch;
    await expect(new RealEngineClient('/api').exportEvidence('run-9', { name: 'n', variable: 'y', comparator: 'above', threshold: 0 })).rejects.toThrow(/422.*built-in engine/);
    globalThis.fetch = vi.fn(async () => new Response('{}', { status: 401 })) as unknown as typeof fetch;
    const onUnauthorized = vi.fn();
    await expect(new RealEngineClient('/api', async () => 'tok', onUnauthorized).exportEvidence('r', { name: 'n', variable: 'y', comparator: 'above', threshold: 0 })).rejects.toThrow(/401/);
    expect(onUnauthorized).toHaveBeenCalledTimes(1);
  });

  it('reads the signing key', async () => {
    globalThis.fetch = vi.fn(async () => new Response(JSON.stringify(testKey), { status: 200 })) as unknown as typeof fetch;
    expect((await new RealEngineClient('/api').evidenceKey()).keyKind).toBe('test');
  });
});
