import React, { useState } from 'react';
import { PreviewTag, MockTag, ValueView, Panel } from '../../components/ui/UiKit';

export interface ObserveStageProps {
  onProposeUpdate?: () => void;
}

export const ObserveStage: React.FC<ObserveStageProps> = ({ onProposeUpdate }) => {
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>('dev-1');

  const devices = [
    { id: 'dev-1', name: 'Physical Test-Bench Alpha', status: 'Connected · 100 Hz', twinSync: 'Aligned' },
    { id: 'dev-2', name: 'Hardware-in-the-Loop Rack 3', status: 'Standby', twinSync: 'Idle' }
  ];

  const residuals = [
    { label: 'Temperature Residual (ΔT)', val: '+2.4 °C', note: 'Higher convective cooling in chamber than model' },
    { label: 'Shaft Vibration RMS', val: '0.12 g', note: 'Within nominal vibration ISO envelope' },
    { label: 'Phase Inrush Difference', val: '-1.8 A', note: 'Supply cable impedance slightly higher' }
  ];

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-transparent z-10 pointer-events-none select-none">
      {/* 1. TOP OBSERVE BAR (Section 6.15) */}
      <div className="h-11 px-5 flex items-center justify-between border-b border-[#22395A]/80 bg-[#0A1422]/90 backdrop-blur-md pointer-events-auto shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-[#F2F6FB]">Observe Stage</span>
            <PreviewTag label="PREVIEW" />
            <MockTag />
          </div>

          <span className="text-[#22395A]">|</span>

          <span className="text-[11px] font-mono text-[#7F93AF]">
            Digital Twin Telemetry Sync · Ingest: <strong className="text-emerald-400">Live 100 Hz</strong>
          </span>
        </div>

        {/* Action Button: Propose Model Update */}
        <div className="flex items-center gap-2.5">
          <button
            onClick={onProposeUpdate}
            className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-gradient-to-r from-[#2F7BFF] to-[#8B5CF6] hover:from-[#3B82F6] hover:to-[#7C3AED] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all"
          >
            Propose model update →
          </button>
        </div>
      </div>

      {/* 2. THREE-COLUMN OBSERVE BODY */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT PANEL: DEVICE LIST (Section 6.15) */}
        <div className="w-72 border-r border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Physical Devices
              </span>
              <span className="text-[10px] font-mono text-emerald-400">1 Online</span>
            </div>

            <div className="space-y-2">
              {devices.map(dev => (
                <div
                  key={dev.id}
                  onClick={() => setSelectedDeviceId(dev.id)}
                  className={`p-3 rounded-[10px] border cursor-pointer transition-all space-y-1 ${
                    selectedDeviceId === dev.id
                      ? 'bg-[#2F7BFF]/20 border-[#2F7BFF] shadow-[0_0_12px_rgba(47,123,255,0.25)]'
                      : 'bg-[#0E1B2E]/60 border-[#22395A]'
                  }`}
                >
                  <div className="flex items-center justify-between text-xs font-semibold text-white">
                    <span>{dev.name}</span>
                    <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
                  </div>
                  <div className="flex items-center justify-between text-[10px] font-mono text-[#7F93AF]">
                    <span>{dev.status}</span>
                    <span className="text-cyan-300">{dev.twinSync}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="p-2.5 rounded-[8px] bg-[#13253D] border border-[#22395A] text-[10px] font-mono text-[#7F93AF]">
            Streaming sensor telemetry blended in real-time onto 3D CAD digital twin mesh.
          </div>
        </div>

        {/* CENTER VIEWPORT OVERLAY */}
        <div className="flex-1 relative flex flex-col justify-between p-4 pointer-events-none">
          <div className="p-3 rounded-[12px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] max-w-sm pointer-events-auto space-y-1">
            <div className="text-xs font-bold text-[#F2F6FB] uppercase font-mono tracking-wider">
              DIGITAL TWIN BLEND OVERLAY
            </div>
            <div className="text-[11px] text-[#7F93AF] font-mono">
              Solid: Simulated model · Dashed: Observed telemetry
            </div>
          </div>

          {/* Bottom Chart: Observed vs Simulated (Section 6.15) */}
          <div className="p-3 rounded-[14px] bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] pointer-events-auto space-y-2">
            <div className="flex items-center justify-between border-b border-[#22395A]/80 pb-2">
              <div className="flex items-center gap-3">
                <span className="text-xs font-bold text-white uppercase font-mono">
                  Observed vs Simulated Telemetry
                </span>
                <div className="flex items-center gap-2 text-[10px] font-mono">
                  <span className="flex items-center gap-1 text-cyan-300">
                    <span className="w-3 h-0.5 bg-cyan-400"></span>
                    <span>Simulated (Solid)</span>
                  </span>
                  <span className="flex items-center gap-1 text-purple-300">
                    <span className="w-3 h-0.5 border-t border-dashed border-purple-400"></span>
                    <span>Observed (Dashed)</span>
                  </span>
                </div>
              </div>
              <span className="text-[10px] font-mono text-[#7F93AF]">Sampling: 10 ms</span>
            </div>

            <div className="h-16 relative">
              <svg className="w-full h-full overflow-visible" viewBox="0 0 400 45">
                <polyline fill="none" stroke="#22d3ee" strokeWidth="2" points="0,35 100,32 200,25 300,18 400,15" />
                <polyline fill="none" stroke="#c084fc" strokeWidth="2" strokeDasharray="4 3" points="0,37 100,34 200,28 300,21 400,17" />
              </svg>
            </div>
          </div>
        </div>

        {/* RIGHT PANEL: RESIDUALS & HONESTY NOTICE (Section 6.15) */}
        <div className="w-80 border-l border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-4 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Residual Analysis
              </span>
              <span className="text-[10px] font-mono text-cyan-300">Delta</span>
            </div>

            <div className="space-y-2">
              {residuals.map((r, idx) => (
                <div key={idx} className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-1">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-white font-medium">{r.label}</span>
                    <span className="text-cyan-300 font-mono font-bold">{r.val}</span>
                  </div>
                  <div className="text-[10px] font-mono text-[#7F93AF]">{r.note}</div>
                </div>
              ))}
            </div>
          </div>

          {/* Mandatory Honesty Rule Notice (Section 6.15 & Rule 2) */}
          <div className="p-3.5 rounded-[12px] bg-[#13253D] border border-cyan-500/40 space-y-1">
            <div className="text-white font-bold font-mono text-xs flex items-center gap-1.5">
              <span>ℹ</span>
              <span>Twin Integrity Invariant</span>
            </div>
            <p className="text-[#D6E2F3] text-[11px]">
              "Observed data never overwrites the model."
            </p>
          </div>

          <button
            onClick={onProposeUpdate}
            className="w-full py-2.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-all shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            Propose Model Update (Branch)
          </button>
        </div>
      </div>
    </div>
  );
};
