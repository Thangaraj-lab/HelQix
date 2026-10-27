import React, { useState } from 'react';
import { Check, Plus, Upload } from 'lucide-react';
import { Button, Chip, ExampleTag, PlaceholderTag, PreviewTag } from '../../components/ui/UiKit';
import { EXAMPLE_COMPONENT_CATEGORIES as componentCategories } from '../../mock/defineExamples';
import { StepFrame } from './StepFrame';

export interface ComponentsStepProps {
  onContinue: () => void;
}

// Components (file 37 section 6.6): for each block, candidates from connected sources with a visible
// ranking reason, plus Define from scratch and Import. Candidates here are the canned example.
export const ComponentsStep: React.FC<ComponentsStepProps> = () => {
  const [chosen, setChosen] = useState<Record<string, string>>(
    Object.fromEntries(componentCategories.map(c => [c.id, c.candidates[0].id]))
  );

  return (
    <StepFrame title="Components" subtitle="Pick a model for each block from your connected sources, or define it yourself." example>
      <div className="space-y-4">
        {componentCategories.map(cat => (
          <section key={cat.id} className="surface p-4 space-y-3" aria-label={cat.subsystem}>
            <h3 className="text-[13px] font-medium text-strong">{cat.subsystem}</h3>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              {cat.candidates.map(c => {
                const on = chosen[cat.id] === c.id;
                return (
                  <article key={c.id} className={`rounded-[12px] border p-4 flex flex-col gap-3 transition-colors ${on ? 'border-accent/50 bg-accent/[0.05]' : 'border-line bg-white/[0.02]'}`}>
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <div className="text-[13px] font-medium text-strong flex items-center gap-1.5 flex-wrap">{c.name}<ExampleTag /></div>
                        <div className="num text-xs text-muted mt-1">{c.specs}</div>
                      </div>
                      {c.maturity === 'Placeholder' ? <PlaceholderTag /> : <Chip tone="neutral">{c.maturity}</Chip>}
                    </div>
                    <p className="text-[11px] text-muted">{c.source}, as of {c.asOf}</p>
                    <p className="text-xs text-ink leading-relaxed"><span className="text-muted">Why it ranks here: </span>{c.reason}</p>
                    <Button
                      size="sm"
                      variant={on ? 'primary' : 'secondary'}
                      className="mt-auto"
                      icon={on ? <Check size={13} /> : undefined}
                      onClick={() => setChosen(p => ({ ...p, [cat.id]: c.id }))}
                    >
                      {on ? 'Selected' : 'Select'}
                    </Button>
                  </article>
                );
              })}
              <div className="rounded-[12px] border border-dashed border-line p-4 flex flex-col items-center justify-center text-center gap-1.5 text-muted">
                <Plus size={16} />
                <span className="text-[13px] text-ink">Define from scratch</span>
                <span className="text-[11px] max-w-[200px]">Available in Assemble: geometry, ports and parameters you enter.</span>
              </div>
              <div className="rounded-[12px] border border-dashed border-line p-4 flex flex-col items-center justify-center text-center gap-1.5 text-muted">
                <Upload size={16} />
                <span className="text-[13px] text-ink flex items-center gap-1.5">Import file <PreviewTag /></span>
                <span className="text-[11px] max-w-[200px]">glTF import is not connected to the Studio yet.</span>
              </div>
            </div>
          </section>
        ))}
      </div>
    </StepFrame>
  );
};
