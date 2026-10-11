/** Create-branch flow: freeze exact state → change one supported parameter → branch. Never mutates the original. */
import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import { createBranch } from "../../api/resources";
import { ApiError } from "../../api/errors";
import type { AuthoritativeState, ModelComponent } from "../../api/types";
import { Badge } from "../common";
import { formatLogicalTime } from "../../utils/format";

interface Props {
  projectId: string;
  executionId: string | null;
  liveState: AuthoritativeState | null;
  components: ReadonlyArray<ModelComponent>;
  onCreated(): void;
}

export function CreateBranchPanel({ projectId, executionId, liveState, components, onCreated }: Props) {
  const [params] = useSearchParams();
  const originStateId = params.get("origin") ?? liveState?.stateId ?? null;
  const originTime = params.get("t") ?? liveState?.time?.nanos ?? null;

  const [componentId, setComponentId] = useState<string>("");
  const [property, setProperty] = useState<string>("");
  const [newValue, setNewValue] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  const component = components.find((c) => c.id === componentId) ?? null;
  const propNames = component?.properties.map((p) => p.name) ?? [];
  const originalValue = liveState?.values.find((v) => v.componentId === componentId && v.property === property);

  async function onCreate() {
    if (!executionId || !originStateId || !componentId || !property || !newValue) return;
    setBusy(true); setError(null);
    try {
      await createBranch({
        projectId,
        originExecutionId: executionId,
        originStateId,
        modifications: [{ componentId, property, newValue }],
      });
      setDone(true);
      onCreated();
    } catch (err) {
      setError(err instanceof ApiError ? err.userMessage : "Branch creation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="hq-panel" aria-label="Create branch">
      <div className="hq-panel__header"><span className="label">Create branch</span>{done ? <Badge tone="success">Branch created</Badge> : null}</div>
      <div className="hq-panel__body">
        <table className="prop-table" style={{ marginBottom: "var(--hq-sp-4)" }}><tbody>
          <tr><td>Branch origin</td><td className="mono">{executionId ? `execution ${executionId.slice(0, 10)}` : "—"}</td></tr>
          <tr><td>Frozen state</td><td className="mono">{originStateId ? originStateId.slice(0, 16) : "awaiting state feed"}</td></tr>
          <tr><td>At time</td><td className="mono">{formatLogicalTime(originTime)}</td></tr>
        </tbody></table>
        <p className="label" style={{ marginBottom: "var(--hq-sp-2)" }}>What do you want to change?</p>
        <label className="hq-field"><span className="label">Component</span>
          <select className="hq-select" value={componentId} onChange={(e) => { setComponentId(e.target.value); setProperty(""); }}>
            <option value="">Select component…</option>
            {components.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
          </select></label>
        <label className="hq-field"><span className="label">Parameter</span>
          <select className="hq-select" value={property} onChange={(e) => setProperty(e.target.value)} disabled={!component}>
            <option value="">Select parameter…</option>
            {propNames.map((p) => <option key={p} value={p}>{p}</option>)}
          </select></label>
        {component && property ? (
          <table className="prop-table" style={{ marginBottom: "var(--hq-sp-3)" }}><tbody>
            <tr><td>Original value</td><td className="mono">{originalValue ? `${originalValue.quantity.value} ${originalValue.quantity.unit}` : "—"}</td></tr>
          </tbody></table>
        ) : null}
        <label className="hq-field"><span className="label">New value</span>
          <input className="hq-input mono" value={newValue} onChange={(e) => setNewValue(e.target.value)} placeholder={originalValue ? `current: ${originalValue.quantity.value}` : undefined} /></label>
        {error ? <div className="hq-banner hq-banner--error" role="alert" style={{ marginBottom: "var(--hq-sp-3)" }}>{error}</div> : null}
        <button className="hq-button hq-button--primary" disabled={busy || !originStateId || !executionId || !componentId || !property || !newValue} onClick={() => void onCreate()}>
          {busy ? "BRANCHING…" : `CREATE BRANCH FROM ${formatLogicalTime(originTime)}`}
        </button>
        <p className="page__subtitle" style={{ marginTop: "var(--hq-sp-3)", fontSize: "var(--hq-text-xs)" }}>The original execution and its history are never modified.</p>
      </div>
    </section>
  );
}
