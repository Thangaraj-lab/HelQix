import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { useStudio } from '../../stores';
import { MockEngineClient } from '../../mock/MockEngineClient';
import { MOCK_SOURCES, EXAMPLE_PROMPT } from '../../mock/examplePack';
import { ExampleTag, MockTag, Panel } from '../../components/ui/UiKit';

export const OverviewPage: React.FC = () => {
  const { setCurrentScreen } = useAppState();
  const { query } = useStudio();
  const [promptText, setPromptText] = useState('');

  const handleCreateNewProject = async (text?: string) => {
    const project = await MockEngineClient.createProject();
    query.setCurrentProject(project);

    if (text) {
      sessionStorage.setItem('helqix_initial_prompt', text);
    }

    const url = new URL(window.location.href);
    url.searchParams.set('stage', 'define');
    url.searchParams.set('step', 'intent');
    url.searchParams.set('pid', project.id);
    window.history.pushState({}, '', url.toString());
    setCurrentScreen('workspace');
  };

  const projectCards = [
    {
      id: 'prj-alpha',
      name: 'High-Torque Powertrain Assembly',
      thumbnail: '/assets/model-editor.jpg',
      lastRun: 'Run 19 finished 4 min ago',
      trustChip: '1 failing safety requirement',
      trustColor: 'text-amber-400 bg-amber-500/15 border-amber-500/30',
      partsCount: 9,
      revision: 'r3'
    },
    {
      id: 'prj-cryo',
      name: 'Thermal Channel Active Cooling Loop',
      thumbnail: '/assets/reality-canvas.jpg',
      lastRun: 'Run 04 completed yesterday',
      trustChip: 'Bitwise match (PASS)',
      trustColor: 'text-emerald-400 bg-emerald-500/15 border-emerald-500/30',
      partsCount: 4,
      revision: 'r1'
    },
    {
      id: 'prj-gearbox',
      name: 'Kinematic Transmission Testbed',
      thumbnail: '/assets/model-workspace.jpg',
      lastRun: 'Run 11 paused at t=8.42s',
      trustChip: '2 gaps in credibility checklist',
      trustColor: 'text-cyan-300 bg-cyan-500/15 border-cyan-500/30',
      partsCount: 6,
      revision: 'r2'
    }
  ];

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto overflow-y-auto">
      {/* 1. Header & Instant New Project Action (Section 6.2) */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#F2F6FB] tracking-wide">Welcome back, Lead Engineer</h1>
          <p className="text-xs text-[#7F93AF] mt-0.5">Authoritative simulation workspaces and verified provenance trails.</p>
        </div>

        <button
          onClick={() => handleCreateNewProject()}
          className="px-4 py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_16px_rgba(47,123,255,0.45)] transition-all flex items-center gap-2"
        >
          <span>+ New project</span>
        </button>
      </div>

      {/* 2. Large Prompt Box (Same as Hero - Section 6.2) */}
      <div className="w-full rounded-[14px] bg-[#0A1422] border border-[#22395A] p-4 flex flex-col gap-3 shadow-[0_8px_32px_rgba(0,0,0,0.4)]">
        <div className="text-xs font-semibold text-[#D6E2F3] flex items-center justify-between">
          <span>Start a new model from intent</span>
          <ExampleTag />
        </div>
        <textarea
          value={promptText}
          onChange={e => setPromptText(e.target.value)}
          placeholder="Describe what you want to engineer (or try: 'A pump driven by an electric motor from a 48 V supply...')"
          rows={2}
          className="w-full bg-[#0E1B2E] rounded-[8px] p-3 border border-[#22395A] text-xs text-[#F2F6FB] placeholder-[#7F93AF] focus:outline-none focus:border-[#2F7BFF]"
        />

        <div className="flex items-center justify-between pt-1">
          <div className="flex items-center gap-2">
            <button
              onClick={() => handleCreateNewProject(EXAMPLE_PROMPT)}
              className="px-2.5 py-1 rounded-[6px] bg-[#13253D] hover:bg-[#1E3A5F] border border-[#22395A] text-[11px] text-[#22D3EE] font-mono"
            >
              Paste example prompt
            </button>
            <button
              onClick={() => handleCreateNewProject()}
              className="px-3 py-1 rounded-[6px] text-xs text-[#7F93AF] hover:text-white"
            >
              Start from scratch
            </button>
          </div>

          <button
            onClick={() => handleCreateNewProject(promptText || EXAMPLE_PROMPT)}
            className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-colors"
          >
            Describe it →
          </button>
        </div>
      </div>

      {/* 3. Main Grid: Left Projects & Recent Runs vs Right Attention & Sources (Section 6.2) */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left Column: Projects & Recent Runs (2 Cols) */}
        <div className="lg:col-span-2 space-y-5">
          <div className="flex items-center justify-between border-b border-[#22395A] pb-2">
            <h2 className="text-sm font-semibold text-[#F2F6FB]">Active Engineering Workspaces</h2>
            <span className="text-xs text-[#7F93AF] font-mono">3 projects</span>
          </div>

          {/* Project Cards with real rendered thumbnail */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {projectCards.map(prj => (
              <div
                key={prj.id}
                onClick={() => {
                  const url = new URL(window.location.href);
                  url.searchParams.set('stage', 'simulate');
                  url.searchParams.set('pid', prj.id);
                  window.history.pushState({}, '', url.toString());
                  setCurrentScreen('workspace');
                }}
                className="group rounded-[14px] bg-[#0A1422] border border-[#22395A] hover:border-[#2F7BFF]/60 hover:shadow-[0_8px_32px_rgba(47,123,255,0.25)] transition-all cursor-pointer overflow-hidden flex flex-col justify-between"
              >
                {/* Real Rendered Thumbnail in its Scene */}
                <div
                  className="w-full h-32 bg-cover bg-center relative"
                  style={{ backgroundImage: `url('${prj.thumbnail}')` }}
                >
                  <div className="absolute inset-0 bg-gradient-to-t from-[#0A1422] to-transparent" />
                  <div className="absolute top-2.5 right-2.5">
                    <span className={`px-2 py-0.5 rounded-[6px] text-[10px] font-mono font-medium border ${prj.trustColor}`}>
                      {prj.trustChip}
                    </span>
                  </div>
                  <div className="absolute bottom-2 left-3 font-mono text-[10px] text-[#22D3EE]">
                    {prj.revision} • {prj.partsCount} components
                  </div>
                </div>

                <div className="p-3.5 space-y-1">
                  <div className="text-xs font-semibold text-[#F2F6FB] group-hover:text-[#22D3EE] transition-colors truncate">
                    {prj.name}
                  </div>
                  <div className="text-[11px] text-[#7F93AF] font-mono">{prj.lastRun}</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Right Column: Needs Attention, Connected Sources & Trust Summary (Section 6.2) */}
        <div className="space-y-4">
          {/* Needs Attention Card */}
          <div className="p-4 rounded-[14px] bg-[#0A1422] border border-[#22395A] space-y-3">
            <div className="flex items-center justify-between border-b border-[#22395A] pb-2">
              <span className="text-xs font-bold text-[#F2F6FB] flex items-center gap-1.5">
                <span className="text-amber-400">⚠</span>
                <span>Needs Attention</span>
              </span>
              <span className="text-[10px] font-mono text-amber-400">1 Item</span>
            </div>
            <div className="p-2.5 rounded-[8px] bg-amber-950/20 border border-amber-500/30 text-xs text-amber-200 space-y-1">
              <div className="font-semibold text-[#F2F6FB]">Trip Latency Out of Bounds</div>
              <p className="text-[11px] text-[#7F93AF]">
                Powertrain thermal trip latency 124 ms exceeds ASIL-D 100 ms limit under Run 19.
              </p>
            </div>
          </div>

          {/* Connected Sources */}
          <div className="p-4 rounded-[14px] bg-[#0A1422] border border-[#22395A] space-y-3">
            <div className="flex items-center justify-between border-b border-[#22395A] pb-2">
              <span className="text-xs font-bold text-[#F2F6FB]">Connected Sources</span>
              <ExampleTag />
            </div>
            <div className="space-y-2 text-xs">
              {MOCK_SOURCES.map(src => (
                <div key={src.id} className="flex items-center justify-between p-2 rounded-[8px] bg-[#0E1B2E] border border-[#22395A]">
                  <div className="truncate text-[#D6E2F3]">{src.name}</div>
                  <span className="w-2 h-2 rounded-full bg-[#22C55E] shrink-0 ml-2" />
                </div>
              ))}
            </div>
          </div>

          {/* Trust Summary */}
          <div className="p-4 rounded-[14px] bg-[#0A1422] border border-[#22395A] space-y-2">
            <span className="text-xs font-bold text-[#F2F6FB]">Trust Summary</span>
            <div className="text-[11px] text-[#7F93AF] space-y-1 font-mono">
              <div className="flex justify-between">
                <span>Formal Proofs:</span>
                <span className="text-[#22D3EE]">18 / 22 Passed</span>
              </div>
              <div className="flex justify-between">
                <span>Determinism:</span>
                <span className="text-[#22C55E]">100% Bitwise Match</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
