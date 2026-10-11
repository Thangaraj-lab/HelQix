/** Compare — original vs branch using backend-produced comparison tables. */
import { useState } from "react";
import { useParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { compareBranches, listBranches } from "../../api/resources";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import "./compare.css";

export function ComparePage() {
  const { projectId = "" } = useParams();
  const branches = useAsyncResource(() => listBranches(projectId), [projectId]);
  const [left, setLeft] = useState<string>("");
  const [right, setRight] = useState<string>("");

  const leftLabel = left === "" ? "Original" : branches.data?.find((b) => b.id === left)?.id.slice(0, 8) ?? "—";
  const rightLabel = branches.data?.find((b) => b.id === right)?.id.slice(0, 8) ?? "—";

  const comparison = useAsyncResource(
    right ? () => compareBranches(projectId, left || "original", right) : null,
    [projectId, left, right],
  );

  return (
    <div className="page">
      <PageHeader title="Compare" subtitle="Outcome comparison across branch executions. Every value is produced by the backend comparison service." />
      <div className="compare-selects">
        <select className="hq-select" style={{ width: 260 }} aria-label="Left side of comparison" value={left} onChange={(e) => setLeft(e.target.value)}>
          <option value="">Original execution</option>
          {(branches.data ?? []).map((b) => <option key={b.id} value={b.id}>Branch {b.id.slice(0, 8)}</option>)}
        </select>
        <span className="compare-vs">VS</span>
        <select className="hq-select" style={{ width: 260 }} aria-label="Right side of comparison" value={right} onChange={(e) => setRight(e.target.value)}>
          <option value="">Select a branch…</option>
          {(branches.data ?? []).map((b) => <option key={b.id} value={b.id}>Branch {b.id.slice(0, 8)}</option>)}
        </select>
      </div>

      {!right ? (
        <PageHint />
      ) : (
        <div className="compare-header">
          <div className="compare-side"><span className="label">{leftLabel}</span></div>
          <div className="compare-vs">VS</div>
          <div className="compare-side compare-side--right"><span className="label">{rightLabel}</span></div>
        </div>
      )}

      {right ? (
        <ResourceGate phase={comparison.phase} error={comparison.error} data={comparison.data} feature="Comparison service (Layer 11/12)" onRetry={comparison.reload}>
          {(table) => (
            <section className="hq-panel" aria-label="Comparison result">
              <div className="hq-panel__header"><span className="label">State · Metric · Execution · Failure · Verification</span><Badge tone="info">{table.rows.length} rows</Badge></div>
              <table className="hq-table">
                <thead><tr><th>Measure</th><th>{table.leftLabel}</th><th>{table.rightLabel}</th><th>Δ</th></tr></thead>
                <tbody>
                  {table.rows.map((r, i) => (
                    <tr key={`${r.label}-${i}`}>
                      <td>{r.label}</td>
                      <td className="mono">{r.left}</td>
                      <td className="mono">{r.right}</td>
                      <td className={`mono ${r.delta?.startsWith("-") ? "delta-pos" : r.delta ? "delta-neg" : ""}`}>{r.delta ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          )}
        </ResourceGate>
      ) : null}
    </div>
  );
}

function PageHint() {
  return <p className="page__subtitle">Choose a branch on the right to compare against the original. The comparison request goes to the backend; nothing is computed in the browser.</p>;
}
