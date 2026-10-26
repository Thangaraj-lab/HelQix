import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { HeroScene } from '../../scene/HeroScene';
import { OriginStatusBadges } from '../../components/ui/HonestyBadges';
import { MOCK_COMPONENTS } from '../../mock/engineeringData';

export const ModelEditorPage: React.FC = () => {
  const { qualityTier, selectedComponentId, setSelectedComponentId } = useAppState();

  const [activeTab, setActiveTab] = useState<'Properties' | 'Parameters' | 'Relations'>('Properties');
  const [transparency, setTransparency] = useState<number>(0.35);
  const [explodedSlider, setExplodedSlider] = useState<number>(0.45);
  const [showConnections, setShowConnections] = useState<boolean>(true);
  const [showSensors, setShowSensors] = useState<boolean>(true);

  const selectedCmp = MOCK_COMPONENTS.find(c => c.id === selectedComponentId) || MOCK_COMPONENTS[0];

  return (
    <div className="relative w-full h-full flex flex-col justify-between overflow-hidden p-3 gap-3">
      {/* Top Breadcrumb & Toolbar */}
      <div className="flex items-center justify-between z-20 shrink-0">
        <div className="flex items-center gap-2 text-xs font-mono">
          <span className="text-cyan-400 font-semibold">MODEL EDITOR // EXPLODED KINEMATIC ASSEMBLY</span>
          <span className="px-2 py-0.5 rounded bg-blue-500/20 text-blue-300 border border-blue-500/30">
            CAD/STEP v3.2
          </span>
        </div>

        {/* Viewport Action Tools (Image 4) */}
        <div className="flex items-center gap-1 p-1 rounded-xl glass-panel">
          {['Pointer', 'Box', 'Graph', 'Ruler', 'Layers', 'Signal'].map((tool, idx) => (
            <button
              key={tool}
              className={`p-1.5 rounded-lg text-xs transition-colors ${
                idx === 0 ? 'bg-blue-600/40 text-cyan-300 border border-blue-400/50' : 'text-slate-400 hover:text-white'
              }`}
              title={tool}
            >
              <span className="text-xs px-1 font-mono">{tool.slice(0, 1)}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Main Center Area: Left Explorer, Center Exploded 3D, Right Properties (Image 4) */}
      <div className="flex-1 grid grid-cols-12 gap-3 min-h-0">
        {/* Left: Model Explorer Tree (3 Cols) */}
        <div className="col-span-3 glass-panel p-3.5 flex flex-col gap-2.5 overflow-y-auto">
          <div className="flex items-center justify-between border-b border-slate-800/80 pb-2">
            <span className="text-xs font-bold text-white flex items-center gap-1.5">
              <svg className="w-3.5 h-3.5 text-cyan-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
              </svg>
              <span>Model Explorer</span>
            </span>
            <button className="text-cyan-400 hover:text-cyan-300 text-xs font-bold px-1.5 py-0.5 rounded bg-blue-950/60 border border-blue-500/40">
              +
            </button>
          </div>

          {/* Hierarchical Tree with colored icons (Image 4) */}
          <div className="space-y-1 text-xs">
            <div className="flex items-center gap-2 p-1.5 rounded bg-blue-950/30 text-cyan-300 font-semibold">
              <svg className="w-3.5 h-3.5 text-blue-400" viewBox="0 0 24 24" fill="currentColor">
                <circle cx="12" cy="12" r="6" />
              </svg>
              <span>System (Powertrain Assembly)</span>
            </div>

            <div className="pl-3 space-y-1">
              <div className="flex items-center gap-2 text-slate-300 font-medium p-1">
                <span className="text-cyan-400">▾</span>
                <span>Structure</span>
              </div>

              <div className="pl-4 space-y-1">
                {MOCK_COMPONENTS.map(cmp => (
                  <button
                    key={cmp.id}
                    onClick={() => setSelectedComponentId(cmp.id)}
                    className={`w-full text-left p-1.5 rounded flex items-center justify-between transition-colors ${
                      selectedComponentId === cmp.id
                        ? 'bg-blue-600/30 text-cyan-200 border border-blue-400/40 font-semibold'
                        : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/40'
                    }`}
                  >
                    <div className="flex items-center gap-2 truncate">
                      <span className="w-2 h-2 rounded bg-cyan-400"></span>
                      <span className="truncate">{cmp.name}</span>
                    </div>
                  </button>
                ))}
              </div>

              {/* Sub-domains */}
              {[
                { name: 'Thermal Domain', color: 'text-amber-400' },
                { name: 'Electrical Domain', color: 'text-purple-400' },
                { name: 'Control Domain', color: 'text-emerald-400' },
                { name: 'Environment', color: 'text-sky-400' },
                { name: 'Sensors & Probes', color: 'text-rose-400' }
              ].map(sub => (
                <div key={sub.name} className="flex items-center justify-between p-1.5 rounded text-slate-400 hover:text-white cursor-pointer hover:bg-slate-800/30">
                  <div className="flex items-center gap-2">
                    <span className={`w-2 h-2 rounded-full ${sub.color}`}></span>
                    <span>{sub.name}</span>
                  </div>
                  <span className="text-[10px] text-slate-600 font-mono">▸</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Center: Exploded View 3D Canvas (6 Cols) */}
        <div className="col-span-6 relative rounded-2xl overflow-hidden border border-blue-500/20 bg-[#050B18]">
          <HeroScene
            variant="model_editor"
            interactive={true}
            qualityTier={qualityTier}
            explodedOffset={explodedSlider}
          />

          {/* Floating Exploded Offset Slider (matches Image 4) */}
          <div className="absolute bottom-4 left-6 right-6 p-2.5 rounded-xl glass-panel flex items-center gap-4 z-10">
            <span className="text-xs font-mono text-cyan-300 shrink-0">Exploded Spacing:</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={explodedSlider}
              onChange={e => setExplodedSlider(parseFloat(e.target.value))}
              className="flex-1"
            />
            <span className="text-xs font-mono text-white num shrink-0">{(explodedSlider * 100).toFixed(0)}%</span>
          </div>
        </div>

        {/* Right: Component Properties, Visual Settings & Performance (3 Cols) */}
        <div className="col-span-3 flex flex-col gap-3 overflow-y-auto">
          {/* Component Properties (Image 4) */}
          <div className="glass-panel p-3.5 flex flex-col gap-2.5">
            <div className="flex items-center justify-between border-b border-slate-800/80 pb-2">
              <span className="text-xs font-bold text-white truncate">{selectedCmp.name}</span>
              <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-500/20 text-emerald-400 border border-emerald-500/40">
                Active
              </span>
            </div>

            {/* Tabs: Properties / Parameters / Relations */}
            <div className="flex items-center gap-1 p-0.5 rounded-lg bg-slate-900/80 border border-slate-800 text-xs">
              {(['Properties', 'Parameters', 'Relations'] as const).map(tab => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={`flex-1 py-1 rounded text-center transition-colors ${
                    activeTab === tab
                      ? 'bg-blue-600/40 text-cyan-300 font-semibold border border-blue-400/40'
                      : 'text-slate-400 hover:text-white'
                  }`}
                >
                  {tab}
                </button>
              ))}
            </div>

            {/* Properties fields */}
            <div className="space-y-2 text-xs">
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Component ID</span>
                <span className="font-mono text-cyan-300 font-semibold">{selectedCmp.id}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Type</span>
                <span className="text-slate-200">{selectedCmp.type}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Material</span>
                <span className="text-slate-200">{selectedCmp.material}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Mass</span>
                <span className="font-mono text-white num">{selectedCmp.mass} {selectedCmp.massUnit}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-400">Dimension</span>
                <span className="font-mono text-white num">{selectedCmp.dimensions} {selectedCmp.dimUnit}</span>
              </div>
            </div>
          </div>

          {/* Visual Settings (Image 4) */}
          <div className="glass-panel p-3.5 flex flex-col gap-2.5">
            <span className="text-xs font-bold text-white">Visual Settings</span>

            <div className="space-y-2">
              <div className="space-y-1">
                <div className="flex items-center justify-between text-[11px] text-slate-400">
                  <span>Transparency</span>
                  <span className="font-mono text-cyan-300 num">{transparency.toFixed(2)}</span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={transparency}
                  onChange={e => setTransparency(parseFloat(e.target.value))}
                />
              </div>

              <div className="flex items-center justify-between text-xs text-slate-300 pt-1">
                <span>Show Connections</span>
                <input
                  type="checkbox"
                  checked={showConnections}
                  onChange={e => setShowConnections(e.target.checked)}
                  className="accent-cyan-400"
                />
              </div>

              <div className="flex items-center justify-between text-xs text-slate-300">
                <span>Show Sensors</span>
                <input
                  type="checkbox"
                  checked={showSensors}
                  onChange={e => setShowSensors(e.target.checked)}
                  className="accent-cyan-400"
                />
              </div>
            </div>
          </div>

          {/* Performance Preview (Image 4) */}
          <div className="glass-panel p-3.5 flex flex-col gap-2">
            <div className="flex items-center justify-between border-b border-slate-800/80 pb-2">
              <span className="text-xs font-bold text-white">Performance Preview</span>
              <OriginStatusBadges origin="Derived" status="Verified" />
            </div>

            <div className="space-y-1.5 text-[11px]">
              {[
                { label: 'Load', val: selectedCmp.load, unit: '%', color: 'bg-blue-500' },
                { label: 'Temperature', val: selectedCmp.temperature, unit: '°C', color: 'bg-purple-500' },
                { label: 'Vibration', val: selectedCmp.vibration, unit: 'mm/s', color: 'bg-cyan-400' },
                { label: 'Efficiency', val: selectedCmp.efficiency, unit: '%', color: 'bg-emerald-400' }
              ].map(p => (
                <div key={p.label} className="space-y-0.5">
                  <div className="flex justify-between">
                    <span className="text-slate-400">{p.label}</span>
                    <span className="font-mono text-white num">{p.val} {p.unit}</span>
                  </div>
                  <div className="w-full h-1 bg-slate-900 rounded-full overflow-hidden">
                    <div className={`h-full ${p.color}`} style={{ width: `${p.val}%` }}></div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Bottom Component Library & Details (Image 4) */}
      <div className="h-32 grid grid-cols-12 gap-3 shrink-0">
        {/* Component Library Grid (7 Cols) */}
        <div className="col-span-7 glass-panel p-3 flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-white flex items-center gap-1.5">
              <svg className="w-3.5 h-3.5 text-cyan-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="7" height="7" />
                <rect x="14" y="3" width="7" height="7" />
                <rect x="14" y="14" width="7" height="7" />
                <rect x="3" y="14" width="7" height="7" />
              </svg>
              <span>Component Library</span>
            </span>
            <input
              type="text"
              placeholder="Search components..."
              className="px-2 py-0.5 rounded bg-slate-900/80 border border-slate-700 text-xs text-white outline-none w-44"
            />
          </div>

          <div className="flex items-center gap-2 overflow-x-auto py-1">
            {MOCK_COMPONENTS.map(cmp => (
              <button
                key={cmp.id}
                onClick={() => setSelectedComponentId(cmp.id)}
                className={`p-2 rounded-xl shrink-0 flex flex-col items-center gap-1 border transition-all ${
                  selectedComponentId === cmp.id
                    ? 'bg-blue-600/30 border-cyan-400 shadow-[0_0_10px_rgba(34,211,238,0.25)]'
                    : 'bg-slate-900/60 border-slate-800 hover:border-slate-700'
                }`}
              >
                <div className="w-10 h-8 rounded bg-gradient-to-tr from-blue-900 to-indigo-800 flex items-center justify-center text-cyan-300 font-mono text-xs">
                  ⚙
                </div>
                <span className="text-[10px] text-slate-300 font-medium truncate max-w-[80px]">{cmp.name.split(' ')[0]}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Component Details Card (5 Cols) */}
        <div className="col-span-5 glass-panel p-3 flex items-center gap-4">
          <div className="w-20 h-20 rounded-xl bg-gradient-to-tr from-blue-950 to-slate-900 border border-blue-500/30 flex items-center justify-center text-3xl text-cyan-400 shadow-[0_0_15px_rgba(47,123,255,0.2)]">
            ⚙
          </div>
          <div className="space-y-1 text-xs">
            <div className="font-bold text-white">{selectedCmp.name}</div>
            <div className="text-[11px] text-slate-400">{selectedCmp.type}</div>
            <div className="flex items-center gap-4 text-[11px] text-slate-300 font-mono pt-1">
              <span>Mass: {selectedCmp.mass} {selectedCmp.massUnit}</span>
              <span>•</span>
              <span>Connected to {selectedCmp.connectedTo} components</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
