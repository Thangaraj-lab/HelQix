import React, { useMemo, useState } from 'react';
import { Check, TriangleAlert } from 'lucide-react';
import { EXAMPLE_REQUIREMENT } from '../../mock/scenarios';
import { BranchParams, scriptedBranchTemperature } from '../../mock/branches';
import { Chip, ExampleTag, MockTag, Reading, SectionLabel, SliderWithUnit } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';
import { LineChart } from '../../components/charts/LineChart';
import { useRun } from '../simulate/useRun';

export interface ImproveStageProps {
  onAdoptBranch: () => void;
  onBackToUnderstand?: () => void;
}

const LIMIT = EXAMPLE_REQUIREMENT.limit;
const firstCrossing = (pts: [number, number][]) => pts.find(p => p[1] >= LIMIT)?.[0] ?? null;
const peak = (pts: [number, number][]) => pts.reduce((m, p) => Math.max(m, p[1]), -Infinity);

// Improve (file 37 section 6.12): the original run next to one intervention branch. The branch is a
// scripted demonstration (src/mock/branches.ts), so every number here is MOCK. The verdict always
// states both facts: what the model shows, and that it is not validated.
export const ImproveStage: React.FC<ImproveStageProps> = () => {
  const run = useRun();
  const [params, setParams] = useState<BranchParams>({ coolingPct: 20, loadPct: -10 });

  const original = useMemo(() => run?.series('stator_winding_temperature') ?? [], [run]);
  const branch = useMemo(() => scriptedBranchTemperature(original, params), [original, params]);
  if (!run) return null;

  const oPeak = peak(original);
  const bPeak = peak(branch);
  const oCross = firstCrossing(original);
  const bCross = firstCrossing(branch);
  const meets = bPeak < LIMIT;

  const left = (
    <>
      <PanelCard title="Branches" aside={<ExampleTag />}>
        <ul className="space-y-1.5">
          <li className="flex items-center justify-between rounded-[10px] border border-line bg-white/[0.02] px-3 py-2"><span className="text-[13px] text-strong"><span className="num text-muted mr-2">R0</span>Original</span><Chip tone={oCross != null ? 'bad' : 'ok'}>{oCross != null ? 'Violates' : 'Meets'} {EXAMPLE_REQUIREMENT.id}</Chip></li>
          <li className="flex items-center justify-between rounded-[10px] border border-accent/45 bg-accent/[0.07] px-3 py-2"><span className="text-[13px] text-strong"><span className="num text-muted mr-2">B1</span>Branch</span><Chip tone={meets ? 'ok' : 'bad'}>{meets ? 'Meets' : 'Violates'} {EXAMPLE_REQUIREMENT.id}</Chip></li>
        </ul>
      </PanelCard>
      <PanelCard title="Interventions" aside={<MockTag />}>
        <SliderWithUnit label="Added cooling" value={params.coolingPct} unit="%" min={0} max={40} step={5} onChange={v => setParams(p => ({ ...p, coolingPct: v }))} />
        <SliderWithUnit label="Load change" value={params.loadPct} unit="%" min={-30} max={0} step={5} onChange={v => setParams(p => ({ ...p, loadPct: v }))} />
        <p className="text-[11px] text-muted">Forked from the start of the original run. The branch is a scripted scaling of the original curve, not a solver result.</p>
      </PanelCard>
    </>
  );

  const right = (
    <>
      <PanelCard title="Metrics" aside={<MockTag />}>
        <div className="grid grid-cols-2 gap-4">
          <Reading label="Peak, original" value={+oPeak.toFixed(1)} unit="°C" tone="bad" mock />
          <Reading label="Peak, branch" value={+bPeak.toFixed(1)} unit="°C" tone={meets ? 'default' : 'bad'} mock />
          <Reading label="Limit reached, original" value={oCross != null ? oCross.toFixed(2) : 'Not reached'} unit={oCross != null ? 's' : ''} mock />
          <Reading label="Limit reached, branch" value={bCross != null ? bCross.toFixed(2) : 'Not reached'} unit={bCross != null ? 's' : ''} mock />
        </div>
        <p className="num text-xs text-muted">Peak change: {(bPeak - oPeak).toFixed(1)} °C</p>
      </PanelCard>
      <PanelCard title="Verdict">
        <div className={`rounded-[10px] border p-3.5 space-y-1.5 ${meets ? 'border-ok/35 bg-ok/[0.06]' : 'border-bad/40 bg-bad/[0.06]'}`}>
          <p className={`text-[13px] font-medium flex items-center gap-1.5 ${meets ? 'text-ok' : 'text-bad'}`}>{meets ? <Check size={14} /> : <TriangleAlert size={14} />}{meets ? `Meets ${EXAMPLE_REQUIREMENT.id} in the model.` : `Does not meet ${EXAMPLE_REQUIREMENT.id} in the model.`}</p>
          <p className="text-[13px] text-strong">Not yet validated.</p>
          <p className="text-[11px] text-muted">Validation needs reference data. See Verify.</p>
        </div>
      </PanelCard>
    </>
  );

  const top = (
    <div className="surface-glass px-4 py-2 flex items-center gap-6 text-[13px]">
      <span className="text-strong">Original</span>
      <span className="text-muted">compared with</span>
      <span className="text-strong">Branch B1</span>
    </div>
  );

  const bottom = (
    <section className="surface-glass p-4 space-y-2" aria-label="Overlay curves">
      <SectionLabel aside={<span className="flex items-center gap-3 text-[11px] text-muted"><span className="inline-flex items-center gap-1.5"><i className="w-3 h-0.5 bg-bad inline-block" />Original</span><span className="inline-flex items-center gap-1.5"><i className="w-3 h-0.5 inline-block" style={{ background: 'var(--color-trace-cyan)' }} />Branch</span></span>}>Winding temperature</SectionLabel>
      <LineChart
        series={[
          { id: 'o', label: 'Original', points: original, color: 'var(--status-critical)' },
          { id: 'b', label: 'Branch', points: branch, color: 'var(--color-trace-cyan)' }
        ]}
        xDomain={[0, run.duration]}
        threshold={{ value: LIMIT, label: `${EXAMPLE_REQUIREMENT.id} limit ${LIMIT} °C` }}
        xUnit="s"
        yUnit="°C"
        height={120}
      />
    </section>
  );

  return <StageGrid left={left} right={right} top={top} footer={bottom} />;
};
