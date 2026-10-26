import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { PreviewTag, DeterminismChip } from '../../components/ui/HonestyBadges';

export const DeployExportPage: React.FC = () => {
  const { determinism, showToast } = useAppState();

  const [selectedFormat, setSelectedFormat] = useState('STEP');

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto overflow-y-auto">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-white tracking-wide">Export & Evidence Distribution Gateway</h1>
            <PreviewTag label="PREVIEW (Phase 5)" />
          </div>
          <p className="text-xs text-slate-400 mt-0.5">Cryptographically signed evidence packages, CAD geometry exports, and hardware deployment.</p>
        </div>

        <div className="flex items-center gap-2">
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Main Grid: Left Export Formats, Center Cloud & Hardware Architecture, Right Deploy Integrations */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left Column: Model & Results Exports (Prompt 15) */}
        <div className="glass-panel p-5 flex flex-col gap-4">
          <div className="border-b border-slate-800 pb-3">
            <h2 className="text-sm font-semibold text-white">Export CAD & Results</h2>
            <p className="text-xs text-slate-400 mt-0.5">Select mathematical artifacts to download</p>
          </div>

          <div className="space-y-3">
            <div>
              <span className="text-xs font-bold text-slate-300">Model Geometry Formats</span>
              <div className="grid grid-cols-2 gap-2 mt-1.5">
                {[
                  { id: 'STEP', label: 'STEP AP242', preview: true },
                  { id: 'STL', label: 'STL Triangulated' },
                  { id: 'OBJ', label: 'Wavefront OBJ' },
                  { id: 'FBX', label: 'Autodesk FBX' }
                ].map(fmt => (
                  <button
                    key={fmt.id}
                    onClick={() => setSelectedFormat(fmt.id)}
                    className={`p-2.5 rounded-xl border text-left text-xs transition-all flex items-center justify-between ${
                      selectedFormat === fmt.id
                        ? 'bg-blue-600/30 border-cyan-400 text-cyan-200 font-semibold shadow-[0_0_10px_rgba(34,211,238,0.25)]'
                        : 'bg-slate-900/60 border-slate-800 text-slate-400 hover:text-white'
                    }`}
                  >
                    <span>{fmt.label}</span>
                    {fmt.preview && (
                      <span className="text-[9px] px-1 py-0.2 rounded bg-amber-500/20 text-amber-300 border border-amber-500/30">
                        PREVIEW
                      </span>
                    )}
                  </button>
                ))}
              </div>
            </div>

            <div className="pt-2 border-t border-slate-800">
              <span className="text-xs font-bold text-slate-300">Simulation Data Outputs</span>
              <div className="space-y-1.5 mt-1.5">
                {[
                  { name: 'Time-Series Raw CSV', size: '24.2 MB' },
                  { name: 'Full Reality State JSON', size: '8.4 MB' },
                  { name: '60 FPS WebM Viewport Recording', size: '42.0 MB' }
                ].map(out => (
                  <div key={out.name} className="p-2 rounded-lg bg-slate-900/40 border border-slate-800 flex items-center justify-between text-xs">
                    <span className="text-slate-300">{out.name}</span>
                    <span className="text-[10px] font-mono text-cyan-400">{out.size}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Center: Cloud-and-Device Graphic & Signed Evidence Package (Prompt 15) */}
        <div className="glass-panel p-5 flex flex-col justify-between gap-4">
          <div className="space-y-3">
            <div className="border-b border-slate-800 pb-3 flex items-center justify-between">
              <h2 className="text-sm font-semibold text-white">Signed Evidence Package</h2>
              <span className="text-[10px] font-mono text-emerald-400">TAR.ZST</span>
            </div>

            {/* Cloud & Device Graphic */}
            <div className="w-full h-36 rounded-xl bg-gradient-to-b from-blue-950/40 to-slate-950/60 border border-blue-500/30 flex flex-col items-center justify-center p-3 relative overflow-hidden">
              <div className="text-3xl text-cyan-400 mb-1">☁ ⇄ ⚡ ⇄ ⚙</div>
              <div className="text-xs font-mono text-slate-200 font-semibold">Immutable Provenance Chain</div>
              <div className="text-[10px] text-slate-400 font-mono mt-0.5">Authoritative Rust Core ⟷ Reality Studio</div>
            </div>

            <div className="p-3 rounded-xl bg-slate-900/80 border border-slate-800 space-y-1.5 text-xs font-mono">
              <div className="flex justify-between">
                <span className="text-slate-500">Root Hash</span>
                <span className="text-cyan-300 font-bold">0x7f8a9e4b...</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Signature</span>
                <span className="text-emerald-400 font-bold">ECDSA P-256 Valid</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Manifest</span>
                <span className="text-slate-300">18 Files Included</span>
              </div>
            </div>
          </div>

          <button
            onClick={() => showToast('Evidence package exported with bitwise SHA256 manifest', 'success')}
            className="w-full ui-btn ui-btn-primary py-3 justify-center text-xs font-semibold shadow-[0_0_20px_rgba(47,123,255,0.4)]"
          >
            <span>Export Evidence Package</span>
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          </button>
        </div>

        {/* Right Column: Deploy to Edge Device & Integrations (Prompt 15) */}
        <div className="glass-panel p-5 flex flex-col justify-between gap-4">
          <div className="space-y-3">
            <div className="border-b border-slate-800 pb-3">
              <h2 className="text-sm font-semibold text-white">Deploy & Integrate</h2>
              <p className="text-xs text-slate-400 mt-0.5">Live streaming and controller flashing</p>
            </div>

            <div className="space-y-2.5">
              {[
                { name: 'Deploy to Edge Device', desc: 'Flash firmware & boundary checks to Jetson/ARM', active: true },
                { name: 'Integrate REST / gRPC API', desc: 'Stream state updates at 1000 Hz', active: false },
                { name: 'IoT MQTT Gateway', desc: 'Secure broker for industrial PLC sensors', active: false },
                { name: 'Real-time Webhook', desc: 'Post verification failure events to Slack/Ops', active: false }
              ].map(dep => (
                <div key={dep.name} className="p-3 rounded-xl bg-slate-900/60 border border-slate-800 hover:border-blue-500/40 transition-colors cursor-pointer group">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-white group-hover:text-cyan-300 transition-colors">
                      {dep.name}
                    </span>
                    <span className="text-[10px] text-slate-500 font-mono">Connect →</span>
                  </div>
                  <p className="text-[11px] text-slate-400 mt-0.5">{dep.desc}</p>
                </div>
              ))}
            </div>
          </div>

          <button
            onClick={() => showToast('Connecting to local Edge Device gateway...', 'info')}
            className="w-full ui-btn ui-btn-outline py-2.5 justify-center text-xs"
          >
            Flash to Edge Controller
          </button>
        </div>
      </div>
    </div>
  );
};
