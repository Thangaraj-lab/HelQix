import React, { useEffect, useMemo, useRef, useState } from 'react';
import { GitBranch, Pause, Play, Snowflake, Square, TriangleAlert } from 'lucide-react';
import { useStudio } from '../../stores';
import { engine, engineMode } from '../../engine';
import { Checkpoint, ModelRevision } from '../../contracts';
import { EXAMPLE_SCENARIOS, MOCK_RUN_ID, RUNNABLE_SCENARIO } from '../../mock/scenarios';
import { violates, limitLabel } from '../../app/requirements';
import { setQuery } from '../../app/router';
import { Button, Chip, EmptyState, ExampleIfMock, ExampleTag, IconButton, MockTag, SectionLabel, Tabs } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';
import { LineChart } from '../../components/charts/LineChart';
import { useModelData } from '../studio/ModelContext';
import { runsChanged } from './useRun';
import { lumpedModel } from '../../app/lumped';
import { valueAtTime } from '../../scene/bindings';

export interface SimulateStageProps {
  onInvestigateFailure: () => void;
  onForkBranch?: () => void;
}

const SPEEDS = ['0.5', '1', '2', '4'] as const;
const pretty = (v: string) => v.replace(/_/g, ' ').replace(/^./, c => c.toUpperCase());
const VERBS: Record<string, string> = { rotation: 'spins with', color: 'heats with', emissive: 'glows with', vibration: 'shakes with', translation: 'slides with', visibility: 'shows with', scale: 'scales with', deformation: 'deforms with', flow: 'flows with' };

// Simulate (file 37 section 6.10). The 3D canvas behind this stage is driven by the same clock as the
// controls: every bound variable moves, heats or shakes its part at time t, and a breached
// requirement marks the part that broke it. The run is a fixed script from the mock engine, so every
// reading carries MOCK, Simulated and Unverified.

// Real engine: continue the frozen state with one changed number and play the result next to the original.
const BRANCH_FIELDS = [['referenceAreaM2', 'Reference area (m²)'], ['dragCoefficient', 'Drag coefficient'], ['massKg', 'Mass (kg)']] as const;
const BranchForm: React.FC<{ checkpoint: Checkpoint }> = ({ checkpoint }) => {
  const [field, setField] = useState<(typeof BRANCH_FIELDS)[number][0]>('referenceAreaM2');
  const [value, setValue] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const num = Number(value);
  const valid = value.trim() !== '' && Number.isFinite(num) && (field === 'massKg' ? num > 0 : num >= 0);
  const go = async () => {
    setBusy(true); setError(null);
    try {
      const { runId } = await engine.branch(checkpoint.id, { id: '', name: 'Branch', checkpointId: checkpoint.id, paramOverrides: { [field]: { v: num, unit: '', origin: 'user_stated', status: 'unverified' } } });
      runsChanged.dispatchEvent(new Event('change'));
      setQuery({ run: runId });
    } catch (e) { setError(e instanceof Error ? e.message : 'The branch could not be run.'); }
    setBusy(false);
  };
  return (
    <div className="space-y-2 pt-2 border-t border-line">
      <p className="text-[11px] text-muted">Change one number from the frozen point on. Everything before it stays the same.</p>
      <div className="flex items-end gap-2">
        <label className="block space-y-1 flex-1"><span className="text-[11px] text-muted">Change</span>
          <select value={field} onChange={e => setField(e.target.value as typeof field)} aria-label="Parameter to change" className="h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60">
            {BRANCH_FIELDS.map(([k, l]) => <option key={k} value={k}>{l}</option>)}
          </select></label>
        <label className="block space-y-1 w-24"><span className="text-[11px] text-muted">To</span>
          <input value={value} inputMode="decimal" onChange={e => setValue(e.target.value)} aria-label="New value" className="h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60" /></label>
      </div>
      {error && <p role="alert" className="text-xs text-bad">{error}</p>}
      <Button size="sm" variant="primary" disabled={!valid || busy} onClick={() => void go()}>Run the branch</Button>
    </div>
  );
};

// Real engine: starts the point-mass model with the numbers typed here (nothing is assumed) and plays the result.
const fmt = (x: number) => String(Number(x.toPrecision(5)));
const NewRunForm: React.FC<{ revision?: ModelRevision | null }> = ({ revision }) => {
  const lumped = useMemo(() => (revision ? lumpedModel(revision) : null), [revision]);
  const [v, setV] = useState({ mass: '0.5', drag: '0.47', area: '0.01', end: '2', height: '20', vx: '3' });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const n = (k: keyof typeof v) => Number(v[k]);
  const valid = Object.values(v).every(x => x.trim() !== '' && Number.isFinite(Number(x))) && n('mass') > 0 && n('end') > 0;
  const go = async () => {
    if (!engine.startPointMassRun || !valid) return;
    setBusy(true); setError(null);
    try {
      const { runId } = await engine.startPointMassRun({ massKg: n('mass'), dragCoefficient: n('drag'), referenceAreaM2: n('area'), endSeconds: n('end'), initialState: [0, n('height'), 0, n('vx'), 0, 0] });
      runsChanged.dispatchEvent(new Event('change'));
      setQuery({ run: runId });
    } catch (e) { setError(e instanceof Error ? e.message : 'The run could not be started.'); }
    setBusy(false);
  };
  const field = (k: keyof typeof v, label: string) => (
    <label key={k} className="block space-y-1"><span className="text-[11px] text-muted">{label}</span>
      <input value={v[k]} inputMode="decimal" onChange={e => setV({ ...v, [k]: e.target.value })} aria-label={label} className="h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60" /></label>
  );
  return (
    <div className="space-y-2.5">
      {lumped && lumped.ok === true && (
        <div className="space-y-1.5">
          <Button size="sm" variant="secondary" onClick={() => setV({ ...v, mass: fmt(lumped.massKg), area: fmt(lumped.areaM2) })}>Use my model's mass and area</Button>
          <p className="text-[11px] text-muted">Treats your {lumped.parts} parts as one body: {fmt(lumped.massKg)} kg from volume x density, {fmt(lumped.areaM2)} m² as the box footprint. Drag coefficient is yours to state.</p>
        </div>
      )}
      {lumped && lumped.ok === false && (
        <p className="text-[11px] text-muted">To run your own model, state a material density for: {lumped.missing.map(m => `${m.name} (${m.reason})`).join('; ')}.</p>
      )}
      <div className="grid grid-cols-2 gap-2">
        {field('mass', 'Mass (kg)')}{field('drag', 'Drag coefficient')}{field('area', 'Reference area (m²)')}{field('end', 'Duration (s)')}{field('height', 'Start height (m)')}{field('vx', 'Sideways speed (m/s)')}
      </div>
      {error && <p role="alert" className="text-xs text-bad">{error}</p>}
      <Button size="sm" variant="primary" disabled={!valid || busy} onClick={() => void go()}>Run the engine</Button>
      <p className="text-[11px] text-muted">A falling body with air drag, run by the engine and stored. The same numbers always give the same run.</p>
    </div>
  );
};

export const SimulateStage: React.FC<SimulateStageProps> = ({ onInvestigateFailure, onForkBranch }) => {
  const { playback } = useStudio();
  const model = useModelData();
  const { run, failure, bindings, series, requirements } = model;
  const [scenario, setScenario] = useState(RUNNABLE_SCENARIO);
  const [frozen, setFrozen] = useState<Checkpoint | null>(null);
  const [branching, setBranching] = useState(false);
  const started = useRef<string | null>(null);

  const duration = run?.duration ?? 12;
  const t = playback.t;

  // Start from the top, playing. Record one run per revision and session.
  useEffect(() => {
    if (!run || !model.revision) return;
    playback.setDuration(run.duration);
    playback.setT(0);
    playback.play();
    const key = `ran:${model.projectId}:${model.revision.id}`;
    if (engineMode === 'mock' && started.current !== key && !sessionStorage.getItem(key)) {
      started.current = key;
      sessionStorage.setItem(key, '1');
      void engine.run(model.projectId, scenario, { revisionNum: model.revision.revisionNum }).catch(() => undefined);
    }
    return () => playback.pause();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [run, model.revision?.id]);

  const variables = useMemo(() => Array.from(new Map(bindings.map(b => [b.variable, b])).values()), [bindings]);
  const at = (variable: string) => valueAtTime(series[variable], t);

  // Events come from the stored frames: a large step in a bound value, and each requirement breach.
  const events = useMemo(() => {
    const out: { t: number; text: string; tone: 'bad' | 'neutral' }[] = [{ t: 0, text: 'Run started', tone: 'neutral' }];
    for (const b of variables) {
      const pts = series[b.variable] ?? [];
      const [lo, hi] = b.range ?? [0, 1];
      if (b.channel === 'rotation') continue;
      for (let i = 1; i < pts.length; i++) {
        if (Math.abs(pts[i][1] - pts[i - 1][1]) > (hi - lo) * 0.25) out.push({ t: pts[i][0], text: `${pretty(b.variable)} stepped to ${pts[i][1]} ${b.units}`, tone: 'neutral' });
      }
    }
    if (failure) out.push({ t: failure.t, text: `${failure.requirement.id} broken: ${pretty(failure.requirement.variable)} passed ${limitLabel(failure.requirement)}`, tone: 'bad' });
    return out.sort((a, b) => a.t - b.t);
  }, [variables, series, failure]);

  if (!run) {
    return (
      <div className="relative z-10 m-auto max-w-md">
        {engineMode === 'real' && model.revision && model.bindings.length > 0 && !model.runChoice.runs?.length && engine.startPointMassRun ? (
          <PanelCard title="Run the engine"><NewRunForm revision={model.revision} /></PanelCard>
        ) : null}
        <EmptyState
          title={model.revision ? 'Nothing to simulate yet' : 'No committed model'}
          description={!model.revision ? 'Create model r1 in Define first.' : engineMode === 'real' && model.bindings.length > 0 ? (model.runChoice.runs?.length ? 'The selected stored run could not be read.' : 'The engine has no stored run to play. Record one with helqix-frames-record, or use the mock engine for a scripted demonstration.') : 'This model has no bindings, so no stored variable can move or colour a part. Add a binding in Define (select a part, then Add binding), then commit again.'}
        />
      </div>
    );
  }

  const failed = !!failure && t >= failure.t;
  const realRunId = model.runChoice.runId;
  const [freezeError, setFreezeError] = useState<string | null>(null);
  const freeze = async () => {
    playback.pause(); setFreezeError(null);
    try { setFrozen(await engine.freeze(engineMode === 'real' && realRunId ? realRunId : MOCK_RUN_ID, t)); }
    catch (e) { setFrozen(null); setFreezeError(e instanceof Error ? e.message : 'The state could not be frozen.'); }
  };

  const left = (
    <>
      {engineMode === 'real' ? (
        <>
        <PanelCard title="Stored run">
          <label className="block space-y-1.5">
            <span className="text-[11px] text-muted">Run to play</span>
            <select value={model.runChoice.runId ?? ''} onChange={e => setQuery({ run: e.target.value })} aria-label="Stored run" className="h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60">
              {(model.runChoice.runs ?? []).map(r => <option key={r.id} value={r.id}>{r.id} ({r.rows} frames)</option>)}
            </select>
          </label>
          <p className="text-[11px] text-muted">A recorded run, not generated from this model. Its variables drive the parts you bound to them.</p>
        </PanelCard>
        {engine.startPointMassRun && <PanelCard title="New run"><NewRunForm revision={model.revision} /></PanelCard>}
        </>
      ) : (
      <PanelCard title="Scenario" aside={<ExampleTag />}>
        <ul className="space-y-1">
          {EXAMPLE_SCENARIOS.map(s => {
            const runnable = s.id === RUNNABLE_SCENARIO;
            const on = scenario === s.id;
            return (
              <li key={s.id}>
                <button
                  onClick={() => runnable && setScenario(s.id)}
                  disabled={!runnable}
                  aria-pressed={on}
                  title={runnable ? s.note : 'No scripted run for this scenario yet'}
                  className={`w-full text-left h-9 px-3 rounded-[8px] border flex items-center justify-between gap-2 transition-colors ${on ? 'border-accent/45 bg-accent/[0.07]' : 'border-transparent'} ${runnable ? 'hover:bg-white/[0.04]' : 'opacity-40'}`}
                >
                  <span className="flex items-center gap-2 min-w-0"><span className="num text-[11px] text-muted">{s.num}</span><span className="text-[13px] text-strong truncate">{s.title}</span></span>
                  {s.tag && <Chip tone={s.tag === 'Current' ? 'accent' : 'neutral'} className="!h-5">{s.tag}</Chip>}
                </button>
              </li>
            );
          })}
        </ul>
        <p className="text-[11px] text-muted">Only the current scenario has a scripted run.</p>
      </PanelCard>
      )}

      <PanelCard title="Controls">
        <div className="flex items-center gap-1">
          <IconButton label={playback.isPlaying ? 'Pause' : 'Play'} onClick={() => { if (t >= duration) playback.setT(0); playback.togglePlay(); }} active>
            {playback.isPlaying ? <Pause size={15} /> : <Play size={15} />}
          </IconButton>
          <IconButton label="Stop and rewind" onClick={() => { playback.pause(); playback.setT(0); }}><Square size={14} /></IconButton>
          <Tabs tabs={SPEEDS.map(s => ({ id: s, label: `${s}x` }))} activeTab={SPEEDS.includes(String(playback.speed) as (typeof SPEEDS)[number]) ? String(playback.speed) : '1'} onChange={id => playback.setSpeed(Number(id))} className="ml-auto" />
        </div>
        <div className="space-y-1.5">
          <div className="flex items-baseline justify-between text-xs"><span className="text-muted">Time</span><span className="num text-strong">{t.toFixed(1)} s <span className="text-muted">of {duration.toFixed(0)} s</span></span></div>
          <div className="relative">
            <input type="range" min={0} max={duration} step={0.1} value={t} onChange={e => { playback.pause(); playback.setT(Number(e.target.value)); }} aria-label="Scrub time" />
            {events.map((e, i) => <span key={i} aria-hidden className={`absolute top-3 w-[3px] h-2 rounded-full ${e.tone === 'bad' ? 'bg-bad' : 'bg-muted/60'}`} style={{ left: `calc(${(e.t / duration) * 100}% - 1px)` }} />)}
          </div>
          <p className="text-[11px] text-muted pt-2">Scrubbing reads stored frames. The same time always shows the same picture, forwards or backwards.</p>
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="secondary" icon={<Snowflake size={13} />} onClick={freeze}>Freeze state</Button>
          <Button size="sm" variant="secondary" icon={<GitBranch size={13} />} disabled={engineMode === 'real' && !frozen} onClick={engineMode === 'real' ? () => setBranching(b => !b) : onForkBranch}>Branch</Button>
        </div>
        {freezeError && <p role="alert" className="text-xs text-bad">{freezeError}</p>}
        {frozen && <p className="text-[11px] text-trace">Frozen at <span className="num">{frozen.t.toFixed(3)} s</span>, hash <span className="num">{frozen.hash.slice(0, 16)}</span></p>}
        {engineMode === 'real' && branching && frozen && <BranchForm checkpoint={frozen} />}
      </PanelCard>

    </>
  );

  const right = (
    <>
      <PanelCard title="What moves what">
        <ul className="space-y-2">
          {bindings.map(b => (
            <li key={b.id} className="text-xs">
              <span className="text-strong">{model.revision?.parts[b.node]?.name.replace(/ \(Example\)$/, '') ?? b.node}</span>
              <span className="block text-muted">{VERBS[b.channel]} {pretty(b.variable).toLowerCase()}</span>
            </li>
          ))}
        </ul>
        <p className="text-[11px] text-muted">A part moves only because a stored variable is bound to it.</p>
      </PanelCard>
      <PanelCard title="Requirements" aside={<ExampleIfMock />}>
        {requirements.length === 0 ? <p className="text-xs text-muted">No requirement is defined for this model.</p> : (
          <ul className="space-y-2.5">
            {requirements.map(r => {
              const pts = series[r.variable] ?? [];
              const broken = pts.some(p => violates(r, p[1]) && p[0] <= t);
              const everBroken = pts.some(p => violates(r, p[1]));
              return (
                <li key={r.id} className="space-y-1">
                  <p className="text-[13px] text-strong"><span className="num text-muted mr-1.5">{r.id}</span>{r.text}</p>
                  <p className={`text-xs ${broken ? 'text-bad' : 'text-muted'}`}>{broken ? 'Broken so far in this run.' : everBroken ? 'Not broken yet. This run breaks it later.' : 'Not broken in this run. That is not verification.'}</p>
                </li>
              );
            })}
          </ul>
        )}
      </PanelCard>
      <PanelCard title="Events">
        <ol className="space-y-1.5 max-h-[170px] overflow-y-auto">
          {events.map((e, i) => (
            <li key={i}>
              <button onClick={() => { playback.pause(); playback.setT(e.t); }} className={`w-full text-left flex items-baseline gap-3 text-xs ${e.t <= t ? 'text-ink' : 'text-muted/60'}`}>
                <span className="num w-10 shrink-0 text-muted">{e.t.toFixed(1)} s</span>
                <span className={e.tone === 'bad' && e.t <= t ? 'text-bad' : ''}>{e.text}</span>
              </button>
            </li>
          ))}
        </ol>
        {engineMode === 'mock' ? <p className="text-[11px] text-muted">Run: scripted demonstration, seed 8421. <MockTag /></p> : <p className="text-[11px] text-muted">Run: <span className="num">{run.id}</span>, determinism {model.runChoice.runs?.find(r => r.id === run.id)?.determinism ?? 'unknown'}.</p>}
      </PanelCard>
    </>
  );

  const top = failed && failure ? (
    <div role="alert" className="surface-glass !border-bad/50 px-5 py-3 flex items-center gap-4 max-w-[560px] animate-rise">
      <TriangleAlert size={18} className="text-bad shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-[13px] font-medium text-strong">{failure.partName.replace(/ \(Example\)$/, '')} passed its limit at {failure.t.toFixed(2)} s</p>
        <p className="text-xs text-muted mt-0.5">{pretty(failure.requirement.variable)} reached <span className="num text-ink">{failure.value} {failure.requirement.unit}</span> against {limitLabel(failure.requirement)} ({failure.requirement.id}).{engineMode === 'mock' ? ' Mock result.' : ''}</p>
      </div>
      <Button variant="danger" size="sm" onClick={onInvestigateFailure}>Investigate failure</Button>
    </div>
  ) : (
    <div className="surface-glass px-4 py-2.5 flex flex-wrap items-center gap-x-6 gap-y-1.5 max-w-full" aria-label="Live readings">
      {variables.slice(0, 4).map(b => {
        const v = at(b.variable);
        return (
          <div key={b.variable} className="flex items-baseline gap-2 text-xs">
            <span className="text-muted">{pretty(b.variable)}</span>
            <span className="num text-[15px] text-strong">{v === null ? '-' : +v.toFixed(1)}</span>
            <span className="num text-muted">{b.units}</span>
          </div>
        );
      })}
      <span className="flex items-center gap-1.5 text-[11px] text-muted"><MockTag />Simulated, Unverified</span>
    </div>
  );

  const keyVar = failure?.requirement.variable ?? variables[0]?.variable;
  const footer = (
    <section className="surface-glass px-4 py-3 space-y-1" aria-label="Graph">
      <SectionLabel aside={<span className="text-[11px] text-muted">Drag to scrub</span>}>{keyVar ? pretty(keyVar) : 'Graph'}</SectionLabel>
      {keyVar && (
        <LineChart
          series={[{ id: 'k', label: pretty(keyVar), points: series[keyVar] ?? [], color: 'var(--color-trace-cyan)' }]}
          xDomain={[0, duration]}
          threshold={failure?.requirement.variable === keyVar ? { value: failure.requirement.limit, label: `${failure.requirement.id} limit ${limitLabel(failure.requirement)}` } : undefined}
          cursor={t}
          xUnit="s"
          yUnit={variables.find(b => b.variable === keyVar)?.units ?? ''}
          onScrub={x => { playback.pause(); playback.setT(+x.toFixed(1)); }}
          height={92}
        />
      )}
    </section>
  );

  return <StageGrid left={left} right={right} top={top} footer={footer} />;
};
