import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { TimelineBar } from '../../components/timeline/TimelineBar';
import { DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';

export const RunPage: React.FC = () => {
  const {
    setCurrentScreen,
    qualityTier,
    determinism,
    isRunActive,
    setIsRunActive,
    showToast
  } = useAppState();

  const [durationMin, setDurationMin] = useState<number>(3.5);
  const [resolution, setResolution] = useState('0.001 mm / 1 ms');

  const runSteps = [
    { label: 'Initializing Solver Core', time: '00:00.12', status: 'Completed' },
    { label: 'Loading Model & Mesh Assembly', time: '00:01.05', status: 'Completed' },
    { label: 'Simulating Dynamic Field Equations', time: '00:04.20', status: 'Running' },
    { label: 'Processing Telemetry & Boundary Check', time: 'Pending', status: 'Pending' },
    { label: 'Finalizing Verification Evidence Hash', time: 'Pending', status: 'Pending' }
  ];

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header: Tabs with Active "Run" & Live Run State Pill */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-1.5 p-1 rounded-xl glass-panel">
          {[
            { id: 'simulation_setup' as const, label: 'Setup' },
            { id: 'simulation_run' as const, label: 'Run', active: true },
            { id: 'simulation_results' as const, label: 'Results' },
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

        {/* Live Status Pill & Play/Pause/Stop Buttons */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-emerald-950/80 border border-emerald-500/60 text-emerald-300 text-xs font-semibold shadow-[0_0_12px_rgba(34,197,94,0.3)]">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-ping"></span>
            <span>Running...</span>
          </div>

          <button
            onClick={() => {
              setIsRunActive(!isRunActive);
              showToast(isRunActive ? 'Simulation Paused' : 'Simulation Resumed');
            }}
            className="ui-btn-icon bg-slate-900 border border-slate-700"
            title={isRunActive ? 'Pause' : 'Resume'}
          >
            {isRunActive ? '⏸' : '▶'}
          </button>

          <button
            onClick={() => {
              setIsRunActive(false);
              showToast('Simulation Aborted by User', 'warning');
            }}
            className="ui-btn-icon bg-slate-900 border border-slate-700 hover:border-rose-500"
            title="Stop Run"
          >
            ⏹
          </button>

          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main 3-Column Area: Left Config, Center Live 3D, Right Status & Metrics */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Left: Run Configuration (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-slate-800 pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-white">Run Configuration</span>
              <span className="text-[10px] font-mono text-cyan-400">SEED: 8421</span>
            </div>

            <div className="space-y-2 text-xs">
              <div>
                <span className="text-slate-400">Scenario</span>
                <div className="text-white font-medium">High Load Thermal Dissipation</div>
              </div>

              <div>
                <span className="text-slate-400">Environment</span>
                <div className="text-white font-medium">Standard Atmospheric 25.0 °C, 101.3 kPa</div>
              </div>

              <div className="space-y-1 pt-1">
                <div className="flex justify-between">
                  <span className="text-slate-400">Duration</span>
                  <span className="font-mono text-cyan-300 num">{durationMin.toFixed(1)} min</span>
                </div>
                <input
                  type="range"
                  min="0.5"
                  max="10.0"
                  step="0.5"
                  value={durationMin}
                  onChange={e => setDurationMin(parseFloat(e.target.value))}
                />
              </div>

              <div>
                <span className="text-slate-400">Spatiotemporal Resolution</span>
                <div className="font-mono text-slate-200">{resolution}</div>
              </div>
            </div>
          </div>

          <div className="p-3 rounded-xl bg-slate-900/60 border border-slate-800 text-xs space-y-1">
            <span className="font-semibold text-slate-300">Determinism Check</span>
            <div className="text-[11px] text-emerald-400 flex items-center gap-1.5 font-mono">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
              <span>Bitwise Seed Exact: 0x8421A</span>
            </div>
          </div>
        </div>

        {/* Center: Live 3D with Flowing Particles & Thermal Hot Spot (6 Cols) */}
        <div className="col-span-6 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18]">
          <HeroScene
            variant="model_editor"
            interactive={true}
            qualityTier={qualityTier}
            thermalHotspot={true}
          />

          <div className="absolute top-4 left-4 z-10 flex items-center gap-2">
            <div className="px-3 py-1 rounded-xl glass-panel text-xs font-mono text-rose-300 border border-rose-500/40 flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-rose-500 animate-ping"></span>
              <span>THERMAL HOT SPOT DETECTED: 92.4 °C</span>
            </div>
          </div>
        </div>

        {/* Right: Live Metrics & Run Status Steps (3 Cols) */}
        <div className="col-span-3 flex flex-col gap-3 overflow-y-auto">
          {/* Live Metrics Bars with Units */}
          <div className="glass-panel p-3.5 flex flex-col gap-2.5">
            <div className="flex items-center justify-between border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Live Metrics</span>
              <OriginStatusBadges origin="Simulated" status="Verified" />
            </div>

            <div className="space-y-2 text-xs">
              {[
                { label: 'Rotor Temperature', val: '92.4', unit: '°C', pct: 85, color: 'bg-rose-500' },
                { label: 'Phase Current', val: '48.2', unit: 'A', pct: 78, color: 'bg-amber-400' },
                { label: 'Mechanical Power', val: '64.5', unit: 'kW', pct: 65, color: 'bg-cyan-400' },
                { label: 'Torque Output', val: '85.4', unit: 'N·m', pct: 88, color: 'bg-blue-500' }
              ].map(m => (
                <div key={m.label} className="space-y-1">
                  <div className="flex justify-between">
                    <span className="text-slate-400">{m.label}</span>
                    <span className="font-mono text-white num">{m.val} {m.unit}</span>
                  </div>
                  <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden">
                    <div className={`h-full ${m.color}`} style={{ width: `${m.pct}%` }}></div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Run Status Steps with Timestamps */}
          <div className="glass-panel p-3.5 flex flex-col gap-2">
            <div className="border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Run Execution Pipeline</span>
            </div>

            <div className="space-y-2 text-xs">
              {runSteps.map((step, idx) => (
                <div key={idx} className="flex items-start justify-between gap-2">
                  <div className="flex items-start gap-2">
                    <span
                      className={`w-2 h-2 rounded-full mt-1 shrink-0 ${
                        step.status === 'Completed'
                          ? 'bg-emerald-400'
                          : step.status === 'Running'
                          ? 'bg-cyan-400 animate-pulse'
                          : 'bg-slate-600'
                      }`}
                    />
                    <span className={step.status === 'Running' ? 'text-cyan-300 font-semibold' : 'text-slate-300'}>
                      {step.label}
                    </span>
                  </div>
                  <span className="font-mono text-[11px] text-slate-500 shrink-0">{step.time}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Timeline with Snapshots Carousel */}
      <div className="space-y-2">
        <TimelineBar showFreezeBtn={true} showHashMatch={false} />

        {/* Snapshots Carousel (Image prompt 06) */}
        <div className="flex items-center gap-3 px-2">
          <span className="text-[11px] font-mono text-slate-400">Snapshots:</span>
          {[
            { label: 't=00:01.00 Init', hash: '0x10a2' },
            { label: 't=00:04.20 Steady', hash: '0x42f8' },
            { label: 't=00:08.42 Critical Peak', hash: '0x84ec' }
          ].map(snap => (
            <button
              key={snap.label}
              className="px-2.5 py-1 rounded-lg bg-slate-900/80 border border-slate-800 hover:border-cyan-400 text-xs font-mono text-slate-300 flex items-center gap-1.5 transition-colors"
            >
              <span className="w-1.5 h-1.5 rounded-full bg-blue-400"></span>
              <span>{snap.label}</span>
              <span className="text-[10px] text-slate-500">({snap.hash})</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};
