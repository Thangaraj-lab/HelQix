import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';

export const ComparePage: React.FC = () => {
  const { qualityTier, determinism } = useAppState();

  const [compareSpeed, setCompareSpeed] = useState('1.0x');

  const comparisonMetrics = [
    { label: 'Peak Core Temperature', scA: '78.2 °C', scB: '92.4 °C', delta: '-14.2 °C (-15.4%)', deltaType: 'positive' },
    { label: 'Max Phase Current', scA: '36.5 A', scB: '48.2 A', delta: '-11.7 A (-24.2%)', deltaType: 'positive' },
    { label: 'Shaft Output Power', scA: '61.8 kW', scB: '64.5 kW', delta: '-2.7 kW (-4.1%)', deltaType: 'neutral' },
    { label: 'System Efficiency', scA: '92.4%', scB: '87.2%', delta: '+5.2% (+5.9%)', deltaType: 'positive' }
  ];

  const insights = [
    { text: 'Peak temperature 15.4% lower in Scenario A with bypass cooling', type: 'positive' },
    { text: 'First divergence detected at t = 00:04.20 during step-load', type: 'neutral' },
    { text: 'Scenario B experiences thermal throttle trip at t = 00:08.42', type: 'negative' }
  ];

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-2">
          <div className="px-3 py-1.5 rounded-xl glass-panel text-xs font-semibold text-cyan-300">
            SCENARIO DUAL COMPARISON & COUNTERFACTUAL
          </div>
          <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-blue-500/20 text-blue-300 border border-blue-500/30">
            A: Bypass Cooling vs B: High Load Base
          </span>
        </div>

        <div className="flex items-center gap-2">
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Two Side-by-Side 3D Panels + Right Comparison Metrics */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Scenario A: 3D Model in Blue (4 Cols) */}
        <div className="col-span-4 relative rounded-2xl overflow-hidden border border-cyan-500/30 bg-[#050B18] flex flex-col justify-between p-3">
          <div className="absolute inset-0">
            <HeroScene
              variant="workspace"
              interactive={true}
              qualityTier={qualityTier}
            />
          </div>

          <div className="relative z-10 self-start px-2.5 py-1 rounded-lg glass-panel text-xs font-mono text-cyan-300 border border-cyan-500/40">
            Scenario A (Bypass Active) • Peak 78.2 °C
          </div>

          <div className="relative z-10 flex items-center gap-2">
            {['t=2.0s', 't=4.2s', 't=8.4s'].map(t => (
              <span key={t} className="px-2 py-0.5 rounded bg-slate-900/80 text-[10px] text-slate-300 font-mono border border-slate-700">
                {t}
              </span>
            ))}
          </div>
        </div>

        {/* Scenario B: 3D Model in Violet/Red Hotspot (4 Cols) */}
        <div className="col-span-4 relative rounded-2xl overflow-hidden border border-rose-500/30 bg-[#050B18] flex flex-col justify-between p-3">
          <div className="absolute inset-0">
            <HeroScene
              variant="failure"
              interactive={true}
              qualityTier={qualityTier}
              thermalHotspot={true}
            />
          </div>

          <div className="relative z-10 self-start px-2.5 py-1 rounded-lg glass-panel text-xs font-mono text-rose-300 border border-rose-500/40">
            Scenario B (Base Baseline) • Peak 92.4 °C
          </div>

          <div className="relative z-10 flex items-center gap-2">
            {['t=2.0s', 't=4.2s (Diverge)', 't=8.4s (Trip)'].map(t => (
              <span key={t} className="px-2 py-0.5 rounded bg-slate-900/80 text-[10px] text-slate-300 font-mono border border-slate-700">
                {t}
              </span>
            ))}
          </div>
        </div>

        {/* Right: Comparison Metrics & Insights (4 Cols) */}
        <div className="col-span-4 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="flex items-center justify-between border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Comparison Metrics</span>
              <OriginStatusBadges origin="Derived" status="Verified" />
            </div>

            <div className="space-y-2.5">
              {comparisonMetrics.map(m => (
                <div key={m.label} className="p-2.5 rounded-xl bg-slate-900/60 border border-slate-800 space-y-1.5">
                  <div className="text-xs text-slate-300 font-medium">{m.label}</div>
                  <div className="grid grid-cols-2 gap-2 text-xs font-mono">
                    <div className="p-1 rounded bg-blue-950/40 text-cyan-300">
                      <span className="text-[10px] text-slate-400 block">Scenario A:</span>
                      <span className="font-bold">{m.scA}</span>
                    </div>
                    <div className="p-1 rounded bg-rose-950/40 text-rose-300">
                      <span className="text-[10px] text-slate-400 block">Scenario B:</span>
                      <span className="font-bold">{m.scB}</span>
                    </div>
                  </div>
                  <div className="text-[11px] font-mono text-emerald-400 font-semibold pt-0.5">
                    Delta: {m.delta}
                  </div>
                </div>
              ))}
            </div>

            {/* Neutral Insight Statements (Prompt 09) */}
            <div className="space-y-1.5 pt-1">
              <span className="text-xs font-bold text-white">Neutral Observations</span>
              {insights.map((ins, idx) => (
                <div key={idx} className="p-2 rounded bg-slate-900/40 border border-slate-800 text-[11px] text-slate-300 flex items-start gap-2">
                  <span className="text-cyan-400 mt-0.5">▸</span>
                  <span>{ins.text}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Overlay Timeline with First-Divergence Marker */}
      <div className="h-28 glass-panel p-3 flex flex-col justify-between shrink-0">
        <div className="flex items-center justify-between text-xs">
          <div className="flex items-center gap-3">
            <span className="font-bold text-white">Overlay Timeline Scrubber</span>
            <span className="text-cyan-300 font-mono">― Scenario A</span>
            <span className="text-rose-400 font-mono">― Scenario B</span>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-[11px] text-slate-400">Replay Speed:</span>
            <select
              value={compareSpeed}
              onChange={e => setCompareSpeed(e.target.value)}
              className="px-2 py-0.5 rounded bg-slate-900 border border-slate-700 text-xs text-white outline-none"
            >
              <option>0.5x</option>
              <option>1.0x</option>
              <option>2.0x</option>
            </select>
          </div>
        </div>

        {/* Dual Curve Overlay with First Divergence Marker at t=4.20s */}
        <div className="relative w-full h-12 flex items-center">
          {/* Divergence vertical marker */}
          <div className="absolute top-0 bottom-0 left-[35%] w-0.5 bg-amber-400 z-10">
            <span className="absolute -top-4 -left-12 px-1.5 rounded bg-amber-950 text-amber-300 text-[9px] font-mono border border-amber-500/40 whitespace-nowrap">
              Divergence t=4.20s
            </span>
          </div>

          <svg className="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 24">
            {/* Scenario A curve (stays low) */}
            <path d="M0,20 Q35,18 50,14 T70,12 T100,10" fill="none" stroke="#22D3EE" strokeWidth="1.5" />
            {/* Scenario B curve (surges after 35%) */}
            <path d="M0,20 Q35,18 50,10 T70,4 T100,2" fill="none" stroke="#EF4444" strokeWidth="1.5" />
          </svg>
        </div>
      </div>
    </div>
  );
};
