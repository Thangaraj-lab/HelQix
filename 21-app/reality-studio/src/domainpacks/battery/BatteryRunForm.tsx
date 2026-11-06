import React, { useEffect, useState } from 'react';
import type { ModelRevision } from '../../contracts';
import { engine } from '../../engine';
import { Button } from '../../components/ui/UiKit';
import { cellPartOf, packNotice } from './batteryCell';
import type { PackInfo } from './types';

const MAX_SAMPLES = 20_000;
const input = 'h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60';

/** Simulate for a model whose part is a battery cell. The pack's label sits above the button, before anyone runs anything. */
export const BatteryRunForm: React.FC<{ revision: ModelRevision; onRan: (runId: string) => void }> = ({ revision, onRan }) => {
  const part = cellPartOf(revision);
  const [end, setEnd] = useState('600');
  const [step, setStep] = useState('30');
  const [pack, setPack] = useState<PackInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    if (!part || !engine.listPacks) return;
    let live = true;
    engine.listPacks().then(p => live && setPack(p[0] ?? null)).catch(() => undefined);
    return () => { live = false; };
  }, [part?.id]);

  const e = Number(end);
  const s = Number(step);
  const valid = end.trim() !== '' && step.trim() !== '' && Number.isFinite(e) && Number.isFinite(s) && e > 0 && s > 0 && Math.round(e / s) >= 1 && e / s + 1 <= MAX_SAMPLES;
  if (!part) return null;

  const go = async () => {
    if (!valid || !engine.runPackRevision) return;
    setBusy(true); setError(null); setNote(null);
    try {
      const r = await engine.runPackRevision(revision.projectId, revision.revisionNum, { endSeconds: e, stepSeconds: s });
      setPack(r.pack);
      setNote(`Ran ${part.name} as a battery cell: ${r.rows} frames, determinism ${r.determinism}.`);
      onRan(r.runId);
    } catch (err) { setError(err instanceof Error ? err.message.replace(/^The engine answered \d+:\s*/, '') : 'The run could not be started.'); }
    setBusy(false);
  };

  return (
    <div className="space-y-2.5">
      <p className="text-xs text-ink">{part.name} runs as a battery cell through PyBaMM.</p>
      {pack && <p role="note" className="text-xs text-warn rounded-[8px] border border-line p-2">{packNotice(pack)}</p>}
      <div className="grid grid-cols-2 gap-2">
        <label className="block space-y-1"><span className="text-[11px] text-muted">Duration (s)</span>
          <input value={end} inputMode="decimal" onChange={ev => setEnd(ev.target.value)} aria-label="Duration (s)" className={input} /></label>
        <label className="block space-y-1"><span className="text-[11px] text-muted">Step (s)</span>
          <input value={step} inputMode="decimal" onChange={ev => setStep(ev.target.value)} aria-label="Step (s)" className={input} /></label>
      </div>
      {error && <p role="alert" className="text-xs text-bad">{error}</p>}
      {note && <p className="text-[11px] text-trace">{note}</p>}
      <Button size="sm" variant="primary" disabled={!valid || busy} onClick={() => void go()}>Run my model</Button>
      <p className="text-[11px] text-muted">The frames are terminal voltage (V) and discharge capacity (A.h). A positive current discharges the cell. This run cannot be frozen or branched.</p>
    </div>
  );
};
