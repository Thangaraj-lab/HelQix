import React, { useState } from 'react';
import { HeroScene } from '../../scene/HeroScene';
import { MockTag, ValueView, Panel } from '../../components/ui/UiKit';

export interface ImproveStageProps {
  onAdoptBranch?: () => void;
  onBackToUnderstand?: () => void;
}

export const ImproveStage: React.FC<ImproveStageProps> = ({
  onAdoptBranch,
  onBackToUnderstand
}) => {
  const [activeBranch, setActiveBranch] = useState<'Original' | 'Cooling +20 %' | 'Load -10 %' | 'New branch'>('Cooling +20 %');

  // Intervention Sliders state (Section 6.12)
  const [coolantFlow, setCoolantFlow] = useState<number>(3.0); // Baseline 2.4 L/min
  const [torqueLimit, setTorqueLimit] = useState<number>(75.0); // Baseline 85.4 N·m
  const [bypassActive, setBypassActive] = useState<boolean>(true);

  // Metric Deltas (Section 6.12)
  const metrics = [
    {
      label: 'Peak Winding Temp',
      original: '124.6 °C',
      branch: '105.8 °C',
      delta: '-18.8 °C',
      status: 'safe',
      unit: '°C'
    },
    {
      label: 'Thermal Margin',
      original: '-4.6 °C',
      branch: '+14.2 °C',
      delta: '+18.8 °C',
      status: 'safe',
      unit: '°C'
    },
    {
      label: 'Phase RMS Current',
      original: '60.5 A',
      branch: '56.3 A',
      delta: '-4.2 A',
      status: 'safe',
      unit: 'A'
    },
    {
      label: 'Mechanical Power Delivered',
      original: '3.12 kW',
      branch: '2.98 kW',
      delta: '-0.14 kW',
      status: 'neutral',
      unit: 'kW'
    }
  ];

  // Branches definition
  const branches = [
    { id: 'b0', name: 'R0 Baseline (Original)', peak: '124.6 °C', status: 'Failed at 8.42s' },
    { id: 'b1', name: 'R1 Cooling +25%', peak: '105.8 °C', status: 'Meets R1 (Model)' },
    { id: 'b2', name: 'R2 Load Derating -12%', peak: '108.4 °C', status: 'Meets R1 (Model)' },
    { id: 'b3', name: 'R3 Dual Optimization', peak: '98.2 °C', status: 'Meets R1 (Model)' }
  ];

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-transparent z-10 pointer-events-none select-none">
      {/* 1. TOP IMPROVE BAR (Section 6.12) */}
      <div className="h-11 px-5 flex items-center justify-between border-b border-[#22395A]/80 bg-[#0A1422]/90 backdrop-blur-md pointer-events-auto shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-[#F2F6FB]">Improve Stage</span>
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider bg-purple-500/15 border border-purple-500/40 text-purple-300">
              Intervention Branch: Cooling +20 %
            </span>
            <MockTag />
          </div>

          <span className="text-[#22395A]">|</span>

          {/* Branch Tabs */}
          <div className="flex items-center gap-1">
            {(['Original', 'Cooling +20 %', 'Load -10 %', 'New branch'] as const).map(tab => (
              <button
                key={tab}
                onClick={() => setActiveBranch(tab)}
                className={`px-2.5 py-0.5 rounded-[6px] text-xs font-mono transition-all ${
                  activeBranch === tab
                    ? 'bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/50'
                    : 'text-[#7F93AF] hover:text-white'
                }`}
              >
                {tab}
              </button>
            ))}
          </div>
        </div>

        {/* Action Button: Adopt Branch */}
        <div className="flex items-center gap-2.5">
          {onBackToUnderstand && (
            <button
              onClick={onBackToUnderstand}
              className="px-3 py-1 rounded-[6px] text-xs font-mono text-[#7F93AF] hover:text-white"
            >
              ← Back to cause
            </button>
          )}

          <button
            onClick={onAdoptBranch}
            className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-[#22C55E] hover:bg-[#16A34A] shadow-[0_0_20px_rgba(34,197,94,0.4)] transition-all flex items-center gap-1.5"
          >
            <span>✓</span>
            <span>Adopt branch into r2</span>
          </button>
        </div>
      </div>

      {/* 2. MAIN SPLIT VIEWPORT: LEFT INTERVENTIONS, CENTER DUAL 3D, RIGHT METRIC DELTAS */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT PANEL: INTERVENTION SLIDERS (Section 6.12) */}
        <div className="w-80 border-r border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-4 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Intervention Controls
              </span>
              <span className="text-[10px] font-mono text-[#22D3EE]">Forked @ 8.0s</span>
            </div>

            <div className="p-2.5 rounded-[8px] bg-[#13253D] border border-[#22395A] text-[11px] text-[#7F93AF] font-mono">
              Forked from freeze state checkpoint at <strong>t = 8.00 s</strong>. Original was tripped at 8.42 s.
            </div>

            {/* Slider 1: Coolant Flow Rate */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between text-xs">
                <span className="text-[#D6E2F3] font-medium">Coolant Flow Rate</span>
                <span className="text-cyan-300 font-mono font-bold">{coolantFlow.toFixed(1)} L/min</span>
              </div>
              <input
                type="range"
                min="1.5"
                max="4.5"
                step="0.1"
                value={coolantFlow}
                onChange={e => setCoolantFlow(parseFloat(e.target.value))}
                className="w-full h-1.5 bg-[#13253D] rounded-lg appearance-none cursor-pointer accent-[#2F7BFF]"
              />
              <div className="flex justify-between text-[9px] font-mono text-[#7F93AF]">
                <span>1.5 L/min</span>
                <span>Baseline: 2.4 L/min</span>
                <span>4.5 L/min</span>
              </div>
            </div>

            {/* Slider 2: Max Torque Limit */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between text-xs">
                <span className="text-[#D6E2F3] font-medium">Torque Derate Cap</span>
                <span className="text-cyan-300 font-mono font-bold">{torqueLimit.toFixed(1)} N·m</span>
              </div>
              <input
                type="range"
                min="60.0"
                max="95.0"
                step="0.5"
                value={torqueLimit}
                onChange={e => setTorqueLimit(parseFloat(e.target.value))}
                className="w-full h-1.5 bg-[#13253D] rounded-lg appearance-none cursor-pointer accent-[#2F7BFF]"
              />
              <div className="flex justify-between text-[9px] font-mono text-[#7F93AF]">
                <span>60 N·m</span>
                <span>Baseline: 85.4 N·m</span>
                <span>95 N·m</span>
              </div>
            </div>

            {/* Toggle: Bypass Loop */}
            <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
              <div>
                <div className="text-xs font-semibold text-white">Liquid Cooling Bypass</div>
                <div className="text-[10px] text-[#7F93AF]">Direct coolant impingement on stator</div>
              </div>
              <button
                onClick={() => setBypassActive(p => !p)}
                className={`px-3 py-1 rounded-[6px] text-xs font-mono font-semibold transition-all ${
                  bypassActive
                    ? 'bg-[#22C55E]/20 text-[#4ADE80] border border-[#22C55E]/50'
                    : 'bg-slate-800 text-slate-400 border border-slate-700'
                }`}
              >
                {bypassActive ? 'ACTIVE' : 'OFF'}
              </button>
            </div>
          </div>

          {/* Fact-based Verdict Card (Section 6.12 & Rule 2 & 10) */}
          <div className="p-3.5 rounded-[12px] bg-[#0E1B2E] border border-[#22C55E]/40 space-y-2 shadow-[0_0_20px_rgba(34,197,94,0.15)]">
            <div className="flex items-center gap-2">
              <span className="text-[#4ADE80] font-bold text-xs">VERDICT</span>
              <span className="text-[9px] px-1 rounded bg-[#22C55E]/20 text-[#4ADE80] font-mono">Fact</span>
            </div>
            <div className="text-xs font-semibold text-white">
              Meets R1 in the model. Not yet validated.
            </div>
            <p className="text-[10px] text-[#7F93AF]">
              Both statements are facts: in the mathematical physics model, temperature reaches 105.8 °C (under 120.0 °C threshold). Physical testbench validation remains required.
            </p>
          </div>
        </div>

        {/* CENTER VIEWPORT: DUAL PHOTOREAL SIDE-BY-SIDE WITH DIVERGENCE MARKER (Section 6.12) */}
        <div className="flex-1 grid grid-cols-2 gap-2 p-3 min-h-0 pointer-events-auto">
          {/* Left View: Original Run (Trips at 8.42s) */}
          <div className="relative rounded-[14px] overflow-hidden border border-red-500/40 bg-[#050B18] flex flex-col justify-between p-3">
            <div className="absolute inset-0">
              <HeroScene
                variant="failure"
                interactive={true}
                thermalHotspot={true}
              />
            </div>

            <div className="relative z-10 self-start px-2.5 py-1 rounded-[8px] bg-[#0A1422]/90 backdrop-blur-md border border-red-500/50 text-xs font-mono text-red-400 space-y-0.5">
              <div className="font-bold">Original Run (R0)</div>
              <div className="text-[10px] text-[#7F93AF]">Peak 124.6 °C · Trips at 8.42 s</div>
            </div>

            <div className="relative z-10 self-center px-3 py-1 rounded-full bg-red-950/80 border border-red-500/60 text-[10px] font-mono text-red-300">
              Overheat Trip at 8.42s
            </div>
          </div>

          {/* Right View: Intervention Branch (Runs safely past 12.0s) */}
          <div className="relative rounded-[14px] overflow-hidden border border-[#22D3EE]/50 bg-[#050B18] flex flex-col justify-between p-3 shadow-[0_0_30px_rgba(34,211,238,0.2)]">
            <div className="absolute inset-0">
              <HeroScene
                variant="workspace"
                interactive={true}
                thermalHotspot={false}
              />
            </div>

            <div className="relative z-10 self-start px-2.5 py-1 rounded-[8px] bg-[#0A1422]/90 backdrop-blur-md border border-[#22D3EE]/50 text-xs font-mono text-[#22D3EE] space-y-0.5">
              <div className="font-bold">Branch (Cooling +20%)</div>
              <div className="text-[10px] text-[#4ADE80]">Peak 105.8 °C · Safe at 12.0 s</div>
            </div>

            <div className="relative z-10 self-center px-3 py-1 rounded-full bg-cyan-950/80 border border-[#22D3EE]/60 text-[10px] font-mono text-cyan-300">
              Divergence @ t=4.20s · Safe
            </div>
          </div>
        </div>

        {/* RIGHT PANEL: METRIC DELTAS & VERDICT (Section 6.12) */}
        <div className="w-80 border-l border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-4 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Metric Deltas
              </span>
              <span className="text-[10px] font-mono text-[#4ADE80]">Resolved</span>
            </div>

            <div className="space-y-2">
              {metrics.map(m => (
                <div key={m.label} className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-1">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-[#D6E2F3] font-medium">{m.label}</span>
                    <span className="text-[#4ADE80] font-mono font-bold">{m.delta}</span>
                  </div>
                  <div className="flex items-center justify-between text-[10px] font-mono text-[#7F93AF]">
                    <span>Original: <span className="text-red-400">{m.original}</span></span>
                    <span>Branch: <span className="text-white font-bold">{m.branch}</span></span>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Primary Action Button: Adopt */}
          <button
            onClick={onAdoptBranch}
            className="w-full py-2.5 rounded-[8px] text-xs font-semibold text-white bg-[#22C55E] hover:bg-[#16A34A] transition-all shadow-[0_0_20px_rgba(34,197,94,0.4)]"
          >
            Adopt this branch into Model r2 →
          </button>
        </div>
      </div>

      {/* 3. BOTTOM OVERLAY CURVES & BRANCH STRIP (Section 6.12) */}
      <div className="h-20 px-5 border-t border-[#22395A]/80 bg-[#0A1422]/95 backdrop-blur-md flex items-center justify-between gap-4 pointer-events-auto shrink-0 z-20">
        {/* SVG Comparative Temperature Curves */}
        <div className="flex-1 flex items-center gap-3">
          <div className="text-[10px] font-mono text-[#7F93AF] space-y-0.5">
            <div className="flex items-center gap-1.5">
              <span className="w-2.5 h-0.5 bg-red-500"></span>
              <span className="text-red-400">Original (Trips 8.42s)</span>
            </div>
            <div className="flex items-center gap-1.5">
              <span className="w-2.5 h-0.5 bg-[#22D3EE]"></span>
              <span className="text-cyan-300">Branch (Peak 105.8 °C)</span>
            </div>
          </div>

          <div className="flex-1 h-12 relative">
            <svg className="w-full h-full overflow-visible" viewBox="0 0 400 40">
              {/* 120C Limit Line */}
              <line x1="0" y1="15" x2="400" y2="15" stroke="#ef4444" strokeWidth="1" strokeDasharray="3 3" />
              <text x="5" y="12" fill="#ef4444" fontSize="8" fontFamily="monospace">120 °C Limit</text>
              {/* Original Red curve */}
              <polyline fill="none" stroke="#ef4444" strokeWidth="1.5" points="0,35 100,32 200,28 280,15 320,6 360,2" />
              {/* Branch Cyan curve (stays below 120C) */}
              <polyline fill="none" stroke="#22d3ee" strokeWidth="2" points="0,35 100,32 200,28 280,24 350,22 400,22" />
              {/* Divergence Point Marker */}
              <circle cx="140" cy="30" r="3" fill="#8b5cf6" />
              <text x="145" y="32" fill="#8b5cf6" fontSize="7" fontFamily="monospace">Divergence @ 4.20s</text>
            </svg>
          </div>
        </div>

        {/* Branch Strip R0 to R3 */}
        <div className="flex items-center gap-1.5 text-xs font-mono">
          {branches.map(b => (
            <div
              key={b.id}
              className={`px-2.5 py-1.5 rounded-[8px] border text-[10px] cursor-pointer transition-all ${
                b.id === 'b1'
                  ? 'bg-[#2F7BFF]/25 border-[#2F7BFF] text-white font-bold'
                  : 'bg-[#13253D] border-[#22395A] text-[#7F93AF] hover:text-white'
              }`}
            >
              <div>{b.name}</div>
              <div className="text-[9px] text-[#22D3EE]">{b.peak}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
