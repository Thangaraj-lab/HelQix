import React from 'react';
import { CircleCheck, CircleDashed, CircleX } from 'lucide-react';
import { limitLabel, violates } from '../../app/requirements';
import { short } from '../../app/evidence';
import { Button, Chip, ExampleIfMock, MockTag, PreviewTag, SectionLabel } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';
import { useEvidencePackage } from '../evidence/usePackage';
import { useModelData } from '../studio/ModelContext';

export interface VerifyStageProps {
  onOpenEvidenceSheet: () => void;
}

type CheckState = 'done' | 'missing' | 'failed';

// Verify (file 37 section 6.13): requirements, a credibility checklist that lists what is missing,
// and the evidence package as it would be assembled. There is no blanket verified state: the page
// says "Not yet verified for this use" until every checklist item has evidence behind it.
export const VerifyStage: React.FC<VerifyStageProps> = ({ onOpenEvidenceSheet }) => {
  const { run, requirements, series } = useModelData();
  const pkg = useEvidencePackage();

  const checklist: { title: string; state: CheckState; note: string }[] = [
    { title: 'Validation error against reference data', state: 'missing', note: 'No reference data is connected.' },
    { title: 'Uncertainty bounds', state: 'missing', note: 'No uncertainty study has been run.' },
    pkg?.replay
      ? { title: 'Determinism', state: pkg.replay.matches ? 'done' : 'failed', note: pkg.replay.matches ? `Replay matches the original over ${pkg.replay.frames} frames.` : 'Replay differs from the original.' }
      : { title: 'Determinism', state: 'missing', note: 'No run to replay.' },
    { title: 'Numerical checks', state: 'missing', note: 'No convergence data from a real solver.' },
    { title: 'Independent review', state: 'missing', note: 'No reviewer has signed off.' }
  ];
  const missing = checklist.filter(c => c.state !== 'done').length;
  const icon = (s: CheckState) => s === 'done' ? <CircleCheck size={15} className="text-ok shrink-0" /> : s === 'failed' ? <CircleX size={15} className="text-bad shrink-0" /> : <CircleDashed size={15} className="text-muted shrink-0" />;

  const left = (
    <PanelCard title="Requirements" aside={<ExampleIfMock />}>
      {requirements.length === 0 ? (
        <p className="text-xs text-muted">No requirement is defined for this model, so there is nothing to check yet.</p>
      ) : (
        <ul className="space-y-2">
          {requirements.map(r => {
            const brokenAt = (series[r.variable] ?? []).find(p => violates(r, p[1]))?.[0] ?? null;
            return (
              <li key={r.id} className="rounded-[10px] border border-line bg-white/[0.02] p-3.5 space-y-1.5">
                <span className="block text-[13px] text-strong"><span className="num text-muted mr-1.5">{r.id}</span>{r.text}</span>
                <span className="block text-[11px] text-muted">Limit {limitLabel(r)}</span>
                <Chip tone={!run ? 'neutral' : brokenAt !== null ? 'bad' : 'ok'}>{!run ? 'Not run' : brokenAt !== null ? `Fails at ${brokenAt.toFixed(2)} s in the run` : 'Passes in the run'}</Chip>
                <p className="text-[11px] text-muted">A pass in a model is not verification.</p>
              </li>
            );
          })}
        </ul>
      )}
    </PanelCard>
  );

  const right = (
    <>
      <PanelCard title="Credibility checklist" aside={<PreviewTag />}>
        <ul className="space-y-3">
          {checklist.map(c => (
            <li key={c.title} className="flex gap-2.5">
              {icon(c.state)}
              <span className="min-w-0"><span className="block text-[13px] text-strong">{c.title}</span><span className="block text-xs text-muted">{c.note}</span></span>
            </li>
          ))}
        </ul>
        <div className="rounded-[10px] border border-warn/30 bg-warn/[0.06] p-3.5 space-y-1">
          <p className="text-[13px] font-medium text-strong">Not yet verified for this use.</p>
          <p className="text-xs text-muted">{missing} of {checklist.length} checks have no evidence.</p>
        </div>
        <Button variant="secondary" className="w-full" disabled>Run validation <PreviewTag /></Button>
      </PanelCard>
    </>
  );

  const bottom = (
    <section className="surface-glass p-4 space-y-3" aria-label="Evidence package">
      <SectionLabel aside={<MockTag />}>Evidence package</SectionLabel>
      {pkg === null ? (
        <p className="text-xs text-muted">Hashing.</p>
      ) : (
        <ul className="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-2">
          {pkg.items.map(i => (
            <li key={i.id} className="flex items-baseline justify-between gap-3 text-xs">
              <span className="min-w-0"><span className="text-strong">{i.title}</span><span className="text-muted"> · {i.detail}</span></span>
              <span className="num text-muted shrink-0">{short(i.hash)}</span>
            </li>
          ))}
        </ul>
      )}
      <div className="flex items-center justify-between gap-3 pt-1 border-t border-line">
        <p className="text-[11px] text-muted">Hashes are SHA-256 over this browser's copy of the data. Nothing is signed yet.</p>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="secondary" onClick={onOpenEvidenceSheet}>Open evidence sheet</Button>
          <Button size="sm" variant="primary" disabled>Sign and export <PreviewTag /></Button>
        </div>
      </div>
    </section>
  );

  return <StageGrid left={left} right={right} footer={bottom} />;
};
