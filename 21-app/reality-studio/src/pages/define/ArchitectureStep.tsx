import React from 'react';
import { EXAMPLE_SUBSYSTEMS as subsystems } from '../../mock/defineExamples';
import { StepFrame } from './StepFrame';

export interface ArchitectureStepProps {
  onContinue: () => void;
}

const LINKS = [
  { name: 'Electrical', color: 'var(--color-primary-blue)', dash: '' },
  { name: 'Mechanical', color: 'var(--color-trace-cyan)', dash: '' },
  { name: 'Data', color: 'var(--color-assistant-violet)', dash: '4 3' },
  { name: 'Thermal', color: 'var(--status-warning)', dash: '2 3' }
];

// Architecture (file 37 section 6.5): a block diagram generated from the briefs. Logical topology
// only, with no physics claims. The content is the canned example until the engine builds it.
export const ArchitectureStep: React.FC<ArchitectureStepProps> = () => (
  <StepFrame title="Architecture" subtitle="The subsystems and how they connect. A logical diagram with no physics claims." example>
    <div className="surface p-6">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
        {subsystems.map(sub => (
          <section key={sub.id} className="rounded-[12px] border border-dashed border-line-strong p-4 space-y-3" aria-label={sub.name}>
            <h3 className="text-xs font-medium text-muted">{sub.name}</h3>
            <div className="space-y-2.5">
              {sub.blocks.map(b => (
                <div key={b.id} className="rounded-[10px] bg-raised border border-line p-3 space-y-2.5">
                  <div className="text-[13px] font-medium text-strong">{b.name}</div>
                  <div className="flex flex-wrap gap-1.5">
                    {b.ports.map(p => (
                      <span key={p} className="num px-2 h-5 inline-flex items-center rounded-full text-[10px] text-muted border border-line">{p}</span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </section>
        ))}
      </div>

      <div className="mt-6 pt-4 border-t border-line flex flex-wrap items-center gap-x-6 gap-y-2 text-xs">
        <span className="text-muted">Links</span>
        {LINKS.map(l => (
          <span key={l.name} className="inline-flex items-center gap-2 text-ink">
            <svg width="22" height="6" aria-hidden><line x1="0" y1="3" x2="22" y2="3" stroke={l.color} strokeWidth="2" strokeDasharray={l.dash} /></svg>
            {l.name}
          </span>
        ))}
      </div>
    </div>
  </StepFrame>
);
