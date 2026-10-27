import React, { useState } from 'react';
import { Check, Pencil } from 'lucide-react';
import { Button, Chip, SectionLabel } from '../../components/ui/UiKit';
import { EXAMPLE_BRIEFS, EXAMPLE_QUESTIONS } from '../../mock/defineExamples';
import { StepFrame } from './StepFrame';

export interface RequirementsStepProps {
  promptText: string;
  onContinue: () => void;
}

// Requirements (file 37 section 6.4): your brief, six brief cards, the requirements extracted from it,
// and at most three questions. The content is the canned example until the engine interprets the prompt.
const EXAMPLE_REQUIREMENTS = [
  { label: 'Supply voltage', value: '48.0', unit: 'V', source: 'User-stated' },
  { label: 'Winding temperature limit', value: '120.0', unit: '°C', source: 'User-stated' },
  { label: 'Nominal current', value: '62.5', unit: 'A', source: 'Calculated', calc: '3 kW at 48 V is about 62.5 A' }
];

type Answer = { kind: 'default'; text: string } | { kind: 'unknown' };

export const RequirementsStep: React.FC<RequirementsStepProps> = ({ promptText }) => {
  const [briefs, setBriefs] = useState(EXAMPLE_BRIEFS.map(b => ({ ...b, edited: false })));
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState('');
  const [answers, setAnswers] = useState<Record<string, Answer>>({});

  const saveEdit = (id: string) => {
    setBriefs(prev => prev.map(b => (b.id === id ? { ...b, desc: draft.trim() || b.desc, status: 'Edited', edited: true } : b)));
    setEditing(null);
  };

  return (
    <StepFrame title="Requirements" subtitle="What was understood from your description. Edit a brief to change only that brief." example>
      <div className="grid grid-cols-1 lg:grid-cols-[260px_1fr_280px] gap-4 items-start">
        <section className="surface p-4 space-y-3" aria-label="Your brief">
          <SectionLabel>Your brief</SectionLabel>
          <p className="text-[13px] text-ink leading-relaxed">{promptText ? promptText : 'No description was given.'}</p>
          <p className="text-[11px] text-muted">Source: your description</p>
        </section>

        <section className="grid grid-cols-1 sm:grid-cols-2 gap-3" aria-label="Briefs">
          {briefs.map(b => (
            <article key={b.id} className="surface p-4 flex flex-col gap-2.5 min-h-[128px]">
              <div className="flex items-center justify-between gap-2">
                <h3 className="text-[13px] font-medium text-strong">{b.title}</h3>
                <Chip tone={b.status === 'Ready' ? 'ok' : b.edited ? 'accent' : 'neutral'}>{b.status}</Chip>
              </div>
              {editing === b.id ? (
                <>
                  <textarea
                    autoFocus
                    value={draft}
                    onChange={e => setDraft(e.target.value)}
                    rows={3}
                    aria-label={`Edit ${b.title} brief`}
                    className="w-full resize-none rounded-[8px] bg-void/70 border border-line focus:border-accent/60 px-2.5 py-2 text-xs leading-relaxed focus:outline-none"
                  />
                  <div className="flex gap-1.5">
                    <Button size="sm" variant="primary" onClick={() => saveEdit(b.id)}>Save</Button>
                    <Button size="sm" variant="ghost" onClick={() => setEditing(null)}>Cancel</Button>
                  </div>
                </>
              ) : (
                <>
                  <p className="text-xs text-muted leading-relaxed flex-1">{b.desc}</p>
                  <button onClick={() => { setEditing(b.id); setDraft(b.desc); }} className="self-start inline-flex items-center gap-1.5 text-xs text-muted hover:text-strong transition-colors">
                    <Pencil size={12} /> Edit
                  </button>
                </>
              )}
            </article>
          ))}
        </section>

        <section className="surface p-4 space-y-3" aria-label="Extracted requirements">
          <SectionLabel>Extracted requirements</SectionLabel>
          <ul className="space-y-3">
            {EXAMPLE_REQUIREMENTS.map(r => (
              <li key={r.label} className="space-y-1">
                <div className="flex items-center justify-between text-[11px] text-muted">
                  <span>{r.label}</span><span>{r.source}</span>
                </div>
                <div className="num text-[18px] text-strong">{r.value} <span className="text-xs text-muted">{r.unit}</span></div>
                {r.calc && <div className="text-[11px] text-trace">Calculated: {r.calc}</div>}
              </li>
            ))}
          </ul>
        </section>
      </div>

      <section className="surface p-4 space-y-3" aria-label="Questions">
        <SectionLabel aside={<span className="text-[11px] text-muted">Rule-based suggestions. Skip leaves the value Unknown.</span>}>Questions</SectionLabel>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {EXAMPLE_QUESTIONS.map(q => {
            const a = answers[q.id];
            return (
              <div key={q.id} className="rounded-[10px] border border-line bg-white/[0.02] p-3.5 space-y-2.5">
                <p className="text-[13px] text-ink">{q.text}</p>
                {a ? (
                  <div className="flex items-center justify-between text-xs">
                    <span className="inline-flex items-center gap-1.5 text-ok"><Check size={13} />{a.kind === 'default' ? `Using ${a.text}` : 'Unknown'}</span>
                    <button className="text-muted hover:text-strong" onClick={() => setAnswers(p => { const n = { ...p }; delete n[q.id]; return n; })}>Change</button>
                  </div>
                ) : (
                  <div className="flex items-center gap-1.5">
                    <Button size="sm" variant="secondary" onClick={() => setAnswers(p => ({ ...p, [q.id]: { kind: 'default', text: q.defaultVal } }))}>Use default: {q.defaultVal}</Button>
                    <Button size="sm" variant="ghost" onClick={() => setAnswers(p => ({ ...p, [q.id]: { kind: 'unknown' } }))}>Skip</Button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </section>
    </StepFrame>
  );
};
