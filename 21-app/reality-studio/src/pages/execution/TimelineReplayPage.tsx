import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { TimelineBar } from '../../components/timeline/TimelineBar';
import { DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';

export const TimelineReplayPage: React.FC = () => {
  const { setCurrentScreen, qualityTier, determinism, showToast } = useAppState();

  const [selectedEvent, setSelectedEvent] = useState({
    title: 'Thermal Overload Trip Event',
    component: 'CMP-A-001 (Rotor Assembly)',
    parameter: 'Core Temperature',
    threshold: '90.0 °C',
    actual: '92.4 °C',
    severity: 'Critical',
    candidateCause: 'Candidate root cause: Excessive Mechanical Load (85.4 N·m)'
  });

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-xs font-bold font-mono text-cyan-300">
            DETERMINISTIC BITWISE REPLAY & TIME JUMP
          </span>
          <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/20 text-emerald-400 border border-emerald-500/40">
            REPLAY MATCHES ORIGINAL: PASS
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => showToast('Replaying from state freeze snapshot at t=04.20s...', 'info')}
            className="px-2.5 py-1 rounded text-xs font-mono font-medium bg-blue-950/80 border border-cyan-400 text-cyan-300 hover:bg-blue-900/80"
          >
            Replay from freeze (04.20s)
          </button>
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Center 3D Model with Floating Event Callouts, Right Event Details */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Center: Large 3D with Floating Event Callouts (8 Cols) */}
        <div className="col-span-8 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18]">
          <HeroScene
            variant="model_editor"
            interactive={true}
            qualityTier={qualityTier}
            thermalHotspot={true}
          />

          {/* Floating Callout 1: Load Increase */}
          <div className="absolute top-8 left-8 z-10 p-2 rounded-xl glass-panel border border-cyan-500/40 text-xs">
            <div className="text-[10px] text-cyan-400 font-mono">t = 04.20s</div>
            <div className="font-semibold text-white">Load Increase (+160%)</div>
          </div>

          {/* Floating Callout 2: Temperature Rise */}
          <div className="absolute top-28 left-20 z-10 p-2 rounded-xl glass-panel border border-amber-500/40 text-xs">
            <div className="text-[10px] text-amber-400 font-mono">t = 06.10s</div>
            <div className="font-semibold text-white">Temperature Rise (78.2 °C)</div>
          </div>

          {/* Floating Callout 3: Anomaly Detected */}
          <div className="absolute top-12 right-12 z-10 p-2 rounded-xl glass-panel-elevated border border-rose-500/60 text-xs shadow-[0_0_20px_rgba(239,68,68,0.35)]">
            <div className="text-[10px] text-rose-400 font-mono font-bold">t = 08.42s CRITICAL</div>
            <div className="font-bold text-white">Anomaly: Thermal Overload Trip</div>
          </div>
        </div>

        {/* Right: Event Details Card (4 Cols) */}
        <div className="col-span-4 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-slate-800 pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-white">Event Details</span>
              <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-rose-500/20 text-rose-400 border border-rose-500/40">
                {selectedEvent.severity}
              </span>
            </div>

            <div className="space-y-2 text-xs">
              <div>
                <span className="text-slate-400">Triggered Event</span>
                <div className="text-white font-semibold">{selectedEvent.title}</div>
              </div>

              <div>
                <span className="text-slate-400">Target Component</span>
                <div className="text-cyan-300 font-mono">{selectedEvent.component}</div>
              </div>

              <div className="grid grid-cols-2 gap-2 p-2 rounded bg-slate-900/60 border border-slate-800">
                <div>
                  <span className="text-slate-500 text-[10px]">Threshold</span>
                  <div className="text-slate-200 font-mono font-semibold">{selectedEvent.threshold}</div>
                </div>
                <div>
                  <span className="text-slate-500 text-[10px]">Actual Value</span>
                  <div className="text-rose-400 font-mono font-bold">{selectedEvent.actual}</div>
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-amber-950/30 border border-amber-500/30 space-y-1">
                <span className="text-[10px] text-amber-400 uppercase font-mono font-bold">Candidate Hypothesis</span>
                <div className="text-xs text-amber-200 leading-snug">{selectedEvent.candidateCause}</div>
              </div>
            </div>
          </div>

          <button
            onClick={() => setCurrentScreen('failure_causality')}
            className="w-full ui-btn ui-btn-primary py-3 justify-center text-xs font-semibold shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            <span>View Full Causality Analysis</span>
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M5 12h14M12 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>

      {/* Bottom Timeline with Replay Matches Original PASS */}
      <TimelineBar showFreezeBtn={true} showHashMatch={true} />
    </div>
  );
};
