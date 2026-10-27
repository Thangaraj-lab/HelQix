import React, { useEffect, useState } from 'react';
import { Source } from '../../contracts';
import { engine } from '../../engine';
import { EXAMPLE_PROMPT } from '../../mock/examplePack';
import { ExampleTag, PreviewTag } from '../../components/ui/UiKit';

export interface IntentStepProps {
  projectId: string;
  initialText: string;
  onContinue: (prompt: string) => void;
  onStartScratch: () => void;
}

// The Prompt screen (file 36 section 6.3): Text, Reference and Voice (both PREVIEW), Blank.
export const IntentStep: React.FC<IntentStepProps> = ({ projectId, initialText, onContinue, onStartScratch }) => {
  const [promptText, setPromptText] = useState(initialText);
  const [sources, setSources] = useState<Source[] | null>(null);
  const [sourcesError, setSourcesError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    engine
      .listSources(projectId)
      .then(s => live && setSources(s))
      .catch((e: unknown) => live && setSourcesError(e instanceof Error ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, [projectId]);

  const canContinue = promptText.trim() !== '';

  return (
    <div className="w-full max-w-3xl mx-auto p-6 flex flex-col justify-between my-auto z-10">
      <div className="rounded-[16px] bg-[#0E1B2E]/90 backdrop-blur-xl border border-[#22395A] p-6 shadow-[0_16px_48px_rgba(0,0,0,0.6)] space-y-4">
        <div className="flex items-center justify-between border-b border-[#22395A] pb-3">
          <h2 className="text-base font-bold text-[#F2F6FB]">What do you want to engineer?</h2>
          <div className="flex items-center gap-1 p-0.5 rounded-[8px] bg-[#0A1422] border border-[#22395A] text-xs">
            <span className="px-3 py-1 rounded-[6px] bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/50">Text</span>
            {(['Reference', 'Voice'] as const).map(tab => (
              <span key={tab} className="px-3 py-1 text-[#7F93AF] flex items-center gap-1.5" title="Not available yet">
                {tab} <PreviewTag />
              </span>
            ))}
            <button onClick={onStartScratch} className="px-3 py-1 rounded-[6px] text-[#7F93AF] hover:text-white">
              Blank
            </button>
          </div>
        </div>

        <div className="space-y-2">
          <textarea
            value={promptText}
            onChange={e => setPromptText(e.target.value)}
            placeholder="Describe the system, operating limits, requirements, interfaces, and known constraints..."
            rows={5}
            aria-label="Describe what you want to engineer"
            className="w-full bg-[#0A1422] rounded-[10px] p-4 border border-[#22395A] text-xs text-[#F2F6FB] placeholder-[#7F93AF] focus:outline-none focus:border-[#2F7BFF] resize-none font-sans leading-relaxed"
          />
          <div className="flex items-center justify-between text-[11px] text-[#7F93AF] px-1">
            <span>{promptText.length} characters</span>
            <button
              onClick={() => setPromptText(EXAMPLE_PROMPT)}
              className="hover:text-[#22D3EE] flex items-center gap-1 font-mono transition-colors"
            >
              <ExampleTag />
              <span>Load example</span>
            </button>
          </div>
        </div>

        <div className="flex items-center justify-between border-t border-[#22395A] pt-4">
          <div className="flex items-center gap-2">
            <button
              onClick={onStartScratch}
              className="px-3.5 py-2 rounded-[8px] text-xs font-medium text-[#D6E2F3] hover:text-white hover:bg-white/5 border border-[#22395A] transition-colors"
            >
              Start from scratch
            </button>
            <span className="px-3.5 py-2 text-xs text-[#7F93AF] flex items-center gap-1.5" title="File import is not connected yet">
              Import file <PreviewTag />
            </span>
          </div>
          <button
            onClick={() => onContinue(promptText.trim())}
            disabled={!canContinue}
            className={`px-6 py-2 rounded-[8px] text-xs font-semibold text-white transition-all flex items-center gap-2 ${
              canContinue ? 'bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_20px_rgba(47,123,255,0.4)]' : 'bg-[#1E293B] text-slate-500 cursor-not-allowed'
            }`}
          >
            <span>Describe it</span>
            <span>→</span>
          </button>
        </div>
      </div>

      <div className="mt-4 p-3 rounded-[12px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] flex items-center gap-2 text-xs flex-wrap">
        <span className="text-[#7F93AF] font-medium">Connected sources:</span>
        {sourcesError ? (
          <span className="text-amber-300 font-mono text-[10px]">unavailable ({sourcesError})</span>
        ) : sources === null ? (
          <span className="text-[#7F93AF] font-mono text-[10px]">loading…</span>
        ) : sources.length === 0 ? (
          <span className="text-[#7F93AF] font-mono text-[10px]">none</span>
        ) : (
          sources.map(src => (
            <span key={src.id} className="px-2.5 py-1 rounded-[6px] bg-[#0E1B2E] border border-[#22395A] text-[10px] font-mono text-[#D6E2F3]">
              {src.name}
            </span>
          ))
        )}
      </div>
    </div>
  );
};
