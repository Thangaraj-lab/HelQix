/** Execution page — RUN/PAUSE/STEP/STOP/REPLAY/REWIND against Layer-05 contracts. */
import { useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { useRealityData } from "../../hooks/useRealityData";
import { getTimeline, pauseExecution, replayExecution, rewindExecution, runExecution, stepExecution, stopExecution } from "../../api/resources";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { Timeline } from "../../components/timeline/Timeline";
import { ControlPanel } from "../../components/control-panel/ControlPanel";
import { StatePanel } from "../../components/state-panel/StatePanel";
import { Badge, PageHeader, UnavailablePanel, ErrorPanel } from "../../components/common";
import { ApiError } from "../../api/errors";
import { formatLogicalTime } from "../../utils/format";
import { useAppState } from "../../state/AppState";
import type { TimelineItem } from "../../api/types";
import "./execution.css";

type Operation = "RUNNING" | "SYNCING" | "REPLAYING" | null;

export function ExecutionPage() {
  const { projectId = "" } = useParams();
  const { state: ui } = useAppState();
  const data = useRealityData(projectId);
  const [op, setOp] = useState<Operation>(null);
  const [cmdError, setCmdError] = useState<string | null>(null);
  const [selectedTimelineId, setSelectedTimelineId] = useState<string | null>(null);

  const timeline = useAsyncResource(data.execution ? () => getTimeline(data.execution!.id) : null, [data.execution?.id]);
  const items = useMemo<TimelineItem[]>(() => (timeline.data ?? []).slice().sort((a, b) => Number(a.time?.nanos ?? 0) - Number(b.time?.nanos ?? 0)), [timeline.data]);
  const selected = items.find((i) => i.id === selectedTimelineId) ?? null;

  const execution = data.execution;
  const canRun = !execution || ["idle", "ready", "paused", "completed", "stopped"].includes(execution.status);

  async function command(fn: (id: string) => Promise<unknown>, label: Operation) {
    if (!execution) return;
    setOp(label); setCmdError(null);
    try {
      await fn(execution.id);
      await data.refresh();
      timeline.reload();
    } catch (err) {
      setCmdError(err instanceof ApiError ? err.userMessage : `${label} failed`);
    } finally {
      setOp(null);
    }
  }

  if (data.realities.phase === "unavailable") {
    return <div className="page"><UnavailablePanel feature="Execution service" detail="GET /realities/:id/executions not implemented by the connected backend." /></div>;
  }
  if (data.realities.phase === "error") return <div className="page"><ErrorPanel error={data.realities.error} onRetry={data.realities.reload} /></div>;

  return (
    <div className="page">
      <PageHeader
        title="Execution"
        subtitle={execution ? `Execution ${execution.id.slice(0, 12)} · t = ${formatLogicalTime(execution.currentTime?.nanos)}` : "No execution exists for this reality yet."}
        actions={op ? <Badge tone="active">{op}</Badge> : execution ? <Badge tone={execution.status === "running" ? "active" : execution.status === "faulted" ? "failure" : "normal"}>{execution.status.toUpperCase()}</Badge> : undefined}
      />
      {!execution ? (
        <UnavailablePanel feature="Execution runtime" detail="Start an execution through the Layer-05 execution service. Controls are bound to real executions only — nothing is simulated in the browser." />
      ) : (
        <>
          <section className="hq-panel" style={{ marginBottom: "var(--hq-sp-4)" }} aria-label="Execution controls">
            <div className="hq-panel__body exec-controls">
              <button className="hq-button hq-button--primary" disabled={!!op || !canRun} onClick={() => void command(runExecution, "RUNNING")}>▶ RUN</button>
              <button className="hq-button" disabled={!!op || execution.status !== "running"} onClick={() => void command(pauseExecution, "SYNCING")}>⏸ PAUSE</button>
              <button className="hq-button" disabled={!!op || !(execution.status === "paused" || execution.status === "ready")} onClick={() => void command(stepExecution, "SYNCING")}>⏭ STEP</button>
              <button className="hq-button hq-button--danger" disabled={!!op || (execution.status !== "running" && execution.status !== "paused")} onClick={() => void command(stopExecution, "SYNCING")}>⏹ STOP</button>
              <button className="hq-button" disabled={!!op} onClick={() => void command(replayExecution, "REPLAYING")}>↻ REPLAY</button>
              <button className="hq-button" disabled={!!op || !selected} title={selected ? `Rewind to ${formatLogicalTime(selected.time?.nanos)}` : "Select a timeline marker first"}
                onClick={() => selected && void command((id) => rewindExecution(id, selected.time?.nanos ?? "0"), "REPLAYING")}>|◀ REWIND TO SELECTION</button>
              {cmdError ? <span role="alert" style={{ color: "var(--hq-failure)", alignSelf: "center", fontSize: "var(--hq-text-sm)" }}>{cmdError}</span> : null}
            </div>
          </section>

          <div className="exec-layout">
            <ControlPanel
              executionId={execution.id} availableActions={data.actions} actionsUnavailable={data.actionsUnavailable}
              targetComponentId={ui.selectedComponentId}
              targetComponentName={data.model.data?.components.find((c) => c.id === ui.selectedComponentId)?.name ?? null}
              recentEvents={items} onActionExecuted={() => { void data.refresh(); timeline.reload(); }}
            />
            <section className="hq-panel" aria-label="Execution activity">
              <div className="hq-panel__header"><span className="label">Execution Timeline</span></div>
              <div className="hq-panel__body">
                <Timeline items={items} selectedId={selectedTimelineId} currentTimeNanos={execution.currentTime?.nanos}
                  onSelectItem={(item) => { setSelectedTimelineId(item.id); void data.refresh(); }} />
              </div>
            </section>
            <StatePanel loading={!data.liveState && !data.stateUnavailable} state={data.liveState}
              selectedComponent={data.model.data?.components.find((c) => c.id === ui.selectedComponentId) ?? null} unavailable={data.stateUnavailable} />
          </div>
        </>
      )}
    </div>
  );
}
