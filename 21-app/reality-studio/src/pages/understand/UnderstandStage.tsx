import React, { useEffect, useMemo, useState } from 'react';
import { CircleCheck, CircleX, TriangleAlert } from 'lucide-react';
import { EXAMPLE_REQUIREMENT } from '../../mock/scenarios';
import { replayCheck, ReplayResult, short } from '../../app/evidence';
import { Button, Chip, ExampleTag, MockTag, Reading } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';
import { valueAt } from '../../components/charts/LineChart';
import { useRun } from '../simulate/useRun';

export interface UnderstandStageProps {
  onTestThisCause: () => void;
  onBackToSimulate?: () => void;
}

interface ChainNode {
  id: string;
  title: string;
  variable: string;
  unit: string;
  before: number;
  atFailure: number;
  at: number;
  candidate?: boolean;
}

// Understand (file 37 section 6.11). The candidate chain is built from the stored frames: which
// values changed, in what order, before the requirement was violated. It states order in time and
// nothing about cause: a cause is a hypothesis until a branch tests it.
export const UnderstandStage: React.FC<UnderstandStageProps> = ({ onTestThisCause, onBackToSimulate }) => {
  const run = useRun();
  const [selected, setSelected] = useState('load');
  const [replay, setReplay] = useState<ReplayResult | null>(null);

  useEffect(() => {
    if (!run) return;
    let live = true;
    replayCheck(run.frames).then(r => live && setReplay(r));
    return () => { live = false; };
  }, [run]);

  const chain = useMemo<ChainNode[]>(() => {
    if (!run || run.failureT == null) return [];
    const torque = run.series('shaft_torque_load');
    const current = run.series('phase_rms_current');
    const temp = run.series('stator_winding_temperature');
    const step = torque.find((p, i) => i > 0 && Math.abs(p[1] - torque[i - 1][1]) > 20)?.[0] ?? 0;
    const f = run.failureT;
    return [
      { id: 'load', title: 'Load rises', variable: 'Output torque', unit: 'N·m', before: valueAt(torque, step - 0.1) ?? 0, atFailure: valueAt(torque, f) ?? 0, at: step, candidate: true },
      { id: 'current', title: 'Phase current rises', variable: 'Phase current', unit: 'A rms', before: valueAt(current, step - 0.1) ?? 0, atFailure: valueAt(current, f) ?? 0, at: step },
      { id: 'temp', title: 'Winding temperature climbs', variable: 'Winding temperature', unit: '°C', before: valueAt(temp, step) ?? 0, atFailure: valueAt(temp, f) ?? 0, at: f },
      { id: 'limit', title: `${EXAMPLE_REQUIREMENT.id} is violated`, variable: 'Winding temperature', unit: '°C', before: EXAMPLE_REQUIREMENT.limit, atFailure: valueAt(temp, f) ?? 0, at: f }
    ];
  }, [run]);

  const node = chain.find(n => n.id === selected) ?? chain[0];

  if (!run || run.failureT == null) {
    return (
      <StageGrid
        top={<div className="surface-glass px-5 py-3 text-[13px] text-ink">Nothing to investigate: this run did not violate {EXAMPLE_REQUIREMENT.id}.{onBackToSimulate && <Button size="sm" variant="ghost" className="ml-2" onClick={onBackToSimulate}>Back to Simulate</Button>}</div>}
      />
    );
  }

  const left = (
    <>
      <PanelCard title="Failures" aside={<MockTag />}>
        <div className="rounded-[10px] border border-bad/40 bg-bad/[0.07] p-3.5 space-y-1.5">
          <div className="flex items-center justify-between gap-2"><span className="text-[13px] font-medium text-strong">{EXAMPLE_REQUIREMENT.id} violated</span><span className="num text-xs text-bad">{run.failureT.toFixed(2)} s</span></div>
          <p className="text-xs text-ink">{EXAMPLE_REQUIREMENT.text}. Reached <span className="num">{chain[2]?.atFailure} °C</span>.</p>
        </div>
      </PanelCard>
      <PanelCard title="Observations">
        <ul className="space-y-2 text-xs">
          {chain.slice(0, 3).map(n => (
            <li key={n.id} className="flex items-baseline gap-3"><span className="num w-10 shrink-0 text-muted">{n.at.toFixed(1)} s</span><span className="text-ink">{n.title}</span></li>
          ))}
        </ul>
      </PanelCard>
      <PanelCard title="Replay check">
        {replay === null ? (
          <p className="text-xs text-muted">Re-reading every frame.</p>
        ) : replay.matches ? (
          <div className="space-y-1">
            <p className="text-[13px] text-ok flex items-center gap-1.5"><CircleCheck size={14} />Replay matches original</p>
            <p className="text-[11px] text-muted"><span className="num">{replay.frames}</span> frames, SHA-256 <span className="num text-ink">{short(replay.replayHash)}</span></p>
          </div>
        ) : (
          <p className="text-[13px] text-bad flex items-center gap-1.5"><CircleX size={14} />Replay differs from the original</p>
        )}
        <p className="text-[11px] text-muted">Checked in this browser over the mock frames. The signed replay check lives in the evidence service.</p>
      </PanelCard>
    </>
  );

  const right = (
    <>
      <PanelCard title="Candidate chain" aside={<Chip tone="neutral">Structural and temporal</Chip>}>
        <ol className="relative space-y-1.5">
          {chain.map((n, i) => {
            const on = selected === n.id;
            return (
              <li key={n.id} className="relative">
                {i < chain.length - 1 && <span aria-hidden className="absolute left-[19px] top-[40px] h-[10px] w-px bg-line-strong" />}
                <button onClick={() => setSelected(n.id)} aria-pressed={on} className={`w-full text-left flex items-center gap-3 px-3 py-2.5 rounded-[10px] border transition-colors ${on ? 'border-accent/45 bg-accent/[0.07]' : 'border-line bg-white/[0.02] hover:border-line-strong'}`}>
                  <span className={`num w-5 h-5 rounded-full inline-flex items-center justify-center text-[10px] shrink-0 ${i === chain.length - 1 ? 'bg-bad/20 text-bad' : 'border border-line-strong text-muted'}`}>{i + 1}</span>
                  <span className="min-w-0">
                    <span className="block text-[13px] text-strong truncate">{n.title}</span>
                    {n.candidate && <span className="block text-[11px] text-warn">Candidate cause</span>}
                  </span>
                </button>
              </li>
            );
          })}
        </ol>
        <p className="text-[11px] text-muted">The order is what the run shows. Whether one step causes the next is not tested.</p>
      </PanelCard>

      {node && (
        <PanelCard title="Node detail" aside={<ExampleTag />}>
          <p className="text-[13px] font-medium text-strong">{node.title}</p>
          <div className="grid grid-cols-2 gap-4">
            <Reading label="Before" value={+node.before.toFixed(1)} unit={node.unit} mock />
            <Reading label="At failure" value={+node.atFailure.toFixed(1)} unit={node.unit} tone="warn" mock />
          </div>
          <p className="text-xs text-muted">{node.variable}. Follows the previous step in time within this run. Origin Simulated, status Unverified.</p>
          {node.candidate && <Button variant="secondary" className="w-full" onClick={onTestThisCause}>Test this cause</Button>}
        </PanelCard>
      )}
    </>
  );

  const top = (
    <div role="alert" className="surface-glass !border-bad/45 px-5 py-3 flex items-center gap-3 max-w-full animate-rise">
      <TriangleAlert size={17} className="text-bad shrink-0" />
      <p className="text-[13px] text-strong">Winding temperature above limit at <span className="num">{run.failureT.toFixed(2)} s</span></p>
      {onBackToSimulate && <Button size="sm" variant="ghost" onClick={onBackToSimulate}>Back to Simulate</Button>}
    </div>
  );

  return <StageGrid left={left} right={right} top={top} />;
};
