import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  Bell, Box, Check, Command, FileCheck2, HelpCircle, Hammer, Moon, Play, Radio, ScanSearch, Settings, ShieldCheck, Sparkles, Sun, TrendingUp
} from 'lucide-react';
import { useStudio } from '../../stores';
import { navigate, setQuery, useLocation } from '../../app/router';
import { engineMode } from '../../engine';
import { ModelScene } from '../../scene/ModelScene';
import { VISUAL_SPEED_SCALE } from '../../scene/bindings';
import type { SceneConnection, ScenePart } from '../../scene/types';
import { scriptedBranchTemperature, BranchParams } from '../../mock/branches';
import { ModelProvider, useModelData } from './ModelContext';
import { ShortcutsSheet } from '../../components/overlays/ShortcutsSheet';
import { engine } from '../../engine';
import { Button, Chip, IconButton, Logo, MockTag, PreviewTag, StatusDot, Tabs } from '../../components/ui/UiKit';
import { DefineStage } from '../define/DefineStage';
import { ModelStage } from '../model/ModelStage';
import { useRevision } from '../model/useRevision';
import { SimulateStage } from '../simulate/SimulateStage';
import { UnderstandStage } from '../understand/UnderstandStage';
import { ImproveStage } from '../improve/ImproveStage';
import { VerifyStage } from '../verify/VerifyStage';
import { ObserveStage } from '../edge/ObserveStage';
import { BuildStage } from '../build/BuildStage';
import { CommandPalette, PaletteCommand } from '../../components/overlays/CommandPalette';
import { SettingsSheet } from '../../components/overlays/SettingsSheet';
import { ActivityDrawer } from '../../components/overlays/ActivityDrawer';
import { SourcesManagerSheet } from '../../components/overlays/SourcesManagerSheet';
import { EvidenceSheet } from '../evidence/EvidenceSheet';
import { AssistantBar, AssistantProposal } from '../../components/overlays/AssistantBar';

export type StudioStage = 'define' | 'model' | 'simulate' | 'understand' | 'improve' | 'verify' | 'observe' | 'build';
export type DefineStep = 'intent' | 'requirements' | 'architecture' | 'components' | 'assemble';

const STAGES: { id: StudioStage; label: string; icon: React.ReactNode; preview?: boolean }[] = [
  { id: 'define', label: 'Define', icon: <Sparkles size={16} /> },
  { id: 'model', label: 'Model', icon: <Box size={16} /> },
  { id: 'simulate', label: 'Simulate', icon: <Play size={16} /> },
  { id: 'understand', label: 'Understand', icon: <ScanSearch size={16} /> },
  { id: 'improve', label: 'Improve', icon: <TrendingUp size={16} /> },
  { id: 'verify', label: 'Verify', icon: <ShieldCheck size={16} />, preview: true },
  { id: 'observe', label: 'Observe', icon: <Radio size={16} />, preview: true },
  { id: 'build', label: 'Build', icon: <Hammer size={16} />, preview: true }
];

const DEFINE_STEPS: { id: DefineStep; label: string }[] = [
  { id: 'intent', label: 'Intent' },
  { id: 'requirements', label: 'Requirements' },
  { id: 'architecture', label: 'Architecture' },
  { id: 'components', label: 'Components' },
  { id: 'assemble', label: 'Assemble' }
];

export const Studio: React.FC<{ projectId: string }> = ({ projectId }) => {
  const { query } = useLocation();
  const { revision, loading } = useRevision(projectId, query.get('rev'));
  return (
    <ModelProvider projectId={projectId} revision={revision} loading={loading}>
      <StudioInner projectId={projectId} />
    </ModelProvider>
  );
};

const StudioInner: React.FC<{ projectId: string }> = ({ projectId }) => {
  const { session, selection, playback } = useStudio();
  const model = useModelData();
  const { revision } = model;
  const [branch, setBranch] = useState<BranchParams>({ coolingPct: 20, loadPct: -10 });
  // Stage, step and committed revision live in the URL (file 37 section 4), so back and forward work.
  const { query: urlQuery } = useLocation();
  const stage = (urlQuery.get('stage') as StudioStage) || 'define';
  const step = (urlQuery.get('step') as DefineStep) || 'intent';
  const rev = urlQuery.get('rev');
  const stageRef = useRef(stage);
  stageRef.current = stage;

  // Parts the Assemble step has placed; the canvas shows these instead of the Example scene.
  const [staged, setStaged] = useState<{ parts: ScenePart[]; connections: SceneConnection[] }>({ parts: [], connections: [] });
  const stagedParts = staged.parts;
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [showAssistant, setShowAssistant] = useState(false);
  const [showCommands, setShowCommands] = useState(false);
  const [showEvidence, setShowEvidence] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showActivity, setShowActivity] = useState(false);
  const [showSources, setShowSources] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [fitSignal, setFitSignal] = useState(0);
  const [name, setName] = useState('Untitled project');
  const [renaming, setRenaming] = useState(false);
  useEffect(() => {
    let live = true;
    engine.listProjects().then(l => { const p = l.find(x => x.id === projectId); if (live && p) setName(p.name); }).catch(() => undefined);
    return () => { live = false; };
  }, [projectId]);
  const commitName = (v: string) => {
    setRenaming(false);
    const next = v.trim();
    if (next && next !== name) { setName(next); void engine.renameProject(projectId, next).catch(() => undefined); }
  };

  const setStage = (s: StudioStage) => setQuery({ stage: s });
  const setStep = (st: DefineStep) => setQuery({ stage: 'define', step: st });

  const toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    document.documentElement.setAttribute('data-theme', next);
  };
  useEffect(() => () => document.documentElement.removeAttribute('data-theme'), []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setShowCommands(p => !p);
        return;
      }
      if (e.key === 'Escape') {
        setShowCommands(false); setShowEvidence(false); setShowSettings(false);
        setShowActivity(false); setShowSources(false); setShowAssistant(false); setShowShortcuts(false);
        return;
      }
      const t = e.target as HTMLElement;
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || t.isContentEditable) return;
      if (e.code === 'KeyF') setFitSignal(n => n + 1);
      else if (e.key === '?') setShowShortcuts(v => !v);
      else if (e.code === 'Space' && stageRef.current === 'simulate') { e.preventDefault(); playback.togglePlay(); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [session, playback]);

  // The one next step for where the user is (file 37 section 5). Intent has its own "Describe it";
  // preview stages have none, rather than a button that goes nowhere useful.
  const primaryAction = ((): { label: string; run: () => void } | null => {
    switch (stage) {
      case 'define': {
        if (step === 'intent') return null;
        if (step === 'assemble') {
          return rev
            ? { label: 'Run simulation', run: () => setStage('simulate') }
            : { label: 'Create model', run: () => window.dispatchEvent(new Event('helqix:commit-request')) };
        }
        return { label: 'Continue', run: () => setStep(DEFINE_STEPS[DEFINE_STEPS.findIndex(x => x.id === step) + 1].id) };
      }
      case 'model': return { label: 'Run simulation', run: () => setStage('simulate') };
      case 'simulate': return { label: 'Understand results', run: () => setStage('understand') };
      case 'understand': return { label: 'Test this cause', run: () => setStage('improve') };
      case 'improve': return { label: 'Verify', run: () => setStage('verify') };
      default: return null;
    }
  })();

  // Rule-based suggestion for the current state. Every sentence describes what will happen; none
  // states a number the engine has not computed.
  const proposal = useMemo<AssistantProposal | null>(() => {
    const placeholders = (stagedParts ?? []).filter(p => p.placeholder);
    if (stage === 'define' && step === 'assemble' && !rev && placeholders.length > 0) {
      return {
        id: 'placeholders',
        intent: 'Replace placeholder models',
        proposal: `${placeholders.map(p => p.name).join(', ')} ${placeholders.length === 1 ? 'has' : 'have'} no model.`,
        why: 'Results that depend on a placeholder part are placeholders too.',
        effect: 'Clears the Placeholder tag once a source or import supplies a model. No result changes until you run again.',
        action: { label: 'Open sources', run: () => setShowSources(true) }
      };
    }
    if (!rev && stage === 'define' && step !== 'assemble' && step !== 'intent') {
      return {
        id: 'to-assemble',
        intent: 'Get to a first model',
        proposal: 'Continue to Assemble to see the model built from your description.',
        why: 'Simulation needs a committed model revision.',
        effect: 'Shows the live build. Nothing is committed until you choose Create model.',
        action: { label: 'Go to Assemble', run: () => setStep('assemble') }
      };
    }
    if (rev && (stage === 'define' || stage === 'model')) {
      return {
        id: 'baseline',
        intent: 'Run a baseline simulation',
        proposal: `Model ${rev} exists and has no run yet.`,
        why: 'A baseline is what later changes are compared against.',
        effect: 'Produces the first result, labelled Simulated and Unverified.' + (engineMode === 'mock' ? ' In this build it is also tagged Mock.' : ''),
        action: { label: 'Run simulation', run: () => setStage('simulate') }
      };
    }
    if (rev && stage === 'simulate') {
      return {
        id: 'understand',
        intent: 'Understand the result',
        proposal: 'Open the candidate chain for what the run showed.',
        why: 'Candidates are hypotheses until tested.',
        effect: 'Lists possible causes with their origin and status. Changes nothing in the model.',
        action: { label: 'Understand results', run: () => setStage('understand') }
      };
    }
    if (rev && stage === 'understand') {
      return {
        id: 'test-cause',
        intent: 'Test a candidate cause',
        proposal: 'Create an intervention branch from the frozen state.',
        why: 'A branch shows whether the change matters, without touching the original.',
        effect: 'Opens Improve with the original and the branch side by side.',
        action: { label: 'Try a change', run: () => setStage('improve') }
      };
    }
    return null;
  }, [stage, step, rev, stagedParts]);

  const commands: PaletteCommand[] = [
    ...STAGES.map(s => ({ id: `stage:${s.id}`, label: `Go to ${s.label}`, group: 'Navigate' })),
    { id: 'run', label: 'Run simulation', group: 'Execution', disabledReason: rev ? undefined : 'Commit a model first' },
    { id: 'evidence', label: 'Open evidence sheet', group: 'Export' },
    { id: 'sources', label: 'Open sources', group: 'Data' },
    { id: 'assistant', label: 'Show assistant suggestion', group: 'Assist' },
    { id: 'theme', label: 'Toggle theme', group: 'View' },
    { id: 'new', label: 'New project', group: 'Project' },
    { id: 'home', label: 'Back to Home', group: 'Project' }
  ];
  const onCommand = (id: string) => {
    if (id.startsWith('stage:')) setStage(id.slice(6) as StudioStage);
    else if (id === 'run') setStage('simulate');
    else if (id === 'evidence') setShowEvidence(true);
    else if (id === 'sources') setShowSources(true);
    else if (id === 'assistant') setShowAssistant(true);
    else if (id === 'theme') toggleTheme();
    else if (id === 'new') navigate('/new');
    else if (id === 'home') navigate('/home');
  };

  const stepIndex = DEFINE_STEPS.findIndex(s => s.id === step);

  // What the canvas shows. While building it is the staged draft; once a revision is committed it is
  // the revision, driven by the stored run at the time the current stage needs.
  const isAssembling = stage === 'define' && step === 'assemble' && !rev;
  const committedView = !!rev && !isAssembling;
  const scene = isAssembling ? staged : committedView ? { parts: model.parts, connections: model.connections } : { parts: [], connections: [] };
  const duration = model.run?.duration ?? 12;
  const sceneT = stage === 'simulate' ? playback.t : stage === 'understand' ? model.failure?.t ?? duration : stage === 'improve' ? duration : 0;
  const failure = model.failure && model.failure.partId ? { partId: model.failure.partId, t: model.failure.t, text: `Over limit: ${model.failure.value} ${model.failure.requirement.unit}` } : null;
  const branchSeries = useMemo(() => {
    const v = model.requirements[0]?.variable;
    return v && model.series[v] ? { ...model.series, [v]: scriptedBranchTemperature(model.series[v], branch) } : model.series;
  }, [model.requirements, model.series, branch]);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-void text-ink">
      {/* Left navigation: 176 px, icons only when narrow (file 33 section 4) */}
      <aside className="w-[176px] max-[1100px]:w-[60px] h-full flex flex-col justify-between border-r border-line bg-deck shrink-0 z-30">
        <div>
          <button onClick={() => navigate('/home')} className="h-16 w-full px-4 flex items-center border-b border-line text-left" title="Back to Home" aria-label="Back to Home">
            <span className="max-[1100px]:hidden"><Logo size="sm" /></span>
            <span className="hidden max-[1100px]:block"><Logo size="sm" markOnly /></span>
          </button>
          <nav className="p-2.5 space-y-0.5" aria-label="Stages">
            {STAGES.map(s => {
              const on = stage === s.id;
              return (
                <button
                  key={s.id}
                  onClick={() => setStage(s.id)}
                  aria-current={on ? 'page' : undefined}
                  title={s.label}
                  className={`relative w-full h-9 flex items-center gap-3 px-3 rounded-[8px] text-[13px] transition-colors ${on ? 'bg-white/[0.07] text-strong font-medium' : 'text-muted hover:text-ink hover:bg-white/[0.04]'}`}
                >
                  {on && <span className="absolute left-0 top-2 bottom-2 w-[2px] rounded-full bg-accent" />}
                  <span className={on ? 'text-accent' : ''}>{s.icon}</span>
                  <span className="max-[1100px]:hidden flex-1 text-left truncate">{s.label}</span>
                  {s.preview && <span className="max-[1100px]:hidden"><PreviewTag /></span>}
                  {s.id === 'define' && !rev && <span className="max-[1100px]:hidden text-[10px] text-muted">Draft</span>}
                  {s.id === 'model' && rev && <span className="max-[1100px]:hidden num text-[10px] text-muted">{rev}</span>}
                </button>
              );
            })}
          </nav>
        </div>

        <div className="p-3 space-y-3 border-t border-line">
          <Button variant="assistant" className="w-full max-[1100px]:px-0" onClick={() => setShowAssistant(v => !v)} icon={<Sparkles size={14} />}>
            <span className="max-[1100px]:hidden">Assistant</span>
          </Button>
          <div className="px-1 flex items-center gap-2 text-[11px] text-muted max-[1100px]:justify-center">
            <StatusDot tone={engineMode === 'mock' ? 'warn' : 'ok'} />
            <span className="max-[1100px]:hidden">Engine: {engineMode === 'mock' ? 'Mock' : 'HELQIX API'}</span>
          </div>
        </div>
      </aside>

      <div className="flex-1 min-w-0 flex flex-col h-full overflow-hidden relative">
        {/* Header card: where am I, what state, what next */}
        <header className="h-16 px-5 flex items-center justify-between gap-4 border-b border-line bg-deck shrink-0 z-20">
          <div className="flex items-center gap-3 min-w-0 flex-1">
            <div className="w-8 h-8 rounded-[9px] bg-white/[0.05] border border-line flex items-center justify-center text-muted shrink-0"><Box size={15} /></div>
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                {renaming ? (
                  <input autoFocus defaultValue={name} aria-label="Project name" onBlur={e => commitName(e.target.value)} onKeyDown={e => { if (e.key === 'Enter') commitName(e.currentTarget.value); if (e.key === 'Escape') setRenaming(false); }} className="h-7 w-[220px] rounded-[7px] bg-void/70 border border-accent/60 px-2 text-[14px] text-strong focus:outline-none" />
                ) : (
                  <h1 className="text-[14px] font-medium text-strong truncate"><button onClick={() => setRenaming(true)} title="Rename project" className="hover:text-accent transition-colors text-left">{name}</button></h1>
                )}
                {engineMode === 'mock' && <MockTag />}
              </div>
              <div className="flex items-center gap-2 text-[11px] text-muted" data-testid="revision-state">
                {rev ? (
                  <><Check size={12} className="text-ok" />{`Model ${rev} (committed)`}</>
                ) : (
                  'Draft - not committed'
                )}
                {stagedParts && stagedParts.length > 0 && <Chip className="!h-5">{stagedParts.length} parts</Chip>}
              </div>
            </div>
          </div>

          {stage === 'define' && (
            <ol className="hidden md:flex items-center gap-1 shrink-0" aria-label="Define steps">
              {DEFINE_STEPS.map((s, i) => {
                const on = step === s.id;
                const done = i < stepIndex;
                return (
                  <li key={s.id} className="flex items-center gap-1">
                    <button
                      onClick={() => setStep(s.id)}
                      aria-current={on ? 'step' : undefined}
                      className={`h-8 pl-1.5 pr-3 rounded-full flex items-center gap-2 text-xs transition-colors ${on ? 'bg-white/[0.07] text-strong' : 'text-muted hover:text-ink'}`}
                    >
                      <span className={`num w-5 h-5 rounded-full inline-flex items-center justify-center text-[10px] ${on ? 'bg-accent text-white' : done ? 'bg-ok/20 text-ok' : 'border border-line-strong'}`}>
                        {done ? <Check size={11} /> : i + 1}
                      </span>
                      <span className={on ? '' : 'max-[1260px]:hidden'}>{s.label}</span>
                    </button>
                    {i < DEFINE_STEPS.length - 1 && <span aria-hidden className="w-4 h-px bg-line-strong" />}
                  </li>
                );
              })}
            </ol>
          )}

          <div className="flex items-center gap-2 shrink-0">
            <Tabs
              tabs={[{ id: 'Realistic', label: 'Realistic' }, { id: 'Engineering', label: 'Engineering' }]}
              activeTab={session.look}
              onChange={id => session.setLook(id as 'Realistic' | 'Engineering')}
              className="max-[1280px]:hidden"
            />
            {primaryAction && (
              <Button variant="primary" onClick={primaryAction.run} data-testid="primary-action">
                {primaryAction.label}
              </Button>
            )}
            <span className="w-px h-5 bg-line mx-1" aria-hidden />
            <IconButton label="Command palette (Ctrl+K)" onClick={() => setShowCommands(true)}><Command size={16} /></IconButton>
            <IconButton label="Evidence" onClick={() => setShowEvidence(true)}><FileCheck2 size={16} /></IconButton>
            <IconButton label="Activity" onClick={() => setShowActivity(true)}><Bell size={16} /></IconButton>
            <IconButton label={theme === 'dark' ? 'Light theme' : 'Dark theme'} onClick={toggleTheme}>{theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}</IconButton>
            <IconButton label="Keyboard and mouse (?)" onClick={() => setShowShortcuts(true)}><HelpCircle size={16} /></IconButton>
            <IconButton label="Settings" onClick={() => setShowSettings(true)}><Settings size={16} /></IconButton>
          </div>
        </header>

        {/* Center: the Reality Canvas never unmounts; stages arrange panels around it. */}
        <div className="flex-1 relative overflow-hidden flex flex-col justify-between">
          <div className={`absolute inset-0 z-0 transition-[filter,opacity] duration-300 ${stage === 'define' && step !== 'assemble' ? 'blur-[7px] opacity-60' : ''}`}>
            <ModelScene
              parts={scene.parts}
              connections={scene.connections}
              bindings={committedView ? model.bindings : []}
              series={model.series}
              t={sceneT}
              compare={stage === 'improve' && model.run ? { series: branchSeries, label: 'Branch B1' } : null}
              primaryLabel="Original"
              failure={stage === 'simulate' || stage === 'understand' || stage === 'improve' ? failure : null}
              selectedId={selection.selectedIds[0] ?? null}
              focusId={stage === 'understand' ? selection.selectedIds[0] ?? model.failure?.partId ?? null : null}
              onSelect={id => (id ? selection.select(id) : selection.deselectAll())}
              look={session.look}
              cameraPreset={session.cameraPreset}
              qualityTier={session.qualityTier}
              draft={isAssembling}
              fitSignal={fitSignal}
              environmentAssetId={session.environment?.assetId ?? null}
              insets={stage === 'define' && step !== 'assemble' ? { left: 0, right: 0 } : { left: window.innerWidth < 1100 ? 240 : 288, right: window.innerWidth < 1100 ? 240 : 300 }}
            />
            {scene.parts.length > 0 && stage !== 'define' && (
              <div className="absolute bottom-3 right-1/2 translate-x-1/2 z-10">
                <Tabs tabs={(['Perspective', 'Top', 'Front', 'Right'] as const).map(p => ({ id: p, label: p }))} activeTab={session.cameraPreset} onChange={id => session.setCameraPreset(id as 'Perspective')} className="!bg-black/40 backdrop-blur" />
              </div>
            )}
            <div className="absolute top-4 left-4 z-10 flex flex-wrap items-center gap-2 pointer-events-none">
              <Chip className="!bg-black/40 backdrop-blur">{scene.parts.length} {scene.parts.length === 1 ? 'part' : 'parts'}</Chip>
              {scene.parts.length > 0 && <span title="Dimensions are drawn at 1 unit per 100 mm and positions at 1 unit per metre, so parts read clearly. It is a schematic, not a true-scale view."><Chip className="!bg-black/40 backdrop-blur">Schematic scale</Chip></span>}
              {committedView && ['simulate', 'understand', 'improve'].includes(stage) && (
                model.bindings.length === 0
                  ? <Chip tone="warn" className="!bg-black/40 backdrop-blur">No visual binding</Chip>
                  : model.bindings.some(b => b.channel === 'rotation') && <Chip className="!bg-black/40 backdrop-blur">Visual speed scaled 1:{VISUAL_SPEED_SCALE}</Chip>
              )}
            </div>
          </div>

          {stage === 'define' ? (
            <DefineStage
              projectId={projectId}
              currentStep={step}
              committedRev={rev}
              onStepChange={setStep}
              onCommitted={r => setQuery({ rev: `r${r.revisionNum}` }, { replace: true })}
              onRunSimulation={() => setStage('simulate')}
              onPartsChange={setStaged}
            />
          ) : stage === 'model' ? (
            <ModelStage rev={rev} />
          ) : stage === 'simulate' ? (
            <SimulateStage onInvestigateFailure={() => setStage('understand')} onForkBranch={() => setStage('improve')} />
          ) : stage === 'understand' ? (
            <UnderstandStage onTestThisCause={() => setStage('improve')} onBackToSimulate={() => setStage('simulate')} />
          ) : stage === 'improve' ? (
            <ImproveStage branch={branch} onBranchChange={setBranch} />
          ) : stage === 'verify' ? (
            <VerifyStage onOpenEvidenceSheet={() => setShowEvidence(true)} />
          ) : stage === 'observe' ? (
            <ObserveStage onProposeUpdate={() => setStage('improve')} />
          ) : stage === 'build' ? (
            <BuildStage />
          ) : (
            <div className="flex-1 p-6 text-center text-muted">Unknown stage</div>
          )}

          <AssistantBar isOpen={showAssistant} proposal={proposal} onClose={() => setShowAssistant(false)} />
        </div>
      </div>

      <CommandPalette isOpen={showCommands} onClose={() => setShowCommands(false)} commands={commands} onSelect={onCommand} />
      <EvidenceSheet isOpen={showEvidence} onClose={() => setShowEvidence(false)} />
      <SettingsSheet isOpen={showSettings} onClose={() => setShowSettings(false)} onToggleTheme={toggleTheme} currentTheme={theme} />
      <ActivityDrawer isOpen={showActivity} onClose={() => setShowActivity(false)} projectId={projectId} />
      <ShortcutsSheet isOpen={showShortcuts} onClose={() => setShowShortcuts(false)} />
      <SourcesManagerSheet isOpen={showSources} onClose={() => setShowSources(false)} projectId={projectId} />
    </div>
  );
};
