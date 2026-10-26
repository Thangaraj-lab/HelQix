import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';

export const ResultsPage: React.FC = () => {
  const { setCurrentScreen, qualityTier, determinism } = useAppState();

  const [selectedRun, setSelectedRun] = useState('Run 01');
  const [viewMode, setViewMode] = useState<'3D' | 'Slice'>('3D');

  const runsList = [
    { id: 'Run 01', label: 'Run 01 (Nominal Base)', color: 'bg-cyan-400', date: '10:14:02', det: 'Bitwise' as const },
    { id: 'Run 02', label: 'Run 02 (High Load 160%)', color: 'bg-rose-500', date: '10:28:44', det: 'Bitwise' as const },
    { id: 'Run 03', label: 'Run 03 (Cold Start -10°C)', color: 'bg-blue-400', date: '11:05:12', det: 'Tolerance' as const },
    { id: 'Run 04', label: 'Run 04 (Harmonic Torque)', color: 'bg-purple-400', date: '11:42:30', det: 'Bitwise' as const }
  ];

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header: Tabs with Active "Results" */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-1.5 p-1 rounded-xl glass-panel">
          {[
            { id: 'simulation_setup' as const, label: 'Setup' },
            { id: 'simulation_run' as const, label: 'Run' },
            { id: 'simulation_results' as const, label: 'Results', active: true },
            { id: 'compare' as const, label: 'Compare' }
          ].map(tab => (
            <button
              key={tab.label}
              onClick={() => setCurrentScreen(tab.id)}
              className={`px-4 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                tab.active
                  ? 'bg-blue-600/40 text-cyan-300 border border-blue-400/50 shadow-[0_0_15px_rgba(47,123,255,0.4)]'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2">
          {/* 3D vs Slice View Toggle */}
          <div className="flex items-center gap-1 p-0.5 rounded-lg bg-slate-900/80 border border-slate-800 text-xs">
            {(['3D', 'Slice'] as const).map(mode => (
              <button
                key={mode}
                onClick={() => setViewMode(mode)}
                className={`px-3 py-1 rounded transition-colors ${
                  viewMode === mode
                    ? 'bg-blue-600/40 text-cyan-300 font-semibold border border-blue-400/40'
                    : 'text-slate-400 hover:text-white'
                }`}
              >
                {mode} View
              </button>
            ))}
          </div>

          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Left Runs List, Center 3D/Slice, Right Key Metrics & State Timeline */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Left: Runs List (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col gap-2.5 overflow-y-auto">
          <div className="flex items-center justify-between border-b border-slate-800 pb-2">
            <span className="text-xs font-bold text-white">Simulation Runs</span>
            <span className="text-[10px] font-mono text-slate-400">4 executions</span>
          </div>

          <div className="space-y-1.5">
            {runsList.map(run => (
              <button
                key={run.id}
                onClick={() => setSelectedRun(run.id)}
                className={`w-full p-2.5 rounded-xl text-left text-xs transition-all flex flex-col gap-1 border ${
                  selectedRun === run.id
                    ? 'bg-blue-600/30 border-cyan-400/60 shadow-[0_0_12px_rgba(34,211,238,0.2)]'
                    : 'bg-slate-900/40 border-slate-800/80 hover:border-slate-700'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className={`w-2 h-2 rounded-full ${run.color}`}></span>
                    <span className="font-semibold text-white">{run.id}</span>
                  </div>
                  <span className="text-[10px] font-mono text-cyan-400">{run.det}</span>
                </div>
                <div className="text-[11px] text-slate-300">{run.label}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Center: 3D / Slice View with Heat Region (6 Cols) */}
        <div className="col-span-6 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18]">
          <HeroScene
            variant="model_editor"
            interactive={true}
            qualityTier={qualityTier}
            thermalHotspot={true}
          />

          <div className="absolute top-4 left-4 z-10 px-3 py-1.5 rounded-xl glass-panel text-xs font-mono text-cyan-300 flex items-center gap-2">
            <span>FIELD SLICE: Z = 0.00 mm</span>
            <span className="text-slate-500">|</span>
            <span className="text-rose-400 font-bold">MAX: 92.4 °C</span>
          </div>
        </div>

        {/* Right: Key Metrics & State Timeline (3 Cols) */}
        <div className="col-span-3 flex flex-col gap-3 overflow-y-auto">
          {/* Key Metrics Rows (Origin x Status badges on each row!) */}
          <div className="glass-panel p-3.5 flex flex-col gap-2.5">
            <div className="border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Key Result Metrics</span>
            </div>

            <div className="space-y-2">
              {[
                { label: 'Peak Core Temperature', val: '92.4', unit: '°C', origin: 'Simulated' as const, status: 'Verified' as const },
                { label: 'RMS Input Power', val: '64.5', unit: 'kW', origin: 'Observed' as const, status: 'Verified' as const },
                { label: 'Mean Output Torque', val: '85.4', unit: 'N·m', origin: 'Derived' as const, status: 'Verified' as const },
                { label: 'System Efficiency', val: '87.2', unit: '%', origin: 'Simulated' as const, status: 'Verified' as const }
              ].map(m => (
                <div key={m.label} className="p-2 rounded-lg bg-slate-900/60 border border-slate-800 space-y-1">
                  <div className="flex items-center justify-between text-[11px] text-slate-400">
                    <span>{m.label}</span>
                    <OriginStatusBadges origin={m.origin} status={m.status} />
                  </div>
                  <div className="text-sm font-bold text-white font-mono num">
                    {m.val} <span className="text-xs font-normal text-cyan-400">{m.unit}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* State Timeline */}
          <div className="glass-panel p-3.5 flex flex-col gap-2">
            <div className="border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">State Timeline</span>
            </div>

            <div className="space-y-1.5 text-xs font-mono">
              {[
                { phase: 'Init', time: '0.00s', status: 'Stable', color: 'text-slate-300' },
                { phase: 'Stable Operation', time: '2.40s', status: 'Within Specs', color: 'text-cyan-300' },
                { phase: 'Transition Phase', time: '6.10s', status: 'Load Stepping', color: 'text-blue-300' },
                { phase: 'Critical Hotspot', time: '8.42s', status: 'Exceeded 90°C', color: 'text-rose-400 font-bold' },
                { phase: 'Recovered / Trip', time: '9.10s', status: 'Thermal Throttle', color: 'text-amber-400' }
              ].map(st => (
                <div key={st.phase} className="flex items-center justify-between py-0.5">
                  <span className={st.color}>{st.phase}</span>
                  <span className="text-slate-400 text-[11px]">{st.time}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Parameter Trends Line Chart & Distribution Histogram */}
      <div className="h-36 grid grid-cols-12 gap-3 shrink-0">
        <div className="col-span-8 glass-panel p-3 flex flex-col justify-between">
          <div className="flex items-center justify-between text-xs">
            <span className="font-bold text-white">Parameter Trends Over Time (0 - 12.00 s)</span>
            <div className="flex items-center gap-3 font-mono text-[11px]">
              <span className="text-rose-400">― Temperature (°C)</span>
              <span className="text-cyan-400">― Current (A)</span>
              <span className="text-blue-400">― Torque (N·m)</span>
            </div>
          </div>

          {/* SVG Trend Curves */}
          <svg className="w-full h-20" preserveAspectRatio="none" viewBox="0 0 100 30">
            {/* Grid lines */}
            <line x1="0" y1="10" x2="100" y2="10" stroke="rgba(255,255,255,0.06)" strokeWidth="0.5" />
            <line x1="0" y1="20" x2="100" y2="20" stroke="rgba(255,255,255,0.06)" strokeWidth="0.5" />
            
            {/* Failure marker vertical */}
            <line x1="70.1" y1="0" x2="70.1" y2="30" stroke="#EF4444" strokeWidth="0.8" strokeDasharray="1,1" />

            {/* Temperature curve (surges near 70%) */}
            <path d="M0,24 Q30,22 50,18 T70.1,6 T100,12" fill="none" stroke="#EF4444" strokeWidth="1.5" />
            {/* Current curve */}
            <path d="M0,26 Q40,25 60,20 T70.1,8 T100,22" fill="none" stroke="#22D3EE" strokeWidth="1.2" />
            {/* Torque curve */}
            <path d="M0,22 Q35,22 55,14 T70.1,10 T100,24" fill="none" stroke="#2F7BFF" strokeWidth="1.2" />
          </svg>
        </div>

        <div className="col-span-4 glass-panel p-3 flex flex-col justify-between">
          <div className="flex items-center justify-between text-xs">
            <span className="font-bold text-white">Distribution (Monte Carlo)</span>
            <span className="text-[10px] text-cyan-400 font-mono">N=10,000</span>
          </div>

          {/* Histogram Bars */}
          <div className="h-20 flex items-end gap-1.5 pt-2">
            {[15, 28, 45, 78, 95, 88, 62, 34, 18, 8].map((val, idx) => (
              <div key={idx} className="flex-1 bg-slate-900 rounded-t overflow-hidden h-full flex items-end">
                <div
                  className={`w-full rounded-t ${idx > 6 ? 'bg-rose-500' : 'bg-blue-500'}`}
                  style={{ height: `${val}%` }}
                  title={`Bin ${idx}: ${val}%`}
                ></div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
