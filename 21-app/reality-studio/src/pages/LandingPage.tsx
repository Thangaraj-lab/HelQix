import React, { useState } from 'react';
import { useAppState } from '../engineering/stateStore';
import { useStudio } from '../stores';
import { MockEngineClient } from '../mock/MockEngineClient';

export const LandingPage: React.FC = () => {
  const { setCurrentScreen } = useAppState();
  const { query } = useStudio();

  // AI Prompt Modal State
  const [showPromptModal, setShowPromptModal] = useState<boolean>(false);
  const [promptText, setPromptText] = useState<string>('');
  const [buildMode, setBuildMode] = useState<'ghost' | 'scratch'>('ghost');
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);

  // Domain-Agnostic Physical Engineering Templates (Rule 1: NOT built in with motor!)
  const promptTemplates = [
    {
      label: 'Thermal Exchange Manifold',
      desc: 'Liquid-cooled dual manifold with active convective bypass and 120 °C trip limit',
      prompt: 'High-performance liquid-cooled thermal exchange manifold with convective bypass loop and real-time thermal limit monitoring'
    },
    {
      label: 'Aerospace Hydraulic Actuator',
      desc: 'Electro-hydrostatic cylinder with dual redundant dynamic pressure seals',
      prompt: 'Aerospace electro-hydrostatic linear actuator with dual dynamic pressure seals, 85 N·m peak load capability, and port alignment'
    },
    {
      label: 'Cryogenic Expansion Valve',
      desc: 'Superconducting fluid regulation assembly with thermal gradient isolation',
      prompt: 'Cryogenic fluid expansion valve assembly with magnetic backpressure compensation and multi-zone heat dissipation fins'
    },
    {
      label: 'Robotic Articulation Module',
      desc: 'High-torque precision robotic joint with integrated strain sensors',
      prompt: 'Multi-axis harmonic drive articulation joint with integrated torque feedback, thermal sensor probe, and digital twin interface'
    }
  ];

  const handleLaunchBuild = async (overridePrompt?: string) => {
    setIsSubmitting(true);
    const textToUse = overridePrompt !== undefined ? overridePrompt : (promptText || promptTemplates[0].prompt);
    
    // Create project through domain-agnostic engine client
    const project = await MockEngineClient.createProject();
    query.setCurrentProject(project);

    // Save prompt to session storage for Studio coordinator
    sessionStorage.setItem('helqix_initial_prompt', textToUse);
    sessionStorage.setItem('helqix_build_mode', buildMode);

    // Transition to Studio live 3D build in Ghost Mode
    const url = new URL(window.location.href);
    url.searchParams.set('stage', 'define');
    url.searchParams.set('step', 'assemble');
    url.searchParams.set('pid', project.id);
    if (buildMode === 'ghost') {
      url.searchParams.set('ghost', 'true');
    }
    window.history.pushState({}, '', url.toString());

    setTimeout(() => {
      setIsSubmitting(false);
      setCurrentScreen('workspace');
    }, 400);
  };

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-[#020611] select-none flex flex-col justify-between items-center text-white font-['Outfit']">
      {/* 1. Cinematic Horizon & Glowing Portal Ring Background (Attached Image Replica) */}
      <div
        className="absolute inset-0 bg-cover bg-center bg-no-repeat pointer-events-none"
        style={{
          backgroundImage: `url('/assets/helqix-portal.jpg')`
        }}
      >
        {/* Deep ambient vignette & subtle ray overlays */}
        <div className="absolute inset-0 bg-gradient-to-t from-[#020611] via-transparent to-[#020611]/60" />
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_45%,rgba(34,211,238,0.12)_0%,transparent_60%)]" />
      </div>

      {/* 2. Top Minimalist Header */}
      <header className="relative z-20 w-full px-8 py-6 flex items-center justify-between">
        {/* Brand Mark */}
        <div className="flex items-center gap-3">
          <div className="w-7 h-7 rounded-md bg-[#2F7BFF] flex items-center justify-center text-white font-mono text-xs font-bold shadow-[0_0_15px_rgba(47,123,255,0.8)]">
            ⬡
          </div>
          <span className="font-['Space_Grotesk'] font-bold tracking-[0.25em] text-[#F2F6FB] text-sm">
            HELQIX
          </span>
        </div>

        {/* Center Nav */}
        <nav className="hidden md:flex items-center gap-8 text-xs font-mono tracking-wider text-[#7F93AF]">
          <span className="hover:text-white cursor-pointer transition-colors">PHYSICAL REALITY STUDIO</span>
          <span className="hover:text-white cursor-pointer transition-colors">CAUSAL TRUTH</span>
          <span className="hover:text-white cursor-pointer transition-colors">ASME V&V 40</span>
          <span className="hover:text-white cursor-pointer transition-colors">EVIDENCE MANIFEST</span>
        </nav>

        {/* Top Right Action: Launch Studio Directly or New Project */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => {
              const url = new URL(window.location.href);
              url.searchParams.set('stage', 'simulate');
              window.history.pushState({}, '', url.toString());
              setCurrentScreen('workspace');
            }}
            className="px-3.5 py-1.5 rounded-lg text-xs font-mono text-[#D6E2F3] hover:text-white hover:bg-white/5 border border-white/10 transition-all"
          >
            Open Studio
          </button>
          <button
            onClick={() => setShowPromptModal(true)}
            className="px-4 py-1.5 rounded-lg text-xs font-mono font-semibold text-white bg-gradient-to-r from-[#2F7BFF] to-[#06B6D4] hover:from-[#3B82F6] hover:to-[#22D3EE] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
          >
            <span>+</span>
            <span>New Project</span>
          </button>
        </div>
      </header>

      {/* 3. Center Hero: Exact Attached Image Branding (H E L Q I X with glowing Q & pill button) */}
      <main className="relative z-20 flex-1 flex flex-col items-center justify-center text-center px-4 my-auto">
        {/* Main Brand Title: H E L Q I X */}
        <div className="flex items-center justify-center tracking-[0.35em] text-5xl sm:text-7xl md:text-8xl font-['Space_Grotesk'] font-light text-white select-none drop-shadow-[0_0_35px_rgba(47,123,255,0.4)]">
          <span>H</span>
          <span className="ml-4 md:ml-8">E</span>
          <span className="ml-4 md:ml-8">L</span>
          {/* Distinctive Glowing Q with luminous cyan ring and angled tail */}
          <span className="relative ml-4 md:ml-8 inline-flex items-center justify-center">
            <span className="text-[#22D3EE] font-medium drop-shadow-[0_0_25px_rgba(34,211,238,0.9)]">
              Q
            </span>
            <span className="absolute inset-0 rounded-full border border-[#22D3EE]/60 blur-[2px] animate-pulse pointer-events-none" />
          </span>
          <span className="ml-4 md:ml-8">I</span>
          <span className="ml-4 md:ml-8">X</span>
        </div>

        {/* Subtitle */}
        <p className="mt-4 text-xs md:text-sm font-['Space_Grotesk'] tracking-[0.2em] text-[#7F93AF] uppercase">
          Physical reality engineering with mathematical proof
        </p>

        {/* Centered Pill Button with Arrow: [   →   ] */}
        <div className="mt-10">
          <button
            onClick={() => setShowPromptModal(true)}
            aria-label="Create New Project"
            className="group relative px-14 py-3.5 rounded-full border border-[#2F7BFF]/60 bg-[#0A1422]/50 backdrop-blur-md hover:bg-[#2F7BFF]/25 hover:border-[#22D3EE] shadow-[0_0_25px_rgba(47,123,255,0.35)] hover:shadow-[0_0_45px_rgba(34,211,238,0.8)] transition-all duration-300 flex items-center justify-center cursor-pointer"
          >
            {/* Subtle glow beacon */}
            <span className="absolute inset-0 rounded-full bg-gradient-to-r from-transparent via-[#22D3EE]/15 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
            
            {/* Arrow icon */}
            <span className="text-2xl text-[#22D3EE] group-hover:translate-x-2 transition-transform duration-300 font-light">
              →
            </span>
          </button>
        </div>
      </main>

      {/* 4. Bottom Engineering Truth Proof Strip */}
      <footer className="relative z-20 w-full max-w-5xl px-8 py-5 border-t border-[#22395A]/40 flex items-center justify-between text-xs font-mono text-[#7F93AF]">
        <div className="flex items-center gap-6">
          <span className="flex items-center gap-1.5 text-white">
            <span className="w-1.5 h-1.5 rounded-full bg-[#22C55E]" />
            <span>Bitwise Deterministic Multi-Physics</span>
          </span>
          <span className="hidden sm:inline text-[#22395A]">|</span>
          <span className="hidden sm:inline">Zero Domain Presumptions</span>
          <span className="hidden sm:inline text-[#22395A]">|</span>
          <span className="hidden sm:inline">Interactive 3D Physical Repair</span>
        </div>
        <div className="text-[11px] text-[#7F93AF]">
          HELQIX v2.4 · Engineering OS
        </div>
      </footer>

      {/* 5. AI Engineering Prompt Modal (Opens upon clicking pill button or New Project) */}
      {showPromptModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-md animate-fade-in">
          <div className="w-full max-w-2xl rounded-2xl bg-[#0A1422]/95 border border-[#22D3EE]/50 shadow-[0_0_50px_rgba(34,211,238,0.25)] p-6 space-y-5">
            {/* Modal Header */}
            <div className="flex items-center justify-between border-b border-[#22395A] pb-3">
              <div className="space-y-0.5">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-bold font-['Space_Grotesk'] text-white">
                    Create New Physical System with AI
                  </span>
                  <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-[#2F7BFF]/20 text-[#22D3EE] border border-[#2F7BFF]/40">
                    LIVE 3D GHOST BUILD
                  </span>
                </div>
                <p className="text-xs text-[#7F93AF]">
                  Describe any mechanical, thermal, fluid, or structural assembly to build in 3D.
                </p>
              </div>
              <button
                onClick={() => setShowPromptModal(false)}
                className="text-[#7F93AF] hover:text-white p-1 text-sm font-mono"
              >
                ✕
              </button>
            </div>

            {/* Prompt Input Box */}
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs font-mono">
                <span className="text-[#D6E2F3] font-medium">Engineering System Prompt</span>
                <span className="text-[10px] text-[#7F93AF]">Natural Language or CAD Specification</span>
              </div>
              <textarea
                rows={3}
                value={promptText}
                onChange={e => setPromptText(e.target.value)}
                placeholder="e.g. Liquid-cooled heat exchange manifold with convective bypass loop, dual sensor ports, and 120 °C thermal limit protection..."
                className="w-full p-3.5 rounded-xl bg-[#050B14] border border-[#22395A] focus:border-[#22D3EE] focus:ring-1 focus:ring-[#22D3EE] text-xs font-mono text-white placeholder-slate-500 outline-none resize-none transition-all"
              />
            </div>

            {/* Domain-Agnostic Quick Template Pills */}
            <div className="space-y-2">
              <span className="text-[11px] font-mono uppercase tracking-wider text-[#7F93AF]">
                Or select an engineering template:
              </span>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {promptTemplates.map(t => (
                  <button
                    key={t.label}
                    onClick={() => setPromptText(t.prompt)}
                    className="p-2.5 rounded-lg bg-[#0E1B2E] hover:bg-[#13253D] border border-[#22395A] hover:border-[#22D3EE]/50 text-left transition-all group"
                  >
                    <div className="text-xs font-semibold text-white group-hover:text-[#22D3EE]">
                      {t.label}
                    </div>
                    <div className="text-[10px] text-[#7F93AF] truncate mt-0.5 font-mono">
                      {t.desc}
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Build Mode Selector */}
            <div className="p-3 rounded-xl bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between text-xs font-mono">
              <div>
                <div className="font-semibold text-white">Execution Mode</div>
                <div className="text-[10px] text-[#7F93AF]">Holographic Ghost Mode assembles parts live in 3D</div>
              </div>
              <div className="flex items-center gap-1 p-0.5 rounded-lg bg-[#050B14] border border-[#22395A]">
                <button
                  onClick={() => setBuildMode('ghost')}
                  className={`px-3 py-1 rounded-[6px] transition-all ${
                    buildMode === 'ghost'
                      ? 'bg-[#2F7BFF] text-white font-bold shadow-[0_0_12px_rgba(47,123,255,0.4)]'
                      : 'text-[#7F93AF] hover:text-white'
                  }`}
                >
                  Ghost 3D Build
                </button>
                <button
                  onClick={() => setBuildMode('scratch')}
                  className={`px-3 py-1 rounded-[6px] transition-all ${
                    buildMode === 'scratch'
                      ? 'bg-[#2F7BFF] text-white font-bold shadow-[0_0_12px_rgba(47,123,255,0.4)]'
                      : 'text-[#7F93AF] hover:text-white'
                  }`}
                >
                  Scratch Grid
                </button>
              </div>
            </div>

            {/* Modal Actions */}
            <div className="flex items-center justify-end gap-3 pt-2 border-t border-[#22395A]">
              <button
                onClick={() => setShowPromptModal(false)}
                className="px-4 py-2 rounded-lg text-xs font-mono text-[#7F93AF] hover:text-white"
              >
                Cancel
              </button>
              <button
                disabled={isSubmitting}
                onClick={() => handleLaunchBuild()}
                className="px-6 py-2.5 rounded-lg text-xs font-mono font-bold text-white bg-gradient-to-r from-[#2F7BFF] via-[#06B6D4] to-[#22D3EE] hover:opacity-90 shadow-[0_0_25px_rgba(47,123,255,0.5)] transition-all flex items-center gap-2"
              >
                <span>✦</span>
                <span>{isSubmitting ? 'Materializing 3D Model...' : 'Launch Live 3D Build →'}</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
