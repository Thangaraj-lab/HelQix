/** Intent — define engineering intent against the backend intent contract. */
import { useEffect, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getIntent, saveIntent } from "../../api/resources";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import { ApiError } from "../../api/errors";
import "./intent.css";

export function IntentPage() {
  const { projectId = "" } = useParams();
  const intent = useAsyncResource(() => getIntent(projectId), [projectId]);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (intent.data) setDraft(intent.data.text);
  }, [intent.data]);

  async function onSave() {
    setSaving(true); setSaveError(null); setSaved(false);
    try {
      await saveIntent(projectId, draft);
      setSaved(true);
      intent.reload();
    } catch (err) {
      setSaveError(err instanceof ApiError ? err.userMessage : "Could not save intent.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="page">
      <PageHeader
        title="Engineering Intent"
        subtitle="State what the system must achieve. HELQIX compiles intent into a model through the backend intent contract — the frontend does not parse or invent structure."
        actions={<Link className="hq-button hq-button--primary" to="../model">COMPILE → CREATE MODEL</Link>}
      />
      <div className="intent-grid">
        <section className="hq-panel intent-editor" aria-label="Intent editor">
          <div className="hq-panel__header"><span className="label">Intent statement</span>{saved ? <Badge tone="success">Saved</Badge> : null}</div>
          <div className="hq-panel__body">
            <textarea
              className="hq-textarea" aria-label="Engineering intent text"
              placeholder={'e.g. "Operate the system within thermal and performance limits."'}
              value={draft} onChange={(e) => { setDraft(e.target.value); setSaved(false); }}
            />
            {saveError ? <div className="hq-banner hq-banner--error" role="alert" style={{ margin: "var(--hq-sp-3) 0" }}>{saveError}</div> : null}
            <div style={{ marginTop: "var(--hq-sp-3)", display: "flex", gap: "var(--hq-sp-3)" }}>
              <button className="hq-button hq-button--primary" onClick={onSave} disabled={saving || !draft.trim()}>
                {saving ? "SAVING…" : "SAVE INTENT"}
              </button>
            </div>
          </div>
        </section>

        <section className="hq-panel" aria-label="Structured interpretation">
          <div className="hq-panel__header"><span className="label">Structured interpretation (from backend)</span></div>
          <div className="hq-panel__body">
            <ResourceGate phase={intent.phase} error={intent.error} data={intent.data} feature="Intent structuring service" onRetry={intent.reload}>
              {(it) => (
                <div className="struct-list">
                  <div className="struct-block"><div className="label">Objective</div><ul>{it.objectives.length ? it.objectives.map((o, i) => <li key={i}>{o}</li>) : <li>No objectives extracted yet.</li>}</ul></div>
                  <div className="struct-block"><div className="label">Constraints</div><ul>{it.constraints.map((c, i) => <li key={i}>{c}</li>)}</ul></div>
                  <div className="struct-block"><div className="label">Targets</div><ul>{it.targets.map((t, i) => <li key={i}>{t}</li>)}</ul></div>
                  <div className="struct-block"><div className="label">Requirements</div><ul>{it.requirements.map((r) => <li key={r.id}><span className="mono">{r.id}</span> — {r.summary}</li>)}</ul></div>
                </div>
              )}
            </ResourceGate>
          </div>
        </section>
      </div>
    </div>
  );
}
