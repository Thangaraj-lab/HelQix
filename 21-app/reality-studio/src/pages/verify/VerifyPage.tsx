/** Verification — verdicts are produced by Layer-13; the UI displays them with evidence. */
import { useState } from "react";
import { useParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getVerification } from "../../api/resources";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import { formatLogicalTime } from "../../utils/format";
import type { Verdict, VerificationCheck } from "../../api/types";
import "./verify.css";

const VERDICT_TONE: Record<Verdict, "success" | "failure" | "warning" | "normal"> = {
  pass: "success", fail: "failure", inconclusive: "warning", not_run: "normal",
};

export function VerifyPage() {
  const { projectId = "" } = useParams();
  const checks = useAsyncResource(() => getVerification(projectId), [projectId]);
  const [selected, setSelected] = useState<VerificationCheck | null>(null);

  return (
    <div className="page">
      <PageHeader title="Verification" subtitle="Requirements and constraints checked against executed realities. PASS/FAIL is never decided in the frontend." />
      <div className="verify-layout">
        <section className="hq-panel" aria-label="Checks">
          <div className="hq-panel__header"><span className="label">Requirements · Constraints · Checks</span></div>
          <ResourceGate phase={checks.phase} error={checks.error} data={checks.data} feature="Verification service (Layer 13)" onRetry={checks.reload} isEmpty={(c) => c.length === 0} emptyTitle="No verification checks defined" emptyBody="Checks appear once requirements are linked to executions by the backend.">
            {(list) => (
              <div>
                {list.map((c) => (
                  <button key={c.requirementId} className="req-row" onClick={() => setSelected(c)}>
                    <span className="mono">{c.requirementId}</span>
                    <span>{c.requirementText}</span>
                    <span className={`verdict verdict--${c.verdict}`}>{c.verdict.replace("_", " ").toUpperCase()}</span>
                  </button>
                ))}
              </div>
            )}
          </ResourceGate>
        </section>
        <aside className="hq-panel" aria-label="Selected check result">
          <div className="hq-panel__header"><span className="label">Result & evidence</span>{selected ? <Badge tone={VERDICT_TONE[selected.verdict]}>{selected.verdict.toUpperCase()}</Badge> : null}</div>
          <div className="hq-panel__body">
            {selected ? (
              <>
                <h3 style={{ marginBottom: "var(--hq-sp-3)" }}>{selected.requirementText}</h3>
                <table className="prop-table"><tbody>
                  <tr><td>Execution</td><td className="mono">{selected.evidence.executionId?.slice(0, 14) ?? "—"}</td></tr>
                  <tr><td>Timestamp</td><td className="mono">{selected.evidence.time ? formatLogicalTime(selected.evidence.time.nanos) : "—"}</td></tr>
                  <tr><td>State</td><td className="mono">{selected.evidence.stateId?.slice(0, 14) ?? "—"}</td></tr>
                </tbody></table>
                <p style={{ marginTop: "var(--hq-sp-3)", fontSize: "var(--hq-text-sm)", color: "var(--hq-text-secondary)" }}>{selected.evidence.detail}</p>
              </>
            ) : (
              <p className="page__subtitle">Select a requirement to see its verdict and the exact execution/state evidence behind it.</p>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}
