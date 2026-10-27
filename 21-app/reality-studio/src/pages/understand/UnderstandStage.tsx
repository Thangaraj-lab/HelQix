import React, { useEffect, useMemo, useState } from 'react';
import { CircleCheck, CircleX, TriangleAlert } from 'lucide-react';
import { limitLabel } from '../../app/requirements';
import { replayCheck, ReplayResult, short } from '../../app/evidence';
import { useStudio } from '../../stores';
import { Button, Chip, ExampleIfMock, MockTag, Reading } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';
import { valueAtTime } from '../../scene/bindings';
import { useModelData } from '../studio/ModelContext';

export interface UnderstandStageProps {
  onTestThisCause: () => void;
  onBackToSimulate?: () => void;
}

interface ChainNode {
  id: string;
  title: string;
  partId: string | null;
  partName: string;
  variable: string;
  unit: string;
  before: number;
  atFailure: number;
  at: number;
  candidate?: boolean;
  isBreach?: boolean;
}

const pretty = (v: string) => v.replace(/_/g, ' ').replace(/^./, c => c.toUpperCase());
const clean = (n: string) => n.replace(/ \(Example\)$/, '');

// Understand (file 37 section 6.11). The candidate chain is built from the stored run: which bound
// variables changed before the requirement broke, and in what order. Selecting a step flies the 3D
// camera to the part it is bound to. The chain states order in time and nothing about cause: a cause
// is a hypothesis until a branch tests it.
export const UnderstandStage: React.FC<UnderstandStageProps> = ({ onTestThisCause, onBackToSimulate }) => {
  const model = useModelData();
  const { run, failure, bindings, series } = model;
  const { selection } = useStudio();
  const [replay, setReplay] = useState<ReplayResult | null>(null);

  useEffect(() => {
    if (!run) return;
    let live = true;
    replayCheck(run.frames).then(r => live && setReplay(r));
    return () => { live = false; };
  }, [run]);

  const chain = useMemo<ChainNode[]>(() => {
    if (!run || !failure) return [];
    const nodes: ChainNode[] = [];
    for (const b of bindings) {
      if (b.channel === 'rotation' || nodes.some(n => n.variable === b.variable)) continue;
      const pts = series[b.variable] ?? [];
      if (pts.length < 2) continue;
      const [lo, hi] = b.range ?? [pts[0][1], pts[pts.length - 1][1]];
      const span = hi - lo || 1;
      // An abrupt step (one sample jumping by a quarter of the range) marks when something was imposed;
      // a smooth variable is dated by when it first drifted 15% of the range.
      let jumpAt: number | null = null;
      for (let i = 1; i < pts.length; i++) if (Math.abs(pts[i][1] - pts[i - 1][1]) > span * 0.25) { jumpAt = pts[i][0]; break; }
      const drift = pts.find(p => Math.abs(p[1] - pts[0][1]) > span * 0.15);
      const onset = jumpAt ?? drift?.[0] ?? null;
      if (onset === null) continue;
      nodes.push({
        id: b.variable,
        title: jumpAt !== null ? `${pretty(b.variable)} steps` : `${pretty(b.variable)} drifts`,
        partId: b.node,
        partName: clean(model.revision?.parts[b.node]?.name ?? b.node),
        variable: pretty(b.variable),
        unit: b.units,
        before: valueAtTime(pts, Math.max(0, onset - 0.1)) ?? pts[0][1],
        atFailure: valueAtTime(pts, failure.t) ?? 0,
        at: onset,
        candidate: jumpAt !== null
      });
    }
    // Imposed changes (steps) come first, then what drifted as a consequence, each in time order.
    nodes.sort((a, b) => Number(b.title.endsWith('steps')) - Number(a.title.endsWith('steps')) || a.at - b.at);
    const firstStep = nodes.findIndex(n => n.candidate);
    nodes.forEach((n, i) => { n.candidate = i === (firstStep >= 0 ? firstStep : 0); });
    nodes.push({
      id: 'breach',
      title: `${failure.requirement.id} is broken`,
      partId: failure.partId,
      partName: clean(failure.partName),
      variable: pretty(failure.requirement.variable),
      unit: failure.requirement.unit,
      before: failure.requirement.limit,
      atFailure: failure.value,
      at: failure.t,
      isBreach: true
    });
    return nodes;
  }, [run, failure, bindings, series, model.revision]);

  // Selecting a step selects its part in the 3D view (the camera follows).
  const selectedPart = selection.selectedIds[0] ?? null;
  const node = chain.find(n => n.partId && n.partId === selectedPart) ?? chain[0];
  const choose = (n: ChainNode) => (n.partId ? selection.select(n.partId) : selection.deselectAll());
  useEffect(() => { if (chain[0]?.partId && !selection.selectedIds[0]) selection.select(chain[0].partId); }, [chain.length]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!run || !failure) {
    return (
      <StageGrid
        top={
          <div className="surface-glass px-5 py-3 text-[13px] text-ink flex items-center gap-3">
            {run ? 'Nothing to investigate: this run did not break a requirement.' : 'Nothing to investigate: there is no run yet.'}
            {onBackToSimulate && <Button size="sm" variant="ghost" onClick={onBackToSimulate}>Back to Simulate</Button>}
          </div>
        }
      />
    );
  }

  const left = (
    <>
      <PanelCard title="Failures" aside={<MockTag />}>
        <div className="rounded-[10px] border border-bad/40 bg-bad/[0.07] p-3.5 space-y-1.5">
          <div className="flex items-center justify-between gap-2"><span className="text-[13px] font-medium text-strong">{failure.requirement.id} broken</span><span className="num text-xs text-bad">{failure.t.toFixed(2)} s</span></div>
          <p className="text-xs text-ink">{failure.requirement.text}. {clean(failure.partName)} reached <span className="num">{failure.value} {failure.requirement.unit}</span> against {limitLabel(failure.requirement)}.</p>
        </div>
      </PanelCard>
      <PanelCard title="Observations">
        <ul className="space-y-2 text-xs">
          {chain.filter(n => !n.isBreach).map(n => (
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
      <PanelCard title="Candidate chain" aside={<Chip>Structural and temporal</Chip>}>
        <ol className="space-y-1.5">
          {chain.map((n, i) => {
            const on = node?.id === n.id;
            return (
              <li key={n.id}>
                <button onClick={() => choose(n)} aria-pressed={on} className={`w-full text-left flex items-center gap-3 px-3 py-2.5 rounded-[10px] border transition-colors ${on ? 'border-accent/45 bg-accent/[0.07]' : 'border-line bg-white/[0.02] hover:border-line-strong'}`}>
                  <span className={`num w-5 h-5 rounded-full inline-flex items-center justify-center text-[10px] shrink-0 ${n.isBreach ? 'bg-bad/20 text-bad' : 'border border-line-strong text-muted'}`}>{i + 1}</span>
                  <span className="min-w-0">
                    <span className="block text-[13px] text-strong truncate">{n.title}</span>
                    <span className="block text-[11px] text-muted truncate">{n.partName}{n.candidate ? ' · candidate cause' : ''}</span>
                  </span>
                </button>
              </li>
            );
          })}
        </ol>
        <p className="text-[11px] text-muted">The order is what the run shows. Whether one step causes the next is not tested. Pick a step to see its part in 3D.</p>
      </PanelCard>

      {node && (
        <PanelCard title="Node detail" aside={<ExampleIfMock />}>
          <p className="text-[13px] font-medium text-strong">{node.title}</p>
          <div className="grid grid-cols-2 gap-4">
            <Reading label={node.isBreach ? 'Limit' : 'Before'} value={+node.before.toFixed(1)} unit={node.unit} mock />
            <Reading label={node.isBreach ? 'Reached' : 'At failure'} value={+node.atFailure.toFixed(1)} unit={node.unit} tone="warn" mock />
          </div>
          <p className="text-xs text-muted">{node.variable}, on {node.partName}. Follows the earlier steps in time within this run. Origin Simulated, status Unverified.</p>
          {node.candidate && <Button variant="secondary" className="w-full" onClick={onTestThisCause}>Test this cause</Button>}
        </PanelCard>
      )}
    </>
  );

  const top = (
    <div role="alert" className="surface-glass !border-bad/45 px-5 py-3 flex items-center gap-3 max-w-full animate-rise">
      <TriangleAlert size={17} className="text-bad shrink-0" />
      <p className="text-[13px] text-strong">{clean(failure.partName)} passed its limit at <span className="num">{failure.t.toFixed(2)} s</span></p>
      {onBackToSimulate && <Button size="sm" variant="ghost" onClick={onBackToSimulate}>Back to Simulate</Button>}
    </div>
  );

  return <StageGrid left={left} right={right} top={top} />;
};
