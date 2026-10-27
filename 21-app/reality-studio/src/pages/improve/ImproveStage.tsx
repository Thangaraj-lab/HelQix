import React, { useMemo } from 'react';
import { Check, TriangleAlert } from 'lucide-react';
import { BranchParams, scriptedBranchTemperature } from '../../mock/branches';
import { limitLabel, violates } from '../../app/requirements';
import { Chip, EmptyState, ExampleTag, MockTag, Reading, SectionLabel, SliderWithUnit } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';
import { LineChart } from '../../components/charts/LineChart';
import { useModelData } from '../studio/ModelContext';

export interface ImproveStageProps {
  branch: BranchParams;
  onBranchChange: (p: BranchParams) => void;
}

const pretty = (v: string) => v.replace(/_/g, ' ').replace(/^./, c => c.toUpperCase());
const peak = (pts: [number, number][]) => pts.reduce((m, p) => Math.max(m, p[1]), -Infinity);

// Improve (file 37 section 6.12): the original and one intervention branch side by side in 3D, each
// part heated by its own series. The branch is a scripted demonstration (src/mock/branches.ts), so
// every number is MOCK. The verdict always states both facts: what the model shows, and that it is
// not validated.
export const ImproveStage: React.FC<ImproveStageProps> = ({ branch, onBranchChange }) => {
  const model = useModelData();
  const req = model.requirements[0];
  const original = useMemo(() => (req ? model.series[req.variable] ?? [] : []), [model.series, req]);
  const scripted = useMemo(() => scriptedBranchTemperature(original, branch), [original, branch]);

  if (!model.run || !req || original.length === 0) {
    return (
      <div className="relative z-10 m-auto max-w-md">
        <EmptyState title="Nothing to compare yet" description="A branch needs a run and a requirement to measure against. Simulate a committed model that has both." />
      </div>
    );
  }

  const first = (pts: [number, number][]) => pts.find(p => violates(req, p[1]))?.[0] ?? null;
  const oPeak = peak(original);
  const bPeak = peak(scripted);
  const oCross = first(original);
  const bCross = first(scripted);
  const meets = bCross === null;
  const unit = req.unit;

  const left = (
    <>
      <PanelCard title="Branches" aside={<ExampleTag />}>
        <ul className="space-y-1.5">
          <li className="flex items-center justify-between rounded-[10px] border border-line bg-white/[0.02] px-3 py-2"><span className="text-[13px] text-strong"><span className="num text-muted mr-2">R0</span>Original</span><Chip tone={oCross != null ? 'bad' : 'ok'}>{oCross != null ? 'Breaks' : 'Meets'} {req.id}</Chip></li>
          <li className="flex items-center justify-between rounded-[10px] border border-accent/45 bg-accent/[0.07] px-3 py-2"><span className="text-[13px] text-strong"><span className="num text-muted mr-2">B1</span>Branch</span><Chip tone={meets ? 'ok' : 'bad'}>{meets ? 'Meets' : 'Breaks'} {req.id}</Chip></li>
        </ul>
      </PanelCard>
      <PanelCard title="Interventions" aside={<MockTag />}>
        <SliderWithUnit label="Added cooling" value={branch.coolingPct} unit="%" min={0} max={40} step={5} onChange={v => onBranchChange({ ...branch, coolingPct: v })} />
        <SliderWithUnit label="Load change" value={branch.loadPct} unit="%" min={-30} max={0} step={5} onChange={v => onBranchChange({ ...branch, loadPct: v })} />
        <p className="text-[11px] text-muted">Forked from the start of the original run. The branch is a scripted scaling of the original curve, not a solver result. Watch the two models in 3D: the branch part runs cooler.</p>
      </PanelCard>
    </>
  );

  const right = (
    <>
      <PanelCard title="Metrics" aside={<MockTag />}>
        <div className="grid grid-cols-2 gap-4">
          <Reading label="Peak, original" value={+oPeak.toFixed(1)} unit={unit} tone="bad" mock />
          <Reading label="Peak, branch" value={+bPeak.toFixed(1)} unit={unit} tone={meets ? 'default' : 'bad'} mock />
          <Reading label="Limit passed, original" value={oCross != null ? oCross.toFixed(2) : 'Never'} unit={oCross != null ? 's' : ''} mock />
          <Reading label="Limit passed, branch" value={bCross != null ? bCross.toFixed(2) : 'Never'} unit={bCross != null ? 's' : ''} mock />
        </div>
        <p className="num text-xs text-muted">Peak change: {(bPeak - oPeak).toFixed(1)} {unit}</p>
      </PanelCard>
      <PanelCard title="Verdict">
        <div className={`rounded-[10px] border p-3.5 space-y-1.5 ${meets ? 'border-ok/35 bg-ok/[0.06]' : 'border-bad/40 bg-bad/[0.06]'}`}>
          <p className={`text-[13px] font-medium flex items-center gap-1.5 ${meets ? 'text-ok' : 'text-bad'}`}>{meets ? <Check size={14} /> : <TriangleAlert size={14} />}{meets ? `Meets ${req.id} in the model.` : `Does not meet ${req.id} in the model.`}</p>
          <p className="text-[13px] text-strong">Not yet validated.</p>
          <p className="text-[11px] text-muted">Validation needs reference data. See Verify.</p>
        </div>
      </PanelCard>
    </>
  );

  const footer = (
    <section className="surface-glass p-4 space-y-2" aria-label="Overlay curves">
      <SectionLabel aside={<span className="flex items-center gap-3 text-[11px] text-muted"><span className="inline-flex items-center gap-1.5"><i className="w-3 h-0.5 bg-bad inline-block" />Original</span><span className="inline-flex items-center gap-1.5"><i className="w-3 h-0.5 inline-block" style={{ background: 'var(--color-trace-cyan)' }} />Branch</span></span>}>{pretty(req.variable)}</SectionLabel>
      <LineChart
        series={[
          { id: 'o', label: 'Original', points: original, color: 'var(--status-critical)' },
          { id: 'b', label: 'Branch', points: scripted, color: 'var(--color-trace-cyan)' }
        ]}
        xDomain={[0, model.run.duration]}
        threshold={{ value: req.limit, label: `${req.id} limit ${limitLabel(req)}` }}
        xUnit="s"
        yUnit={unit}
        height={120}
      />
    </section>
  );

  return <StageGrid left={left} right={right} footer={footer} />;
};
