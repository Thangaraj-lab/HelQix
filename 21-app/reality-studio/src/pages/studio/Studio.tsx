import React, { useState, useEffect } from 'react';
import { useStudio } from '../../stores';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { TimelineBar } from '../../components/timeline/TimelineBar';
import { MockTag, PreviewTag, Panel } from '../../components/ui/UiKit';
import { DefineStage } from '../define/DefineStage';
import { ModelStage } from '../model/ModelStage';
import { SimulateStage } from '../simulate/SimulateStage';
import { UnderstandStage } from '../understand/UnderstandStage';
import { ImproveStage } from '../improve/ImproveStage';
import { VerifyStage } from '../verify/VerifyStage';
import { ObserveStage } from '../edge/ObserveStage';
import { BuildStage } from '../build/BuildStage';
import { CommandPalette } from '../../components/overlays/CommandPalette';
import { SettingsSheet } from '../../components/overlays/SettingsSheet';
import { ActivityDrawer } from '../../components/overlays/ActivityDrawer';
import { SourcesManagerSheet } from '../../components/overlays/SourcesManagerSheet';
import { EvidenceSheet } from '../evidence/EvidenceSheet';
import { AssistantBar } from '../../components/overlays/AssistantBar';

export type StudioStage =
  | 'define'
  | 'model'
  | 'simulate'
  | 'understand'
  | 'improve'
  | 'verify'
  | 'observe'
  | 'build';

export type DefineStep = 'intent' | 'requirements' | 'architecture' | 'components' | 'assemble';

export const Studio: React.FC = () => {
  const { session, selection, playback, query, assistant } = useStudio();
  const { setCurrentScreen } = useAppState();

  // Read URL query params to drive Studio state (Section 4)
  const getQueryParam = (k: string) => {
    const params = new URLSearchParams(window.location.search);
    return params.get(k);
  };

  const [stage, setStageInternal] = useState<StudioStage>(
    (getQueryParam('stage') as StudioStage) || 'simulate'
  );
  const [step, setStepInternal] = useState<DefineStep>(
    (getQueryParam('step') as DefineStep) || 'intent'
  );
  const isGhostModeRequested = getQueryParam('ghost') === 'true';
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [showAssistantModal, setShowAssistantModal] = useState(false);
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [showEvidenceSheet, setShowEvidenceSheet] = useState(false);
  const [showSettingsSheet, setShowSettingsSheet] = useState(false);
  const [showActivityDrawer, setShowActivityDrawer] = useState(false);
  const [showSourcesManager, setShowSourcesManager] = useState(false);

  // Engineering Keyboard Hotkeys Suite (Section 8 & Section 11: G, R, S, F, Esc, Ctrl+K, Space)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Command palette trigger works everywhere
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setShowCommandPalette(prev => !prev);
        return;
      }

      // Escape closes open overlays
      if (e.key === 'Escape') {
        setShowCommandPalette(false);
        setShowEvidenceSheet(false);
        setShowSettingsSheet(false);
        setShowActivityDrawer(false);
        setShowSourcesManager(false);
        setShowAssistantModal(false);
        return;
      }

      // Ignore single-key shortcuts while typing in inputs
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement ||
        (e.target as HTMLElement).isContentEditable
      ) {
        return;
      }

      if (e.code === 'KeyG') {
        session.setActiveTool('move');
      } else if (e.code === 'KeyR') {
        session.setActiveTool('rotate');
      } else if (e.code === 'KeyS') {
        session.setActiveTool('scale');
      } else if (e.code === 'KeyF') {
        session.setCameraPreset('Perspective');
      } else if (e.code === 'Space' && stage === 'simulate') {
        e.preventDefault();
        // Spacebar handled in simulation
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [stage, session]);

  // Sync state to URL without full page reload
  const setStage = (s: StudioStage) => {
    setStageInternal(s);
    const url = new URL(window.location.href);
    url.searchParams.set('stage', s);
    window.history.pushState({}, '', url.toString());
  };

  const setStep = (st: DefineStep) => {
    setStepInternal(st);
    const url = new URL(window.location.href);
    url.searchParams.set('step', st);
    window.history.pushState({}, '', url.toString());
  };

  // Toggle Theme
  const toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    document.documentElement.setAttribute('data-theme', next);
    if (next === 'light') {
      document.documentElement.classList.add('light');
    } else {
      document.documentElement.classList.remove('light');
    }
  };

  const stages: { id: StudioStage; label: string; preview?: boolean; badge?: string }[] = [
    { id: 'define', label: 'Define Model', badge: 'Draft' },
    { id: 'model', label: 'Model', badge: 'r1' },
    { id: 'simulate', label: 'Simulate', badge: 'Run 19' },
    { id: 'understand', label: 'Understand', badge: '1 failure' },
    { id: 'improve', label: 'Improve', badge: '2 branches' },
    { id: 'verify', label: 'Verify', preview: true, badge: '2 gaps' },
    { id: 'observe', label: 'Observe', preview: true },
    { id: 'build', label: 'Build', preview: true }
  ];

  const defineSteps: { id: DefineStep; label: string; num: number }[] = [
    { id: 'intent', label: 'Intent', num: 1 },
    { id: 'requirements', label: 'Requirements', num: 2 },
    { id: 'architecture', label: 'Architecture', num: 3 },
    { id: 'components', label: 'Components', num: 4 },
    { id: 'assemble', label: 'Assemble', num: 5 }
  ];

  // Dynamic Primary Split Button based on project state (Section 5)
  const getPrimaryAction = () => {
    switch (stage) {
      case 'define':
        return step === 'assemble' ? 'Create model r1' : 'Continue';
      case 'model':
        return 'Run simulation';
      case 'simulate':
        return 'Investigate failure';
      case 'understand':
        return 'Test this cause';
      case 'improve':
        return 'Compare branches';
      case 'verify':
        return 'Run validation';
      default:
        return 'Create model r1';
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[var(--bg-void)] text-[var(--text-ink)] select-none">
      {/* 1. LEFT NAVIGATION (176px, icons-only when narrow - Section 5) */}
      <aside className="w-44 h-full flex flex-col justify-between border-r border-[var(--border-line)] bg-[var(--bg-deck)] shrink-0 z-30">
        <div>
          {/* Logo & Brand (Clickable: returns to Landing Page) */}
          <div
            onClick={() => {
              const url = new URL(window.location.href);
              url.search = '';
              window.history.pushState({}, '', url.toString());
              setCurrentScreen('landing');
            }}
            className="h-14 px-4 flex items-center gap-2.5 border-b border-[var(--border-line)] cursor-pointer hover:bg-white/5 transition-colors"
            title="Return to HELQIX Portal"
          >
            <div className="w-6 h-6 rounded-md bg-[#2F7BFF] flex items-center justify-center text-white font-mono text-xs font-bold shadow-[0_0_12px_rgba(47,123,255,0.6)]">
              ⬡
            </div>
            <div className="flex flex-col">
              <span className="font-bold tracking-widest text-[#F2F6FB] text-xs font-mono">HELQIX</span>
              <span className="text-[9px] text-[#7F93AF] font-sans -mt-0.5">Engineering studio</span>
            </div>
          </div>

          {/* Stages Navigation List */}
          <nav className="p-2 space-y-1">
            {stages.map(st => {
              const isActive = stage === st.id;
              return (
                <button
                  key={st.id}
                  onClick={() => setStage(st.id)}
                  className={`w-full relative flex items-center justify-between px-3 py-2 rounded-[8px] text-xs font-medium transition-all ${
                    isActive
                      ? 'bg-[#2F7BFF]/20 text-[#F2F6FB] font-semibold border border-[#2F7BFF]/50 shadow-[0_0_12px_rgba(47,123,255,0.3)]'
                      : 'text-[#7F93AF] hover:text-[#D6E2F3] hover:bg-white/5 border border-transparent'
                  }`}
                >
                  <span className="truncate">{st.label}</span>
                  <div className="flex items-center gap-1 shrink-0">
                    {st.preview && <span className="text-[8px] px-1 py-0.2 rounded bg-amber-500/20 text-amber-300">PREVIEW</span>}
                    {st.badge && !st.preview && (
                      <span className="text-[9px] font-mono text-[#22D3EE] bg-[#22D3EE]/10 px-1 rounded">
                        {st.badge}
                      </span>
                    )}
                  </div>
                </button>
              );
            })}
          </nav>
        </div>

        {/* Bottom Left: Assistant Button (Section 5) & Engine Status */}
        <div className="p-3 border-t border-[var(--border-line)] space-y-2">
          <button
            onClick={() => setShowAssistantModal(true)}
            className="w-full py-2 px-2.5 rounded-[8px] text-xs font-medium text-white bg-gradient-to-r from-[#8B5CF6]/40 to-[#6D28D9]/40 hover:from-[#8B5CF6]/60 hover:to-[#6D28D9]/60 border border-[#8B5CF6]/50 transition-all flex items-center justify-between shadow-[0_0_15px_rgba(139,92,246,0.25)]"
          >
            <span className="flex items-center gap-1.5">
              <span>✦</span>
              <span>Assistant</span>
            </span>
            <span className="text-[9px] px-1 py-0.2 rounded bg-[#8B5CF6]/30 text-purple-200">PREVIEW</span>
          </button>

          <div className="text-[10px] text-[#7F93AF] font-mono flex items-center justify-between px-1">
            <span>Bitwise Engine</span>
            <span className="w-1.5 h-1.5 rounded-full bg-[#22C55E]"></span>
          </div>
        </div>
      </aside>

      {/* 2. MAIN STUDIO VIEWPORT */}
      <div className="flex-1 flex flex-col h-full overflow-hidden relative">
        {/* TOP BAR / HEADER CARD (Section 5) */}
        <header className="h-14 px-5 flex items-center justify-between border-b border-[var(--border-line)] bg-[var(--bg-deck)] shrink-0 z-20">
          <div className="flex items-center gap-3">
            {/* Project Thumbnail & Inline Name */}
            <div className="w-7 h-7 rounded-md bg-[#13253D] border border-[#22395A] flex items-center justify-center text-xs">
              ⚙
            </div>
            <div className="flex items-center gap-2">
              <span className="font-semibold text-xs text-[#F2F6FB]">Untitled project</span>
              <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-[#22C55E]/15 border border-[#22395A] text-[#22C55E]">
                Active
              </span>
            </div>
            <span className="text-slate-600">|</span>
            <span className="text-[11px] text-[#7F93AF] font-mono">Model r1 · Saved 2 min ago</span>
            
            {/* Count & State Chips */}
            <div className="hidden lg:flex items-center gap-1.5 ml-2 text-[10px] font-mono text-[#7F93AF]">
              <span className="px-1.5 py-0.5 rounded bg-[var(--bg-panel)] border border-[var(--border-line)]">4 parts</span>
              <span className="px-1.5 py-0.5 rounded bg-[var(--bg-panel)] border border-[var(--border-line)]">2 runs</span>
              <span className="px-1.5 py-0.5 rounded bg-[var(--bg-panel)] border border-[#22D3EE]/30 text-[#22D3EE]">Bitwise</span>
            </div>
          </div>

          {/* Stepper (Only visible while defining - Section 5) */}
          {stage === 'define' && (
            <div className="flex items-center gap-1 p-1 rounded-[10px] bg-[var(--bg-panel)] border border-[var(--border-line)]">
              {defineSteps.map(s => (
                <button
                  key={s.id}
                  onClick={() => setStep(s.id)}
                  className={`px-2.5 py-1 rounded-[6px] text-xs font-mono flex items-center gap-1.5 transition-all ${
                    step === s.id
                      ? 'bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/50'
                      : 'text-[#7F93AF] hover:text-white'
                  }`}
                >
                  <span className="w-3.5 h-3.5 rounded-full bg-[#13253D] flex items-center justify-center text-[9px]">
                    {s.num}
                  </span>
                  <span>{s.label}</span>
                </button>
              ))}
            </div>
          )}

          {/* Right Controls: Primary Split Button, Theme Toggle, Bell, Avatar */}
          <div className="flex items-center gap-3">
            {/* Realistic vs Engineering Look Toggle (Section 3) */}
            <div className="flex items-center gap-1 p-0.5 rounded-[8px] bg-[var(--bg-panel)] border border-[var(--border-line)] text-xs font-mono">
              {(['Realistic', 'Engineering'] as const).map(l => (
                <button
                  key={l}
                  onClick={() => session.setLook(l)}
                  className={`px-2.5 py-0.5 rounded-[6px] transition-colors ${
                    session.look === l
                      ? 'bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/40'
                      : 'text-[#7F93AF] hover:text-white'
                  }`}
                >
                  {l}
                </button>
              ))}
            </div>

            {/* PRIMARY SPLIT BUTTON (Always the next best step - Section 5) */}
            <div className="flex items-center rounded-[8px] bg-[#2F7BFF] text-white shadow-[0_0_20px_rgba(47,123,255,0.45)] hover:bg-[#3B82F6] transition-colors">
              <button
                onClick={() => {
                  if (stage === 'define') {
                    if (step === 'assemble') setStage('simulate');
                    else {
                      const nextIdx = defineSteps.findIndex(x => x.id === step) + 1;
                      if (nextIdx < defineSteps.length) setStep(defineSteps[nextIdx].id);
                    }
                  } else if (stage === 'simulate') {
                    setStage('understand');
                  } else {
                    setStage('simulate');
                  }
                }}
                className="px-3.5 py-1.5 text-xs font-semibold"
              >
                {getPrimaryAction()}
              </button>
              <div className="w-[1px] h-4 bg-blue-300/40"></div>
              <button className="px-2 py-1.5 text-xs hover:bg-black/10">▾</button>
            </div>

            {/* Command Palette Button (Section 6.17) */}
            <button
              onClick={() => setShowCommandPalette(true)}
              className="px-2 py-1 rounded-[6px] text-xs font-mono text-[#7F93AF] hover:text-white bg-[#13253D] hover:bg-[#1E3A5F] border border-[#22395A] transition-colors"
              title="Command Palette (Ctrl+K)"
            >
              ⌘K
            </button>

            {/* Activity Bell (Section 6.17) */}
            <button
              onClick={() => setShowActivityDrawer(true)}
              className="p-1.5 rounded-[8px] text-[#7F93AF] hover:text-white hover:bg-white/5 border border-transparent transition-colors text-xs"
              title="Activity Log"
            >
              🔔
            </button>

            {/* Theme Toggle */}
            <button
              onClick={toggleTheme}
              className="p-1.5 rounded-[8px] text-[#7F93AF] hover:text-white hover:bg-white/5 border border-transparent"
              title="Toggle Theme"
            >
              {theme === 'dark' ? '☀️' : '🌙'}
            </button>

            {/* Avatar / Settings Trigger (Section 6.17) */}
            <button
              onClick={() => setShowSettingsSheet(true)}
              className="w-7 h-7 rounded-full bg-gradient-to-tr from-blue-600 to-cyan-400 p-[1px] hover:scale-105 transition-transform"
              title="Engineer Settings"
            >
              <div className="w-full h-full rounded-full bg-[#050B14] flex items-center justify-center text-[10px] font-bold text-white">
                HQ
              </div>
            </button>
          </div>
        </header>

        {/* 3. CENTER STUDIO BODY (Reality Canvas is PERSISTENT - never unmounts - Section 2) */}
        <div className="flex-1 relative overflow-hidden flex flex-col justify-between">
          {/* Background Persistent 3D Canvas */}
          <div className="absolute inset-0 z-0 pointer-events-auto">
            <HeroScene
              variant={
                stage === 'understand'
                  ? 'failure'
                  : stage === 'improve'
                  ? 'compare'
                  : stage === 'model'
                  ? 'model_editor'
                  : 'workspace'
              }
              interactive={stage !== 'define'}
              qualityTier={session.qualityTier}
              thermalHotspot={stage === 'understand'}
              isGhostMode={isGhostModeRequested || (stage === 'define' && step === 'assemble')}
              repairState={stage === 'understand' ? 'fault' : stage === 'improve' ? 'repaired' : 'none'}
              onExecuteRepair={() => setStage('improve')}
            />
          </div>

          {/* If stage === 'define', render the Define Stage wizard steps (Intent, Requirements, Architecture, Components, Assemble) */}
          {stage === 'define' ? (
            <DefineStage
              currentStep={step}
              onStepChange={setStep}
              onCommitToAssemble={() => {
                setStage('simulate');
              }}
            />
          ) : stage === 'model' ? (
            <ModelStage onRunSimulation={() => setStage('simulate')} />
          ) : stage === 'simulate' ? (
            <SimulateStage
              onInvestigateFailure={() => setStage('understand')}
              onForkBranch={() => setStage('improve')}
            />
          ) : stage === 'understand' ? (
            <UnderstandStage
              onTestThisCause={() => setStage('improve')}
              onBackToSimulate={() => setStage('simulate')}
            />
          ) : stage === 'improve' ? (
            <ImproveStage
              onAdoptBranch={() => setStage('verify')}
              onBackToUnderstand={() => setStage('understand')}
            />
          ) : stage === 'verify' ? (
            <VerifyStage onOpenEvidenceSheet={() => setShowEvidenceSheet(true)} />
          ) : stage === 'observe' ? (
            <ObserveStage onProposeUpdate={() => setStage('improve')} />
          ) : stage === 'build' ? (
            <BuildStage />
          ) : (
            <div className="flex-1 p-6 text-center text-slate-400">Unknown Stage</div>
          )}

          {/* Floating Rule-Based Assistant (PREVIEW) - Non-overlapping and dismissible */}
          <AssistantBar
            isOpen={showAssistantModal && stage !== 'define'}
            onClose={() => setShowAssistantModal(false)}
            onApply={() => {
              assistant.applyProposal('prop-cooling-bypass');
              setStage('improve');
              setShowAssistantModal(false);
            }}
            onSimulate={() => {
              setStage('simulate');
              setShowAssistantModal(false);
            }}
            onBranch={() => {
              setStage('improve');
              setShowAssistantModal(false);
            }}
          />
        </div>
      </div>

      {/* OVERLAYS (Section 6.17) */}
      <CommandPalette
        isOpen={showCommandPalette}
        onClose={() => setShowCommandPalette(false)}
        onSelectAction={id => {
          if (id === 'run_simulation') setStage('simulate');
          else if (id === 'open_evidence') setShowEvidenceSheet(true);
          else if (id === 'open_sources') setShowSourcesManager(true);
          else if (id === 'branch_intervention') setStage('improve');
          else if (id === 'verify_evidence') setStage('verify');
          else if (id === 'compare_branches') setStage('improve');
        }}
      />

      <EvidenceSheet
        isOpen={showEvidenceSheet}
        onClose={() => setShowEvidenceSheet(false)}
      />

      <SettingsSheet
        isOpen={showSettingsSheet}
        onClose={() => setShowSettingsSheet(false)}
        onToggleTheme={toggleTheme}
        currentTheme={theme}
      />

      <ActivityDrawer
        isOpen={showActivityDrawer}
        onClose={() => setShowActivityDrawer(false)}
      />

      <SourcesManagerSheet
        isOpen={showSourcesManager}
        onClose={() => setShowSourcesManager(false)}
      />
    </div>
  );
};
