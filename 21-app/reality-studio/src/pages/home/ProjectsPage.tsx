/** Projects list with search / filter / sort — real backend data only. */
import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { listProjects } from "../../api/resources";
import { ResourceGate, Badge, PageHeader } from "../../components/common";
import { formatLogicalTime } from "../../utils/format";
import type { ProjectSummary } from "../../api/types";
import { CreateProjectModal } from "./CreateProjectModal";
import "./workspace.css";

type SortKey = "name" | "activity" | "status";

export function ProjectsPage() {
  const [showCreate, setShowCreate] = useState(false);
  const [query, setQuery] = useState("");
  const [domainFilter, setDomainFilter] = useState<string>("all");
  const [sortKey, setSortKey] = useState<SortKey>("activity");
  const projects = useAsyncResource(() => listProjects(), []);

  const filtered = useMemo(() => {
    const list = (projects.data ?? []).filter(
      (p) =>
        (domainFilter === "all" || p.domain === domainFilter) &&
        p.name.toLowerCase().includes(query.toLowerCase()),
    );
    const by: Record<SortKey, (a: ProjectSummary, b: ProjectSummary) => number> = {
      name: (a, b) => a.name.localeCompare(b.name),
      activity: (a, b) => Number(b.lastActivity?.nanos ?? 0) - Number(a.lastActivity?.nanos ?? 0),
      status: (a, b) => a.status.localeCompare(b.status),
    };
    return [...list].sort(by[sortKey]);
  }, [projects.data, query, domainFilter, sortKey]);

  const domains = useMemo(() => Array.from(new Set((projects.data ?? []).map((p) => p.domain))).sort(), [projects.data]);

  return (
    <div className="page">
      <PageHeader
        title="Projects"
        actions={
          <>
            <input className="hq-input" style={{ width: 220 }} placeholder="Search projects…" aria-label="Search projects" value={query} onChange={(e) => setQuery(e.target.value)} />
            <select className="hq-select" style={{ width: 150 }} aria-label="Filter by domain" value={domainFilter} onChange={(e) => setDomainFilter(e.target.value)}>
              <option value="all">All domains</option>
              {domains.map((d) => <option key={d} value={d}>{d}</option>)}
            </select>
            <select className="hq-select" style={{ width: 140 }} aria-label="Sort projects" value={sortKey} onChange={(e) => setSortKey(e.target.value as SortKey)}>
              <option value="activity">Last activity</option>
              <option value="name">Name</option>
              <option value="status">Status</option>
            </select>
            <button className="hq-button hq-button--primary" onClick={() => setShowCreate(true)}>+ New Project</button>
          </>
        }
      />
      <ResourceGate
        phase={projects.phase} error={projects.error} data={projects.data} feature="Project listing"
        onRetry={projects.reload}
        isEmpty={(p) => p.length === 0}
        emptyTitle="No projects yet"
        emptyBody="A project holds your intent, model, realities, executions, and evidence."
        emptyAction={<button className="hq-button hq-button--primary" onClick={() => setShowCreate(true)}>CREATE PROJECT</button>}
      >
        {(list) => (
          <div className="project-list" role="list">
            {(list.length === 0 ? [] : filtered).map((p) => (
              <Link key={p.id} role="listitem" className="project-row" to={`/projects/${encodeURIComponent(p.id)}`}>
                <span className="project-row__name">{p.name}<br /><small className="project-row__meta">{p.description}</small></span>
                <span className="project-row__meta">{p.domain}</span>
                <span className="project-row__meta mono">{formatLogicalTime(p.lastActivity?.nanos)}</span>
                <Badge tone={p.status === "active" ? "active" : p.status === "archived" ? "normal" : "warning"}>{p.status}</Badge>
              </Link>
            ))}
            {list.length > 0 && filtered.length === 0 ? (
              <div className="hq-empty"><div>No projects match the current filters.</div></div>
            ) : null}
          </div>
        )}
      </ResourceGate>
      {showCreate ? <CreateProjectModal onClose={() => setShowCreate(false)} /> : null}
    </div>
  );
}
