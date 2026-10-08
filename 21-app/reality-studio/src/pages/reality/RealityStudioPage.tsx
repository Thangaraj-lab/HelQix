/**
 * REALITY STUDIO — the core HELQIX screen.
 * Component tree · visual runtime · live state inspector · action console · timeline.
 */
import { useMemo, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { useRealityData } from "../../hooks/useRealityData";
import { getTimeline } from "../../api/resources";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { ComponentTree } from "../../components/component-tree/ComponentTree";
import { RealityView } from "../../components/reality-view/RealityView";
import { StatePanel } from "../../components/state-panel/StatePanel";
import { ControlPanel } from "../../components/control-panel/ControlPanel";
import { Timeline } from "../../components/timeline/Timeline";
import { Badge, UnavailablePanel, ErrorPanel } from "../../components/common";
import { useAppState } from "../../state/AppState";
import { formatLogicalTime } from "../../utils/format";
import type { TimelineItem } from "../../api/types";
import "./reality.css";

const statusTone = (s: string) => (s === "running" ? "active" : s === "faulted" || s === "failed" ? "failure" : s === "paused" ? "warning" : "normal") as const;

export function RealityStudioPage() {
  const { projectId = "" } = useParams();
  const { state: ui, dispatch } = useAppState();
  const data = useRealityData(projectId);
  const [inspectorOpen, setInspectorOpen] = useState(false);

  const timeline = useAsyncResource(
    data.execution ? () => getTimeline(data.execution!.id) : null,
    [data.execution?.id],
  );
  const timelineItems = useMemo<TimelineItem[]>(() => (timeline.data ?? []).slice().sort((a, b) => Number(a.time?.nanos ?? 0) - Number(b.time?.nanos ?? 0)), [timeline.data]);
  const [selectedTimelineId, setSelectedTimelineId] = useState<string | null>(null);

  const selectedComponent = data.model.data?.components.find((c) => c.id === ui.selectedComponentId) ?? null;

  if (data.realities.phase === "unavailable" || (data.realities.phase === "ready" && data.realities.data && data.realities.data.length === 0)) {
    return (
      <div className="page">
        <UnavailablePanel
          feature="Reality construction service"
          detail={data.realities.phase === "unavailable" ? "GET /projects/:id/realities not implemented yet" : "No reality constructed for this project — compile the model into a reality via the backend."}
        />
        <p style={{ marginTop: "var(--hq-sp-4)" }}><Link to="../model">← Back to Model</Link></p>
      </div>
    );
  }
  if (data.realities.phase === "error") return <div className="page"><ErrorPanel error={data.realities.error} onRetry={data.realities.reload} /></div>;

  return (
    <div className="studio">
      <div className="hq-panel studio__bar">
        <span className="label">Project / Reality</span>
        <Badge tone={statusTone(data.execution?.status ?? "idle")}>
          {data.execution ? `${data.execution.status.toUpperCase()} @ ${formatLogicalTime(data.execution.currentTime?.nanos)}` : "NO EXECUTION"}
        </Badge>
        <span style={{ flex: 1 }} />
        <button className="hq-button hq-button--sm" style={{ display: "none" }} id="inspector-toggle" onClick={() => setInspectorOpen((o) => !o)}>State ⇥</button>
        <Link className="hq-button hq-button--sm" to="../execution">EXECUTION CONTROLS →</Link>
      </div>

      <section className="hq-panel studio__tree" aria-label="Component tree">
        <div className="hq-panel__header"><span className="label">Components</span></div>
        <div style={{ padding: "var(--hq-sp-2)" }}>
          {data.model.phase === "loading" ? <p className="page__subtitle">Loading model…</p> : null}
          {data.model.data ? (
            <ComponentTree components={data.model.data.components} selectedId={ui.selectedComponentId} onSelect={(id) => dispatch({ type: "context/selectComponent", componentId: id })} />
          ) : null}
        </div>
      </section>

      <div className="studio__center">
        <RealityView
          components={data.model.data?.components ?? []}
          liveState={data.liveState}
          selectedId={ui.selectedComponentId}
          onSelect={(id) => dispatch({ type: "context/selectComponent", componentId: id })}
        />
      </div>

      <div className="studio__inspector" data-open={inspectorOpen}>
        <StatePanel loading={data.liveState === null && !data.stateUnavailable} state={data.liveState} selectedComponent={selectedComponent} unavailable={data.stateUnavailable} />
      </div>

      <div className="studio__console">
        <ControlPanel
          executionId={data.execution?.id ?? null}
          availableActions={data.actions}
          actionsUnavailable={data.actionsUnavailable}
          targetComponentId={ui.selectedComponentId}
          targetComponentName={selectedComponent?.name ?? null}
          recentEvents={timelineItems.filter((i) => i.severity !== "info").concat(timelineItems).slice(0, 60)}
          onActionExecuted={() => { void data.refresh(); timeline.reload(); }}
        />
      </div>

      <section className="hq-panel studio__timeline" aria-label="Execution timeline">
        <div className="hq-panel__header"><span className="label">Execution Timeline</span><Link to="../timeline" className="hq-button hq-button--ghost hq-button--sm">Full timeline →</Link></div>
        <div className="hq-panel__body">
          <Timeline items={timelineItems} selectedId={selectedTimelineId} currentTimeNanos={data.execution?.currentTime?.nanos}
            onSelectItem={(item) => { setSelectedTimelineId(item.id); void data.refresh(); }} />
        </div>
      </section>
    </div>
  );
}
