import React, { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import type { EngineClient } from '../../contracts';
import type { Requirement } from '../../app/requirements';
import { exportEvidence, ExportedEvidence, saveBlob, verifierCommand } from '../../app/evidenceExport';
import { Button, SectionLabel } from '../../components/ui/UiKit';

export interface EvidenceExportPanelProps {
  client: Pick<EngineClient, 'exportEvidence' | 'evidenceKey'>;
  /** Null: there is no stored run to package. */
  runId: string | null;
  requirements: Requirement[];
  /** The mock engine has no signing service. */
  engineIsReal: boolean;
  save?: (blob: Blob, filename: string) => void;
}

const opWord = { '<': 'below', '<=': 'at most', '>': 'above', '>=': 'at least' } as const;

/** Why export is not possible now, in words; null when it is. */
export function whyNotExportable(p: Pick<EvidenceExportPanelProps, 'client' | 'runId' | 'requirements' | 'engineIsReal'>): string | null {
  if (!p.engineIsReal || !p.client.exportEvidence) return 'Signed export needs the real engine. This build is running the mock engine, which has no signing service.';
  if (!p.runId) return 'There is no stored run to package. Run a simulation first.';
  if (p.requirements.length === 0) return 'Define a requirement first: the package states whether it held in the run.';
  return null;
}

export const EvidenceExportPanel: React.FC<EvidenceExportPanelProps> = ({ client, runId, requirements, engineIsReal, save = saveBlob }) => {
  const [chosen, setChosen] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<(ExportedEvidence & { runId: string }) | null>(null);
  const [copied, setCopied] = useState(false);

  const blocked = whyNotExportable({ client, runId, requirements, engineIsReal });
  const requirement = requirements.find(r => r.id === chosen) ?? requirements[0] ?? null;

  const run = async () => {
    if (!runId || !requirement) return;
    setBusy(true);
    setError(null);
    setDone(null);
    try { setDone({ ...(await exportEvidence(client, runId, requirement, save)), runId }); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  const command = verifierCommand(done?.runId ?? runId);
  const copy = () => { void navigator.clipboard?.writeText(command); setCopied(true); setTimeout(() => setCopied(false), 1800); };

  return (
    <div className="space-y-7">
      <section className="space-y-2">
        <SectionLabel>Signature</SectionLabel>
        {done ? (
          <>
            <p className="text-[13px] text-strong">Signed by {done.key?.signer ?? 'the evidence service'}{done.key ? `, key ${done.key.keyId}` : ''}.</p>
            {done.key?.keyKind === 'test' && <p role="note" className="text-xs text-warn">This is a test key held by the operator. It shows origin and integrity for testing; it is not a production signature.</p>}
            {done.keyError && <p role="alert" className="text-xs text-bad">The package was saved, but the signing key could not be read: {done.keyError}</p>}
            {done.key && <p className="num text-[11px] text-ink break-all">Public key {done.key.publicKey}</p>}
            <p className="text-xs text-muted">Saved {done.filename} ({done.bytes.toLocaleString()} bytes). Signing proves origin and integrity, not correct physics. The verdict inside says what was not validated.</p>
          </>
        ) : (
          <>
            <p className="text-[13px] text-strong">Not signed yet.</p>
            <p className="text-xs text-muted">Export signs on the server. The browser never holds a key. Signing proves origin and integrity, not correct physics.</p>
          </>
        )}
      </section>

      <section className="space-y-2">
        <SectionLabel>Verify a signed package</SectionLabel>
        <div className="flex items-start gap-2 surface p-3">
          <code className="num text-[11px] text-ink break-all flex-1">{command}</code>
          <button onClick={copy} aria-label="Copy verifier command" className="text-muted hover:text-strong shrink-0 mt-0.5">{copied ? <Check size={14} className="text-ok" /> : <Copy size={14} />}</button>
        </div>
        <p className="text-[11px] text-muted">Unzip the download first. The check needs no network and no HELQIX service.</p>
      </section>

      <section className="space-y-2">
        {requirements.length > 1 && (
          <label className="block text-xs text-muted">
            Requirement to state
            <select className="mt-1 block w-full surface px-2 py-1.5 text-[13px] text-strong" value={requirement?.id ?? ''} onChange={e => setChosen(e.target.value)}>
              {requirements.map(r => <option key={r.id} value={r.id}>{r.id}: {r.variable} {r.op} {r.limit}{r.unit ? ' ' + r.unit : ''}</option>)}
            </select>
          </label>
        )}
        {requirement && requirements.length === 1 && <p className="text-xs text-muted">The package states whether {requirement.variable} stays {opWord[requirement.op]} {requirement.limit}{requirement.unit ? ' ' + requirement.unit : ''} on every recorded sample.</p>}
        {blocked && <p className="text-xs text-muted">{blocked}</p>}
        {error && <p role="alert" className="text-xs text-bad">{error}</p>}
        <Button variant="primary" disabled={blocked !== null || busy} onClick={() => void run()}>{busy ? 'Signing' : 'Export evidence'}</Button>
      </section>
    </div>
  );
};
