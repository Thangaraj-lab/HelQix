/** Project command center — visual engineering pipeline instead of generic dashboard. */
import { useParams, Link } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getProject, listRealities } from "../../api/resources";
import { ResourceGate, Badge, PageHeader, Skeleton } from "../../components/common";
import { formatLogicalTime } from "../../utils/format";
import "./workspace.css";

const PIPELINE: ReadonlyArray<{ step: string; route: string; desc: string }> = [
  { step: "INTENT", route: "intent", desc: "Define what to achieve" },
  { step: "MODEL", route: "model", desc: "Structure the system" },
  { step: "ARCHITECTURE", route: "architecture", desc: "Relationships & interfaces" },
  { step: "REALITY", route: "reality", desc: "Construct executable reality" },
  { step: "EXECUTION", route: "execution", desc: "Run & act" },
  { step: "OBSERVATION", route: "observation", desc: "Measure state" },
  { step: "ANALYSIS", route: "failure", desc: "Failures & causes" },
  { step: "VERIFICATION", route: "verify", desc: "Verify with evidence" },
];

export function ProjectOverviewPage() {
  const { projectId = "" } = useParams();
  const project = useAsyncResource(() => getProject(projectId), [projectId]);
  const realities = useAsyncResource(() => listRealities(projectId), [projectId]);

  return (
    <div className="page">
      <ResourceGate phase={project.phase} error={project.error} data={project.data} feature="Project details" onRetry={project.reload}>
        {(p) => (
          <PageHeader
            title={p.name}
            subtitle={p.description || undefined}
            actions={
              <>
                <Badge tone={p.status === "active" ? "active" : "normal"}>{p.status}</Badge>
                <Badge tone="info">{p.domain}</Badge>
                <span className="mono page__subtitle">last activity {formatLogicalTime(p.lastActivity?.nanos)}</span>
              </>
            }
          />
        )}
      </ResourceGate>

      <section className="hq-panel" aria-label="Engineering pipeline" style={{ marginBottom: "var(--hq-sp-5)" }}>
        <div className="hq-panel__header"><span className="label">Executable-reality pipeline</span></div>
        <div className="hq-panel__body">
          <div className="workflow-pipeline">
            {PIPELINE.map((s) => (
              <div className="pipeline-step" key={s.step}>
                <Link to={`${s.route}`}>{s.step}</Link>
                <small>{s.desc}</small>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="hq-panel" aria-label="Realities">
        <div className="hq-panel__header">
          <span className="label">Realities</span>
          <Link className="hq-button hq-button--sm" to="reality">Open Reality Studio →</Link>
        </div>
        <ResourceGate
          phase={realities.phase} error={realities.error} data={realities.data} feature="Reality listing"
          onRetry={realities.reload} isEmpty={(r) => r.length === 0}
          emptyTitle="No realities constructed yet"
          emptyBody="A reality is built from the engineering model and carries live authoritative state."
        >
          {(list) => (
            <table className="hq-table">
              <thead><tr><th>Name</th><th>Status</th><th>ID</th></tr></thead>
              <tbody>
                {list.map((r) => (
                  <tr key={r.id}>
                    <td><Link to="reality">{r.name}</Link></td>
                    <td><Badge tone={r.status === "running" ? "active" : r.status === "faulted" ? "failure" : "normal"}>{r.status}</Badge></td>
                    <td className="mono" style={{ color: "var(--hq-text-muted)" }}>{r.id.slice(0, 12)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </ResourceGate>
      </section>
      {project.phase === "loading" ? <div style={{ marginTop: "var(--hq-sp-4)" }}><Skeleton height={80} /></div> : null}
    </div>
  );
}
