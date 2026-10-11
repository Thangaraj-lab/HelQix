/** Causal graph renderer — draws the Layer-10 event→effect structure verbatim. */
import type { CausalGraphView, CausalNode } from "../../api/types";
import { formatLogicalTime } from "../../utils/format";
import "../../pages/causal/causal.css";

interface Props { graph: CausalGraphView; selectedNodeId: string | null; onSelect(nodeId: string): void }

const NODE_W = 210, NODE_H = 54, COL_GAP = 90, ROW_GAP = 26;

export function CausalGraph({ graph, selectedNodeId, onSelect }: Props) {
  // Rank nodes by longest path from roots to lay out cause → effect vertically.
  const incoming = new Map<string, string[]>();
  for (const e of graph.edges) incoming.set(e.to, [...(incoming.get(e.to) ?? []), e.from]);
  const rank = new Map<string, number>();
  function computeRank(id: string, guard: Set<string>): number {
    const cached = rank.get(id);
    if (cached !== undefined) return cached;
    if (guard.has(id)) return 0;
    guard.add(id);
    const parents = incoming.get(id) ?? [];
    const r = parents.length === 0 ? 0 : Math.max(...parents.map((p) => computeRank(p, guard))) + 1;
    guard.delete(id);
    rank.set(id, r);
    return r;
  }
  for (const n of graph.nodes) computeRank(n.id, new Set());

  const byRank = new Map<number, CausalNode[]>();
  for (const n of graph.nodes) {
    const r = rank.get(n.id) ?? 0;
    byRank.set(r, [...(byRank.get(r) ?? []), n]);
  }
  const positions = new Map<string, { x: number; y: number }>();
  let maxCols = 1;
  for (const [r, nodes] of byRank) {
    maxCols = Math.max(maxCols, nodes.length);
    nodes.forEach((n, i) => positions.set(n.id, { x: 40 + i * (NODE_W + COL_GAP), y: 30 + r * (NODE_H + ROW_GAP) }));
  }
  const width = 40 + maxCols * (NODE_W + COL_GAP);
  const height = 60 + (Math.max(0, ...[...rank.values()]) + 1) * (NODE_H + ROW_GAP);

  return (
    <div className="causal-canvas" aria-label={`Causal graph: ${graph.nodes.length} events, ${graph.edges.length} relations`}>
      <svg width={width} height={height} role="img">
        <defs>
          <marker id="hq-causal-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--hq-border-strong)" />
          </marker>
        </defs>
        {graph.edges.map((e, idx) => {
          const a = positions.get(e.from), b = positions.get(e.to);
          if (!a || !b) return null;
          return <line key={`${e.from}-${e.to}-${idx}`} className="causal-edge" x1={a.x + NODE_W / 2} y1={a.y + NODE_H} x2={b.x + NODE_W / 2} y2={b.y} />;
        })}
        {graph.nodes.map((n) => {
          const p = positions.get(n.id);
          if (!p) return null;
          const cls = `causal-node${n.kind === "failure" ? " causal-node--failure" : ""}${graph.rootCauseIds.includes(n.id) ? " causal-node--root" : ""}${selectedNodeId === n.id ? " causal-node--selected" : ""}`;
          return (
            <g key={n.id} className={cls} transform={`translate(${p.x},${p.y})`} onClick={() => onSelect(n.id)} onKeyDown={(e) => { if (e.key === "Enter") onSelect(n.id); }} tabIndex={0} role="button" aria-label={`Causal event ${n.label}`}>
              <rect width={NODE_W} height={NODE_H} />
              <text x={10} y={20}>{n.label}</text>
              <text className="causal-node__sub" x={10} y={38}>
                {[n.valueChange, n.time ? formatLogicalTime(n.time.nanos) : null].filter(Boolean).join("  ·  ")}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}
