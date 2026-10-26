import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';

export const BuildOptimizationPage: React.FC = () => {
  const { setCurrentScreen, qualityTier, determinism, showToast } = useAppState();

  const [algorithm, setAlgorithm] = useState('Pareto NSGA-II Multi-Objective');
  const [iterations, setIterations] = useState('500');
  const [isOptimizing, setIsOptimizing] = useState(true);

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-xs font-bold font-mono text-cyan-300">
            BUILD // AUTOMATED MULTI-OBJECTIVE DESIGN OPTIMIZATION
          </span>
          <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-blue-500/20 text-cyan-300 border border-blue-500/40">
            BRANCH EXPLORER
          </span>
        </div>

        <div className="flex items-center gap-2">
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Left Goals & Settings, Center 3D Progress Ring, Right Proposed Changes */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Left: Optimization Goals & Controls (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <span className="text-xs font-bold text-white border-b border-slate-800 pb-2 block">
              Optimization Goals
            </span>

            <div className="space-y-2 text-xs">
              {[
                { label: 'Maximize Electrical Efficiency', priority: 'Weight: 0.45' },
                { label: 'Minimize Rotor Assembly Mass', priority: 'Weight: 0.35' },
                { label: 'Improve Dynamic Thermal Stability', priority: 'Weight: 0.20' }
              ].map((g, idx) => (
                <label key={g.label} className="p-2 rounded-xl bg-slate-900/60 border border-slate-800 flex items-start gap-2 cursor-pointer">
                  <input type="checkbox" defaultChecked className="mt-0.5 accent-cyan-400" />
                  <div>
                    <div className="text-white font-medium">{g.label}</div>
                    <div className="text-[10px] text-cyan-400 font-mono">{g.priority}</div>
                  </div>
                </label>
              ))}
            </div>

            <div className="space-y-2 pt-2 border-t border-slate-800 text-xs">
              <div>
                <label className="text-slate-400">Search Algorithm</label>
                <select
                  value={algorithm}
                  onChange={e => setAlgorithm(e.target.value)}
                  className="w-full p-2 mt-1 rounded-lg bg-slate-900/90 border border-slate-700 text-xs text-white outline-none"
                >
                  <option>Pareto NSGA-II Multi-Objective</option>
                  <option>Bayesian Surrogate Gaussian Process</option>
                  <option>Gradient Topology Optimization</option>
                </select>
              </div>

              <div>
                <label className="text-slate-400">Target Iterations</label>
                <input
                  type="text"
                  value={iterations}
                  onChange={e => setIterations(e.target.value)}
                  className="w-full p-1.5 mt-1 rounded-lg bg-slate-900/90 border border-slate-700 font-mono text-white text-xs"
                />
              </div>
            </div>
          </div>

          <button
            onClick={() => {
              setIsOptimizing(!isOptimizing);
              showToast(isOptimizing ? 'Optimization Paused' : 'Optimization Started', 'info');
            }}
            className="w-full ui-btn ui-btn-primary py-2.5 justify-center text-xs"
          >
            {isOptimizing ? 'Pause Optimization' : 'Start Optimization'}
          </button>
        </div>

        {/* Center: 3D Assembly with Circular Progress Ring (6 Cols - Prompt 12) */}
        <div className="col-span-6 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18] flex items-center justify-center">
          <div className="absolute inset-0">
            <HeroScene
              variant="model_editor"
              interactive={true}
              qualityTier={qualityTier}
            />
          </div>

          {/* Floating Circular Progress Ring (72%, Iteration 366/500) */}
          <div className="relative z-10 p-5 rounded-2xl glass-panel-elevated border border-cyan-400/50 flex flex-col items-center gap-2 shadow-[0_0_35px_rgba(34,211,238,0.3)]">
            <div className="relative w-28 h-28 flex items-center justify-center">
              <svg className="w-full h-full -rotate-90" viewBox="0 0 100 100">
                <circle cx="50" cy="50" r="40" stroke="#0e244d" strokeWidth="8" fill="none" />
                <circle
                  cx="50"
                  cy="50"
                  r="40"
                  stroke="#22D3EE"
                  strokeWidth="8"
                  fill="none"
                  strokeDasharray="251.2"
                  strokeDashoffset="70.3"
                  strokeLinecap="round"
                />
              </svg>
              <div className="absolute inset-0 flex flex-col items-center justify-center">
                <span className="text-2xl font-black text-white font-mono num">72%</span>
                <span className="text-[10px] font-mono text-cyan-300">CONVERGED</span>
              </div>
            </div>

            <div className="text-xs font-mono text-white font-semibold">Running... Iteration 366/500</div>
            <div className="text-[10px] font-mono text-slate-400">Active Front: 42 Pareto Solutions</div>
          </div>
        </div>

        {/* Right: Results Preview with Honest Unverified Proposals (3 Cols - Prompt 12) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Proposed Change (Unverified)</span>
              <p className="text-[11px] text-amber-300/90 mt-0.5">
                AI/Algorithmic suggestion requires branch verification before merge.
              </p>
            </div>

            <div className="space-y-2.5">
              <div className="p-2.5 rounded-xl bg-slate-900/60 border border-slate-800 space-y-1">
                <div className="flex justify-between text-xs text-slate-400">
                  <span>Rotor Core Mass</span>
                  <OriginStatusBadges origin="AI-hypothesis" status="Unverified" />
                </div>
                <div className="flex items-baseline justify-between">
                  <span className="text-base font-bold text-white font-mono num">2.18 kg</span>
                  <span className="text-xs font-mono text-emerald-400 font-semibold">-0.30 kg (-12.1%)</span>
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-slate-900/60 border border-slate-800 space-y-1">
                <div className="flex justify-between text-xs text-slate-400">
                  <span>Operating Efficiency</span>
                  <OriginStatusBadges origin="AI-hypothesis" status="Unverified" />
                </div>
                <div className="flex items-baseline justify-between">
                  <span className="text-base font-bold text-white font-mono num">93.4%</span>
                  <span className="text-xs font-mono text-emerald-400 font-semibold">+6.2% (+7.1%)</span>
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-slate-900/60 border border-slate-800 space-y-1">
                <div className="flex justify-between text-xs text-slate-400">
                  <span>Peak Heat Dissipation</span>
                  <OriginStatusBadges origin="AI-hypothesis" status="Unverified" />
                </div>
                <div className="flex items-baseline justify-between">
                  <span className="text-base font-bold text-white font-mono num">74.6 °C</span>
                  <span className="text-xs font-mono text-emerald-400 font-semibold">-17.8 °C (-19.2%)</span>
                </div>
              </div>
            </div>
          </div>

          {/* Action: Validate by branch simulation */}
          <button
            onClick={() => {
              showToast('Spinning up dedicated verification branch...', 'info');
              setCurrentScreen('compare');
            }}
            className="w-full ui-btn ui-btn-primary py-3 justify-center text-xs font-semibold shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            <span>Validate by Branch Simulation</span>
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M5 12h14M12 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
};
