/** Architecture canvas — nodes and edges rendered from actual model interfaces. */
import { useMemo } from "react";
import { useParams, Link } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getModel } from "../../api/resources";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import { useAppState } from "../../state/AppState";
import { trimFloat } from "../../utils/format";
import type { EngineeringModel } from "../../api/types";
import "./architecture.css";

/** Deterministic layered layout derived from the component tree (no magic numbers). */
function layout(model: EngineeringModel) {
  const depthOf = (id: string | null): number => {
    let d = 0;
    let cur = id;
    while (cur) {
      const node = model.components.find((c) => c.id === cur);
      if (!node) break;
      cur = node.parentId;
      d += 1;
    }
    return Math.max(0, d - 1);
  };
  const byDepth = new Map<number, string[]>();
  for (const c of model.components) {
    const d = depthOf(c.parentId === null ? c.id : c.parentId !== null ? depthOf(c.parentId) + 1 : 0);
    const list = byDepth.get(d) ?? [];
    list.push(c.id);
    byDepth.set(d, list);
  }
  const positions = new Map<string, { x: number; y: number }>();
  const W = 900, H = 560;
  const depths = [...byDepth.keys()].sort((a, b) => a - b);
  depths.forEach((d) => {
    const ids = byDepth.get(d)!;
    ids.forEach((id, i) => {
      positions.set(id, { x: ((i + 1) / (ids.length + 1)) * W, y: 60 + (H - 120) * (depths.length <= 1 ? 0.5 : d / (depths.length - 1)) });
    });
  });
  return positions;
}

export function ArchitecturePage() {
  const { projectId = "" } = useParams();
  const { state: ui, dispatch } = useAppState();
  const model = useAsyncResource(() => getModel(projectId), [projectId]);
  const positions = useMemo(() => (model.data ? layout(model.data) : new Map()), [model.data]);

  const selected = model.data?.components.find((c) => c.id === ui.selectedComponentId) ?? null;

  return (
    <div className="page" style={{ maxWidth: "none" }}>
      <PageHeader
        title="Architecture"
        subtitle="System relationships as recorded in the engineering model. No synthetic graph data."
        actions={<Link className="hq-button hq-button--primary" to="../reality">ENTER REALITY STUDIO →</Link>}
      />
      <ResourceGate phase={model.phase} error={model.error} data={model.data} feature="Engineering model service" onRetry={model.reload}>
        {(m) => (
          <div className="arch-layout">
            <section className="arch-canvas" aria-label="Architecture graph">
              <svg viewBox="0 0 900 560" role="img" aria-label={`Architecture graph with ${m.components.length} components and ${m.interfaces.length} interfaces`}>
                <defs>
                  <marker id="hq-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                    <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--hq-border-strong)" />
                  </marker>
                </defs>
                {m.interfaces.map((iface, idx) => {
                  const a = positions.get(iface.sourceComponentId);
                  const b = positions.get(iface.targetComponentId);
                  if (!a || !b) return null;
                  return <line key={`${iface.id}-${idx}`} className="arch-edge" x1={a.x} y1={a.y} x2={b.x} y2={b.y} />;
                })}
                {m.components.map((c) => {
                  const p = positions.get(c.id);
                  if (!p) return null;
                  const isSel = c.id === ui.selectedComponentId;
                  return (
                    <g key={c.id} className={`arch-node${isSel ? " arch-node--selected" : ""}`} onClick={() => dispatch({ type: "context/selectComponent", componentId: c.id })} tabIndex={0} role="button" aria-label={`Component ${c.name}`}
                       onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") dispatch({ type: "context/selectComponent", componentId: c.id }); }}>
                      <rect x={p.x - 70} y={p.y - 18} width={140} height={36} />
                      <text x={p.x} y={p.y + 4} textAnchor="middle">{c.name}</text>
                    </g>
                  );
                })}
              </svg>
            </section>
            <aside className="hq-panel" aria-label="Selected component">
              <div className="hq-panel__header"><span className="label">Selected component</span></div>
              <div className="hq-panel__body">
                {selected ? (
                  <>
                    <h3 style={{ marginBottom: "var(--hq-sp-2)" }}>{selected.name}</h3>
                    <Badge tone="info">{selected.kind}</Badge>
                    <div className="label" style={{ margin: "var(--hq-sp-4) 0 var(--hq-sp-2)" }}>Properties</div>
                    <table className="prop-table"><tbody>
                      {selected.properties.map((p) => (
                        <tr key={p.name}><td>{p.name}</td><td className="mono">{p.quantity ? `${trimFloat(p.quantity.value)} ${p.quantity.unit}` : "—"}</td></tr>
                      ))}
                    </tbody></table>
                    <div className="label" style={{ margin: "var(--hq-sp-4) 0 var(--hq-sp-2)" }}>Interfaces</div>
                    <ul style={{ paddingLeft: "var(--hq-sp-4)", fontSize: "var(--hq-text-sm)", color: "var(--hq-text-secondary)" }}>
                      {m.interfaces
                        .filter((i) => i.sourceComponentId === selected.id || i.targetComponentId === selected.id)
                        .map((i, idx) => {
                          const other = i.sourceComponentId === selected.id ? i.targetComponentId : i.sourceComponentId;
                          const name = m.components.find((c) => c.id === other)?.name ?? other.slice(0, 8);
                          return <li key={`${i.id}-${idx}`}>{i.kind} ⇄ {name}</li>;
                        })}
                    </ul>
                  </>
                ) : (
                  <p className="page__subtitle">Click a node on the canvas to inspect its properties, interfaces, and dependencies.</p>
                )}
              </div>
            </aside>
          </div>
        )}
      </ResourceGate>
    </div>
  );
}
