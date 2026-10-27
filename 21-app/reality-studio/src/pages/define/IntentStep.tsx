import React, { useEffect, useState } from 'react';
import { FileUp, Mic, PencilRuler } from 'lucide-react';
import { Source } from '../../contracts';
import { engine } from '../../engine';
import { EXAMPLE_PROMPT } from '../../mock/examplePack';
import { Button, ExampleTag, PreviewTag, SectionLabel, StatusDot } from '../../components/ui/UiKit';

export interface IntentStepProps {
  projectId: string;
  initialText: string;
  onContinue: (prompt: string) => void;
  onStartScratch: () => void;
}

// The Prompt screen (file 37 section 6.3): a calm panel over a blurred scene. Nothing else is asked.
export const IntentStep: React.FC<IntentStepProps> = ({ projectId, initialText, onContinue, onStartScratch }) => {
  const [text, setText] = useState(initialText);
  const [sources, setSources] = useState<Source[] | null>(null);
  const [sourcesError, setSourcesError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    engine
      .listSources(projectId)
      .then(s => live && setSources(s))
      .catch((e: unknown) => live && setSourcesError(e instanceof Error ? e.message : String(e)));
    return () => { live = false; };
  }, [projectId]);

  const can = text.trim() !== '';

  return (
    <div className="relative z-10 flex-1 min-h-0 overflow-y-auto">
      <div className="min-h-full flex flex-col items-center justify-center px-6 py-10 gap-4">
        <section className="w-full max-w-[720px] surface-glass !bg-panel/85 p-6 animate-rise" aria-labelledby="intent-title">
          <div className="flex items-center justify-between gap-3">
            <h2 id="intent-title" className="font-display font-light text-[24px] text-strong">What do you want to engineer?</h2>
            <div role="tablist" className="inline-flex items-center gap-0.5 p-0.5 rounded-[9px] bg-white/[0.04] border border-line text-xs">
              <span role="tab" aria-selected className="h-7 px-3 rounded-[7px] bg-raised text-strong inline-flex items-center font-medium">Text</span>
              <span role="tab" aria-disabled className="h-7 px-2.5 inline-flex items-center gap-1.5 text-muted" title="Not available yet"><FileUp size={13} />Reference <PreviewTag /></span>
              <span role="tab" aria-disabled className="h-7 px-2.5 inline-flex items-center gap-1.5 text-muted" title="Not available yet"><Mic size={13} />Voice <PreviewTag /></span>
              <button role="tab" aria-selected={false} onClick={onStartScratch} className="h-7 px-2.5 inline-flex items-center gap-1.5 text-muted hover:text-ink"><PencilRuler size={13} />Blank</button>
            </div>
          </div>

          <label htmlFor="intent-text" className="sr-only">Describe what you want to engineer</label>
          <textarea
            id="intent-text"
            value={text}
            onChange={e => setText(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && (e.metaKey || e.ctrlKey) && can && onContinue(text.trim())}
            placeholder="Describe the system, its operating limits, requirements, interfaces and known constraints."
            rows={6}
            className="mt-5 w-full resize-none rounded-[12px] bg-void/70 border border-line focus:border-accent/60 px-4 py-3.5 text-[14px] leading-relaxed placeholder:text-muted/70 focus:outline-none transition-colors"
          />
          <div className="mt-2 flex items-center justify-between text-[11px] text-muted">
            <span className="num">{text.length} characters</span>
            <button onClick={() => setText(EXAMPLE_PROMPT)} className="inline-flex items-center gap-1.5 hover:text-strong transition-colors">
              <ExampleTag /> Load the example prompt
            </button>
          </div>

          <div className="mt-6 flex items-center justify-between gap-3">
            <div className="flex items-center gap-1">
              <Button variant="secondary" onClick={onStartScratch}>Start from scratch</Button>
              <Button variant="ghost" disabled>Import file <PreviewTag /></Button>
            </div>
            <Button variant="primary" size="lg" disabled={!can} onClick={() => onContinue(text.trim())}>Describe it</Button>
          </div>
        </section>

        <section className="w-full max-w-[720px] surface-glass !bg-panel/80 px-4 py-3 flex flex-wrap items-center gap-x-3 gap-y-2 text-xs" aria-label="Connected sources">
          <SectionLabel>Connected sources</SectionLabel>
          {sourcesError ? (
            <span className="text-warn">unavailable ({sourcesError})</span>
          ) : sources === null ? (
            <span className="text-muted">loading</span>
          ) : sources.length === 0 ? (
            <span className="text-muted">No sources connected. Parts will be placeholders or defined from scratch.</span>
          ) : (
            sources.map(s => (
              <span key={s.id} className="inline-flex items-center gap-1.5 text-ink">
                <StatusDot tone={s.status === 'connected' ? 'ok' : 'warn'} />
                {s.name}
              </span>
            ))
          )}
        </section>
      </div>
    </div>
  );
};
