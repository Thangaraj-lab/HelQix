/** Failure inspection surface driven by Layer-09 failure records. */
import { Link } from "react-router-dom";
import type { FailureRecord } from "../../api/types";
import { Badge } from "../common";
import { formatLogicalTime } from "../../utils/format";
import "../../pages/failure/failure.css";

export function FailureHero({ hasFailure }: { hasFailure: boolean }) {
  return (
    <div className={`failure-hero ${hasFailure ? "failure-hero--failed" : "failure-hero--healthy"}`} role="status">
      <h2>{hasFailure ? "FAILURE DETECTED" : "SYSTEM HEALTHY"}</h2>
      <span style={{ marginLeft: "auto" }} className="mono">{hasFailure ? "layer 09 record active" : "no failure records"}</span>
    </div>
  );
}

export function FailureCard({ failure }: { failure: FailureRecord }) {
  return (
    <article className="hq-panel" aria-label={`Failure ${failure.id}`}>
      <div className="hq-panel__header">
        <span className="mono">{failure.id.slice(0, 16)}</span>
        <Badge tone={failure.severity === "critical" ? "failure" : failure.severity === "warning" ? "warning" : "info"}>{failure.severity}</Badge>
      </div>
      <div className="hq-panel__body">
        <table className="prop-table"><tbody>
          <tr><td>Component</td><td>{failure.componentName}</td></tr>
          <tr><td>Time</td><td className="mono">{formatLogicalTime(failure.time?.nanos)}</td></tr>
          <tr><td>Condition</td><td className="mono">{failure.condition}</td></tr>
          <tr><td>State at failure</td><td className="mono">{failure.stateAtFailure.slice(0, 16)}</td></tr>
        </tbody></table>
        <div style={{ display: "flex", gap: "var(--hq-sp-2)", marginTop: "var(--hq-sp-4)", flexWrap: "wrap" }}>
          <Link className="hq-button hq-button--sm" to="../timeline">Inspect exact state</Link>
          <Link className="hq-button hq-button--sm" to="../execution">Replay</Link>
          <Link className="hq-button hq-button--sm hq-button--primary" to={`../branches?origin=${encodeURIComponent(failure.stateAtFailure)}&t=${encodeURIComponent(failure.time?.nanos ?? "")}`}>CREATE BRANCH FROM {formatLogicalTime(failure.time?.nanos)}</Link>
          <Link className="hq-button hq-button--sm" to={`../causal?failure=${encodeURIComponent(failure.id)}`}>Open causal analysis →</Link>
        </div>
      </div>
    </article>
  );
}
