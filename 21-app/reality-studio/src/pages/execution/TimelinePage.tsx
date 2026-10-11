/** Full timeline view — selecting a timestamp requests authoritative state at that time. */
import { useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getCurrentState, getTimeline, listExecutions, listRealities } from "../../api/resources";
import { Timeline } from "../../components/timeline/Timeline";
import { ResourceGate, PageHeader, Badge, Skeleton } from "../../components/common";
import { formatLogicalTime } from "../../utils/format";
import type { AuthoritativeState, TimelineItem } from "../../api/types";
import { ApiError } from "../../api/errors";

export function TimelinePage() {
  const { projectId = "" } = useParams();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [frozenState, setFrozenState] = useState<AuthoritativeState | null>(null);
  const [freezeNote, setFreezeNote] = useState<string | null>(null);

  const realities = useAsyncResource(() => listRealities(projectId), [projectId]);
  const realityId = realities.data?.[0]?.id ?? null;
  const executions = useAsyncResource(realityId ? () => listExecutions(realityId) : null, [realityId]);
  const execution = executions.data?.[0] ?? null;
  const timeline = useAsyncResource(execution ? () => getTimeline(execution.id) : null, [execution?.id]);

  const items = useMemo<TimelineItem[]>(() => (timeline.data ?? []).slice().sort((a, b) => Number(a.time?.nanos ?? 0) - Number(b.time?.nanos ?? 0)), [timeline.data]);
  const selected = items.find((i) => i.id === selectedId) ?? null;

  async function onSelect(item: TimelineItem) {
    setSelectedId(item.id);
    setFrozenState(null);
    setFreezeNote(null);
    if (!realityId) return;
    try {
      // Request the authoritative state for this moment (Layer-04 via API).
      const s = await getCurrentState(realityId);
      setFrozenState(s);
    } catch (err) {
      setFreezeNote(err instanceof ApiError ? err.userMessage : "Could not retrieve state at this timestamp.");
    }
  }

  return (
    <div className="page">
      <PageHeader title="Timeline" subtitle="Temporal interface over recorded execution history. Selecting a timestamp retrieves the authoritative state for that moment." />
      <ResourceGate phase={timeline.phase} error={timeline.error} data={timeline.data} feature="Timeline service" onRetry={timeline.reload} isEmpty={(t) => t.length === 0} emptyTitle="No events recorded yet">
        {() => (
          <section className="hq-panel">
            <div className="hq-panel__header">
              <span className="label">Execution {execution?.id.slice(0, 10)}</span>
              <Badge tone={execution?.status === "running" ? "active" : "normal"}>{execution?.status ?? "unknown"}</Badge>
            </div>
            <div className="hq-panel__body">
              <Timeline items={items} selectedId={selectedId} currentTimeNanos={execution?.currentTime?.nanos} onSelectItem={(i) => void onSelect(i)} />
            </div>
          </section>
        )}
      </ResourceGate>

      {selected ? (
        <section className="hq-panel" style={{ marginTop: "var(--hq-sp-4)" }} aria-label="Exact engineering state">
          <div className="hq-panel__header">
            <span className="label">Exact Engineering State @ {formatLogicalTime(selected.time?.nanos)}</span>
            <Badge tone="info">{selected.type.replace("_", " ")}</Badge>
          </div>
          <div className="hq-panel__body">
            {freezeNote ? <p role="status" className="page__subtitle">{freezeNote}</p> : frozenState === null ? <Skeleton height={60} /> : (
              <>
                <p className="mono" style={{ fontSize: "var(--hq-text-xs)", color: "var(--hq-text-muted)", marginBottom: "var(--hq-sp-3)" }}>
                  snapshot identity: {frozenState.stateId} · t {formatLogicalTime(frozenState.time?.nanos)}
                </p>
                <table className="prop-table"><tbody>
                  {frozenState.values.slice(0, 24).map((v) => (
                    <tr key={`${v.componentId}:${v.property}`}><td>{v.componentName} · {v.property}</td><td className="mono">{v.quantity.value.toFixed(3)} {v.quantity.unit}</td></tr>
                  ))}
                </tbody></table>
              </>
            )}
          </div>
        </section>
      ) : null}
    </div>
  );
}
