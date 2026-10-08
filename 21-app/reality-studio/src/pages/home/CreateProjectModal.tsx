import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { createProject } from "../../api/resources";
import { ApiError } from "../../api/errors";
import type { EngineeringDomain } from "../../api/types";

const DOMAINS: EngineeringDomain[] = ["aerospace", "automotive", "robotics", "energy", "generic"];

export function CreateProjectModal({ onClose }: { onClose: () => void }) {
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [domain, setDomain] = useState<EngineeringDomain>("generic");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (!name.trim()) { setError("Project name is required."); return; }
    setBusy(true);
    setError(null);
    try {
      const created = await createProject({ name: name.trim(), description: description.trim(), domain });
      onClose();
      navigate(`/projects/${encodeURIComponent(created.id)}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.userMessage : "Project creation failed.");
      setBusy(false);
    }
  }

  return (
    <div className="hq-modal-backdrop" role="dialog" aria-modal="true" aria-label="Create project" onClick={onClose}>
      <form className="hq-modal" onClick={(e) => e.stopPropagation()} onSubmit={onSubmit}>
        <div className="hq-modal__header"><h2 style={{ fontSize: "var(--hq-text-lg)" }}>New Project</h2></div>
        <div className="hq-modal__body">
          {error ? <div className="hq-banner hq-banner--error" role="alert" style={{ marginBottom: "var(--hq-sp-4)" }}>{error}</div> : null}
          <label className="hq-field"><span className="label">Project name</span>
            <input className="hq-input" value={name} onChange={(e) => setName(e.target.value)} autoFocus /></label>
          <label className="hq-field"><span className="label">Description</span>
            <textarea className="hq-textarea" style={{ fontFamily: "var(--hq-font-ui)", minHeight: 64 }} value={description} onChange={(e) => setDescription(e.target.value)} /></label>
          <label className="hq-field"><span className="label">Engineering domain</span>
            <select className="hq-select" value={domain} onChange={(e) => setDomain(e.target.value as EngineeringDomain)}>
              {DOMAINS.map((d) => <option key={d} value={d}>{d}</option>)}
            </select></label>
        </div>
        <div className="hq-modal__footer">
          <button type="button" className="hq-button" onClick={onClose}>Cancel</button>
          <button className="hq-button hq-button--primary" disabled={busy}>{busy ? "CREATING…" : "CREATE PROJECT"}</button>
        </div>
      </form>
    </div>
  );
}
