import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Behavior, FmuDescription, FmuInfo, ModelOp } from '../../contracts';
import { engine } from '../../engine';
import { Button } from '../../components/ui/UiKit';
import { buildBehaviorOp, outputCandidates, parameterCandidates } from './fmuBehavior';

const short = (sha: string) => `${sha.slice(0, 8)}…${sha.slice(-4)}`;
const input = 'h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60';

/** The Behaviour of a part (real engine): choose an FMU the operator installed, see its parameters and outputs, attach it.
 *  There is no upload: an FMU is native code, so only files an operator installed are ever run. */
export const FmuBehaviorPanel: React.FC<{ partId: string; locked: boolean; current?: Behavior; onApply: (op: ModelOp) => void }> = ({ partId, locked, current, onApply }) => {
  const [library, setLibrary] = useState<FmuInfo[] | null>(null);
  const [sha, setSha] = useState<string>('');
  const [desc, setDesc] = useState<FmuDescription | null>(null);
  const [typed, setTyped] = useState<Record<string, string>>({});
  const [outputs, setOutputs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const latest = useRef('');

  useEffect(() => {
    let live = true;
    if (!engine.listFmus) { setLibrary([]); return; }
    engine.listFmus().then(l => live && setLibrary(l)).catch(e => { if (live) { setLibrary([]); setError(e instanceof Error ? e.message : 'The FMU library could not be read.'); } });
    return () => { live = false; };
  }, []);

  const choose = (next: string) => {
    latest.current = next;
    setSha(next); setDesc(null); setTyped({}); setOutputs([]); setError(null);
    if (!next || !engine.describeFmu) return;
    setBusy(true);
    engine.describeFmu(next)
      .then(d => { if (latest.current !== next) return; setDesc(d); setOutputs(outputCandidates(d).filter(o => o.isOutput).map(o => o.name)); })
      .catch(e => { if (latest.current === next) setError(e instanceof Error ? e.message.replace(/^The engine answered \d+:\s*/, '') : 'The FMU could not be described.'); })
      .finally(() => { if (latest.current === next) setBusy(false); });
  };

  const params = useMemo(() => (desc ? parameterCandidates(desc) : []), [desc]);
  const outs = useMemo(() => (desc ? outputCandidates(desc) : []), [desc]);
  const attach = () => {
    if (!desc) return;
    const r = buildBehaviorOp(partId, sha, desc, typed, outputs);
    if ('error' in r) { setError(r.error); return; }
    setError(null);
    onApply(r.op);
  };

  const now = current?.kind === 'fmu' ? (
    <p className="text-xs text-ink">FMU <span className="num">{short(current.fmuSha256)}</span>, records {current.outputs.join(', ')}{current.parameters ? `, sets ${Object.keys(current.parameters).join(', ')}` : ''}.</p>
  ) : current ? (
    <p className="text-xs text-ink">This part has a behaviour from a domain pack. Choosing an FMU here replaces it.</p>
  ) : <p className="text-xs text-muted">None. A part with an FMU behaviour is run through FMPy; without one the model runs as a simple falling body.</p>;

  return (
    <div className="space-y-2" aria-label="Behaviour">
      <p className="text-[11px] text-muted">Behaviour</p>
      {now}
      {locked ? <p className="text-[11px] text-muted">The model is committed, so its behaviour will not change. Start a new draft to change it.</p> : library === null ? <p className="text-xs text-muted">Reading the FMU library…</p> : library.length === 0 ? (
        <p className="text-xs text-muted">No FMU is installed in the library. An operator installs FMUs on the server; they cannot be uploaded here, because an FMU contains code that runs on the server.</p>
      ) : (
        <>
          <label className="block space-y-1"><span className="text-[11px] text-muted">FMU from the library</span>
            <select value={sha} onChange={e => choose(e.target.value)} aria-label="FMU from the library" className={input}>
              <option value="">Choose…</option>
              {library.map(f => <option key={f.sha256} value={f.sha256}>{short(f.sha256)} ({Math.round(f.bytes / 1024)} KB)</option>)}
            </select></label>
          {busy && <p className="text-xs text-muted">Reading the FMU…</p>}
          {desc && (
            <div className="space-y-2.5">
              <p className="text-xs text-ink">{desc.modelName} <span className="text-muted">(FMI {desc.fmiVersion})</span></p>
              {params.length > 0 && (
                <div className="space-y-1.5">
                  <p className="text-[11px] text-muted">Parameters. Leave one empty to keep the FMU's own value.</p>
                  {params.map(p => (
                    <label key={p.name} className="block space-y-0.5"><span className="text-[11px] text-muted">{p.name} ({p.unit === 'one' ? 'no unit' : p.unit}){p.description ? `, ${p.description}` : ''}</span>
                      <input value={typed[p.name] ?? ''} inputMode="decimal" placeholder={p.start === null ? '' : String(p.start)} onChange={e => setTyped({ ...typed, [p.name]: e.target.value })} aria-label={`Parameter ${p.name}`} className={input} /></label>
                  ))}
                </div>
              )}
              <div className="space-y-1">
                <p className="text-[11px] text-muted">Record these variables as the run's frames</p>
                {outs.map(o => (
                  <label key={o.name} className="flex items-center gap-2 text-xs text-ink">
                    <input type="checkbox" checked={outputs.includes(o.name)} onChange={e => setOutputs(e.target.checked ? [...outputs, o.name] : outputs.filter(x => x !== o.name))} aria-label={`Record ${o.name}`} />
                    <span className="num">{o.name}</span><span className="text-muted">{o.unit === 'one' ? '' : o.unit}{o.isOutput ? '' : ', internal'}</span>
                  </label>
                ))}
              </div>
              <Button size="sm" variant="secondary" disabled={busy} onClick={attach}>Attach FMU</Button>
            </div>
          )}
        </>
      )}
      {error && <p role="alert" className="text-xs text-bad">{error}</p>}
    </div>
  );
};
