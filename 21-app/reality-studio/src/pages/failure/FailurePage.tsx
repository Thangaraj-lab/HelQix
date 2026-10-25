/** Failure screen — real Layer-09 records or an honest SYSTEM HEALTHY state. */
import { useParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { listExecutions, listFailures, listRealities } from "../../api/resources";
import { ResourceGate, PageHeader } from "../../components/common";
import { FailureCard, FailureHero } from "../../components/failure-view/FailureView";

export function FailurePage() {
  const { projectId = "" } = useParams();
  const realities = useAsyncResource(() => listRealities(projectId), [projectId]);
  const realityId = realities.data?.[0]?.id ?? null;
  const executions = useAsyncResource(realityId ? () => listExecutions(realityId) : null, [realityId]);
  const executionId = executions.data?.[0]?.id ?? null;
  const failures = useAsyncResource(executionId ? () => listFailures(executionId) : null, [executionId]);

  return (
    <div className="page">
      <PageHeader title="Failure Analysis" subtitle="Records produced by the Layer-09 failure engine. The frontend reports failures — it never detects or invents them." />
      <ResourceGate
        phase={failures.phase} error={failures.error} data={failures.data} feature="Failure engine feed"
        onRetry={failures.reload} isEmpty={(f) => f.length === 0}
        emptyTitle="SYSTEM HEALTHY" emptyBody="No failure records exist for the current execution."
      >
        {(list) => (
          <>
            <FailureHero hasFailure={list.length > 0} />
            <div style={{ display: "grid", gap: "var(--hq-sp-4)" }}>
              {list.map((f) => <FailureCard key={f.id} failure={f} />)}
            </div>
          </>
        )}
      </ResourceGate>
    </div>
  );
}
