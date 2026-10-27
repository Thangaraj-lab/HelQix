import React, { useMemo, useState } from 'react';
import { Network } from 'lucide-react';
import { ModelRevision } from '../../contracts';
import { useStudio } from '../../stores';
import { Chip, EmptyState, MockTag, PlaceholderTag, SectionLabel, StatusDot, Tabs } from '../../components/ui/UiKit';
import { PanelCard, StageGrid } from '../../components/ui/StageGrid';

export interface ModelStageProps {
  revision: ModelRevision | null;
  loading: boolean;
  /** Shown to explain an empty stage; the header's primary button runs the simulation. */
  rev: string | null;
}

const LAYERS = ['structure', 'physics', 'environment', 'state', 'flow', 'sensors', 'connections', 'failures', 'observations'] as const;

// Model stage (file 37 section 6.9): the committed revision, read through the engine. Components on
// the left, details on the right, connections as a port graph or over the canvas. Nothing here is
// built in: every part, port and connection is in the revision.
export const ModelStage: React.FC<ModelStageProps> = ({ revision, loading, rev }) => {
  const { session } = useStudio();
  const [selected, setSelected] = useState<string | null>(null);
  const [view, setView] = useState<'3d' | 'graph'>('3d');
  const [detailTab, setDetailTab] = useState<'parameters' | 'behavior' | 'ports' | 'source'>('parameters');
  const [bottomTab, setBottomTab] = useState<'connections' | 'issues' | 'history'>('connections');

  const parts = useMemo(() => (revision ? Object.values(revision.parts) : []), [revision]);
  const ports = useMemo(() => (revision ? Object.values(revision.ports) : []), [revision]);
  const connections = useMemo(() => (revision ? Object.values(revision.connections) : []), [revision]);
  const sel = parts.find(p => p.id === (selected ?? parts[0]?.id));
  const nameOf = (id: string) => revision?.parts[id]?.name ?? id;

  if (!rev || (!loading && !revision)) {
    return (
      <div className="relative z-10 m-auto max-w-md">
        <EmptyState title="No committed model yet" description="Create model r1 in Define to see its components, ports and connections here." />
      </div>
    );
  }

  const left = (
    <>
      <PanelCard title="Components" aside={<span className="num text-xs text-muted">{parts.length}</span>}>
        {loading ? (
          <p className="text-xs text-muted">Loading the revision.</p>
        ) : (
          <ul className="space-y-1 -mx-1">
            {parts.map(p => (
              <li key={p.id}>
                <button onClick={() => setSelected(p.id)} className={`w-full text-left px-3 py-2 rounded-[8px] border transition-colors ${sel?.id === p.id ? 'bg-accent/10 border-accent/40' : 'border-transparent hover:bg-white/[0.04]'}`}>
                  <span className="flex items-center justify-between gap-2">
                    <span className="text-[13px] text-strong truncate">{p.name}</span>
                    {p.maturity === 'placeholder' && <PlaceholderTag label="Placeholder" />}
                  </span>
                  <span className="block text-[11px] text-muted mt-0.5">
                    {p.geometry.kind === 'primitive' ? p.geometry.shape : 'Imported'} · {ports.filter(x => x.partId === p.id).length} ports
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </PanelCard>

      <PanelCard title="Layers">
        <ul className="space-y-1.5">
          {LAYERS.map(l => (
            <li key={l}>
              <label className="flex items-center justify-between text-[13px] text-ink cursor-pointer">
                <span className="capitalize">{l}</span>
                <input type="checkbox" checked={session.layers[l]} onChange={() => session.toggleLayer(l)} className="accent-[var(--color-primary-blue)]" />
              </label>
            </li>
          ))}
        </ul>
        <p className="text-[11px] text-muted">Layers with nothing to show yet stay empty.</p>
      </PanelCard>
    </>
  );

  const right = (
    <PanelCard title="Component details" aside={revision && <Chip>Model r{revision.revisionNum}</Chip>}>
      {!sel ? (
        <p className="text-xs text-muted">Select a component.</p>
      ) : (
        <div className="space-y-4">
          <div>
            <p className="text-[14px] font-medium text-strong">{sel.name}</p>
            <p className="text-xs text-muted mt-0.5">{sel.maturity.replace(/_/g, ' ')}</p>
          </div>
          <Tabs
            tabs={[{ id: 'parameters', label: 'Parameters' }, { id: 'behavior', label: 'Behavior' }, { id: 'ports', label: 'Ports' }, { id: 'source', label: 'Source' }]}
            activeTab={detailTab}
            onChange={id => setDetailTab(id as typeof detailTab)}
            className="max-w-full flex-wrap"
          />
          {detailTab === 'parameters' && (
            sel.geometry.kind === 'primitive' ? (
              <div className="space-y-2">
                <div className="flex items-center justify-between text-[11px] text-muted"><span>Dimensions</span><span className="flex items-center gap-1.5">{sel.geometry.dims.some(d => d.mock) && <MockTag />}{sel.geometry.dims[0]?.origin}, {sel.geometry.dims[0]?.status}</span></div>
                <dl className="grid grid-cols-3 gap-2">
                  {sel.geometry.dims.map((d, i) => (
                    <div key={i} className="rounded-[8px] bg-white/[0.03] border border-line px-2.5 py-1.5">
                      <dt className="text-[10px] text-muted">{['X', 'Y', 'Z'][i]}</dt>
                      <dd className="num text-[13px] text-strong">{d.v} <span className="text-[11px] text-muted">{d.unit}</span></dd>
                    </div>
                  ))}
                </dl>
                <div>
                  <p className="text-[11px] text-muted mb-1">Position (m)</p>
                  <dl className="grid grid-cols-3 gap-2">
                    {sel.transform.pos.map((v, i) => (
                      <div key={i} className="rounded-[8px] bg-white/[0.03] border border-line px-2.5 py-1.5"><dt className="text-[10px] text-muted">{['X', 'Y', 'Z'][i]}</dt><dd className="num text-[13px] text-strong">{v.v}</dd></div>
                    ))}
                  </dl>
                </div>
              </div>
            ) : (
              <p className="text-xs text-muted">Imported geometry. Dimensions come from the asset.</p>
            )
          )}
          {detailTab === 'behavior' && <p className="text-xs text-muted">No behavior is defined for this component. Relations and solver bindings appear here once they are added to the model.</p>}
          {detailTab === 'ports' && (
            ports.filter(p => p.partId === sel.id).length === 0 ? <p className="text-xs text-muted">No ports.</p> : (
              <ul className="space-y-1.5">
                {ports.filter(p => p.partId === sel.id).map(p => (
                  <li key={p.id} className="flex items-center justify-between text-xs"><span className="num text-ink">{p.id}</span><span className="text-muted">{p.direction}, {p.kind.replace(/_/g, ' ')}{p.unit ? `, ${p.unit}` : ''}</span></li>
                ))}
              </ul>
            )
          )}
          {detailTab === 'source' && <p className="text-xs text-muted">Maturity: {sel.maturity.replace(/_/g, ' ')}. The revision does not record which source supplied this part.</p>}
        </div>
      )}
    </PanelCard>
  );

  const top = (
    <div className="surface-glass px-2 py-1.5 flex items-center gap-1">
      <Tabs tabs={[{ id: '3d', label: '3D' }, { id: 'graph', label: 'Port graph' }]} activeTab={view} onChange={id => setView(id as typeof view)} />
    </div>
  );

  const bottom = (
    <div className="space-y-3">
      {view === 'graph' && <PortGraph parts={parts.map(p => ({ id: p.id, name: p.name }))} links={connections.map(c => ({ id: c.id, from: c.from.partId, to: c.to.partId, kind: c.kind }))} />}
      <section className="surface-glass overflow-hidden" aria-label="Model details">
        <div className="px-3 h-11 flex items-center border-b border-line">
          <Tabs tabs={[{ id: 'connections', label: 'Connections', badge: String(connections.length) }, { id: 'issues', label: 'Issues' }, { id: 'history', label: 'History' }]} activeTab={bottomTab} onChange={id => setBottomTab(id as typeof bottomTab)} />
        </div>
        <div className="p-3 max-h-[150px] overflow-y-auto text-[13px]">
          {bottomTab === 'connections' && (connections.length === 0 ? <p className="text-xs text-muted">No connections.</p> : (
            <ul className="space-y-1.5">
              {connections.map(c => (
                <li key={c.id} className="flex items-center justify-between gap-3 text-xs">
                  <span className="text-ink truncate">{nameOf(c.from.partId)} <span className="text-muted">to</span> {nameOf(c.to.partId)}</span>
                  <Chip>{c.kind.replace(/_/g, ' ')}</Chip>
                </li>
              ))}
            </ul>
          ))}
          {bottomTab === 'issues' && <p className="text-xs text-muted">No issues recorded in this revision.</p>}
          {bottomTab === 'history' && revision && (
            <p className="text-xs text-ink flex items-center gap-2"><StatusDot tone="accent" />Model r{revision.revisionNum} committed {new Date(revision.committedAt).toLocaleString()}</p>
          )}
        </div>
      </section>
    </div>
  );

  return <StageGrid left={left} right={right} top={top} bottom={bottom} />;
};

// Parts on a circle, connections as typed lines (a read-only view of the revision's connections).
const KIND_COLOR: Record<string, string> = {
  electrical_dc: 'var(--color-primary-blue)',
  mechanical_torque: 'var(--color-trace-cyan)',
  telemetry_signal: 'var(--color-assistant-violet)',
  thermal: 'var(--status-warning)'
};

const PortGraph: React.FC<{ parts: { id: string; name: string }[]; links: { id: string; from: string; to: string; kind: string }[] }> = ({ parts, links }) => {
  const W = 520;
  const H = 190;
  const pos = new Map(parts.map((p, i) => {
    const a = (i / Math.max(1, parts.length)) * Math.PI * 2 - Math.PI / 2;
    return [p.id, { x: W / 2 + Math.cos(a) * 190, y: H / 2 + Math.sin(a) * 66 }] as const;
  }));
  return (
    <div className="surface-glass p-3" aria-label="Port graph">
      <SectionLabel aside={<Network size={14} className="text-muted" />}>Port graph</SectionLabel>
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full mt-1">
        {links.map(l => {
          const a = pos.get(l.from); const b = pos.get(l.to);
          return a && b ? <line key={l.id} x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke={KIND_COLOR[l.kind] ?? 'var(--text-muted)'} strokeWidth="1.5" /> : null;
        })}
        {parts.map(p => {
          const c = pos.get(p.id)!;
          return (
            <g key={p.id}>
              <circle cx={c.x} cy={c.y} r="6" fill="var(--bg-panel-raised)" stroke="var(--text-authoritative)" strokeWidth="1.5" />
              <text x={c.x} y={c.y + 20} textAnchor="middle" fontSize="10" fill="var(--text-ink)">{p.name.replace(/ \(Example\)$/, '')}</text>
            </g>
          );
        })}
      </svg>
    </div>
  );
};

