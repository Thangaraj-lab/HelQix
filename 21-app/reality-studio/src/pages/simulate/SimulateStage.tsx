import React, { useEffect, useMemo, useState } from 'react';
import { GitBranch, Pause, Play, Snowflake, Square, TriangleAlert } from 'lucide-react';
import { useStudio } from '../../stores';
import { engine } from '../../engine';
import { Checkpoint } from '../../contracts';
import { EXAMPLE_REQUIREMENT, EXAMPLE_SCENARIOS, MOCK_RUN_ID, RUNNABLE_SCENARIO } from '../../mock/scenarios';
import { Button, Chip, ExampleTag, IconButton, MockTag, PreviewTag, Reading, SectionLabel, Tabs } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';
import { LineChart, Sparkline } from '../../components/charts/LineChart';
import { useRun } from './useRun';

export interface SimulateStageProps {
  onInvestigateFailure: () => void;
  onForkBranch?: () => void;
}

const SPEEDS = ['0.5', '1', '2', '4'] as const;
const LIMIT = EXAMPLE_REQUIREMENT.limit;

// Simulate (file 37 section 6.10): scenarios and controls on the left, the readings over the canvas,
// the environment and parameters on the right, telemetry, graph and events below. The run is a fixed
// script from the mock engine, so every reading carries MOCK, Simulated and Unverified.
export const SimulateStage: React.FC<SimulateStageProps> = ({ onInvestigateFailure, onForkBranch }) => {
  const { session } = useStudio();
  const run = useRun();
  const [t, setT] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [speed, setSpeed] = useState<(typeof SPEEDS)[number]>('1');
  const [scenario, setScenario] = useState(RUNNABLE_SCENARIO);
  const [frozen, setFrozen] = useState<Checkpoint | null>(null);

  const duration = run?.duration ?? 12;
  useEffect(() => {
    if (!playing || !run) return;
    const id = setInterval(() => {
      setT(prev => {
        const next = +(prev + 0.1 * Number(speed)).toFixed(2);
        if (next >= duration) { setPlaying(false); return duration; }
        return next;
      });
    }, 100);
    return () => clearInterval(id);
  }, [playing, speed, duration, run]);

  const frame = run?.at(t) ?? null;
  const failed = run?.failureT != null && t >= run.failureT;
  const temp = frame?.values[EXAMPLE_REQUIREMENT.variable];

  const series = useMemo(() => ({
    temp: run?.series('stator_winding_temperature') ?? [],
    speed: run?.series('rotor_angular_velocity') ?? [],
    current: run?.series('phase_rms_current') ?? [],
    torque: run?.series('shaft_torque_load') ?? []
  }), [run]);

  // Events come from the stored frames: a step in a value, and the first crossing of the limit.
  const events = useMemo(() => {
    if (!run) return [] as { t: number; text: string; tone: 'bad' | 'neutral' }[];
    const out: { t: number; text: string; tone: 'bad' | 'neutral' }[] = [{ t: 0, text: 'Run started', tone: 'neutral' }];
    const cur = series.torque;
    for (let i = 1; i < cur.length; i++) {
      if (Math.abs(cur[i][1] - cur[i - 1][1]) > 20) out.push({ t: cur[i][0], text: `Load changed to ${cur[i][1]} N·m`, tone: 'neutral' });
    }
    if (run.failureT != null) out.push({ t: run.failureT, text: `Winding temperature reached ${LIMIT} ${EXAMPLE_REQUIREMENT.unit}, the limit of ${EXAMPLE_REQUIREMENT.id}`, tone: 'bad' });
    return out.sort((a, b) => a.t - b.t);
  }, [run, series.torque]);

  const freeze = async () => {
    setPlaying(false);
    setFrozen(await engine.freeze(MOCK_RUN_ID, t));
  };

  const left = (
    <>
      <PanelCard title="Scenarios" aside={<ExampleTag />}>
        <ul className="space-y-1.5">
          {EXAMPLE_SCENARIOS.map(s => {
            const runnable = s.id === RUNNABLE_SCENARIO;
            const on = scenario === s.id;
            return (
              <li key={s.id}>
                <button
                  onClick={() => runnable && setScenario(s.id)}
                  disabled={!runnable}
                  aria-pressed={on}
                  title={runnable ? undefined : 'No scripted run for this scenario'}
                  className={`w-full text-left px-3 py-2 rounded-[10px] border transition-colors ${on ? 'border-accent/45 bg-accent/[0.07]' : 'border-line bg-white/[0.02]'} ${runnable ? 'hover:border-line-strong' : 'opacity-45'}`}
                >
                  <span className="flex items-center justify-between gap-2">
                    <span className="flex items-center gap-2 min-w-0"><span className="num text-[11px] text-muted">{s.num}</span><span className="text-[13px] text-strong truncate">{s.title}</span></span>
                    {s.tag && <Chip tone={s.tag === 'Current' ? 'accent' : 'neutral'} className="!h-5">{s.tag}</Chip>}
                  </span>
                  <span className="block text-[11px] text-muted mt-0.5">{runnable ? `${s.duration} · ${s.note}` : 'No scripted run yet'}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </PanelCard>

      <PanelCard title="Controls">
        <div className="flex items-center gap-1">
          <IconButton label={playing ? 'Pause' : 'Play'} onClick={() => { if (t >= duration) setT(0); setPlaying(p => !p); }} active>
            {playing ? <Pause size={15} /> : <Play size={15} />}
          </IconButton>
          <IconButton label="Stop and rewind" onClick={() => { setPlaying(false); setT(0); }}><Square size={14} /></IconButton>
          <Tabs tabs={SPEEDS.map(s => ({ id: s, label: `${s}x` }))} activeTab={speed} onChange={id => setSpeed(id as (typeof SPEEDS)[number])} className="ml-auto" />
        </div>
        <div className="space-y-1.5">
          <div className="flex items-baseline justify-between text-xs"><span className="text-muted">Time</span><span className="num text-strong">{t.toFixed(1)} s <span className="text-muted">of {duration.toFixed(0)} s</span></span></div>
          <input type="range" min={0} max={duration} step={0.1} value={t} onChange={e => { setPlaying(false); setT(Number(e.target.value)); }} aria-label="Scrub time" />
          <p className="text-[11px] text-muted">Scrubbing reads stored frames. The same time always shows the same values.</p>
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="secondary" icon={<Snowflake size={13} />} onClick={freeze}>Freeze state</Button>
          <Button size="sm" variant="secondary" icon={<GitBranch size={13} />} onClick={onForkBranch}>Branch</Button>
        </div>
        {frozen && <p className="text-[11px] text-trace">Frozen at <span className="num">{frozen.t.toFixed(1)} s</span>, hash <span className="num">{frozen.hash}</span></p>}
      </PanelCard>

      <PanelCard title="Run settings" aside={<MockTag />}>
        <dl className="space-y-1.5 text-xs">
          {[['Solver', 'Scripted demonstration'], ['Seed', '8421'], ['Duration', `${duration.toFixed(0)} s`], ['Determinism', 'Same frames for the same time']].map(([k, v]) => (
            <div key={k} className="flex justify-between gap-3"><dt className="text-muted">{k}</dt><dd className="text-ink text-right">{v}</dd></div>
          ))}
        </dl>
      </PanelCard>
    </>
  );

  const right = (
    <>
      <PanelCard title="Environment" aside={<PreviewTag />}>
        <dl className="space-y-1.5 text-xs">
          <div className="flex justify-between"><dt className="text-muted">Ambient temperature</dt><dd className="num text-ink">25.0 °C</dd></div>
        </dl>
        <p className="text-[11px] text-muted">Only fields this scenario supports are shown. Editing them arrives with the real solver.</p>
      </PanelCard>
      <PanelCard title="Scenario parameters" aside={<PreviewTag />}>
        <dl className="space-y-1.5 text-xs">
          <div className="flex justify-between"><dt className="text-muted">Peak load torque</dt><dd className="num text-ink">85.4 N·m</dd></div>
          <div className="flex justify-between"><dt className="text-muted">Load step at</dt><dd className="num text-ink">4.2 s</dd></div>
        </dl>
        <p className="text-[11px] text-muted">A changed parameter will show a Draft preview first, and results change only after a re-run.</p>
      </PanelCard>
      <PanelCard title="Requirement" aside={<ExampleTag />}>
        <p className="text-[13px] text-strong"><span className="num text-muted mr-1.5">{EXAMPLE_REQUIREMENT.id}</span>{EXAMPLE_REQUIREMENT.text}</p>
        {run?.failureT != null ? (
          <p className={`text-xs ${failed ? 'text-bad' : 'text-muted'}`}>{failed ? `Violated at ${run.failureT.toFixed(2)} s in this run.` : `Not violated yet. This run crosses the limit at ${run.failureT.toFixed(2)} s.`}</p>
        ) : (
          <p className="text-xs text-muted">Not violated in this run. This is not verification.</p>
        )}
      </PanelCard>
    </>
  );

  const top = (
    <div className="surface-glass px-4 py-3 flex flex-wrap items-start gap-x-6 gap-y-3 max-w-full" aria-label="Live readings">
      {([
        ['Winding temperature', temp?.v ?? '-', '°C', series.temp, 'var(--status-critical)', failed ? 'bad' : 'default'],
        ['Rotor speed', frame?.values.rotor_angular_velocity?.v ?? '-', 'rad/s', series.speed, 'var(--color-primary-blue)', 'default'],
        ['Phase current', frame?.values.phase_rms_current?.v ?? '-', 'A rms', series.current, 'var(--color-trace-cyan)', 'default'],
        ['Output torque', frame?.values.shaft_torque_load?.v ?? '-', 'N·m', series.torque, 'var(--color-assistant-violet)', 'default']
      ] as const).map(([label, value, unit, pts, color, tone]) => (
        <div key={label} className="w-[128px]">
          <Reading label={label} value={value} unit={unit} tone={tone} mock />
          <Sparkline points={pts as [number, number][]} color={color} cursor={t} className="mt-1" />
        </div>
      ))}
      <div className="flex flex-col items-end gap-2 ml-auto">
        <Tabs tabs={(['Perspective', 'Top', 'Front', 'Right'] as const).map(p => ({ id: p, label: p }))} activeTab={session.cameraPreset} onChange={id => session.setCameraPreset(id as 'Perspective')} />
        <Chip tone="warn" className="!h-5">No visual binding: the canvas is not driven by this run</Chip>
      </div>
    </div>
  );

  const bottom = (
    <div className="space-y-3">
      {failed && run?.failureT != null && (
        <div role="alert" className="mx-auto max-w-xl surface-glass !border-bad/50 px-5 py-3.5 flex items-center gap-4 animate-rise">
          <TriangleAlert size={18} className="text-bad shrink-0" />
          <div className="flex-1">
            <p className="text-[13px] font-medium text-strong">Winding temperature above limit at {run.failureT.toFixed(2)} s</p>
            <p className="text-xs text-muted mt-0.5">Reached <span className="num text-ink">{temp?.v} °C</span> against the {LIMIT} °C limit of {EXAMPLE_REQUIREMENT.id} (example). Mock result.</p>
          </div>
          <Button variant="danger" size="sm" onClick={onInvestigateFailure}>Investigate failure</Button>
        </div>
      )}
      <div className="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-3">
        <section className="surface-glass p-4 space-y-2" aria-label="Winding temperature graph">
          <SectionLabel aside={<span className="text-[11px] text-muted">Drag to scrub</span>}>Winding temperature</SectionLabel>
          <LineChart
            series={[{ id: 'temp', label: 'Winding temperature', points: series.temp, color: 'var(--color-trace-cyan)' }]}
            xDomain={[0, duration]}
            threshold={{ value: LIMIT, label: `${EXAMPLE_REQUIREMENT.id} limit ${LIMIT} °C` }}
            cursor={t}
            xUnit="s"
            yUnit="°C"
            onScrub={x => { setPlaying(false); setT(+x.toFixed(1)); }}
            height={128}
          />
        </section>

        <section className="surface-glass p-4 space-y-2" aria-label="Event stream">
          <SectionLabel>Events</SectionLabel>
          <ol className="space-y-1.5 max-h-[150px] overflow-y-auto">
            {events.map((e, i) => (
              <li key={i}>
                <button onClick={() => { setPlaying(false); setT(e.t); }} className={`w-full text-left flex items-baseline gap-3 text-xs ${e.t <= t ? 'text-ink' : 'text-muted/60'}`}>
                  <span className="num w-12 shrink-0 text-muted">{e.t.toFixed(1)} s</span>
                  <span className={e.tone === 'bad' && e.t <= t ? 'text-bad' : ''}>{e.text}</span>
                </button>
              </li>
            ))}
          </ol>
        </section>
      </div>
    </div>
  );

  return <StageGrid left={left} right={right} top={top} footer={bottom} />;
};
