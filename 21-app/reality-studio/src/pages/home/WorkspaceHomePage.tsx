/** Workspace Home — orient the engineer. Real data only; honest empty states. */
import { useState } from "react";
import { Link } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { listProjects } from "../../api/resources";
import { ResourceGate, Badge, PageHeader } from "../../components/common";
import { formatLogicalTime } from "../../utils/format";
import { CreateProjectModal } from "./CreateProjectModal";
import "./workspace.css";

export function WorkspaceHomePage() {
  const [showCreate, setShowCreate] = useState(false);
  const projects = useAsyncResource(() => listProjects(), []);
  const hour = new Date().getHours();
  const greeting = hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";

  return (
    <div className="page">
      <PageHeader
        title={`${greeting}. What are we executing today?`}
        subtitle="Your engineering workspace — real projects, executions, failures, and branches only."
        actions={<button className="hq-button hq-button--primary" onClick={() => setShowCreate(true)}>CREATE PROJECT</button>}
      />
      <div className="ws-grid">
        <section className="hq-panel" aria-label="Recent projects">
          <div className="hq-panel__header"><span className="label">Recent Projects</span><Link to="/projects" className="hq-button hq-button--ghost hq-button--sm">All projects →</Link></div>
          <ResourceGate
            phase={projects.phase} error={projects.error} data={projects.data} feature="Project listing"
            onRetry={projects.reload}
            isEmpty={(p) => p.length === 0}
            emptyTitle="Your engineering workspace is empty."
            emptyBody="Create your first project to begin defining intent, models, and realities."
            emptyAction={<button className="hq-button hq-button--primary" onClick={() => setShowCreate(true)}>CREATE PROJECT</button>}
          >
            {(list) => (
              <div className="project-list">
                {list.slice(0, 6).map((p) => (
                  <Link key={p.id} className="project-row" to={`/projects/${encodeURIComponent(p.id)}`}>
                    <span className="project-row__name">{p.name}</span>
                    <span className="project-row__meta">{p.domain}</span>
                    <span className="project-row__meta mono">{formatLogicalTime(p.lastActivity?.nanos)}</span>
                    <Badge tone={p.status === "active" ? "active" : p.status === "archived" ? "normal" : "warning"}>{p.status}</Badge>
                  </Link>
                ))}
              </div>
            )}
          </ResourceGate>
        </section>
        <aside className="hq-panel" aria-label="Platform status">
          <div className="hq-panel__header"><span className="label">Runtime signals</span></div>
          <div className="hq-panel__body" style={{ display: "grid", gap: "var(--hq-sp-3)" }}>
            <p className="page__subtitle" style={{ marginBottom: 0 }}>
              Recent executions, failures, and branches appear here once the Layer&nbsp;22 activity feed is
              connected. HELQIX does not pre-populate these panels with invented activity.
            </p>
            <div style={{ display: "flex", gap: "var(--hq-sp-2)", flexWrap: "wrap" }}>
              <Badge tone="normal">Executions: awaiting feed</Badge>
              <Badge tone="normal">Failures: awaiting feed</Badge>
              <Badge tone="normal">Branches: awaiting feed</Badge>
            </div>
          </div>
        </aside>
      </div>
      {showCreate ? <CreateProjectModal onClose={() => setShowCreate(false)} /> : null}
    </div>
  );
}
