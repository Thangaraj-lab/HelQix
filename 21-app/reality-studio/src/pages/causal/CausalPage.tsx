/** Causal analysis — the graph comes exclusively from Layer-10 via API. */
import { useState } from "react";
import { useParams, useSearchParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getCausalGraph, listExecutions, listFailures, listRealities } from "../../api/resources";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import { CausalGraph } from "../../components/causal-view/CausalGraph";
import { formatLogicalTime } from "../../utils/format";

export function CausalPage() {
  const { projectId = "" } = useParams();
  const [params] = useSearchParams();
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  const realities = useAsyncResource(() => listRealities(projectId), [projectId]);
  const realityId = realities.data?.[0]?.id ?? null;
  const executions = useAsyncResource(realityId ? () => listExecutions(realityId) : null, [realityId]);
  const executionId = executions.data?.[0]?.id ?? null;
  const failures = useAsyncResource(executionId ? () => listFailures(executionId) : null, [executionId]);

  const failureId = params.get("failure") ?? failures.data?.[0]?.id ?? null;
  const graph = useAsyncResource(failureId ? () => getCausalGraph(failureId) : null, [failureId]);

  const selectedNode = graph.data?.nodes.find((n) => n.id === selectedNodeId) ?? null;
  const selectedEdges = graph.data?.edges.filter((e) => e.to === selectedNodeId) ?? [];

  return (
    <div className="page">
      <PageHeader title="Causal Analysis" subtitle="Deterministic cause→effect structure produced by the Layer-10 causality engine. The frontend renders it; it never infers causes." actions={failureId ? <Badge tone="failure">failure {failureId.slice(0, 10)}</Badge> : undefined} />
      {!failureId ? (
        <ResourceGate phase={failures.phase} error={failures.error} data={failures.data} feature="Causality service" onRetry={failures.reload} isEmpty={() => true} emptyTitle="No failure selected" emptyBody="Open causal analysis from a failure record to trace its root cause.">
          {() => null}
        </ResourceGate>
      ) : (
        <div className="causal-layout">
          <ResourceGate phase={graph.phase} error={graph.error} data={graph.data} feature="Causal graph endpoint (Layer 10)" onRetry={graph.reload}>
            {(g) => <CausalGraph graph={g} selectedNodeId={selectedNodeId} onSelect={setSelectedNodeId} />}
          </ResourceGate>
          <aside className="hq-panel" aria-label="Selected causal event">
            <div className="hq-panel__header"><span className="label">Causal event</span></div>
            <div className="hq-panel__body">
              {selectedNode ? (
                <>
                  <h3 style={{ marginBottom: 6 }}>{selectedNode.label}</h3>
                  <Badge tone={selectedNode.kind === "failure" ? "failure" : selectedNode.kind === "root" ? "warning" : "info"}>{selectedNode.kind}</Badge>
                  <table className="prop-table" style={{ marginTop: "var(--hq-sp-4)" }}><tbody>
                    <tr><td>Component</td><td>{selectedNode.componentId?.slice(0, 10) ?? "—"}</td></tr>
                    <tr><td>Property</td><td className="mono">{selectedNode.propertyName ?? "—"}</td></tr>
                    <tr><td>Value change</td><td className="mono">{selectedNode.valueChange ?? "—"}</td></tr>
                    <tr><td>Timestamp</td><td className="mono">{selectedNode.time ? formatLogicalTime(selectedNode.time.nanos) : "—"}</td></tr>
                    <tr><td>Direct causes</td><td>{selectedEdges.length}</td></tr>
                    <tr><td>Evidence refs</td><td className="mono">{selectedNode.evidenceRefs.length ? selectedNode.evidenceRefs.join(", ") : "—"}</td></tr>
                  </tbody></table>
                </>
              ) : (
                <p className="page__subtitle">Select a node in the causal graph to inspect component, property change, timestamp, and evidence.</p>
              )}
            </div>
          </aside>
        </div>
      )}
    </div>
  );
}
