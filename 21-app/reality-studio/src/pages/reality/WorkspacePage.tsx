import React from 'react';
import { useAppState, WorkspaceMode } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { TimelineBar } from '../../components/timeline/TimelineBar';
import { DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';
import { MOCK_COMPONENTS } from '../../mock/engineeringData';

export const WorkspacePage: React.FC = () => {
  const {
    workspaceMode,
    setWorkspaceMode,
    runId,
    revisionHash,
    determinism,
    qualityTier,
    layers,
    toggleLayer,
    selectedComponentId,
    setSelectedComponentId
  } = useAppState();

  const modes: WorkspaceMode[] = ['Reality', 'Time', 'Causality', 'Branch', 'Verification'];

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header Mode Tabs & Run Information (Image 3) */}
      <div className="flex items-center justify-between z-20 shrink-0">
        {/* Mode Tabs */}
        <div className="flex items-center gap-1.5 p-1 rounded-xl glass-panel">
          {modes.map(mode => (
            <button
              key={mode}
              onClick={() => setWorkspaceMode(mode)}
              className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold transition-all flex items-center gap-1.5 ${
                workspaceMode === mode
                  ? 'bg-blue-600/40 text-cyan-300 border border-blue-400/50 shadow-[0_0_15px_rgba(47,123,255,0.4)]'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
              }`}
            >
              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400"></span>
              <span>{mode}</span>
            </button>
          ))}
        </div>

        {/* Run Metadata Chips */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2 px-3 py-1.5 rounded-xl glass-panel text-xs font-mono">
            <span className="text-slate-400">RUN:</span>
            <span className="text-cyan-300 font-semibold">{runId}</span>
            <span className="text-slate-600">|</span>
            <span className="text-slate-400">REV:</span>
            <span className="text-purple-300">{revisionHash}</span>
          </div>

          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Center 3D Reality Canvas with Floating Panels (Image 3) */}
      <div className="flex-1 relative rounded-2xl overflow-hidden border border-blue-500/20 shadow-[0_0_40px_rgba(5,11,24,0.8)]">
        {/* 3D Scene Viewport */}
        <div className="absolute inset-0">
          <HeroScene
            variant="workspace"
            interactive={true}
            qualityTier={qualityTier}
          />
        </div>

        {/* Right Floating Panels: System State, Components, Layers (Image 3) */}
        <div className="absolute right-3 top-3 bottom-3 w-72 flex flex-col gap-3 pointer-events-auto z-10 overflow-y-auto">
          {/* System State Panel */}
          <div className="glass-panel p-3.5 flex flex-col gap-2.5">
            <div className="flex items-center justify-between border-b border-slate-800/80 pb-2">
              <span className="text-xs font-bold text-white flex items-center gap-1.5">
                <svg className="w-3.5 h-3.5 text-cyan-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
                </svg>
                <span>System State</span>
              </span>
              <OriginStatusBadges origin="Simulated" status="Verified" />
            </div>

            <div className="space-y-2">
              {[
                { label: 'System Load', val: 82, unit: '%', color: 'bg-cyan-400' },
                { label: 'Core Temperature', val: 64, unit: '°C', color: 'bg-blue-400' },
                { label: 'Active Power Draw', val: 71, unit: 'kW', color: 'bg-indigo-400' },
                { label: 'Rotational Velocity', val: 55, unit: 'rad/s', color: 'bg-emerald-400' },
                { label: 'Overall Efficiency', val: 88, unit: '%', color: 'bg-cyan-300' }
              ].map(st => (
                <div key={st.label} className="space-y-1">
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="text-slate-400">{st.label}</span>
                    <span className="font-mono text-white num">{st.val} {st.unit}</span>
                  </div>
                  <div className="w-full h-1.5 rounded-full bg-slate-900 overflow-hidden">
                    <div className={`h-full ${st.color} rounded-full`} style={{ width: `${st.val}%` }} />
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Components List Panel */}
          <div className="glass-panel p-3.5 flex flex-col gap-2">
            <div className="flex items-center justify-between border-b border-slate-800/80 pb-2">
              <span className="text-xs font-bold text-white flex items-center gap-1.5">
                <svg className="w-3.5 h-3.5 text-cyan-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="2" y="2" width="20" height="8" rx="2" ry="2" />
                  <rect x="2" y="14" width="20" height="8" rx="2" ry="2" />
                  <line x1="6" y1="6" x2="6.01" y2="6" />
                  <line x1="6" y1="18" x2="6.01" y2="18" />
                </svg>
                <span>Components</span>
              </span>
              <span className="text-[10px] text-slate-400 font-mono">4 items</span>
            </div>

            <div className="space-y-1.5">
              {MOCK_COMPONENTS.map(cmp => (
                <button
                  key={cmp.id}
                  onClick={() => setSelectedComponentId(cmp.id)}
                  className={`w-full p-2 rounded-lg text-left text-xs transition-all flex items-center justify-between ${
                    selectedComponentId === cmp.id
                      ? 'bg-blue-600/30 border border-cyan-400/60 shadow-[0_0_10px_rgba(34,211,238,0.2)]'
                      : 'bg-slate-900/40 hover:bg-slate-800/40 border border-transparent'
                  }`}
                >
                  <div className="truncate font-medium text-slate-200">{cmp.name}</div>
                  <div className="flex items-center gap-1.5 shrink-0 ml-2">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        cmp.status === 'Critical'
                          ? 'bg-rose-500 shadow-[0_0_8px_#EF4444]'
                          : cmp.status === 'Warning'
                          ? 'bg-amber-400'
                          : 'bg-emerald-400'
                      }`}
                    />
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* Model Layers Toggle Panel */}
          <div className="glass-panel p-3.5 flex flex-col gap-2">
            <div className="flex items-center justify-between border-b border-slate-800/80 pb-2">
              <span className="text-xs font-bold text-white flex items-center gap-1.5">
                <svg className="w-3.5 h-3.5 text-cyan-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <polygon points="12 2 2 7 12 12 22 7 12 2" />
                  <polyline points="2 17 12 22 22 17" />
                  <polyline points="2 12 12 17 22 12" />
                </svg>
                <span>Layers</span>
              </span>
            </div>

            <div className="space-y-1.5">
              {(['structure', 'physics', 'state', 'flow', 'environment'] as (keyof typeof layers)[]).map(l => (
                <label
                  key={l}
                  className="flex items-center justify-between text-xs text-slate-300 hover:text-white cursor-pointer select-none p-1 rounded hover:bg-slate-800/30"
                >
                  <span className="capitalize">{l}</span>
                  <input
                    type="checkbox"
                    checked={layers[l]}
                    onChange={() => toggleLayer(l)}
                    className="accent-cyan-400 rounded cursor-pointer"
                  />
                </label>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Timeline Scrubber (Image 3) */}
      <TimelineBar showFreezeBtn={true} showHashMatch={false} />
    </div>
  );
};
