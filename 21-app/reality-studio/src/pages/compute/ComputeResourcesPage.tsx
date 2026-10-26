import React, { useState } from 'react';
import { useAppState } from '../../engineering/stateStore';
import { PreviewTag, DeterminismChip } from '../../components/ui/HonestyBadges';
import { MOCK_COMPUTE_NODES, ComputeNode } from '../../mock/engineeringData';

export const ComputeResourcesPage: React.FC = () => {
  const { determinism, showToast } = useAppState();

  const [activeSegment, setActiveSegment] = useState<'Local' | 'Edge' | 'Cloud' | 'Quantum'>('Local');

  const filteredNodes = MOCK_COMPUTE_NODES.filter(
    n => activeSegment === 'Local' ? true : n.type === activeSegment
  );

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto overflow-y-auto">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-white tracking-wide">Hybrid Compute & Distributed Solver Fabric</h1>
            <PreviewTag label="PREVIEW (Phase 6)" />
          </div>
          <p className="text-xs text-slate-400 mt-0.5">Heterogeneous execution routing between Local Workstations, Edge Nodes, and Cloud Clusters.</p>
        </div>

        <div className="flex items-center gap-2">
          <DeterminismChip level={determinism} />
        </div>
      </div>

      {/* Segmented Control (Local, Edge, Cloud, Quantum marked PREVIEW) */}
      <div className="flex items-center gap-2 p-1 rounded-xl glass-panel w-fit">
        {(['Local', 'Edge', 'Cloud', 'Quantum'] as const).map(seg => (
          <button
            key={seg}
            onClick={() => setActiveSegment(seg)}
            className={`px-4 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-2 transition-all ${
              activeSegment === seg
                ? 'bg-blue-600/40 text-cyan-300 border border-blue-400/50 shadow-[0_0_12px_rgba(47,123,255,0.3)]'
                : 'text-slate-400 hover:text-white'
            }`}
          >
            <span>{seg}</span>
            {seg === 'Quantum' && (
              <span className="text-[9px] px-1 py-0.2 rounded bg-amber-500/20 text-amber-300 border border-amber-500/30">
                PREVIEW, simulator only
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Resource Cards: CPU, GPU, RAM, Storage (Prompt 13) */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {[
          { label: 'Workstation CPU Load', val: '42.8%', detail: '16 Cores @ 4.8 GHz', pct: 42.8, color: 'bg-cyan-400' },
          { label: 'Cluster GPU Allocation', val: '88.5%', detail: '64x H100 80GB VRAM', pct: 88.5, color: 'bg-blue-500' },
          { label: 'System Memory (RAM)', val: '54.2 GB', detail: 'Of 128 GB ECC Registered', pct: 42.3, color: 'bg-indigo-400' },
          { label: 'Fast ZFS SSD Cache', val: '1.84 TB', detail: 'NVMe Gen5 Raided', pct: 64.0, color: 'bg-purple-500' }
        ].map(res => (
          <div key={res.label} className="glass-panel p-4 flex flex-col justify-between gap-3">
            <span className="text-xs text-slate-400 font-medium">{res.label}</span>
            <div>
              <div className="text-2xl font-bold text-white font-mono num">{res.val}</div>
              <div className="text-[11px] text-slate-500 font-mono mt-0.5">{res.detail}</div>
            </div>
            <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden">
              <div className={`h-full ${res.color}`} style={{ width: `${res.pct}%` }} />
            </div>
          </div>
        ))}
      </div>

      {/* Cluster Nodes Cards (Prompt 13) */}
      <div className="space-y-3">
        <h2 className="text-sm font-semibold text-white">Active Infrastructure Nodes</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {MOCK_COMPUTE_NODES.map(node => (
            <div key={node.id} className="glass-panel p-4 flex flex-col justify-between gap-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-mono font-bold text-cyan-400">{node.id}</span>
                  <span className="text-xs font-semibold text-white">{node.name}</span>
                </div>
                {node.isPreview ? (
                  <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-amber-500/20 text-amber-300 border border-amber-500/40">
                    PREVIEW, simulator only
                  </span>
                ) : (
                  <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-500/20 text-emerald-400 border border-emerald-500/40">
                    {node.status}
                  </span>
                )}
              </div>

              <div className="text-xs text-slate-300 font-mono">{node.specs}</div>

              {/* Routing Reason Line (Prompt 13) */}
              <div className="p-2.5 rounded-lg bg-blue-950/40 border border-blue-500/30 text-xs text-cyan-300 font-mono">
                {node.routingReason}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
