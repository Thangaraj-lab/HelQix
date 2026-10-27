import React, { useState } from 'react';
import { MockEngineClient } from '../../mock/MockEngineClient';
import { MockTag, PlaceholderTag, ExampleTag, ValueView, Panel } from '../../components/ui/UiKit';

export interface ModelStageProps {
  onRunSimulation?: () => void;
}

export const ModelStage: React.FC<ModelStageProps> = ({ onRunSimulation }) => {
  const [selectedPartId, setSelectedPartId] = useState<string>('part-1');
  const [activeTab, setActiveTab] = useState<'parameters' | 'behavior' | 'ports' | 'source'>('parameters');
  const [viewMode, setViewMode] = useState<'3d' | 'graph'>('3d');
  const [bottomTab, setBottomTab] = useState<'issues' | 'connections' | 'history' | 'sources'>('connections');

  // Layers toggle (Section 6.9)
  const [layers, setLayers] = useState<Record<string, boolean>>({
    Structure: true,
    Physics: true,
    Environment: true,
    State: true,
    Flow: false,
    Sensors: true,
    Connections: true,
    Failures: false,
    Observations: false
  });

  const toggleLayer = (k: string) => {
    setLayers(prev => ({ ...prev, [k]: !prev[k] }));
  };

  const parts = [
    { id: 'part-1', name: 'Power Conversion Unit (Example)', maturity: 'validated', shape: 'Box', dims: '120 × 80 × 40 mm', ports: 2 },
    { id: 'part-2', name: 'Actuator Core Subsystem (Example)', maturity: 'validated', shape: 'Cylinder', dims: '90 × 140 × 90 mm', ports: 3 },
    { id: 'part-3', name: 'Thermal Sensor Probe (Example)', maturity: 'validated', shape: 'Sphere', dims: '15 × 15 × 15 mm', ports: 1 },
    { id: 'part-4', name: 'Hydraulic Expeller (no model) (Example)', maturity: 'placeholder', shape: 'Box', dims: '110 × 110 × 90 mm', ports: 2 }
  ];

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-transparent z-10 pointer-events-none select-none">
      {/* 1. TOP STATUS STRIP (Section 6.9) */}
      <div className="h-11 px-5 flex items-center justify-between border-b border-[#22395A]/80 bg-[#0A1422]/90 backdrop-blur-md pointer-events-auto shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-[#F2F6FB]">Model Stage</span>
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider bg-blue-500/15 border border-blue-500/40 text-[#22D3EE]">
              Model r1 · Engineering View
            </span>
            <MockTag />
          </div>

          <span className="text-[#22395A]">|</span>

          {/* 3D vs Graph View Toggle (Section 6.9) */}
          <div className="flex items-center gap-1 p-0.5 rounded-[6px] bg-[#13253D] border border-[#22395A] text-xs font-mono">
            <button
              onClick={() => setViewMode('3d')}
              className={`px-2 py-0.5 rounded-[4px] ${
                viewMode === '3d' ? 'bg-[#2F7BFF] text-white font-bold' : 'text-[#7F93AF] hover:text-white'
              }`}
            >
              3D Assembly
            </button>
            <button
              onClick={() => setViewMode('graph')}
              className={`px-2 py-0.5 rounded-[4px] ${
                viewMode === 'graph' ? 'bg-[#2F7BFF] text-white font-bold' : 'text-[#7F93AF] hover:text-white'
              }`}
            >
              Port Graph
            </button>
          </div>
        </div>

        {/* Primary Action Button */}
        <div className="flex items-center gap-2.5">
          <button
            onClick={onRunSimulation}
            className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] shadow-[0_0_20px_rgba(47,123,255,0.4)] transition-all flex items-center gap-1.5"
          >
            <span>▶</span>
            <span>Run simulation →</span>
          </button>
        </div>
      </div>

      {/* 2. THREE-COLUMN MODEL BODY */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT PANEL: COMPONENT HIERARCHY & LAYERS (Section 6.9) */}
        <div className="w-72 border-r border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-3.5 space-y-3 overflow-y-auto">
          {/* Component List */}
          <div className="space-y-2">
            <div className="flex items-center justify-between pb-1 border-b border-[#22395A]">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Components (04)
              </span>
              <span className="text-[10px] font-mono text-[#7F93AF]">Hierarchy</span>
            </div>

            <div className="space-y-1.5">
              {parts.map(p => {
                const isSelected = selectedPartId === p.id;
                return (
                  <div
                    key={p.id}
                    onClick={() => setSelectedPartId(p.id)}
                    className={`p-2 rounded-[8px] border cursor-pointer transition-all ${
                      isSelected
                        ? 'bg-[#2F7BFF]/20 border-[#2F7BFF] text-white shadow-[0_0_12px_rgba(47,123,255,0.25)]'
                        : 'bg-[#0E1B2E]/60 border-[#22395A] hover:bg-[#13253D]/80 text-[#D6E2F3]'
                    }`}
                  >
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-semibold truncate">{p.name}</span>
                      {p.maturity === 'placeholder' && <PlaceholderTag />}
                    </div>
                    <div className="flex items-center justify-between text-[10px] font-mono text-[#7F93AF] mt-1">
                      <span>{p.shape}</span>
                      <span>{p.ports} ports</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Layers Toggle Strip (Section 6.9: Structure, Physics, Environment, etc.) */}
          <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-1.5">
            <div className="text-xs font-bold text-white uppercase tracking-wider font-mono">
              Display Layers
            </div>
            <div className="grid grid-cols-2 gap-1 text-[10px] font-mono">
              {Object.entries(layers).map(([layerName, isEnabled]) => (
                <button
                  key={layerName}
                  onClick={() => toggleLayer(layerName)}
                  className={`p-1 rounded text-left flex items-center justify-between border ${
                    isEnabled
                      ? 'bg-[#2F7BFF]/20 border-[#2F7BFF]/50 text-cyan-300'
                      : 'bg-[#13253D]/50 border-transparent text-[#7F93AF]'
                  }`}
                >
                  <span className="truncate">{layerName}</span>
                  <span className="text-[8px]">{isEnabled ? '●' : '○'}</span>
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* CENTER VIEWPORT OVERLAY */}
        <div className="flex-1 relative flex flex-col justify-between p-4 pointer-events-none">
          <div className="p-3 rounded-[12px] bg-[#0A1422]/85 backdrop-blur-md border border-[#22395A] max-w-sm pointer-events-auto space-y-1">
            <div className="text-xs font-bold text-[#F2F6FB] uppercase font-mono tracking-wider">
              CAD INSPECTION & COORDINATES
            </div>
            <div className="text-[11px] text-[#7F93AF] font-mono">
              Selection outline active · Snapping grid 1 mm
            </div>
          </div>

          {/* Bottom Tabs Drawer (Issues, Connections, History, Sources) */}
          <div className="p-3 rounded-[14px] bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] pointer-events-auto space-y-2">
            <div className="flex items-center justify-between border-b border-[#22395A]/80 pb-2">
              <div className="flex items-center gap-2">
                {(['issues', 'connections', 'history', 'sources'] as const).map(tab => (
                  <button
                    key={tab}
                    onClick={() => setBottomTab(tab)}
                    className={`px-3 py-1 rounded-[6px] text-xs font-mono font-medium transition-all ${
                      bottomTab === tab
                        ? 'bg-[#2F7BFF]/30 text-[#22D3EE] font-semibold border border-[#2F7BFF]/50'
                        : 'text-[#7F93AF] hover:text-white'
                    }`}
                  >
                    <span className="capitalize">{tab}</span>
                  </button>
                ))}
              </div>
              <span className="text-[10px] font-mono text-[#7F93AF]">Revision r1 Snapshot</span>
            </div>

            <div className="max-h-24 overflow-y-auto text-xs font-mono text-[#7F93AF]">
              {bottomTab === 'connections' && (
                <div className="grid grid-cols-3 gap-2">
                  <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A] text-[11px] text-white">
                    dc-out → dc-in (Electrical DC, 48.0 V)
                  </div>
                  <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A] text-[11px] text-white">
                    mech-out → mech-in (Mechanical Torque)
                  </div>
                  <div className="p-1.5 rounded bg-[#13253D] border border-[#22395A] text-[11px] text-white">
                    analog-out → feedback-in (Telemetry Signal)
                  </div>
                </div>
              )}
              {bottomTab === 'issues' && <div className="text-slate-400 py-1">0 blocking issues. All units compatible.</div>}
              {bottomTab === 'history' && <div className="text-slate-400 py-1">Model revision r1 committed from live assembly.</div>}
              {bottomTab === 'sources' && <div className="text-slate-400 py-1">3 CAD repositories & catalog sources synchronized.</div>}
            </div>
          </div>
        </div>

        {/* RIGHT PANEL: COMPONENT DETAILS (Section 6.9) */}
        <div className="w-80 border-l border-[#22395A]/80 bg-[#0A1422]/85 backdrop-blur-md flex flex-col justify-between shrink-0 pointer-events-auto p-4 space-y-4 overflow-y-auto">
          <div className="space-y-3">
            <div className="border-b border-[#22395A] pb-2 flex items-center justify-between">
              <span className="text-xs font-bold text-[#F2F6FB] uppercase tracking-wider font-mono">
                Component Details
              </span>
              <span className="text-[10px] font-mono text-[#22D3EE]">Properties</span>
            </div>

            {/* Tabs: Parameters, Behavior, Ports, Source (Section 6.9) */}
            <div className="flex items-center gap-1 border-b border-[#22395A]/60 pb-1.5">
              {(['parameters', 'behavior', 'ports', 'source'] as const).map(tab => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={`px-2 py-0.5 rounded text-[11px] font-mono capitalize transition-all ${
                    activeTab === tab
                      ? 'bg-[#2F7BFF]/25 text-[#22D3EE] font-bold border border-[#2F7BFF]/40'
                      : 'text-[#7F93AF] hover:text-white'
                  }`}
                >
                  {tab}
                </button>
              ))}
            </div>

            {activeTab === 'parameters' && (
              <div className="space-y-2 text-xs font-mono">
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
                  <span className="text-[#7F93AF]">Rated Voltage:</span>
                  <span className="text-white font-bold">48.0 V</span>
                </div>
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
                  <span className="text-[#7F93AF]">Nominal Speed:</span>
                  <span className="text-white font-bold">3000 RPM</span>
                </div>
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A] flex items-center justify-between">
                  <span className="text-[#7F93AF]">Thermal Limit:</span>
                  <span className="text-red-400 font-bold">120.0 °C</span>
                </div>
              </div>
            )}

            {activeTab === 'ports' && (
              <div className="space-y-1.5 text-xs font-mono">
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A]">
                  <div className="text-cyan-300 font-bold">dc-in (Input)</div>
                  <div className="text-[10px] text-[#7F93AF]">Electrical DC · 48.0 V</div>
                </div>
                <div className="p-2 rounded bg-[#0E1B2E] border border-[#22395A]">
                  <div className="text-purple-300 font-bold">mech-out (Output)</div>
                  <div className="text-[10px] text-[#7F93AF]">Mechanical Torque · N·m</div>
                </div>
              </div>
            )}

            {/* PBR Material Card (Section 6.9) */}
            <div className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-2">
              <div className="text-xs font-bold text-white uppercase font-mono">PBR Material Swatch</div>
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-gradient-to-tr from-slate-700 via-slate-400 to-cyan-300 shadow-[0_0_15px_rgba(34,211,238,0.3)] border border-[#22395A]"></div>
                <div className="text-xs font-mono">
                  <div className="text-white font-semibold">Anodized 6061-T6</div>
                  <div className="text-[10px] text-[#7F93AF]">Roughness: 0.22 · Metalness: 0.88</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
