/** Branch workspace — lineage tree + creation + branch identity. */
import { useState } from "react";
import { useParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { listBranches } from "../../api/resources";
import { useRealityData } from "../../hooks/useRealityData";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import { BranchTree } from "../../components/branch-view/BranchTree";
import { CreateBranchPanel } from "../../components/branch-view/CreateBranchPanel";
import { formatLogicalTime } from "../../utils/format";

export function BranchesPage() {
  const { projectId = "" } = useParams();
  const data = useRealityData(projectId);
  const branches = useAsyncResource(() => listBranches(projectId), [projectId]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = branches.data?.find((b) => b.id === selectedId) ?? null;

  return (
    <div className="page">
      <PageHeader title="Branches" subtitle="Every branch starts from a frozen, identified state. The original run remains untouched." />
      <div className="branch-layout">
        <section className="hq-panel" aria-label="Branch tree">
          <div className="hq-panel__header"><span className="label">Lineage</span>{branches.data ? <Badge tone="normal">{branches.data.length} branch(es)</Badge> : null}</div>
          <div className="hq-panel__body">
            <ResourceGate phase={branches.phase} error={branches.error} data={branches.data} feature="Branching service (Layer 11)" onRetry={branches.reload} isEmpty={(b) => b.length === 0} emptyTitle="No branches yet" emptyBody="Freeze an exact state and branch from it using the panel beside this tree.">
              {(list) => <BranchTree branches={list} selectedId={selectedId} onSelect={setSelectedId} />}
            </ResourceGate>
          </div>
        </section>
        <div style={{ display: "grid", gap: "var(--hq-sp-3)" }}>
          <CreateBranchPanel projectId={projectId} executionId={data.execution?.id ?? null} liveState={data.liveState} components={data.model.data?.components ?? []} onCreated={() => branches.reload()} />
          {selected ? (
            <section className="hq-panel" aria-label="Selected branch">
              <div className="hq-panel__header"><span className="label">Active branch identity</span><Badge tone="active">{selected.id.slice(0, 8)}</Badge></div>
              <div className="hq-panel__body">
                <table className="prop-table"><tbody>
                  <tr><td>Origin state</td><td className="mono">{selected.originStateId.slice(0, 16)}</td></tr>
                  <tr><td>Origin execution</td><td className="mono">{selected.originExecutionId.slice(0, 16)}</td></tr>
                  <tr><td>Created</td><td className="mono">{formatLogicalTime(selected.createdAt?.nanos)}</td></tr>
                  <tr><td>Modification</td><td>{selected.modification}</td></tr>
                  <tr><td>Status</td><td>{selected.status}</td></tr>
                </tbody></table>
                <p className="page__subtitle" style={{ marginTop: "var(--hq-sp-3)", fontSize: "var(--hq-text-xs)" }}>
                  You are viewing branch identity metadata. Executing this branch happens in Execution with the branch badge active.
                </p>
              </div>
            </section>
          ) : null}
        </div>
      </div>
    </div>
  );
}
