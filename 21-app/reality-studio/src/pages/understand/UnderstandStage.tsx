import React, { useState } from 'react';
import { MockEngineClient } from '../../mock/MockEngineClient';
import { MockTag, ValueView, Panel } from '../../components/ui/UiKit';

export interface UnderstandStageProps {
  onTestThisCause: () => void;
  onBackToSimulate?: () => void;
}

export interface CausalNode {
  id: string;
  step: string;
  name: string;
  component: string;
  relation: string;
  prevVal: string;
  failureVal: string;
  isCandidateRootCause?: boolean;
  origin: 'simulated' | 'derived' | 'observed';
  status: 'verified' | 'unverified';
}

export const UnderstandStage: React.FC<UnderstandStageProps> = ({
  onTestThisCause,
  onBackToSimulate
}) => {
  const [engine] = useState(() => MockEngineClient);

  // 5 Vertical Candidate Chain Nodes (Section 6.11)
  const candidateChain: CausalNode[] = [
    {
      id: 'node-1',
      step: '01',
      name: 'Dynamic Step-Load Torque Injection',
      component: 'Shaft Output Node',
      relation: 'Torque demanded by external hydraulic backpressure',
      prevVal: '32.0 N·m',
      failureVal: '85.4 N·m',
      origin: 'simulated',
      status: 'verified'
    },
    {
      id: 'node-2',
      step: '02',
      name: 'Stator Phase RMS Current Surge',
      component: 'Power Conversion Unit',
      relation: 'I_rms = T_load / (k_t * sqrt(3))',
      prevVal: '42.0 A',
      failureVal: '60.5 A',
      origin: 'simulated',
      status: 'verified'
    },
    {
      id: 'node-3',
      step: '03',
      name: 'Ohmic Joule Heat Dissipation Surge',
      component: 'Actuator Stator Winding',
      relation: 'P_loss = 3 * I_rms^2 * R_winding',
      prevVal: '215.4 W',
      failureVal: '480.2 W',
      origin: 'derived',
      status: 'verified'
    },
    {
      id: 'node-4',
      step: '04',
      name: 'Coolant Convective Boundary Saturation',
      component: 'Thermal Heat Jacket',
      relation: 'q_out = h_conv * A * (T_stator - T_coolant)',
      prevVal: '210.0 W',
      failureVal: '310.5 W (Saturated)',
      isCandidateRootCause: true,
      origin: 'derived',
      status: 'verified'
    },
    {
      id: 'node-5',
      step: '05',
      name: 'Winding Temperature Safety Limit Breach',
      component: 'Actuator Stator Insulation',
      relation: 'dT/dt = (P_loss - q_out) / C_thermal > threshold',
      prevVal: '98.5 °C',
      failureVal: '124.6 °C (Trip > 120.0 °C)',
      origin: 'simulated',
      status: 'unverified'
    }
  ];

  const [selectedNode, setSelectedNode] = useState<CausalNode>(candidateChain[3]); // Candidate root cause by default

  const handleTestCauseClick = async () => {
    // Create intervention branch via engine
    await engine.branch('cp-800', {
      id: 'int-cooling-bypass',
      name: 'Cooling Bypass Loop (+15% flow)',
      checkpointId: 'cp-800',
      paramOverrides: { coolant_flow_rate: { v: 3.2, unit: 'L/min', origin: 'derived', status: 'unverified' } }
    });
    onTestThisCause();
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-transparent z-10 pointer-events-none select-none">
      {/* 1. TOP FAILURE STATUS STRIP (Section 6.11) */}
      <div className="h-11 px-5 flex items-center justify-between border-b border-red-500/40 bg-[#0A1422]/90 backdrop-blur-md pointer-events-auto shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-[#F2F6FB]">Understand Stage</span>
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-red-500/20 border border-red-500/50 text-red-400 flex items-center gap-1.5 shadow-[0_0_12px_rgba(239,68,68,0.3)]">
              <span className="w-2 h-2 rounded-full bg-red-500 animate-ping"></span>
              <span>Winding temperature above limit at 8.42 s</span>
            </span>
            <MockTag />
          </div>

          <span className="text-[#22395A]">|</span>

          <span className="text-[11px] font-mono text-[#7F93AF]">
            Peak Temp: <strong className="text-red-400 font-bold">124.6 °C</strong> (Threshold: 120.0 °C)
          </span>
        </div>

        {/* Action Button: Test This Cause (Section 6.11) */}
        <div className="flex items-center gap-2.5">
          {onBackToSimulate && (
            <button
              onClick={onBackToSimulate}
              className="px-3 py-1 rounded-[6px] text-xs font-mono text-[#7F93AF] hover:text-white"
            >
              ← Back to simulation
            </button>
          )}

          <button
            onClick={handleTestCauseClick}
            className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-gradient-to-r from-[#2F7BFF] to-[#8B5CF6] hover:from-[#3B82F6] hover:to-[#7C3AED] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
          >
            <span>✦</span>
            <span>Test this cause (Branch) →</span>
          </button>
        </div>
      </div>

      {/* 2. THREE-COLUMN UNDERSTAND BODY */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT PANEL: FAILURES & OBSERVATIONS (Section 6.11) */}
        <div className="w-80 border-r border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-3 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Failures & Observations
              </span>
              <span className="text-[10px] font-mono text-red-400">1 Failure</span>
            </div>

            {/* Failure Card */}
            <div className="p-3 rounded-[10px] bg-red-950/40 border border-red-500/50 space-y-1">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-mono font-bold text-red-400 uppercase">FAILURE 01</span>
                <span className="text-[10px] font-mono text-red-300">t = 8.42 s</span>
              </div>
              <div className="text-xs font-semibold text-white">Stator Winding Over-Temperature</div>
              <p className="text-[11px] text-[#D6E2F3]">
                Peak temperature 124.6 °C exceeds limit of 120.0 °C. Thermal protection trip triggered.
              </p>
            </div>

            {/* Observations List */}
            <div className="space-y-2 pt-1">
              <div className="text-[10px] font-mono uppercase tracking-wider text-[#7F93AF]">
                Correlated Observations
              </div>

              <div className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-0.5 text-xs">
                <div className="flex items-center justify-between font-mono text-[10px] text-[#22D3EE]">
                  <span>Obs 01 · t = 4.20 s</span>
                  <span>Torque Step</span>
                </div>
                <div className="text-white font-medium">Mechanical Load Surge</div>
                <div className="text-[10px] text-[#7F93AF]">Load torque increased +53.4 N·m under high hydraulic head.</div>
              </div>

              <div className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-0.5 text-xs">
                <div className="flex items-center justify-between font-mono text-[10px] text-[#22D3EE]">
                  <span>Obs 02 · t = 4.25 s</span>
                  <span>Joule Heat</span>
                </div>
                <div className="text-white font-medium">Phase Current Inrush</div>
                <div className="text-[10px] text-[#7F93AF]">RMS current jumped to 60.5 A, causing 480.2 W ohmic heating.</div>
              </div>

              <div className="p-2.5 rounded-[8px] bg-[#0E1B2E] border border-[#22395A] space-y-0.5 text-xs">
                <div className="flex items-center justify-between font-mono text-[10px] text-[#22D3EE]">
                  <span>Obs 03 · t = 6.80 s</span>
                  <span>Boundary Saturation</span>
                </div>
                <div className="text-white font-medium">Coolant Dissipation Deficit</div>
                <div className="text-[10px] text-[#7F93AF]">Convective heat transfer capacity saturated at 310.5 W.</div>
              </div>
            </div>
          </div>

          {/* Bottom Card: Replay Check Card (Section 6.11) */}
          <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-1 font-mono text-xs">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-[#7F93AF]">Replay Check</span>
              <span className="text-[#4ADE80] font-bold">✓ Matches original</span>
            </div>
            <div className="text-[10px] text-[#7F93AF] flex items-center justify-between">
              <span>Bitwise State Hash:</span>
              <span className="text-cyan-300">0x789030</span>
            </div>
          </div>
        </div>

        {/* CENTER VIEWPORT OVERLAY: THERMAL HEAT GRADIENT CALLOUT */}
        <div className="flex-1 relative flex flex-col justify-between p-4 pointer-events-none">
          {/* Top Floating Hotspot Callout */}
          <div className="p-3 rounded-[12px] bg-[#0A1422]/85 backdrop-blur-md border border-red-500/60 max-w-sm pointer-events-auto space-y-1.5 shadow-[0_0_30px_rgba(239,68,68,0.35)]">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-xs font-bold text-red-400">
                <span className="w-2 h-2 rounded-full bg-red-500 animate-ping"></span>
                <span>Thermal Focus Point</span>
              </div>
              <MockTag />
            </div>

            <div className="text-xs font-semibold text-white">
              Actuator Stator Winding Insulation
            </div>

            <div className="flex items-baseline gap-2 font-mono">
              <span className="text-xl font-bold text-red-400">124.6 °C</span>
              <span className="text-xs text-[#7F93AF]">/ Limit 120.0 °C</span>
            </div>

            {/* Thermal Gradient Legend */}
            <div className="pt-1 space-y-1">
              <div className="flex justify-between text-[9px] font-mono text-[#7F93AF]">
                <span>20 °C</span>
                <span>Heat Gradient</span>
                <span>125 °C</span>
              </div>
              <div className="w-full h-1.5 rounded-full bg-gradient-to-r from-blue-500 via-cyan-400 via-amber-400 to-red-600"></div>
            </div>
          </div>

          {/* Center Leader Line Tag */}
          <div className="self-center p-2 rounded-[8px] bg-[#0A1422]/90 border border-[#22395A] backdrop-blur-md pointer-events-auto text-center space-y-0.5">
            <div className="text-[10px] font-mono text-[#22D3EE] font-bold">Candidate: structural and temporal</div>
            <div className="text-xs text-white">Coolant convection bottleneck at high-torque step</div>
          </div>

          {/* Bottom Checkpoint Bar */}
          <div className="p-3 rounded-[12px] bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] pointer-events-auto flex items-center justify-between text-xs font-mono">
            <div className="flex items-center gap-2 text-[#7F93AF]">
              <span>Freeze Checkpoint:</span>
              <span className="text-white font-bold">t = 8.00 s</span>
              <span>(0.42 s before trip)</span>
            </div>
            <button
              onClick={handleTestCauseClick}
              className="px-3 py-1 rounded-[6px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6]"
            >
              Test this cause →
            </button>
          </div>
        </div>

        {/* RIGHT PANEL: VERTICAL CANDIDATE CHAIN (5 NODES) & DETAIL CARD (Section 6.11) */}
        <div className="w-80 border-l border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-3 overflow-y-auto">
          {/* Vertical Chain */}
          <div className="space-y-2">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Candidate Chain (5 Nodes)
              </span>
              <span className="text-[9px] px-1.5 py-0.2 rounded bg-purple-500/20 text-purple-300 font-mono">
                Temporal
              </span>
            </div>

            <div className="space-y-1.5 relative">
              {candidateChain.map((node, idx) => {
                const isSelected = selectedNode.id === node.id;
                return (
                  <div
                    key={node.id}
                    onClick={() => setSelectedNode(node)}
                    className={`p-2.5 rounded-[8px] border cursor-pointer transition-all ${
                      isSelected
                        ? 'bg-[#2F7BFF]/25 border-[#2F7BFF] shadow-[0_0_12px_rgba(47,123,255,0.3)]'
                        : node.isCandidateRootCause
                        ? 'bg-amber-500/10 border-amber-500/40 text-amber-200'
                        : 'bg-[#0E1B2E]/60 border-[#22395A] hover:bg-[#13253D]/80 text-[#D6E2F3]'
                    }`}
                  >
                    <div className="flex items-center justify-between text-xs">
                      <div className="flex items-center gap-1.5">
                        <span className="font-mono text-[10px] text-slate-400">{node.step}</span>
                        <span className="font-semibold truncate">{node.name}</span>
                      </div>
                      {node.isCandidateRootCause && (
                        <span className="text-[8px] px-1 rounded bg-amber-500/20 text-amber-300 font-mono font-bold">
                          ROOT CAUSE
                        </span>
                      )}
                    </div>
                    <div className="flex items-center justify-between text-[10px] font-mono text-[#7F93AF] mt-1">
                      <span>{node.component}</span>
                      <span className="text-white">{node.failureVal}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Selected Node Detail Card (Section 6.11) */}
          <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-2">
            <div className="flex items-center justify-between border-b border-[#22395A] pb-1.5">
              <span className="text-xs font-bold text-white">Node Detail</span>
              <div className="flex items-center gap-1">
                <span className="px-1.5 py-0.2 rounded text-[9px] font-mono uppercase bg-[#8B5CF6]/15 border border-[#8B5CF6]/30 text-purple-300">
                  {selectedNode.origin}
                </span>
                <span className="px-1.5 py-0.2 rounded text-[9px] font-mono uppercase bg-[#22C55E]/15 border border-[#22C55E]/40 text-[#4ADE80]">
                  {selectedNode.status}
                </span>
              </div>
            </div>

            <div className="space-y-1.5 text-xs">
              <div className="font-semibold text-[#22D3EE]">{selectedNode.name}</div>
              <div className="text-[11px] text-[#7F93AF] font-mono">
                Component: <span className="text-white">{selectedNode.component}</span>
              </div>
              <div className="p-2 rounded bg-[#13253D] border border-[#22395A] font-mono text-[10px] text-[#D6E2F3]">
                {selectedNode.relation}
              </div>

              <div className="grid grid-cols-2 gap-2 font-mono text-center pt-1">
                <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                  <div className="text-[9px] text-[#7F93AF]">Previous (t=4.0s)</div>
                  <div className="text-xs font-bold text-white">{selectedNode.prevVal}</div>
                </div>
                <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A]">
                  <div className="text-[9px] text-[#7F93AF]">At Trip (t=8.42s)</div>
                  <div className="text-xs font-bold text-red-400">{selectedNode.failureVal}</div>
                </div>
              </div>
            </div>

            <button
              onClick={handleTestCauseClick}
              className="w-full py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-colors shadow-[0_0_15px_rgba(47,123,255,0.4)] mt-2"
            >
              Test this cause (Create branch)
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
