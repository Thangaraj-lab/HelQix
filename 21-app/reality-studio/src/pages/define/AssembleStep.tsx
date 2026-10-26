import React, { useState, useEffect, useRef } from 'react';
import {
  BuildEvent,
  ModelOp,
  ModelRevision,
  Issue,
  Value,
  Source
} from '../../contracts';
import { MockEngineClient } from '../../mock/MockEngineClient';
import { createMockValue, MOCK_SOURCES } from '../../mock/examplePack';
import { MockTag, PlaceholderTag, ExampleTag, Panel, ValueView } from '../../components/ui/UiKit';

export interface AssembleStepProps {
  onCommitSuccess: (revision: ModelRevision) => void;
  onRunSimulation?: () => void;
  onStartScratch?: boolean;
}

export const AssembleStep: React.FC<AssembleStepProps> = ({
  onCommitSuccess,
  onRunSimulation,
  onStartScratch = false
}) => {
  const [engine] = useState(() => MockEngineClient);
  const [buildId, setBuildId] = useState<string>('');
  const [events, setEvents] = useState<BuildEvent[]>([]);
  const [appliedOps, setAppliedOps] = useState<ModelOp[]>([]);
  const [activeStep, setActiveStep] = useState<string>('interpret');
  const [playbackSpeed, setPlaybackSpeed] = useState<number>(1);
  const [isPaused, setIsPaused] = useState<boolean>(false);
  const [isStagedReady, setIsStagedReady] = useState<boolean>(false);
  const [isCommitted, setIsCommitted] = useState<boolean>(false);
  const [committedRevision, setCommittedRevision] = useState<ModelRevision | null>(null);

  // Model Tree state
  interface StagedPart {
    id: string;
    name: string;
    geometry: { kind: string; shape?: string; dims?: Value[] };
    maturity: string;
    transform?: { pos: [Value, Value, Value]; rot: [Value, Value, Value]; scale: [number, number, number] };
    status: 'draft' | 'placed' | 'connected';
    ports?: { id: string; kind: string; direction: 'in' | 'out' | 'bi'; unit?: string }[];
    params?: Record<string, Value>;
  }

  const [parts, setParts] = useState<Map<string, StagedPart>>(new Map());
  const [connections, setConnections] = useState<Array<{ id: string; from: string; to: string; kind: string }>>([]);
  const [issues, setIssues] = useState<Issue[]>([]);
  const [selectedPartId, setSelectedPartId] = useState<string | null>(null);
  const [bottomTab, setBottomTab] = useState<'issues' | 'connections' | 'sources' | 'history'>('issues');

  // Scratch Definition Modal State (Section 6.8: Only opens when user clicks button)
  const [showScratchDrawer, setShowScratchDrawer] = useState<boolean>(false);
  const [scratchName, setScratchName] = useState<string>('Custom Module');
  const [scratchShape, setScratchShape] = useState<'box' | 'cylinder' | 'sphere' | 'plate'>('box');
  const [scratchDimX, setScratchDimX] = useState<number>(120);
  const [scratchDimY, setScratchDimY] = useState<number>(80);
  const [scratchDimZ, setScratchDimZ] = useState<number>(60);
  const [scratchPortName, setScratchPortName] = useState<string>('power-in');
  const [scratchPortKind, setScratchPortKind] = useState<string>('electrical_dc');
  const [scratchPortDir, setScratchPortDir] = useState<'in' | 'out' | 'bi'>('in');
  const [scratchPortUnit, setScratchPortUnit] = useState<string>('V');

  // Placement Gizmo / Tool Settings (Section 6.8)
  const [gridSnapMm, setGridSnapMm] = useState<number>(1);
  const [rotSnapDeg, setRotSnapDeg] = useState<number>(15);

  const eventStreamRef = useRef<BuildEvent[]>([]);
  const isPlayingRef = useRef<boolean>(true);
  isPlayingRef.current = !isPaused;

  // Initialize build session
  useEffect(() => {
    let unmounted = false;

    async function initBuild() {
      const prj = await engine.createProject();
      const storedPrompt = typeof sessionStorage !== 'undefined' ? sessionStorage.getItem('helqix_initial_prompt') : null;
      const input = onStartScratch
        ? { scratch: true as const }
        : { prompt: storedPrompt || 'Assemble staged system architecture' };
      const { buildId: newBuildId } = await engine.startBuild(prj.id, input);
      if (unmounted) return;
      setBuildId(newBuildId);

      // Collect all events from mock generator
      const collected: BuildEvent[] = [];
      for await (const evt of engine.buildEvents(newBuildId)) {
        collected.push(evt);
      }
      eventStreamRef.current = collected;

      // Start stepping through events
      playEvents(collected, newBuildId);
    }

    initBuild();

    return () => {
      unmounted = true;
    };
  }, [engine, onStartScratch]);

  // Live Event Playback Engine
  const playEvents = async (allEvents: BuildEvent[], bId: string) => {
    for (let i = 0; i < allEvents.length; i++) {
      const evt = allEvents[i];

      // Delay between events according to playbackSpeed
      const delay = Math.max(120, Math.floor(450 / playbackSpeed));
      await new Promise(r => setTimeout(r, delay));

      applyEventToState(evt);
    }
  };

  const applyEventToState = (evt: BuildEvent) => {
    setEvents(prev => [...prev, evt]);

    if (evt.step) {
      setActiveStep(evt.step);
    }

    if (evt.kind === 'op.applied' && evt.op) {
      const op = evt.op;
      setAppliedOps(prev => [...prev, op]);

      if (op.op === 'part.add') {
        setParts(prev => {
          const next = new Map(prev);
          next.set(op.id, {
            id: op.id,
            name: op.name,
            geometry: op.geometry,
            maturity: op.maturity,
            status: 'draft',
            ports: [],
            params: {}
          });
          return next;
        });
        setSelectedPartId(op.id);
      } else if (op.op === 'part.place') {
        setParts(prev => {
          const next = new Map(prev);
          const p = next.get(op.id);
          if (p) {
            next.set(op.id, {
              ...p,
              transform: op.transform,
              status: 'placed'
            });
          }
          return next;
        });
      } else if (op.op === 'connection.add') {
        setConnections(prev => [
          ...prev,
          {
            id: op.id,
            from: `${op.from.partId}:${op.from.portId}`,
            to: `${op.to.partId}:${op.to.portId}`,
            kind: op.kind
          }
        ]);
        setParts(prev => {
          const next = new Map(prev);
          const pFrom = next.get(op.from.partId);
          if (pFrom) pFrom.status = 'connected';
          const pTo = next.get(op.to.partId);
          if (pTo) pTo.status = 'connected';
          return next;
        });
      } else if (op.op === 'part.setParam') {
        setParts(prev => {
          const next = new Map(prev);
          const p = next.get(op.id);
          if (p) {
            next.set(op.id, {
              ...p,
              params: {
                ...(p.params || {}),
                [op.key]: op.value
              }
            });
          }
          return next;
        });
      }
    } else if (evt.kind === 'issue.raised' && evt.issue) {
      setIssues(prev => [...prev, evt.issue!]);
    } else if (evt.kind === 'staged.ready') {
      setIsStagedReady(true);
    }
  };

  // Step Forward
  const handleStepForward = () => {
    const nextIdx = events.length;
    if (nextIdx < eventStreamRef.current.length) {
      applyEventToState(eventStreamRef.current[nextIdx]);
    }
  };

  // Skip To End
  const handleSkipToEnd = () => {
    const remaining = eventStreamRef.current.slice(events.length);
    remaining.forEach(applyEventToState);
  };

  // Handle Undo (calls EngineClient.undo)
  const handleUndo = async () => {
    if (!buildId) return;
    await engine.undo(buildId);
    setAppliedOps(prev => prev.slice(0, -1));
  };

  // Handle Redo (calls EngineClient.redo)
  const handleRedo = async () => {
    if (!buildId) return;
    await engine.redo(buildId);
  };

  // Handle Issue Fix (Section 6.7: emits corrected op)
  const handleFixIssue = async (issue: Issue) => {
    if (!buildId || !issue.fixOp) return;
    const newEvents = await engine.applyOps(buildId, [issue.fixOp]);
    newEvents.forEach(applyEventToState);
    setIssues(prev => prev.filter(iss => iss.id !== issue.id));
  };

  // Handle Commit Model Revision r1 (Section 6.7)
  const handleCommitRevision = async () => {
    if (!buildId) return;
    const rev = await engine.commit(buildId);
    setIsCommitted(true);
    setCommittedRevision(rev);
    onCommitSuccess(rev);
  };

  // Scratch Add Part handler (Section 6.8)
  const handleAddScratchPart = async () => {
    if (!buildId) return;
    const partId = `part-${Date.now().toString(36)}`;
    const addOp: ModelOp = {
      op: 'part.add',
      id: partId,
      name: scratchName,
      geometry: {
        kind: 'primitive',
        shape: scratchShape,
        dims: [
          createMockValue(scratchDimX, 'mm'),
          createMockValue(scratchDimY, 'mm'),
          createMockValue(scratchDimZ, 'mm')
        ]
      },
      maturity: 'user_defined',
      prov: { kind: 'user_stated', note: 'Created via Scratch Path' }
    };

    const placeOp: ModelOp = {
      op: 'part.place',
      id: partId,
      transform: {
        pos: [createMockValue(0, 'm'), createMockValue(0.5, 'm'), createMockValue(0, 'm')],
        rot: [createMockValue(0, 'deg'), createMockValue(0, 'deg'), createMockValue(0, 'deg')],
        scale: [1, 1, 1],
        frame: 'world'
      }
    };

    const portOp: ModelOp = {
      op: 'port.add',
      part: partId,
      id: scratchPortName,
      kind: scratchPortKind,
      direction: scratchPortDir,
      unit: scratchPortUnit
    };

    const newEvents = await engine.applyOps(buildId, [addOp, placeOp, portOp]);
    newEvents.forEach(applyEventToState);
    setShowScratchDrawer(false);
  };

  // Build log checklist definitions
  const checklistSteps = [
    { id: 'interpret', label: 'Resolving specifications & constraints', count: '1 brief parsed' },
    { id: 'resolve', label: 'Importing geometry & models', count: `${parts.size} parts resolved` },
    { id: 'place', label: 'Placing components on coordinates', count: `${Array.from(parts.values()).filter(p => p.status !== 'draft').length} of ${parts.size} placed` },
    { id: 'connect', label: 'Routing electrical, mechanical & signal ports', count: `${connections.length} routes active` },
    { id: 'check', label: 'Verifying engineering units & behavior', count: `${issues.length} warnings raised` }
  ];

  const selectedPart = selectedPartId ? parts.get(selectedPartId) : null;
  const hasBlockingIssues = issues.some(i => i.severity === 'blocking');

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-transparent z-10">
      {/* 1. TOP STATUS STRIP (Draft - not committed vs r1) */}
      <div className="h-11 px-5 flex items-center justify-between border-b border-[#22395A]/80 bg-[#0A1422]/90 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-[#F2F6FB]">Assembly Stage</span>
            <span
              className={`px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider border ${
                isCommitted
                  ? 'bg-[#22C55E]/15 border-[#22C55E]/40 text-[#4ADE80]'
                  : 'bg-amber-500/15 border-amber-500/40 text-amber-300'
              }`}
            >
              {isCommitted ? 'Model r1 · Committed' : 'Draft - not committed'}
            </span>
          </div>
          <span className="text-[#22395A]">|</span>
          <div className="flex items-center gap-2 text-[11px] font-mono text-[#7F93AF]">
            <span>Build events: <strong className="text-white">{events.length}</strong></span>
            <span>•</span>
            <span>Ops: <strong className="text-cyan-400">{appliedOps.length}</strong></span>
            <span>•</span>
            <span>Parts: <strong className="text-white">{parts.size}</strong></span>
          </div>
        </div>

        {/* Right Action Buttons */}
        <div className="flex items-center gap-2.5">
          <button
            onClick={() => setShowScratchDrawer(prev => !prev)}
            className="px-3 py-1 rounded-[6px] text-xs font-mono font-medium text-[#22D3EE] bg-[#22D3EE]/10 border border-[#22D3EE]/30 hover:bg-[#22D3EE]/20 transition-colors flex items-center gap-1.5"
          >
            <span>+</span>
            <span>Define a part (Scratch)</span>
          </button>

          {isCommitted ? (
            <button
              onClick={onRunSimulation}
              className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-[#22C55E] hover:bg-[#16A34A] shadow-[0_0_20px_rgba(34,197,94,0.4)] transition-all flex items-center gap-1.5"
            >
              <span>▶</span>
              <span>Run simulation</span>
            </button>
          ) : (
            <button
              onClick={handleCommitRevision}
              disabled={!isStagedReady || hasBlockingIssues}
              className={`px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white transition-all flex items-center gap-1.5 ${
                isStagedReady && !hasBlockingIssues
                  ? 'bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_20px_rgba(47,123,255,0.4)] cursor-pointer'
                  : 'bg-[#1E293B] text-slate-500 cursor-not-allowed border border-[#334155]'
              }`}
              title={
                !isStagedReady
                  ? 'Waiting for assembly steps to complete...'
                  : hasBlockingIssues
                  ? 'Resolve blocking issues before committing'
                  : 'Commit staged assembly into Model r1'
              }
            >
              <span>✓</span>
              <span>Create model r1</span>
            </button>
          )}
        </div>
      </div>

      {/* 2. THREE-COLUMN WORKSPACE BODY */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT COLUMN: BUILD LOG CHECKLIST & DYNAMIC MODEL TREE */}
        <div className="w-80 border-r border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0">
          {/* Top: Build Log Checklist (Section 6.7) */}
          <div className="p-4 border-b border-[#22395A] space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Build Log Checklist
              </span>
              <span className="text-[10px] font-mono text-[#22D3EE] bg-[#22D3EE]/10 px-1.5 py-0.5 rounded border border-[#22D3EE]/20">
                Live stream
              </span>
            </div>

            <div className="space-y-2">
              {checklistSteps.map((step, idx) => {
                const isCurrent = activeStep === step.id;
                const isDone = events.some(e => e.kind === 'step.done' && e.step === step.id) || isStagedReady;
                return (
                  <div
                    key={step.id}
                    className={`p-2 rounded-[8px] border transition-all ${
                      isCurrent
                        ? 'bg-[#2F7BFF]/15 border-[#2F7BFF]/50 text-white'
                        : isDone
                        ? 'bg-[#0E1B2E]/60 border-[#22395A] text-[#7F93AF]'
                        : 'bg-transparent border-transparent text-slate-600'
                    }`}
                  >
                    <div className="flex items-center justify-between text-xs">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[10px] text-slate-400">0{idx + 1}</span>
                        <span className={`font-medium ${isCurrent ? 'text-[#22D3EE]' : ''}`}>
                          {step.label}
                        </span>
                      </div>
                      {isDone ? (
                        <span className="text-[#22C55E] text-xs">✓</span>
                      ) : isCurrent ? (
                        <span className="w-2 h-2 rounded-full bg-[#22D3EE] animate-pulse"></span>
                      ) : (
                        <span className="text-slate-600 text-xs">○</span>
                      )}
                    </div>
                    <div className="text-[10px] text-[#7F93AF] font-mono mt-0.5 pl-5">
                      {step.count}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Playback & History Controls */}
            <div className="pt-2 border-t border-[#22395A]/60 flex items-center justify-between">
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setIsPaused(p => !p)}
                  className="px-2 py-1 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[11px] text-[#D6E2F3] border border-[#22395A]"
                  title="Pause / Resume playback"
                >
                  {isPaused ? '▶' : '⏸'}
                </button>
                <button
                  onClick={handleStepForward}
                  className="px-2 py-1 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[11px] text-[#D6E2F3] border border-[#22395A]"
                  title="Step forward +1 op"
                >
                  ⏭
                </button>
                <button
                  onClick={handleSkipToEnd}
                  className="px-2 py-1 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[10px] text-[#D6E2F3] border border-[#22395A]"
                  title="Skip to end of build"
                >
                  Fast-forward
                </button>
              </div>

              {/* Speed Multiplier */}
              <div className="flex items-center gap-1 text-[10px] font-mono">
                {([1, 2, 4] as const).map(spd => (
                  <button
                    key={spd}
                    onClick={() => setPlaybackSpeed(spd)}
                    className={`px-1.5 py-0.5 rounded ${
                      playbackSpeed === spd
                        ? 'bg-[#2F7BFF] text-white font-bold'
                        : 'bg-[#13253D] text-[#7F93AF] hover:text-white'
                    }`}
                  >
                    {spd}x
                  </button>
                ))}
              </div>
            </div>

            {/* Undo / Redo */}
            <div className="flex items-center justify-between text-[11px] font-mono pt-1 text-[#7F93AF]">
              <span>Build log playback (real op log)</span>
              <div className="flex items-center gap-1.5">
                <button onClick={handleUndo} className="hover:text-white underline">Undo</button>
                <span>/</span>
                <button onClick={handleRedo} className="hover:text-white underline">Redo</button>
              </div>
            </div>
          </div>

          {/* Bottom: Growing Model Tree (Section 6.7) */}
          <div className="p-4 flex-1 overflow-y-auto space-y-2">
            <div className="flex items-center justify-between pb-1">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Model Tree ({parts.size})
              </span>
              <span className="text-[10px] text-[#7F93AF] font-mono">Hierarchy</span>
            </div>

            {parts.size === 0 ? (
              <div className="text-xs text-slate-500 py-6 text-center italic">
                Awaiting components from build event stream...
              </div>
            ) : (
              <div className="space-y-1.5">
                {Array.from(parts.values()).map(part => {
                  const isSelected = selectedPartId === part.id;
                  const hasIssue = issues.some(i => i.partId === part.id);
                  return (
                    <div
                      key={part.id}
                      onClick={() => setSelectedPartId(part.id)}
                      className={`p-2.5 rounded-[8px] border cursor-pointer transition-all ${
                        isSelected
                          ? 'bg-[#2F7BFF]/20 border-[#2F7BFF] shadow-[0_0_12px_rgba(47,123,255,0.25)]'
                          : hasIssue
                          ? 'bg-amber-500/10 border-amber-500/40 text-amber-200'
                          : 'bg-[#0E1B2E]/60 border-[#22395A] hover:bg-[#13253D]/80 text-[#D6E2F3]'
                      }`}
                    >
                      <div className="flex items-center justify-between text-xs">
                        <span className="font-semibold truncate">{part.name}</span>
                        {part.maturity === 'placeholder' && <PlaceholderTag />}
                        {part.name.includes('(Example)') && <ExampleTag />}
                      </div>

                      <div className="flex items-center justify-between text-[10px] font-mono text-[#7F93AF] mt-1">
                        <span className="uppercase">{part.geometry?.shape || 'CAD Node'}</span>
                        <span
                          className={`px-1.5 py-0.2 rounded ${
                            part.status === 'connected'
                              ? 'bg-[#22C55E]/15 text-[#4ADE80]'
                              : part.status === 'placed'
                              ? 'bg-[#22D3EE]/15 text-[#22D3EE]'
                              : 'bg-amber-500/15 text-amber-300'
                          }`}
                        >
                          {part.status}
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* CENTER COLUMN: REALITY CANVAS OVERLAY / HUD */}
        <div className="flex-1 relative flex flex-col justify-between p-4 pointer-events-none">
          {/* Top Floating HUD: Part Count & Snapping Info */}
          <div className="flex items-center justify-between pointer-events-auto">
            <div className="p-3 rounded-[12px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] space-y-1">
              <div className="flex items-center gap-2">
                <span className="font-bold text-[#F2F6FB] text-xs font-mono uppercase tracking-wider">
                  REALITY CANVAS: ASSEMBLE
                </span>
                <MockTag />
              </div>
              <div className="text-[11px] text-[#7F93AF] font-mono">
                Transform snapping: Grid {gridSnapMm} mm · Rotation {rotSnapDeg}°
              </div>
            </div>

            {/* Quick Placement Gizmo Tools (Section 6.8) */}
            <div className="p-1.5 rounded-[10px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] flex items-center gap-1 text-xs font-mono">
              <button
                onClick={() => setGridSnapMm(prev => (prev === 1 ? 5 : 1))}
                className="px-2 py-1 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[#D6E2F3] border border-[#22395A]"
                title="Toggle grid snapping"
              >
                Snap: {gridSnapMm} mm
              </button>
              <button
                onClick={() => setRotSnapDeg(prev => (prev === 15 ? 45 : 15))}
                className="px-2 py-1 rounded bg-[#13253D] hover:bg-[#1E3A5F] text-[#D6E2F3] border border-[#22395A]"
                title="Toggle angle snapping"
              >
                Snap: {rotSnapDeg}°
              </button>
            </div>
          </div>

          {/* Staged Ready Banner Notification */}
          {isStagedReady && !isCommitted && (
            <div className="p-3 rounded-[12px] bg-[#2F7BFF]/20 border border-[#2F7BFF] backdrop-blur-md max-w-md mx-auto pointer-events-auto text-center space-y-1 animate-fade-in shadow-[0_0_30px_rgba(47,123,255,0.3)]">
              <div className="text-xs font-bold text-white flex items-center justify-center gap-1.5">
                <span>✦</span>
                <span>Assembly Staged & Ready</span>
              </div>
              <div className="text-[11px] text-[#D6E2F3]">
                Camera framed on full model. Review parameters and click <strong>Create model r1</strong> to commit.
              </div>
            </div>
          )}

          {/* Committed Banner Notification */}
          {isCommitted && (
            <div className="p-3 rounded-[12px] bg-[#22C55E]/20 border border-[#22C55E] backdrop-blur-md max-w-md mx-auto pointer-events-auto text-center space-y-1 animate-fade-in shadow-[0_0_30px_rgba(34,197,94,0.3)]">
              <div className="text-xs font-bold text-[#4ADE80] flex items-center justify-center gap-1.5">
                <span>✓</span>
                <span>Model Revision r1 Committed</span>
              </div>
              <div className="text-[11px] text-[#D6E2F3]">
                Solid geometry locked. Ready to run baseline physics simulation.
              </div>
            </div>
          )}

          {/* Bottom Tabs Drawer Bar (Section 6.7: Issues, Connections, Sources, History) */}
          <div className="p-3 rounded-[14px] bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] pointer-events-auto space-y-2">
            <div className="flex items-center justify-between border-b border-[#22395A]/80 pb-2">
              <div className="flex items-center gap-2">
                {(['issues', 'connections', 'sources', 'history'] as const).map(tab => (
                  <button
                    key={tab}
                    onClick={() => setBottomTab(tab)}
                    className={`px-3 py-1 rounded-[6px] text-xs font-mono font-medium transition-all ${
                      bottomTab === tab
                        ? 'bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/50'
                        : 'text-[#7F93AF] hover:text-white'
                    }`}
                  >
                    <span className="capitalize">{tab}</span>
                    {tab === 'issues' && issues.length > 0 && (
                      <span className="ml-1.5 px-1.5 py-0.2 rounded-full bg-amber-500/20 text-amber-300 text-[10px]">
                        {issues.length}
                      </span>
                    )}
                    {tab === 'connections' && (
                      <span className="ml-1.5 text-slate-400 text-[10px]">({connections.length})</span>
                    )}
                  </button>
                ))}
              </div>

              <div className="text-[11px] font-mono text-[#7F93AF]">
                {bottomTab === 'history' ? `${appliedOps.length} ops applied` : 'Studio Inspector'}
              </div>
            </div>

            {/* Tab Contents */}
            <div className="max-h-36 overflow-y-auto text-xs">
              {bottomTab === 'issues' && (
                <div className="space-y-1.5">
                  {issues.length === 0 ? (
                    <div className="text-slate-400 py-2 italic text-center">No active issues detected. All units and ports are compatible.</div>
                  ) : (
                    issues.map(iss => (
                      <div
                        key={iss.id}
                        className="p-2.5 rounded-[8px] bg-amber-500/10 border border-amber-500/40 flex items-center justify-between"
                      >
                        <div className="space-y-0.5">
                          <div className="font-semibold text-amber-200 flex items-center gap-2">
                            <span>⚠</span>
                            <span>{iss.message}</span>
                          </div>
                          <div className="text-[10px] font-mono text-[#7F93AF]">
                            Part: {iss.partId || 'System'} · Severity: {iss.severity}
                          </div>
                        </div>

                        {iss.fixOp && (
                          <button
                            onClick={() => handleFixIssue(iss)}
                            className="px-3 py-1 rounded-[6px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-colors"
                          >
                            Fix
                          </button>
                        )}
                      </div>
                    ))
                  )}
                </div>
              )}

              {bottomTab === 'connections' && (
                <div className="grid grid-cols-3 gap-2">
                  {connections.map(c => (
                    <div key={c.id} className="p-2 rounded-[8px] bg-[#13253D] border border-[#22395A] space-y-0.5 font-mono text-[11px]">
                      <div className="text-white font-medium truncate">{c.id}</div>
                      <div className="text-[#22D3EE] text-[10px]">{c.kind}</div>
                      <div className="text-[#7F93AF] text-[10px] truncate">{c.from} → {c.to}</div>
                    </div>
                  ))}
                </div>
              )}

              {bottomTab === 'sources' && (
                <div className="grid grid-cols-3 gap-2">
                  {MOCK_SOURCES.map(s => (
                    <div key={s.id} className="p-2 rounded-[8px] bg-[#13253D] border border-[#22395A] space-y-0.5 font-mono text-[11px]">
                      <div className="text-white font-semibold truncate">{s.name}</div>
                      <div className="text-[#7F93AF] text-[10px]">Status: <span className="text-[#22C55E]">{s.status}</span> · As of: {s.lastSync}</div>
                    </div>
                  ))}
                </div>
              )}

              {bottomTab === 'history' && (
                <div className="space-y-1 font-mono text-[10px] text-[#7F93AF]">
                  {appliedOps.map((op, idx) => (
                    <div key={idx} className="p-1 rounded bg-[#13253D]/50 border border-[#22395A]/40 flex items-center justify-between">
                      <span className="text-white">#{idx + 1} {op.op}</span>
                      <span className="text-[#22D3EE]">{('id' in op && op.id) || ('part' in op && op.part) || 'model'}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* RIGHT COLUMN: PROPOSALS, ISSUES, & INSPECTOR */}
        <div className="w-80 border-l border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 p-4 space-y-4 overflow-y-auto">
          {/* Selected Part Inspector (Section 6.7) */}
          <div className="space-y-3">
            <div className="flex items-center justify-between pb-1 border-b border-[#22395A]">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Component Inspector
              </span>
              <span className="text-[10px] font-mono text-[#22D3EE]">Properties</span>
            </div>

            {selectedPart ? (
              <div className="space-y-3">
                <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-1.5">
                  <div className="text-xs font-semibold text-white">{selectedPart.name}</div>
                  <div className="flex items-center gap-1.5">
                    <span className="text-[10px] font-mono text-[#7F93AF]">ID: {selectedPart.id}</span>
                    {selectedPart.maturity === 'placeholder' && <PlaceholderTag />}
                  </div>
                </div>

                {/* Dimensions with Units (Rule 6) */}
                <div className="space-y-1">
                  <div className="text-[11px] font-mono text-[#7F93AF]">Dimensions</div>
                  <div className="grid grid-cols-3 gap-1.5 text-center font-mono">
                    <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                      <div className="text-[9px] text-[#7F93AF]">X</div>
                      <div className="text-xs font-semibold text-white">120 mm</div>
                    </div>
                    <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                      <div className="text-[9px] text-[#7F93AF]">Y</div>
                      <div className="text-xs font-semibold text-white">80 mm</div>
                    </div>
                    <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                      <div className="text-[9px] text-[#7F93AF]">Z</div>
                      <div className="text-xs font-semibold text-white">40 mm</div>
                    </div>
                  </div>
                </div>

                {/* Transform Coordinates (Section 6.8: editable X/Y/Z) */}
                <div className="space-y-1">
                  <div className="text-[11px] font-mono text-[#7F93AF]">World Transform</div>
                  <div className="grid grid-cols-3 gap-1.5 text-center font-mono">
                    <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                      <div className="text-[9px] text-[#7F93AF]">Pos X</div>
                      <div className="text-xs font-semibold text-cyan-300">
                        {selectedPart.transform?.pos[0]?.v ?? 0} m
                      </div>
                    </div>
                    <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                      <div className="text-[9px] text-[#7F93AF]">Pos Y</div>
                      <div className="text-xs font-semibold text-cyan-300">
                        {selectedPart.transform?.pos[1]?.v ?? 0.5} m
                      </div>
                    </div>
                    <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                      <div className="text-[9px] text-[#7F93AF]">Pos Z</div>
                      <div className="text-xs font-semibold text-cyan-300">
                        {selectedPart.transform?.pos[2]?.v ?? 0} m
                      </div>
                    </div>
                  </div>
                </div>

                {/* Parameters with mandatory unit and provenance */}
                {selectedPart.params && Object.keys(selectedPart.params).length > 0 && (
                  <div className="space-y-1">
                    <div className="text-[11px] font-mono text-[#7F93AF]">Engineering Parameters</div>
                    {Object.entries(selectedPart.params).map(([k, val]) => (
                      <ValueView key={k} label={k} value={val} />
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="text-xs text-slate-500 py-6 text-center italic">
                Select a component in the Model Tree to inspect engineering parameters.
              </div>
            )}
          </div>

          {/* Proposals Card (Rule-based suggestions - Section 6.7) */}
          <div className="space-y-2 pt-2 border-t border-[#22395A]">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Rule-Based Proposals
              </span>
              <span className="text-[10px] font-mono text-purple-300 bg-purple-500/20 px-1.5 py-0.5 rounded">
                Suggestions
              </span>
            </div>

            <div className="p-3 rounded-[10px] bg-[#13253D]/80 border border-[#8B5CF6]/40 space-y-2">
              <div className="text-xs font-semibold text-white">
                Add Liquid Cooling Loop
              </div>
              <p className="text-[11px] text-[#D6E2F3]">
                Proposal based on thermal density check. Lowers peak temperature by ~14.2 °C.
              </p>
              <div className="flex items-center justify-end gap-2 pt-1">
                <button className="px-2.5 py-1 rounded text-[11px] font-medium text-[#7F93AF] hover:text-white">
                  Reject
                </button>
                <button className="px-2.5 py-1 rounded text-[11px] font-semibold text-white bg-[#8B5CF6] hover:bg-[#7C3AED]">
                  Apply
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 3. SCRATCH PATH MODAL DRAWER (Section 6.8: Define a Part) */}
      {showScratchDrawer && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-6">
          <div className="w-full max-w-lg rounded-[14px] bg-[#0E1B2E] border border-[#2F7BFF]/50 p-6 space-y-4 shadow-[0_0_40px_rgba(47,123,255,0.35)]">
            <div className="flex items-center justify-between border-b border-[#22395A] pb-3">
              <div className="flex items-center gap-2">
                <span className="text-white font-bold text-sm">Define a Part (Scratch Path)</span>
                <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-blue-500/20 text-cyan-300">
                  Section 6.8
                </span>
              </div>
              <button
                onClick={() => setShowScratchDrawer(false)}
                className="text-[#7F93AF] hover:text-white text-base"
              >
                ✕
              </button>
            </div>

            <div className="space-y-3 text-xs">
              <div>
                <label className="text-[#7F93AF] font-mono block mb-1">Part Name</label>
                <input
                  type="text"
                  value={scratchName}
                  onChange={e => setScratchName(e.target.value)}
                  className="w-full px-3 py-1.5 rounded-[8px] bg-[#13253D] border border-[#22395A] text-white focus:outline-none focus:border-[#2F7BFF]"
                />
              </div>

              <div>
                <label className="text-[#7F93AF] font-mono block mb-1">Primitive Geometry</label>
                <div className="grid grid-cols-4 gap-2">
                  {(['box', 'cylinder', 'sphere', 'plate'] as const).map(shape => (
                    <button
                      key={shape}
                      onClick={() => setScratchShape(shape)}
                      className={`py-1.5 rounded-[6px] font-mono capitalize transition-all border ${
                        scratchShape === shape
                          ? 'bg-[#2F7BFF]/30 border-[#2F7BFF] text-white font-bold'
                          : 'bg-[#13253D] border-[#22395A] text-[#7F93AF] hover:text-white'
                      }`}
                    >
                      {shape}
                    </button>
                  ))}
                </div>
              </div>

              <div>
                <label className="text-[#7F93AF] font-mono block mb-1">Dimensions (Units: mm)</label>
                <div className="grid grid-cols-3 gap-2">
                  <div>
                    <span className="text-[10px] text-[#7F93AF]">Width</span>
                    <input
                      type="number"
                      value={scratchDimX}
                      onChange={e => setScratchDimX(Number(e.target.value))}
                      className="w-full px-2 py-1 rounded bg-[#13253D] border border-[#22395A] text-white"
                    />
                  </div>
                  <div>
                    <span className="text-[10px] text-[#7F93AF]">Height</span>
                    <input
                      type="number"
                      value={scratchDimY}
                      onChange={e => setScratchDimY(Number(e.target.value))}
                      className="w-full px-2 py-1 rounded bg-[#13253D] border border-[#22395A] text-white"
                    />
                  </div>
                  <div>
                    <span className="text-[10px] text-[#7F93AF]">Depth</span>
                    <input
                      type="number"
                      value={scratchDimZ}
                      onChange={e => setScratchDimZ(Number(e.target.value))}
                      className="w-full px-2 py-1 rounded bg-[#13253D] border border-[#22395A] text-white"
                    />
                  </div>
                </div>
              </div>

              <div className="p-3 rounded-[8px] bg-[#13253D] border border-[#22395A] space-y-2">
                <div className="font-mono text-[11px] text-[#22D3EE]">Initial Engineering Port</div>
                <div className="grid grid-cols-4 gap-2">
                  <div>
                    <span className="text-[9px] text-[#7F93AF]">Port ID</span>
                    <input
                      type="text"
                      value={scratchPortName}
                      onChange={e => setScratchPortName(e.target.value)}
                      className="w-full px-2 py-1 rounded bg-[#0A1422] border border-[#22395A] text-white text-[11px]"
                    />
                  </div>
                  <div>
                    <span className="text-[9px] text-[#7F93AF]">Port Kind</span>
                    <select
                      value={scratchPortKind}
                      onChange={e => setScratchPortKind(e.target.value)}
                      className="w-full px-1.5 py-1 rounded bg-[#0A1422] border border-[#22395A] text-white text-[11px]"
                    >
                      <option value="electrical_dc">Electrical DC</option>
                      <option value="mechanical_torque">Mechanical Torque</option>
                      <option value="telemetry_signal">Telemetry Signal</option>
                      <option value="fluid_port">Fluid Port</option>
                    </select>
                  </div>
                  <div>
                    <span className="text-[9px] text-[#7F93AF]">Direction</span>
                    <select
                      value={scratchPortDir}
                      onChange={e => setScratchPortDir(e.target.value as any)}
                      className="w-full px-1.5 py-1 rounded bg-[#0A1422] border border-[#22395A] text-white text-[11px]"
                    >
                      <option value="in">Input</option>
                      <option value="out">Output</option>
                      <option value="bi">Bidirectional</option>
                    </select>
                  </div>
                  <div>
                    <span className="text-[9px] text-[#7F93AF]">Unit</span>
                    <input
                      type="text"
                      value={scratchPortUnit}
                      onChange={e => setScratchPortUnit(e.target.value)}
                      className="w-full px-2 py-1 rounded bg-[#0A1422] border border-[#22395A] text-white text-[11px]"
                    />
                  </div>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[#22395A]">
              <button
                onClick={() => setShowScratchDrawer(false)}
                className="px-4 py-2 rounded-[8px] text-xs font-medium text-[#7F93AF] hover:text-white"
              >
                Cancel
              </button>
              <button
                onClick={handleAddScratchPart}
                className="px-5 py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-colors shadow-[0_0_20px_rgba(47,123,255,0.4)]"
              >
                Place into Workspace →
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
