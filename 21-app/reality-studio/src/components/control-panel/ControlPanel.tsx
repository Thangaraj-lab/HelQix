/**
 * Actions / Events / Failure console.
 * Action kinds and parameter schemas are supplied BY THE BACKEND
 * (GET /executions/:id/actions). The form is generated from that schema —
 * the frontend never hardcodes which actions exist.
 */
import { useState, type FormEvent } from "react";
import type { ActionDescriptor, ExecuteActionResponse, TimelineItem } from "../../api/types";
import { executeAction } from "../../api/resources";
import { ApiError } from "../../api/errors";
import { Badge } from "../common";
import { formatLogicalTime } from "../../utils/format";
import "./control.css";

interface Props {
  executionId: string | null;
  availableActions: ReadonlyArray<ActionDescriptor>;
  actionsUnavailable: boolean;
  targetComponentName: string | null;
  targetComponentId: string | null;
  recentEvents: ReadonlyArray<TimelineItem>;
  onActionExecuted: (result: ExecuteActionResponse) => void;
}

export function ControlPanel({ executionId, availableActions, actionsUnavailable, targetComponentName, targetComponentId, recentEvents, onActionExecuted }: Props) {
  const [tab, setTab] = useState<"actions" | "events">("actions");
  const [kind, setKind] = useState<string>("");
  const [params, setParams] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);

  const descriptor = availableActions.find((a) => a.kind === kind) ?? availableActions[0] ?? null;

  async function onExecute(e: FormEvent) {
    e.preventDefault();
    if (!executionId || !descriptor || !targetComponentId) return;
    setBusy(true);
    setResult(null);
    try {
      const numeric: Record<string, string | number | boolean> = {};
      for (const p of descriptor.parameterSchema) {
        const raw = params[p.name];
        if (raw === undefined || raw === "") continue;
        numeric[p.name] = p.type === "number" ? Number(raw) : p.type === "boolean" ? raw === "true" : raw;
      }
      const res = await executeAction({ executionId, targetComponentId, actionKind: descriptor.kind, parameters: numeric });
      setResult({ ok: res.accepted, message: res.message || (res.accepted ? `Action accepted · event ${res.eventId?.slice(0, 8) ?? ""}` : "Action rejected") });
      onActionExecuted(res);
    } catch (err) {
      setResult({ ok: false, message: err instanceof ApiError ? err.userMessage : "Action failed." });
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="hq-panel" aria-label="Actions and events console">
      <div className="control-tabs" role="tablist">
        <button role="tab" aria-selected={tab === "actions"} onClick={() => setTab("actions")}>ACTIONS</button>
        <button role="tab" aria-selected={tab === "events"} onClick={() => setTab("events")}>EVENTS / FAILURE / CAUSALITY</button>
      </div>
      <div className="hq-panel__body">
        {tab === "actions" ? (
          actionsUnavailable ? (
            <p className="page__subtitle">The execution service has not exposed an action catalogue yet (GET /executions/:id/actions). Controls appear automatically when available. HELQIX will not offer actions the backend cannot execute.</p>
          ) : !executionId ? (
            <p className="page__subtitle">Select or start an execution to issue engineering actions.</p>
          ) : (
            <form className="action-form" onSubmit={onExecute}>
              <div style={{ display: "flex", gap: "var(--hq-sp-3)", alignItems: "center", flexWrap: "wrap" }}>
                <span className="label">Target:</span>
                <Badge tone={targetComponentName ? "active" : "normal"}>{targetComponentName ?? "no component selected"}</Badge>
              </div>
              <label className="hq-field" style={{ marginBottom: 0 }}>
                <span className="label">Action type (backend-provided)</span>
                <select className="hq-select" value={descriptor?.kind ?? ""} onChange={(e) => { setKind(e.target.value); setParams({}); }}>
                  {availableActions.map((a) => <option key={a.kind} value={a.kind}>{a.label}</option>)}
                </select>
              </label>
              {descriptor?.parameterSchema.map((p) => (
                <label className="hq-field" key={p.name} style={{ marginBottom: 0 }}>
                  <span className="label">{p.name}{p.unit ? ` (${p.unit})` : ""}{p.required ? " *" : ""}</span>
                  <input className="hq-input mono" type={p.type === "number" ? "number" : "text"} step="any"
                    value={params[p.name] ?? ""} onChange={(e) => setParams((s) => ({ ...s, [p.name]: e.target.value }))} required={p.required} />
                </label>
              ))}
              <div style={{ display: "flex", gap: "var(--hq-sp-3)", alignItems: "center" }}>
                <button className="hq-button hq-button--primary" disabled={busy || !descriptor || !targetComponentId}>
                  {busy ? "EXECUTING…" : `EXECUTE ON ${targetComponentName ?? "—"}`}
                </button>
                {result ? (
                  <span style={{ color: result.ok ? "var(--hq-success)" : "var(--hq-failure)", fontSize: "var(--hq-text-sm)" }} role="status">{result.message}</span>
                ) : null}
              </div>
            </form>
          )
        ) : (
          <div className="event-log" aria-live="polite">
            {recentEvents.length === 0 ? <p className="page__subtitle">No events recorded yet.</p> : null}
            {recentEvents.slice(0, 60).map((ev) => (
              <div className="event-log__row" key={ev.id}>
                <span className="mono" style={{ color: "var(--hq-text-muted)" }}>{formatLogicalTime(ev.time?.nanos)}</span>
                <span style={{ color: ev.severity === "failure" ? "var(--hq-failure)" : ev.severity === "warning" ? "var(--hq-warning)" : undefined }}>{ev.label}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
