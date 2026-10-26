import React, { useState, useEffect } from 'react';
import { MOCK_SOURCES, EXAMPLE_PROMPT } from '../../mock/examplePack';
import { ExampleTag, PreviewTag } from '../../components/ui/UiKit';

export interface IntentStepProps {
  onContinue: (prompt: string) => void;
  onStartScratch: () => void;
}

export const IntentStep: React.FC<IntentStepProps> = ({ onContinue, onStartScratch }) => {
  const [activeTab, setActiveTab] = useState<'Text' | 'Reference' | 'Voice' | 'Blank'>('Text');
  const [promptText, setPromptText] = useState('');

  useEffect(() => {
    const saved = sessionStorage.getItem('helqix_initial_prompt');
    if (saved) {
      setPromptText(saved);
    }
  }, []);

  return (
    <div className="w-full max-w-3xl mx-auto p-6 flex flex-col justify-between my-auto z-10">
      {/* Calm Panel over Blurred Photoreal Scene (Section 6.3) */}
      <div className="rounded-[16px] bg-[#0E1B2E]/90 backdrop-blur-xl border border-[#22395A] p-6 shadow-[0_16px_48px_rgba(0,0,0,0.6)] space-y-4">
        <div className="flex items-center justify-between border-b border-[#22395A] pb-3">
          <h2 className="text-base font-bold text-[#F2F6FB]">What do you want to engineer?</h2>
          
          {/* Mode Tabs: Text, Reference, Voice (PREVIEW), Blank */}
          <div className="flex items-center gap-1 p-0.5 rounded-[8px] bg-[#0A1422] border border-[#22395A] text-xs">
            {(['Text', 'Reference', 'Voice', 'Blank'] as const).map(tab => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-3 py-1 rounded-[6px] font-medium transition-colors flex items-center gap-1.5 ${
                  activeTab === tab
                    ? 'bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/50'
                    : 'text-[#7F93AF] hover:text-white'
                }`}
              >
                <span>{tab}</span>
                {tab === 'Voice' && <PreviewTag />}
              </button>
            ))}
          </div>
        </div>

        {/* Large Text Area */}
        <div className="space-y-2">
          <textarea
            value={promptText}
            onChange={e => setPromptText(e.target.value)}
            placeholder="Describe the system, operating limits, requirements, interfaces, and known constraints..."
            rows={5}
            className="w-full bg-[#0A1422] rounded-[10px] p-4 border border-[#22395A] text-xs text-[#F2F6FB] placeholder-[#7F93AF] focus:outline-none focus:border-[#2F7BFF] resize-none font-sans leading-relaxed"
          />

          <div className="flex items-center justify-between text-[11px] text-[#7F93AF] px-1">
            <span>{promptText.length} characters</span>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setPromptText(EXAMPLE_PROMPT)}
                className="hover:text-[#22D3EE] flex items-center gap-1 font-mono transition-colors"
              >
                <ExampleTag />
                <span>Load canonical example</span>
              </button>
            </div>
          </div>
        </div>

        {/* Buttons: Describe it, Start from scratch, Import */}
        <div className="flex items-center justify-between border-t border-[#22395A] pt-4">
          <div className="flex items-center gap-2">
            <button
              onClick={onStartScratch}
              className="px-3.5 py-2 rounded-[8px] text-xs font-medium text-[#D6E2F3] hover:text-white hover:bg-white/5 border border-[#22395A] transition-colors"
            >
              Start from scratch
            </button>
            <button
              onClick={() => alert('CAD Import: select glTF, STEP AP242, or SLDPRT archive.')}
              className="px-3.5 py-2 rounded-[8px] text-xs font-medium text-[#7F93AF] hover:text-white hover:bg-white/5 transition-colors"
            >
              Import file
            </button>
          </div>

          <button
            onClick={() => onContinue(promptText || EXAMPLE_PROMPT)}
            className="px-6 py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all flex items-center gap-2"
          >
            <span>Describe it</span>
            <span>→</span>
          </button>
        </div>
      </div>

      {/* Connected Sources Strip Below (Section 6.3) */}
      <div className="mt-4 p-3 rounded-[12px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs">
          <span className="text-[#7F93AF] font-medium">Connected sources:</span>
          {MOCK_SOURCES.map(src => (
            <span
              key={src.id}
              className="px-2.5 py-1 rounded-[6px] bg-[#0E1B2E] border border-[#22395A] text-[10px] font-mono text-[#D6E2F3] flex items-center gap-1.5"
            >
              <span className="w-1.5 h-1.5 rounded-full bg-[#22C55E]" />
              <span>{src.name}</span>
            </span>
          ))}
        </div>

        <button className="text-xs text-[#22D3EE] hover:underline font-mono">
          + Connect source
        </button>
      </div>
    </div>
  );
};
