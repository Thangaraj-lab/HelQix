import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Check, FastForward, Pause, Play, Plus, Redo2, SkipForward, Undo2 } from 'lucide-react';
import { BuildEvent, GeometryKind, Issue, Maturity, ModelOp, ModelRevision, Source, Transform, Value } from '../../contracts';
import { engine } from '../../engine';
import type { ScenePart } from '../../scene/HeroScene';
import { createMockValue } from '../../mock/examplePack';
import { Button, Chip, Drawer, IconButton, MockTag, PlaceholderTag, SectionLabel, StatusDot, Tabs, ValueView } from '../../components/ui/UiKit';

export interface AssembleStepProps {
  projectId: string;
  /** The user's prompt, or null for the scratch path. */
  prompt: string | null;
  /** Revision label from the URL (rev), when this project already has a committed model. */
  committedRev: string | null;
  onCommitSuccess: (revision: ModelRevision) => void;
  onRunSimulation?: () => void;
  /** Placed parts, for the Studio canvas. */
  onPartsChange?: (parts: ScenePart[]) => void;
}

// --- State derived from the op log: the log is the truth, the tree and canvas are views of it ---

interface StagedPart {
  id: string;
  name: string;
  geometry: GeometryKind;
  maturity: Maturity;
  transform?: Transform;
  ports: { id: string; kind: string; direction: 'in' | 'out' | 'bi'; unit?: string }[];
  params: Record<string, Value>;
  connected: boolean;
}
interface StagedConnection { id: string; from: string; to: string; kind: string }

function reduceOps(ops: ModelOp[]): { parts: Map<string, StagedPart>; connections: StagedConnection[] } {
  const parts = new Map<string, StagedPart>();
  let connections: StagedConnection[] = [];
  for (const op of ops) {
    if (op.op === 'part.add') parts.set(op.id, { id: op.id, name: op.name, geometry: op.geometry, maturity: op.maturity, ports: [], params: {}, connected: false });
    else if (op.op === 'part.place') { const p = parts.get(op.id); if (p) p.transform = op.transform; }
    else if (op.op === 'port.add') parts.get(op.part)?.ports.push({ id: op.id, kind: op.kind, direction: op.direction, unit: op.unit });
    else if (op.op === 'part.setParam') { const p = parts.get(op.id); if (p) p.params[op.key] = op.value; }
    else if (op.op === 'part.remove') parts.delete(op.id);
    else if (op.op === 'connection.add') {
      connections.push({ id: op.id, from: `${op.from.partId}:${op.from.portId}`, to: `${op.to.partId}:${op.to.portId}`, kind: op.kind });
      const a = parts.get(op.from.partId); const b = parts.get(op.to.partId);
      if (a) a.connected = true;
      if (b) b.connected = true;
    } else if (op.op === 'connection.remove') connections = connections.filter(c => c.id !== op.id);
  }
  return { parts, connections };
}

const STEPS: { id: NonNullable<BuildEvent['step']>; label: string }[] = [
  { id: 'interpret', label: 'Resolving the brief' },
  { id: 'resolve', label: 'Importing geometry and models' },
  { id: 'place', label: 'Placing parts' },
  { id: 'connect', label: 'Connecting ports' },
  { id: 'check', label: 'Checking units' }
];

const SPEEDS = [1, 2, 4] as const;
const BASE_DELAY_MS = 450;

export const AssembleStep: React.FC<AssembleStepProps> = ({ projectId, prompt, committedRev, onCommitSuccess, onPartsChange }) => {
  const [buildId, setBuildId] = useState('');
  const [events, setEvents] = useState<BuildEvent[]>([]);
  const [ops, setOps] = useState<ModelOp[]>([]);
  const [redo, setRedo] = useState<ModelOp[]>([]);
  const [issues, setIssues] = useState<Issue[]>([]);
  const [activeStep, setActiveStep] = useState<string>('interpret');
  const [doneSteps, setDoneSteps] = useState<Set<string>>(new Set());
  const [ready, setReady] = useState(false);
  const [committed, setCommitted] = useState<ModelRevision | null>(null);
  const [paused, setPaused] = useState(false);
  const [speed, setSpeed] = useState<(typeof SPEEDS)[number]>(1);
  const [cursor, setCursor] = useState(0);
  const [total, setTotal] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [tab, setTab] = useState<'issues' | 'connections' | 'sources' | 'history'>('issues');
  const [sources, setSources] = useState<Source[] | null>(null);
  const [showDefine, setShowDefine] = useState(false);
  const stream = useRef<BuildEvent[]>([]);

  const isCommitted = committed !== null || committedRev !== null;
  const { parts, connections } = useMemo(() => reduceOps(ops), [ops]);

  const applyEvent = useCallback((evt: BuildEvent) => {
    setEvents(prev => [...prev, evt]);
    if (evt.kind === 'step.start' && evt.step) setActiveStep(evt.step);
    else if (evt.kind === 'step.done' && evt.step) setDoneSteps(prev => new Set(prev).add(evt.step!));
    else if (evt.kind === 'op.applied' && evt.op) {
      const op = evt.op;
      setOps(prev => [...prev, op]);
      if (op.op === 'part.add') setSelected(op.id);
    } else if (evt.kind === 'issue.raised' && evt.issue) setIssues(prev => [...prev, evt.issue!]);
    else if (evt.kind === 'issue.cleared' && evt.issue) setIssues(prev => prev.filter(i => i.id !== evt.issue!.id));
    else if (evt.kind === 'staged.ready') setReady(true);
  }, []);

  // Start the build in the project from the URL, then collect its event stream.
  useEffect(() => {
    let dead = false;
    (async () => {
      try {
        const { buildId: id } = await engine.startBuild(projectId, prompt === null ? { scratch: true as const } : { prompt });
        if (dead) return;
        setBuildId(id);
        const collected: BuildEvent[] = [];
        for await (const e of engine.buildEvents(id)) collected.push(e);
        if (dead) return;
        stream.current = collected;
        setTotal(collected.length);
      } catch (e) {
        if (!dead) setError(e instanceof Error ? e.message : String(e));
      }
    })();
    return () => { dead = true; };
  }, [projectId, prompt]);

  // Playback of the real op log: one event per tick, honouring pause and speed.
  useEffect(() => {
    if (paused || cursor >= total) return;
    const t = setTimeout(() => {
      applyEvent(stream.current[cursor]);
      setCursor(c => c + 1);
    }, Math.max(120, BASE_DELAY_MS / speed));
    return () => clearTimeout(t);
  }, [paused, cursor, total, speed, applyEvent]);

  const playing = cursor < total;
  const step = () => { if (cursor < total) { applyEvent(stream.current[cursor]); setCursor(c => c + 1); } };
  const skip = () => { stream.current.slice(cursor).forEach(applyEvent); setCursor(total); };

  const apply = useCallback(async (list: ModelOp[]) => {
    if (!buildId) return;
    const evts = await engine.applyOps(buildId, list);
    evts.forEach(applyEvent);
    setRedo([]);
  }, [buildId, applyEvent]);

  const undo = async () => {
    if (!buildId || ops.length === 0) return;
    await engine.undo(buildId);
    const last = ops[ops.length - 1];
    setOps(prev => prev.slice(0, -1));
    setRedo(prev => [...prev, last]);
  };
  const redoOp = async () => {
    if (!buildId || redo.length === 0) return;
    await engine.redo(buildId);
    const next = redo[redo.length - 1];
    setRedo(prev => prev.slice(0, -1));
    setOps(prev => [...prev, next]);
  };

  const fix = async (issue: Issue) => {
    if (!issue.fixOp) return;
    await apply([issue.fixOp]);
    setIssues(prev => prev.filter(i => i.id !== issue.id));
  };

  // The canvas draws placed parts only.
  useEffect(() => {
    onPartsChange?.(
      Array.from(parts.values())
        .filter(p => p.transform)
        .map(p => ({
          id: p.id,
          name: p.name,
          geometry: p.geometry,
          pos: [p.transform!.pos[0].v, p.transform!.pos[1].v, p.transform!.pos[2].v] as [number, number, number],
          placeholder: p.maturity === 'placeholder'
        }))
    );
  }, [parts, onPartsChange]);

  const blocking = issues.filter(i => i.severity === 'blocking');
  const canCommit = !!buildId && ready && !isCommitted && blocking.length === 0;
  const commit = async () => {
    if (!canCommit) return;
    const rev = await engine.commit(buildId);
    setCommitted(rev);
    onCommitSuccess(rev);
  };
  // The Studio header's primary button asks for a commit while this step is open.
  const commitRef = useRef(commit);
  commitRef.current = commit;
  useEffect(() => {
    const on = () => void commitRef.current();
    window.addEventListener('helqix:commit-request', on);
    return () => window.removeEventListener('helqix:commit-request', on);
  }, []);
  const revLabel = committed ? `r${committed.revisionNum}` : committedRev ?? 'r1';

  useEffect(() => {
    if (tab === 'sources' && sources === null) engine.listSources(projectId).then(setSources).catch(() => setSources([]));
  }, [tab, sources, projectId]);

  const sel = selected ? parts.get(selected) : undefined;
  const placed = Array.from(parts.values()).filter(p => p.transform).length;
  const counts: Record<string, string> = {
    interpret: prompt === null ? 'Scratch path' : '1 brief',
    resolve: `${parts.size} ${parts.size === 1 ? 'part' : 'parts'}`,
    place: `${placed} of ${parts.size} placed`,
    connect: `${connections.length} ${connections.length === 1 ? 'connection' : 'connections'}`,
    check: `${issues.length} open ${issues.length === 1 ? 'issue' : 'issues'}`
  };

  const status = isCommitted
    ? { tone: 'ok' as const, text: `Model ${revLabel} is committed and will not change.` }
    : !ready
    ? { tone: 'muted' as const, text: playing ? 'The build is running.' : 'Waiting for the build.' }
    : blocking.length > 0
    ? { tone: 'warn' as const, text: `Resolve ${blocking.length} blocking ${blocking.length === 1 ? 'issue' : 'issues'} to create the model.` }
    : { tone: 'accent' as const, text: 'Ready. Use Create model to commit this draft.' };

  const setPosition = (axis: 0 | 1 | 2, value: number) => {
    if (!sel || !sel.transform || Number.isNaN(value)) return;
    const pos = [...sel.transform.pos] as Transform['pos'];
    pos[axis] = createMockValue(value, 'm');
    void apply([{ op: 'part.place', id: sel.id, transform: { ...sel.transform, pos } }]);
  };

  if (error) {
    return (
      <div className="relative z-10 m-auto surface p-6 max-w-md space-y-3" role="alert">
        <p className="text-sm font-medium text-bad">The build could not start.</p>
        <p className="text-xs text-muted">{error}</p>
        <Button onClick={() => window.location.reload()}>Try again</Button>
      </div>
    );
  }

  return (
    <div className="relative z-10 flex-1 min-h-0 grid grid-cols-[288px_minmax(0,1fr)_300px] max-[1100px]:grid-cols-[240px_minmax(0,1fr)_260px] gap-3 p-3 pointer-events-none">
      {/* Left: build log and model tree */}
      <div className="flex flex-col gap-3 min-h-0 pointer-events-auto">
        <section className="surface p-4 space-y-3" aria-label="Build log">
          <SectionLabel aside={<Chip tone={isCommitted ? 'ok' : 'neutral'}>{isCommitted ? `Model ${revLabel}` : 'Draft'}</Chip>}>Build</SectionLabel>
          <ol className="space-y-1">
            {STEPS.map((s, i) => {
              const done = doneSteps.has(s.id) || ready;
              const current = !done && activeStep === s.id && (playing || !ready);
              return (
                <li key={s.id} className={`flex items-center gap-3 px-2 py-1.5 rounded-[8px] ${current ? 'bg-white/[0.05]' : ''}`}>
                  <span className={`w-5 h-5 rounded-full inline-flex items-center justify-center shrink-0 text-[10px] ${done ? 'bg-ok/15 text-ok' : current ? 'bg-accent text-white' : 'border border-line-strong text-muted'}`}>
                    {done ? <Check size={11} /> : <span className="num">{i + 1}</span>}
                  </span>
                  <span className="min-w-0">
                    <span className={`block text-[13px] ${done || current ? 'text-strong' : 'text-muted'}`}>{s.label}</span>
                    <span className="block text-[11px] text-muted">{counts[s.id]}</span>
                  </span>
                </li>
              );
            })}
          </ol>
          <p className="text-[11px] text-muted pt-1 border-t border-line">Playback of the real op log, not a progress bar.</p>
        </section>

        <section className="surface p-4 flex-1 min-h-0 flex flex-col gap-3" aria-label="Model tree">
          <SectionLabel aside={<span className="num text-xs text-muted">{parts.size}</span>}>Model tree</SectionLabel>
          {parts.size === 0 ? (
            <p className="text-xs text-muted">{prompt === null ? 'The canvas is empty. Define a part to begin.' : 'Parts appear here as the build adds them.'}</p>
          ) : (
            <ul className="space-y-1 overflow-y-auto -mx-1 px-1">
              {Array.from(parts.values()).map(p => {
                const hasIssue = issues.some(i => i.partId === p.id);
                return (
                  <li key={p.id}>
                    <button onClick={() => setSelected(p.id)} className={`w-full text-left px-3 py-2 rounded-[8px] border transition-colors ${selected === p.id ? 'bg-accent/10 border-accent/40' : 'border-transparent hover:bg-white/[0.04]'}`}>
                      <span className="flex items-center justify-between gap-2">
                        <span className="text-[13px] text-strong truncate">{p.name}</span>
                        {p.maturity === 'placeholder' && <PlaceholderTag label="Placeholder" />}
                      </span>
                      <span className="flex items-center gap-2 mt-0.5 text-[11px] text-muted">
                        <StatusDot tone={hasIssue ? 'warn' : p.connected ? 'ok' : p.transform ? 'accent' : 'muted'} />
                        {p.geometry.kind === 'primitive' ? p.geometry.shape : 'Imported'}
                        <span aria-hidden>·</span>
                        {p.connected ? 'Connected' : p.transform ? 'Placed' : 'Draft'}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      </div>

      {/* Center: canvas shows through; toolbar on top, bottom cards below */}
      <div className="flex flex-col justify-between min-h-0 min-w-0">
        <div className="pointer-events-auto self-center surface-glass px-2 py-1.5 flex items-center gap-1 flex-wrap justify-center">
          <IconButton label={paused ? 'Resume playback' : 'Pause playback'} onClick={() => setPaused(p => !p)} disabled={!playing}>{paused ? <Play size={15} /> : <Pause size={15} />}</IconButton>
          <IconButton label="Step one event" onClick={step} disabled={!playing}><SkipForward size={15} /></IconButton>
          <IconButton label="Skip to end of the build" onClick={skip} disabled={!playing}><FastForward size={15} /></IconButton>
          <Tabs tabs={SPEEDS.map(s => ({ id: String(s), label: `${s}x` }))} activeTab={String(speed)} onChange={id => setSpeed(Number(id) as (typeof SPEEDS)[number])} className="mx-1" />
          <span className="w-px h-5 bg-line mx-1" aria-hidden />
          <IconButton label="Undo" onClick={undo} disabled={!ready || ops.length === 0 || isCommitted}><Undo2 size={15} /></IconButton>
          <IconButton label="Redo" onClick={redoOp} disabled={!ready || redo.length === 0 || isCommitted}><Redo2 size={15} /></IconButton>
          <span className="w-px h-5 bg-line mx-1" aria-hidden />
          <Button size="sm" variant="ghost" icon={<Plus size={14} />} onClick={() => setShowDefine(true)} disabled={!buildId || isCommitted}>Define a part</Button>
        </div>

        <div className="pointer-events-auto space-y-3">
          <p role="status" className={`mx-auto w-fit max-w-full surface-glass px-4 py-2 text-xs flex items-center gap-2 ${status.tone === 'ok' ? 'text-ok' : status.tone === 'warn' ? 'text-warn' : 'text-ink'}`}>
            <StatusDot tone={status.tone === 'muted' ? 'muted' : status.tone} />
            {status.text}
          </p>

          <section className="surface-glass overflow-hidden" aria-label="Details">
            <div className="flex items-center justify-between px-3 h-11 border-b border-line">
              <Tabs
                tabs={[
                  { id: 'issues', label: 'Issues', badge: issues.length ? String(issues.length) : undefined },
                  { id: 'connections', label: 'Connections', badge: connections.length ? String(connections.length) : undefined },
                  { id: 'sources', label: 'Sources' },
                  { id: 'history', label: 'History', badge: ops.length ? String(ops.length) : undefined }
                ]}
                activeTab={tab}
                onChange={id => setTab(id as typeof tab)}
              />
            </div>
            <div className="max-h-[168px] overflow-y-auto p-3 text-[13px]">
              {tab === 'issues' && (issues.length === 0 ? (
                <p className="text-xs text-muted">No open issues.</p>
              ) : (
                <ul className="space-y-2">
                  {issues.map(i => (
                    <li key={i.id} className="flex items-center justify-between gap-3 rounded-[10px] border border-warn/25 bg-warn/[0.06] px-3 py-2">
                      <span className="min-w-0">
                        <span className="block text-ink">{i.message}</span>
                        <span className="block text-[11px] text-muted">{i.severity === 'blocking' ? 'Blocking' : 'Warning'}{i.partId ? ` · ${parts.get(i.partId)?.name ?? i.partId}` : ''}</span>
                      </span>
                      {i.fixOp && <Button size="sm" variant="secondary" onClick={() => fix(i)}>Fix</Button>}
                    </li>
                  ))}
                </ul>
              ))}
              {tab === 'connections' && (connections.length === 0 ? (
                <p className="text-xs text-muted">No connections yet.</p>
              ) : (
                <ul className="space-y-1.5">
                  {connections.map(c => (
                    <li key={c.id} className="flex items-center justify-between text-xs"><span className="num text-ink">{c.from} to {c.to}</span><Chip>{c.kind.replace(/_/g, ' ')}</Chip></li>
                  ))}
                </ul>
              ))}
              {tab === 'sources' && (sources === null ? <p className="text-xs text-muted">Loading.</p> : sources.length === 0 ? (
                <p className="text-xs text-muted">No sources connected.</p>
              ) : (
                <ul className="space-y-1.5">{sources.map(s => <li key={s.id} className="flex items-center gap-2 text-xs text-ink"><StatusDot tone={s.status === 'connected' ? 'ok' : 'warn'} />{s.name}</li>)}</ul>
              ))}
              {tab === 'history' && (ops.length === 0 ? (
                <p className="text-xs text-muted">No operations yet.</p>
              ) : (
                <ol className="space-y-1">
                  {ops.map((op, i) => (
                    <li key={i} className="flex items-center justify-between text-xs"><span className="num text-muted">{i + 1}</span><span className="text-ink flex-1 ml-3">{op.op}</span><span className="num text-muted">{('id' in op && op.id) || ('part' in op && op.part) || ''}</span></li>
                  ))}
                </ol>
              ))}
            </div>
          </section>
        </div>
      </div>

      {/* Right: inspector and proposals */}
      <div className="flex flex-col gap-3 min-h-0 pointer-events-auto overflow-y-auto">
        <section className="surface p-4 space-y-3" aria-label="Inspector">
          <SectionLabel>Inspector</SectionLabel>
          {!sel ? (
            <p className="text-xs text-muted">Select a part in the model tree to see its properties.</p>
          ) : (
            <div className="space-y-4">
              <div>
                <p className="text-[14px] font-medium text-strong">{sel.name}</p>
                <p className="text-xs text-muted mt-0.5">{sel.geometry.kind === 'primitive' ? `Primitive ${sel.geometry.shape}` : 'Imported geometry'} · {sel.maturity.replace(/_/g, ' ')}</p>
              </div>
              {sel.geometry.kind === 'primitive' && (
                <div>
                  <p className="text-[11px] text-muted mb-1.5 flex items-center justify-between">
                    <span>Dimensions</span>
                    <span className="flex items-center gap-1.5">
                      {sel.geometry.dims.some(d => d.mock) && <MockTag />}
                      <span className="capitalize">{sel.geometry.dims[0]?.origin.replace('_', ' ')}, {sel.geometry.dims[0]?.status}</span>
                    </span>
                  </p>
                  <dl className="grid grid-cols-3 gap-2">
                    {sel.geometry.dims.map((d, i) => (
                      <div key={i} className="rounded-[8px] bg-white/[0.03] border border-line px-2.5 py-1.5">
                        <dt className="text-[10px] text-muted">{['X', 'Y', 'Z'][i]}</dt>
                        <dd className="num text-[13px] text-strong">{d.v} <span className="text-[11px] text-muted">{d.unit}</span></dd>
                      </div>
                    ))}
                  </dl>
                </div>
              )}
              {sel.transform && (
                <div>
                  <p className="text-[11px] text-muted mb-1.5">Position (m)</p>
                  <div className="grid grid-cols-3 gap-2">
                    {(['X', 'Y', 'Z'] as const).map((a, i) => (
                      <label key={a} className="flex items-center gap-1.5 rounded-[8px] bg-void/70 border border-line px-2 h-8 focus-within:border-accent/60">
                        <span className="text-[11px] text-muted">{a}</span>
                        <input
                          type="number"
                          step={0.1}
                          defaultValue={sel.transform!.pos[i].v}
                          key={`${sel.id}-${i}-${sel.transform!.pos[i].v}`}
                          onBlur={e => e.currentTarget.value !== String(sel.transform!.pos[i].v) && !isCommitted && setPosition(i as 0 | 1 | 2, parseFloat(e.currentTarget.value))}
                          disabled={isCommitted}
                          aria-label={`Position ${a} in metres`}
                          className="num w-full bg-transparent text-xs focus:outline-none"
                        />
                      </label>
                    ))}
                  </div>
                </div>
              )}
              {sel.ports.length > 0 && (
                <div>
                  <p className="text-[11px] text-muted mb-1.5">Ports</p>
                  <ul className="space-y-1">
                    {sel.ports.map(p => <li key={p.id} className="flex items-center justify-between text-xs text-ink"><span className="num">{p.id}</span><span className="text-muted">{p.kind.replace(/_/g, ' ')}{p.unit ? `, ${p.unit}` : ''}</span></li>)}
                  </ul>
                </div>
              )}
              {Object.keys(sel.params).length > 0 && (
                <div className="space-y-2">{Object.entries(sel.params).map(([k, v]) => <ValueView key={k} label={k} value={v} />)}</div>
              )}
            </div>
          )}
        </section>

        <section className="surface p-4 space-y-2" aria-label="Proposals">
          <SectionLabel>Proposals</SectionLabel>
          <p className="text-xs text-muted">None yet. Rule-based proposals appear after the checks run, and state their effect only once a simulation has measured it.</p>
        </section>
      </div>

      <DefinePartDrawer
        open={showDefine}
        onClose={() => setShowDefine(false)}
        onAdd={async list => { await apply(list); setShowDefine(false); }}
      />
    </div>
  );
};

// --- Define a part (scratch path): geometry, port, then placed at the origin; position is editable after ---

const DefinePartDrawer: React.FC<{ open: boolean; onClose: () => void; onAdd: (ops: ModelOp[]) => Promise<void> }> = ({ open, onClose, onAdd }) => {
  const [name, setName] = useState('Custom module');
  const [shape, setShape] = useState<'box' | 'cylinder' | 'sphere' | 'plate'>('box');
  const [dims, setDims] = useState<[number, number, number]>([120, 80, 60]);
  const [portName, setPortName] = useState('power-in');
  const [portKind, setPortKind] = useState('electrical_dc');
  const [portDir, setPortDir] = useState<'in' | 'out' | 'bi'>('in');
  const [portUnit, setPortUnit] = useState('V');

  const field = 'h-8 rounded-[8px] bg-void/70 border border-line px-2.5 text-[13px] focus:outline-none focus:border-accent/60 w-full';

  const add = () => {
    const id = `part-${Date.now().toString(36)}`;
    void onAdd([
      { op: 'part.add', id, name: name.trim() || 'Part', geometry: { kind: 'primitive', shape, dims: dims.map(d => createMockValue(d, 'mm')) }, maturity: 'user_defined', prov: { kind: 'user_stated', note: 'Defined from scratch' } },
      { op: 'part.place', id, transform: { pos: [createMockValue(0, 'm'), createMockValue(0.5, 'm'), createMockValue(0, 'm')], rot: [createMockValue(0, 'deg'), createMockValue(0, 'deg'), createMockValue(0, 'deg')], scale: [1, 1, 1], frame: 'world' } },
      { op: 'port.add', part: id, id: portName.trim() || 'port', kind: portKind, direction: portDir, unit: portUnit }
    ]);
  };

  return (
    <Drawer isOpen={open} onClose={onClose} title="Define a part">
      <div className="space-y-6">
        <p className="text-xs text-muted">The part is tagged User-defined and Unverified. It is placed at the origin; edit its position in the inspector.</p>
        <label className="block space-y-1.5"><span className="text-xs text-muted">Name</span><input className={field} value={name} onChange={e => setName(e.target.value)} /></label>
        <div className="space-y-1.5">
          <span className="text-xs text-muted">Geometry</span>
          <Tabs tabs={(['box', 'cylinder', 'sphere', 'plate'] as const).map(s => ({ id: s, label: s[0].toUpperCase() + s.slice(1) }))} activeTab={shape} onChange={id => setShape(id as typeof shape)} />
        </div>
        <div className="grid grid-cols-3 gap-2">
          {(['X', 'Y', 'Z'] as const).map((a, i) => (
            <label key={a} className="space-y-1.5"><span className="text-xs text-muted">{a} (mm)</span>
              <input type="number" min={1} className={`${field} num`} value={dims[i]} onChange={e => setDims(d => d.map((v, j) => (j === i ? Number(e.target.value) : v)) as [number, number, number])} />
            </label>
          ))}
        </div>
        <div className="space-y-3 rounded-[12px] border border-line p-4">
          <p className="text-xs font-medium text-strong">First port</p>
          <div className="grid grid-cols-2 gap-3">
            <label className="space-y-1.5"><span className="text-xs text-muted">Name</span><input className={field} value={portName} onChange={e => setPortName(e.target.value)} /></label>
            <label className="space-y-1.5"><span className="text-xs text-muted">Unit</span><input className={field} value={portUnit} onChange={e => setPortUnit(e.target.value)} /></label>
            <label className="space-y-1.5"><span className="text-xs text-muted">Kind</span>
              <select className={field} value={portKind} onChange={e => setPortKind(e.target.value)}>
                <option value="electrical_dc">Electrical DC</option><option value="mechanical_torque">Mechanical torque</option><option value="telemetry_signal">Data signal</option><option value="fluid_port">Fluid</option>
              </select>
            </label>
            <label className="space-y-1.5"><span className="text-xs text-muted">Direction</span>
              <select className={field} value={portDir} onChange={e => setPortDir(e.target.value as typeof portDir)}>
                <option value="in">Input</option><option value="out">Output</option><option value="bi">Bidirectional</option>
              </select>
            </label>
          </div>
        </div>
        <div className="flex justify-end gap-2"><Button variant="ghost" onClick={onClose}>Cancel</Button><Button variant="primary" onClick={add}>Add part</Button></div>
      </div>
    </Drawer>
  );
};
