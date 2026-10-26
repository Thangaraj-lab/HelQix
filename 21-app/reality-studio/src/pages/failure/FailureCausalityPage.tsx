import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { PreviewTag, DeterminismChip, OriginStatusBadges } from '../../components/ui/HonestyBadges';
import { MOCK_CAUSAL_CHAIN, CausalNode } from '../../mock/engineeringData';

export const FailureCausalityPage: React.FC = () => {
  const { setCurrentScreen, qualityTier, determinism, showToast } = useAppState();

  const [activeTab, setActiveTab] = useState<'Causality' | 'Timeline' | 'Evidence' | 'Report'>('Causality');
  const [selectedNode, setSelectedNode] = useState<CausalNode>(MOCK_CAUSAL_CHAIN[4]); // Candidate root cause

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Header: Breadcrumbs & Causality Tabs */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1.5 p-1 rounded-xl glass-panel">
            {(['Causality', 'Timeline', 'Evidence', 'Report'] as const).map(tab => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-4 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  activeTab === tab
                    ? 'bg-blue-600/40 text-cyan-300 border border-blue-400/50 shadow-[0_0_15px_rgba(47,123,255,0.4)]'
                    : 'text-slate-400 hover:text-white'
                }`}
              >
                {tab}
              </button>
            ))}
          </div>
          <PreviewTag label="PREVIEW (Phase 3)" />
        </div>

        <div className="flex items-center gap-2">
          <span className="px-2.5 py-1 rounded-lg text-xs font-mono font-semibold bg-rose-950/80 border border-rose-500/60 text-rose-300 flex items-center gap-1.5 shadow-[0_0_12px_rgba(239,68,68,0.4)]">
            <span className="w-2 h-2 rounded-full bg-rose-500 animate-ping"></span>
            <span>FAILURE EVENT AT 8.42s</span>
          </span>
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Center-Left 3D Heatmap, Center Causal Chain, Right Cause Details */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Center-Left: 3D Rotor Assembly with Heatmap & Callouts (4 Cols) */}
        <div className="col-span-4 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18] flex flex-col justify-between p-3">
          <div className="absolute inset-0">
            <HeroScene
              variant="failure"
              interactive={true}
              qualityTier={qualityTier}
              thermalHotspot={true}
            />
          </div>

          {/* Floating Callout: Thermal Overload */}
          <div className="relative z-10 self-start p-2.5 rounded-xl glass-panel-elevated border border-rose-500/50 shadow-[0_0_20px_rgba(239,68,68,0.3)]">
            <div className="flex items-center gap-2 text-rose-400 font-bold text-xs">
              <span className="w-2 h-2 rounded-full bg-rose-500 animate-pulse"></span>
              <span>Thermal Overload Event</span>
            </div>
            <div className="text-[11px] text-slate-300 font-mono mt-0.5">Rotor Assembly CMP-A-001</div>
            <div className="text-base font-extrabold text-white font-mono num mt-1">92.4 °C</div>
            <span className="text-[10px] px-1.5 py-0.2 rounded bg-rose-500/20 text-rose-300 border border-rose-500/30">
              CRITICAL LIMIT: 90.0 °C
            </span>
          </div>

          {/* Color Scale: 20°C to 120°C (Prompt 08) */}
          <div className="relative z-10 p-2 rounded-xl glass-panel flex flex-col gap-1">
            <div className="flex justify-between text-[10px] font-mono text-slate-400">
              <span>20 °C</span>
              <span>Thermal Heatmap Scale</span>
              <span>120 °C</span>
            </div>
            <div className="w-full h-2 rounded-full bg-gradient-to-r from-blue-500 via-cyan-400 via-amber-400 to-rose-600"></div>
          </div>
        </div>

        {/* Center: Causal Chain Vertical Nodes (4 Cols) */}
        <div className="col-span-4 glass-panel p-3.5 flex flex-col gap-2.5 overflow-y-auto">
          <div className="border-b border-slate-800 pb-2">
            <div className="text-xs font-bold text-white">Candidate chain (structural + temporal)</div>
            <div className="text-[10px] text-slate-400 mt-0.5">
              Causal hypothesis graph generated from formal state equations
            </div>
          </div>

          <div className="space-y-2">
            {MOCK_CAUSAL_CHAIN.map((node, idx) => {
              const isSelected = selectedNode.id === node.id;
              return (
                <div key={node.id} className="relative">
                  {/* Vertical connector line */}
                  {idx < MOCK_CAUSAL_CHAIN.length - 1 && (
                    <div className="absolute left-4 top-8 bottom-0 w-0.5 bg-blue-500/30 -z-10" />
                  )}

                  <button
                    onClick={() => setSelectedNode(node)}
                    className={`w-full p-2.5 rounded-xl text-left text-xs transition-all flex items-start gap-3 border ${
                      isSelected
                        ? 'bg-blue-600/30 border-cyan-400 shadow-[0_0_15px_rgba(34,211,238,0.25)]'
                        : 'bg-slate-900/60 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    <span
                      className={`w-3 h-3 rounded-full mt-1 shrink-0 ${
                        node.isCandidateRoot
                          ? 'bg-amber-400 shadow-[0_0_8px_#F59E0B]'
                          : node.type === 'event'
                          ? 'bg-rose-500 shadow-[0_0_8px_#EF4444]'
                          : 'bg-cyan-400'
                      }`}
                    />
                    <div className="flex-1">
                      <div className="font-semibold text-slate-200">{node.label}</div>
                      <div className="flex items-center gap-2 text-[11px] font-mono text-slate-400 mt-1">
                        <span>{node.parameter}:</span>
                        <span className="text-white num font-bold">{node.currentVal}</span>
                        <span className="text-rose-400 font-bold">{node.change}</span>
                      </div>
                    </div>
                  </button>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: Selected Cause Details (4 Cols) */}
        <div className="col-span-4 glass-panel p-3.5 flex flex-col justify-between gap-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-slate-800 pb-2">
              <span className="text-xs font-bold text-white">Selected Cause Details</span>
              <div className="text-[11px] text-cyan-300 font-medium mt-0.5">{selectedNode.label}</div>
            </div>

            <div className="space-y-2 text-xs">
              <div className="flex justify-between p-2 rounded bg-slate-900/60 border border-slate-800">
                <span className="text-slate-400">Previous Value</span>
                <span className="font-mono text-slate-200 num">{selectedNode.previousVal}</span>
              </div>
              <div className="flex justify-between p-2 rounded bg-slate-900/60 border border-slate-800">
                <span className="text-slate-400">Current Value</span>
                <span className="font-mono text-white font-bold num">{selectedNode.currentVal}</span>
              </div>
              <div className="flex justify-between p-2 rounded bg-slate-900/60 border border-slate-800">
                <span className="text-slate-400">Parameter Delta</span>
                <span className="font-mono text-rose-400 font-bold">{selectedNode.change}</span>
              </div>
            </div>

            {/* Evidence Checklist */}
            <div className="space-y-2 pt-1">
              <span className="text-xs font-bold text-white">Supporting Evidence</span>
              <div className="space-y-1.5 text-[11px]">
                {[
                  { text: 'Simulation Result Verified', status: 'Verified' as const },
                  { text: 'Sensor Data Matching', status: 'Verified' as const },
                  { text: 'Causal Link Candidate', status: 'Unverified' as const },
                  { text: 'Model Equation Validated', status: 'Verified' as const }
                ].map(ev => (
                  <div key={ev.text} className="p-2 rounded bg-slate-900/40 border border-slate-800 flex items-center justify-between">
                    <span className="text-slate-300">{ev.text}</span>
                    <OriginStatusBadges origin="Derived" status={ev.status} />
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Action: Run Counterfactual Simulation */}
          <button
            onClick={() => {
              showToast('Spinning up counterfactual branch simulation...', 'info');
              setCurrentScreen('compare');
            }}
            className="w-full ui-btn ui-btn-primary py-3 justify-center text-xs font-semibold shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            <span>Run Counterfactual Simulation</span>
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
              <polyline points="17 6 23 6 23 12" />
            </svg>
          </button>
        </div>
      </div>

      {/* Bottom Area: Event Timeline, Chart with 8.42s Marker, Simulation State Card */}
      <div className="h-36 grid grid-cols-12 gap-3 shrink-0">
        {/* Bottom-left: Event Timeline List (3 Cols) */}
        <div className="col-span-3 glass-panel p-3 overflow-y-auto text-xs space-y-1 font-mono">
          <div className="font-bold text-white border-b border-slate-800 pb-1 mb-1">Event Timeline</div>
          <div className="text-slate-400">0.00s Normal Operation</div>
          <div className="text-cyan-300">4.20s Step Load Increase</div>
          <div className="text-rose-400 font-bold">8.42s Thermal Failure (92.4°C)</div>
          <div className="text-amber-400">9.10s System Safe Shutdown</div>
        </div>

        {/* Bottom-center: Chart with red failure marker (6 Cols) */}
        <div className="col-span-6 glass-panel p-3 flex flex-col justify-between">
          <div className="flex items-center justify-between text-xs">
            <span className="font-bold text-white">Dynamic Event Response</span>
            <span className="text-rose-400 font-mono text-[11px]">Marker at t = 8.42 s</span>
          </div>

          <svg className="w-full h-20" preserveAspectRatio="none" viewBox="0 0 100 30">
            <line x1="70.1" y1="0" x2="70.1" y2="30" stroke="#EF4444" strokeWidth="1.2" strokeDasharray="1,1" />
            <path d="M0,24 Q35,22 55,16 T70.1,5 T100,10" fill="none" stroke="#EF4444" strokeWidth="1.8" />
            <path d="M0,26 Q40,25 60,20 T70.1,8 T100,24" fill="none" stroke="#22D3EE" strokeWidth="1.2" />
          </svg>
        </div>

        {/* Bottom-right: Simulation State Card (3 Cols) */}
        <div className="col-span-3 glass-panel p-3 flex flex-col justify-between text-xs">
          <span className="font-bold text-white border-b border-slate-800 pb-1">Simulation State at 8.42s</span>
          <div className="space-y-1 font-mono text-[11px]">
            <div className="flex justify-between">
              <span className="text-slate-400">Temperature</span>
              <span className="text-rose-400 font-bold num">92.4 °C</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Phase Current</span>
              <span className="text-cyan-300 num">48.2 A</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Torque Load</span>
              <span className="text-white num">85.4 N·m</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Mechanical Power</span>
              <span className="text-slate-200 num">64.5 kW</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
