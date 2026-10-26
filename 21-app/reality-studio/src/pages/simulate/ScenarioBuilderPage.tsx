import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { DeterminismChip } from '../../components/ui/HonestyBadges';

export const ScenarioBuilderPage: React.FC = () => {
  const { setCurrentScreen, qualityTier, determinism, showToast } = useAppState();

  const [activeStep, setActiveStep] = useState(1);
  const [selectedScenario, setSelectedScenario] = useState('High Load');

  const scenarioList = ['Base', 'High Load', 'Low Temperature', 'Stress Test', 'Custom 01'];
  const steps = [
    { num: 1, label: 'Scenario' },
    { num: 2, label: 'Environment' },
    { num: 3, label: 'Inputs' },
    { num: 4, label: 'Constraints' },
    { num: 5, label: 'Events' }
  ];

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Step Bar & Action Buttons (Prompt 10) */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-2 p-1.5 rounded-xl glass-panel">
          {steps.map(s => (
            <button
              key={s.num}
              onClick={() => setActiveStep(s.num)}
              className={`px-3 py-1 rounded-lg text-xs font-semibold flex items-center gap-2 transition-all ${
                activeStep === s.num
                  ? 'bg-blue-600/40 text-cyan-300 border border-blue-400/50 shadow-[0_0_12px_rgba(47,123,255,0.3)]'
                  : 'text-slate-400 hover:text-white'
              }`}
            >
              <span className="w-4 h-4 rounded-full bg-slate-800 text-[10px] flex items-center justify-center font-mono">
                {s.num}
              </span>
              <span>{s.label}</span>
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => showToast('Scenario Definition Saved to Reality Store', 'success')}
            className="ui-btn ui-btn-outline text-xs"
          >
            Save Scenario
          </button>
          <button
            onClick={() => setCurrentScreen('simulation_setup')}
            className="ui-btn ui-btn-primary text-xs"
          >
            Continue
          </button>
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Left Scenario List, Center Glowing 3D, Right Sliders & Mini Chart */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Left: Scenario List (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col gap-2 overflow-y-auto">
          <span className="text-xs font-bold text-white border-b border-slate-800 pb-2">
            Preset Scenarios
          </span>
          <div className="space-y-1.5">
            {scenarioList.map(sc => (
              <button
                key={sc}
                onClick={() => setSelectedScenario(sc)}
                className={`w-full p-2.5 rounded-xl text-left text-xs transition-all flex items-center justify-between ${
                  selectedScenario === sc
                    ? 'bg-blue-600/30 border border-cyan-400/60 text-cyan-300 font-semibold'
                    : 'bg-slate-900/40 border border-slate-800/80 text-slate-400 hover:text-white'
                }`}
              >
                <span>{sc}</span>
                <span className="text-slate-500 font-mono text-[10px]">Active</span>
              </button>
            ))}
          </div>
        </div>

        {/* Center: Glowing 3D System (6 Cols) */}
        <div className="col-span-6 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18]">
          <HeroScene
            variant="workspace"
            interactive={true}
            qualityTier={qualityTier}
          />
        </div>

        {/* Right: Parameters & Condition Preview Mini Chart (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <span className="text-xs font-bold text-white border-b border-slate-800 pb-2 block">
              Scenario Parameters
            </span>

            <div className="space-y-2.5 text-xs">
              <div className="space-y-1">
                <div className="flex justify-between text-slate-400">
                  <span>Inlet Temperature</span>
                  <span className="text-white font-mono num">25.0 °C</span>
                </div>
                <input type="range" min="0" max="100" defaultValue="25" />
              </div>

              <div className="space-y-1">
                <div className="flex justify-between text-slate-400">
                  <span>Step Voltage Amplitude</span>
                  <span className="text-white font-mono num">400.0 V</span>
                </div>
                <input type="range" min="200" max="800" defaultValue="400" />
              </div>
            </div>

            {/* Condition Preview Mini Chart with 3 readouts */}
            <div className="p-3 rounded-xl bg-slate-900/60 border border-slate-800 space-y-2">
              <div className="text-[11px] font-bold text-slate-300">Condition Preview (t=0 to 12s)</div>
              <svg className="w-full h-12" preserveAspectRatio="none" viewBox="0 0 100 24">
                <path d="M0,20 L30,20 L30,6 L100,6" fill="none" stroke="#22D3EE" strokeWidth="1.5" />
              </svg>
              <div className="grid grid-cols-3 gap-1 text-[10px] font-mono text-center pt-1 border-t border-slate-800">
                <div>
                  <span className="text-slate-500 block">Peak</span>
                  <span className="text-cyan-400 font-bold">400 V</span>
                </div>
                <div>
                  <span className="text-slate-500 block">Step</span>
                  <span className="text-white font-bold">4.20 s</span>
                </div>
                <div>
                  <span className="text-slate-500 block">Slew</span>
                  <span className="text-emerald-400 font-bold">12 kV/μs</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Node Flow Builder & Quick Presets (Prompt 10) */}
      <div className="h-32 glass-panel p-3 flex flex-col justify-between shrink-0">
        <div className="flex items-center justify-between text-xs">
          <span className="font-bold text-white">Node Flow Architecture</span>
          <div className="flex items-center gap-2">
            <span className="text-slate-400 text-[11px]">Quick Presets:</span>
            {['Normal', 'Extreme', 'Cold', 'Hot'].map(p => (
              <button
                key={p}
                onClick={() => showToast(`Applied ${p} Scenario Preset`, 'info')}
                className="px-2 py-0.5 rounded bg-slate-900 border border-slate-700 hover:border-cyan-400 text-[11px] text-slate-300"
              >
                {p}
              </button>
            ))}
          </div>
        </div>

        {/* Node Flow Links */}
        <div className="flex items-center justify-between gap-2 overflow-x-auto py-1">
          {['Initial State', 'Environment', 'Inputs', 'Constraints', 'Events'].map((node, idx) => (
            <React.Fragment key={node}>
              <div className="p-2.5 rounded-xl bg-slate-900/90 border border-blue-500/40 text-xs text-center shrink-0 min-w-[120px]">
                <div className="text-[10px] text-cyan-400 font-mono uppercase">Node 0{idx + 1}</div>
                <div className="font-semibold text-white mt-0.5">{node}</div>
              </div>
              {idx < 4 && (
                <div className="text-cyan-400 font-mono text-xs px-1">───▶</div>
              )}
            </React.Fragment>
          ))}
        </div>
      </div>
    </div>
  );
};
