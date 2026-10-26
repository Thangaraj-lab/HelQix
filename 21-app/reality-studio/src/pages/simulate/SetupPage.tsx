import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';

export const SetupPage: React.FC = () => {
  const { setCurrentScreen, qualityTier, determinism, setDeterminism } = useAppState();

  const [activeStep, setActiveStep] = useState<'Initial Condition' | 'Environment' | 'Inputs' | 'Constraints' | 'Events'>('Initial Condition');
  const [solver, setSolver] = useState('Symplectic Gauss-Legendre (Energy-Preserving)');
  const [timestep, setTimestep] = useState('0.001 s');
  const [duration, setDuration] = useState('12.0 s');
  const [precision, setPrecision] = useState('IEEE 754 Float64 Bitwise');
  const [torqueInput, setTorqueInput] = useState<number>(45.0);
  const [ambientTemp, setAmbientTemp] = useState<number>(25.0);

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Tabs: Setup (active), Run, Results, Compare */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-1.5 p-1 rounded-xl glass-panel">
          {[
            { id: 'simulation_setup' as const, label: 'Setup', active: true },
            { id: 'simulation_run' as const, label: 'Run', active: false },
            { id: 'simulation_results' as const, label: 'Results', active: false },
            { id: 'compare' as const, label: 'Compare', active: false }
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
          <DeterminismChip level={determinism} />
          <span className="text-xs font-mono text-slate-400">SEED: 8421</span>
        </div>
      </div>

      {/* Main 3-Column Layout: Left Scenario Selector, Center 3D Model, Right Parameters & Solvers */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Left: Scenario Selector (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="flex items-center justify-between border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Scenario Definition</span>
              <OriginStatusBadges origin="Derived" status="Verified" />
            </div>

            <div className="space-y-1.5">
              {(['Initial Condition', 'Environment', 'Inputs', 'Constraints', 'Events'] as const).map(step => (
                <button
                  key={step}
                  onClick={() => setActiveStep(step)}
                  className={`w-full p-2.5 rounded-xl text-left text-xs transition-all flex items-center justify-between ${
                    activeStep === step
                      ? 'bg-blue-600/30 text-cyan-300 border border-cyan-400/50 font-semibold shadow-[0_0_12px_rgba(34,211,238,0.2)]'
                      : 'bg-slate-900/40 text-slate-400 hover:text-white hover:bg-slate-800/40'
                  }`}
                >
                  <span>{step}</span>
                  <span className="w-1.5 h-1.5 rounded-full bg-cyan-400"></span>
                </button>
              ))}
            </div>

            <div className="p-3 rounded-xl bg-slate-900/60 border border-slate-800 space-y-2 text-xs">
              <div className="font-semibold text-slate-200">Active Scenario: High Load Test</div>
              <p className="text-[11px] text-slate-400">
                Torque stepped to 160% nominal at t=8.42s to evaluate thermal dissipation headroom.
              </p>
            </div>
          </div>

          {/* Bottom-left Run Configuration Radio */}
          <div className="p-3 rounded-xl bg-slate-900/80 border border-slate-800 space-y-2">
            <span className="text-[11px] font-bold text-slate-300">Run Configuration</span>
            <div className="space-y-1.5 text-xs">
              {[
                { id: 'Bitwise' as const, label: 'Deterministic', sub: 'Bitwise Exact Replay' },
                { id: 'Tolerance' as const, label: 'Performance', sub: 'Adaptive Step 1e-4' },
                { id: 'Statistical' as const, label: 'Balanced / Monte Carlo', sub: 'Uncertainty Bounds' }
              ].map(opt => (
                <label
                  key={opt.id}
                  onClick={() => setDeterminism(opt.id)}
                  className={`flex items-start gap-2 p-1.5 rounded cursor-pointer transition-colors ${
                    determinism === opt.id ? 'bg-blue-950/60 text-cyan-300' : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  <input
                    type="radio"
                    name="runConfig"
                    checked={determinism === opt.id}
                    onChange={() => setDeterminism(opt.id)}
                    className="mt-0.5 accent-cyan-400"
                  />
                  <div>
                    <div className="font-semibold leading-none">{opt.label}</div>
                    <div className="text-[10px] text-slate-500 font-mono mt-0.5">{opt.sub}</div>
                  </div>
                </label>
              ))}
            </div>
          </div>
        </div>

        {/* Center: 3D Model with Field Lines (6 Cols) */}
        <div className="col-span-6 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18]">
          <HeroScene
            variant="workspace"
            interactive={true}
            qualityTier={qualityTier}
          />

          <div className="absolute top-4 left-4 z-10 px-3 py-1.5 rounded-xl glass-panel text-xs font-mono text-cyan-300">
            SIMULATION SOLVER READY // MESH CELLS: 240,000
          </div>
        </div>

        {/* Right: Parameters Sliders & Simulation Settings (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-4">
            <div className="border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Parameters & Boundary Values</span>
            </div>

            {/* Parameter sliders with units */}
            <div className="space-y-3">
              <div className="space-y-1">
                <div className="flex justify-between text-xs">
                  <span className="text-slate-400">Shaft Input Torque</span>
                  <span className="font-mono text-white num">{torqueInput.toFixed(1)} N·m</span>
                </div>
                <input
                  type="range"
                  min="10"
                  max="120"
                  step="0.5"
                  value={torqueInput}
                  onChange={e => setTorqueInput(parseFloat(e.target.value))}
                />
              </div>

              <div className="space-y-1">
                <div className="flex justify-between text-xs">
                  <span className="text-slate-400">Ambient Temperature</span>
                  <span className="font-mono text-white num">{ambientTemp.toFixed(1)} °C</span>
                </div>
                <input
                  type="range"
                  min="-20"
                  max="80"
                  step="0.5"
                  value={ambientTemp}
                  onChange={e => setAmbientTemp(parseFloat(e.target.value))}
                />
              </div>
            </div>

            {/* Simulation Settings Dropdowns */}
            <div className="space-y-2.5 pt-2 border-t border-slate-800">
              <span className="text-xs font-bold text-white">Solver Settings</span>

              <div className="space-y-1 text-xs">
                <label className="text-slate-400">Integration Solver</label>
                <select
                  value={solver}
                  onChange={e => setSolver(e.target.value)}
                  className="w-full p-2 rounded-lg bg-slate-900/90 border border-slate-700 text-xs text-white outline-none"
                >
                  <option>Symplectic Gauss-Legendre (Energy-Preserving)</option>
                  <option>Runge-Kutta 4th Order (RK4)</option>
                  <option>BDF Multi-Step Stiff Solver</option>
                </select>
              </div>

              <div className="grid grid-cols-2 gap-2 text-xs">
                <div className="space-y-1">
                  <label className="text-slate-400">Time Step</label>
                  <input
                    type="text"
                    value={timestep}
                    onChange={e => setTimestep(e.target.value)}
                    className="w-full p-1.5 rounded-lg bg-slate-900/90 border border-slate-700 font-mono text-white text-xs"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-slate-400">Total Duration</label>
                  <input
                    type="text"
                    value={duration}
                    onChange={e => setDuration(e.target.value)}
                    className="w-full p-1.5 rounded-lg bg-slate-900/90 border border-slate-700 font-mono text-white text-xs"
                  />
                </div>
              </div>

              <div className="space-y-1 text-xs">
                <label className="text-slate-400">Arithmetic Precision</label>
                <select
                  value={precision}
                  onChange={e => setPrecision(e.target.value)}
                  className="w-full p-2 rounded-lg bg-slate-900/90 border border-slate-700 text-xs text-white outline-none"
                >
                  <option>IEEE 754 Float64 Bitwise</option>
                  <option>Mixed Precision FP32/FP64</option>
                </select>
              </div>
            </div>
          </div>

          {/* Primary Action Button: Next (leads to Run) */}
          <button
            onClick={() => setCurrentScreen('simulation_run')}
            className="w-full ui-btn ui-btn-primary py-3 justify-center text-xs font-semibold shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            <span>Proceed to Simulation Run</span>
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M5 12h14M12 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
};
