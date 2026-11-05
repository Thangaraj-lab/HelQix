import React, { useEffect, useRef, useState } from 'react';
import type { ModelRevision } from '../../contracts';
import { engine } from '../../engine';
import { Button } from '../../components/ui/UiKit';
import { fmuPartOf } from './fmuRun';

const MAX_STEPS = 200_000;
const input = 'h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60';

/** Simulate for a model whose part has an FMU behaviour: the engine runs that FMU through FMPy. The time span starts from the FMU's own
 *  default experiment (read from its model description), and the run's frames are the variables the part records. */
export const FmuRunForm: React.FC<{ revision: ModelRevision; onRan: (runId: string) => void }> = ({ revision, onRan }) => {
  const part = fmuPartOf(revision);
  const sha = part?.behavior?.fmuSha256 ?? null;
  const [end, setEnd] = useState('3');
  const [step, setStep] = useState('0.01');
  const [model, setModel] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const typed = useRef({ end: false, step: false });

  useEffect(() => {
    if (!sha || !engine.describeFmu) return;
    let live = true;
    engine.describeFmu(sha).then(d => {
      if (!live) return;
      setModel(d.modelName);
      const e = d.defaultExperiment;
      if (!typed.current.end && e.stopTime !== null && e.stopTime > 0) setEnd(String(e.stopTime));
      if (!typed.current.step && e.stepSize !== null && e.stepSize > 0) setStep(String(e.stepSize));
    }).catch(() => undefined);
    return () => { live = false; };
  }, [sha]);

  const e = Number(end);
  const s = Number(step);
  const valid = end.trim() !== '' && step.trim() !== '' && Number.isFinite(e) && Number.isFinite(s) && e > 0 && s > 0 && e / s <= MAX_STEPS;
  if (!part || !sha) return null;

  const go = async () => {
    if (!valid || !engine.runFmuRevision) return;
    setBusy(true); setError(null); setNote(null);
    try {
      const r = await engine.runFmuRevision(revision.projectId, revision.revisionNum, { endSeconds: e, stepSeconds: s });
      setNote(`Ran the FMU ${model ?? ''} of ${part.name}: ${r.rows} frames, determinism ${r.determinism} (an FMU is never bit-exact across machines).`.replace('FMU  of', 'FMU of'));
      onRan(r.runId);
    } catch (err) { setError(err instanceof Error ? err.message.replace(/^The engine answered \d+:\s*/, '') : 'The run could not be started.'); }
    setBusy(false);
  };

  return (
    <div className="space-y-2.5">
      <p className="text-xs text-ink">{part.name} runs the FMU {model ?? <span className="num">{sha.slice(0, 8)}…</span>} through FMPy.</p>
      <div className="grid grid-cols-2 gap-2">
        <label className="block space-y-1"><span className="text-[11px] text-muted">Duration (s)</span>
          <input value={end} inputMode="decimal" onChange={ev => { typed.current.end = true; setEnd(ev.target.value); }} aria-label="Duration (s)" className={input} /></label>
        <label className="block space-y-1"><span className="text-[11px] text-muted">Step (s)</span>
          <input value={step} inputMode="decimal" onChange={ev => { typed.current.step = true; setStep(ev.target.value); }} aria-label="Step (s)" className={input} /></label>
      </div>
      {error && <p role="alert" className="text-xs text-bad">{error}</p>}
      {note && <p className="text-[11px] text-trace">{note}</p>}
      <Button size="sm" variant="primary" disabled={!valid || busy} onClick={() => void go()}>Run my model</Button>
      <p className="text-[11px] text-muted">The frames are the variables this part records. A run of an FMU cannot be frozen or branched: an FMU cannot restore a saved state.</p>
    </div>
  );
};
