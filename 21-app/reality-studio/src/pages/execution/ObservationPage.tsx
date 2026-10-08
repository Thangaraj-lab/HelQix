/** Observation — charts rendered ONLY from actual metric samples via API. */
import { useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { listExecutions, listMetrics, getMetricSeries } from "../../api/resources";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import { formatLogicalTime, nanosToSecondsFloat, trimFloat } from "../../utils/format";
import type { MetricSeries } from "../../api/types";
import "./charts.css";

const W = 760, H = 320, PAD = 44;

export function MetricChart({ series, limitMax }: { series: MetricSeries; limitMax?: number | null }) {
  const points = useMemo(() => {
    const samples = series.samples;
    if (samples.length === 0) return { path: "", area: "", tMin: 0, tMax: 1, vMin: 0, vMax: 1 };
    const ts = samples.map((s) => nanosToSecondsFloat(s.time?.nanos) ?? 0);
    const vs = samples.map((s) => s.value);
    const tMin = Math.min(...ts), tMax = Math.max(...ts, tMin + 1e-9);
    let vMin = Math.min(...vs), vMax = Math.max(...vs);
    if (limitMax !== undefined && limitMax !== null) vMax = Math.max(vMax, limitMax);
    if (vMax - vMin < 1e-9) { vMin -= 1; vMax += 1; }
    const x = (t: number) => PAD + ((t - tMin) / (tMax - tMin)) * (W - PAD * 2);
    const y = (v: number) => H - PAD - ((v - vMin) / (vMax - vMin)) * (H - PAD * 2);
    const path = samples.map((s, i) => `${i === 0 ? "M" : "L"}${x(ts[i] ?? 0).toFixed(1)},${y(vs[i] ?? 0).toFixed(1)}`).join(" ");
    const area = `${path} L${x(tMax).toFixed(1)},${H - PAD} L${x(tMin).toFixed(1)},${H - PAD} Z`;
    return { path, area, tMin, tMax, vMin, vMax };
  }, [series, limitMax]);

  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={`Metric chart with ${series.samples.length} observed samples`}>
      {[0, 0.25, 0.5, 0.75, 1].map((f) => {
        const gy = PAD + f * (H - PAD * 2);
        const val = points.vMax - f * (points.vMax - points.vMin);
        return (
          <g key={f}>
            <line className="chart__grid" x1={PAD} y1={gy} x2={W - PAD} y2={gy} />
            <text className="chart__label" x={PAD - 6} y={gy + 3} textAnchor="end">{trimFloat(val, 1)}</text>
          </g>
        );
      })}
      <line className="chart__axis" x1={PAD} y1={H - PAD} x2={W - PAD} y2={H - PAD} />
      <text className="chart__label" x={PAD} y={H - PAD + 16}>{trimFloat(points.tMin, 1)} s</text>
      <text className="chart__label" x={W - PAD} y={H - PAD + 16} textAnchor="end">{trimFloat(points.tMax, 1)} s</text>
      {limitMax !== undefined && limitMax !== null && points.vMax >= limitMax ? (
        <g>
          <line className="chart__limit" x1={PAD} y1={H - PAD - ((limitMax - points.vMin) / (points.vMax - points.vMin)) * (H - PAD * 2)} x2={W - PAD} y2={H - PAD - ((limitMax - points.vMin) / (points.vMax - points.vMin)) * (H - PAD * 2)} />
          <text className="chart__label" x={W - PAD} y={H - PAD - ((limitMax - points.vMin) / (points.vMax - points.vMin)) * (H - PAD * 2) - 4} textAnchor="end">limit {trimFloat(limitMax, 1)}</text>
        </g>
      ) : null}
      <path className="chart__area" d={points.area} />
      <path className="chart__series" d={points.path} />
    </svg>
  );
}

export function ObservationPage() {
  const { projectId = "" } = useParams();
  const [metricId, setMetricId] = useState<string | null>(null);

  // Observations belong to an execution of the project's reality.
  const realities = useAsyncResource(async () => {
    const { listRealities } = await import("../../api/resources");
    const r = await listRealities(projectId);
    return r[0]?.id ?? null;
  }, [projectId]);
  const executions = useAsyncResource(realities.data ? () => listExecutions(realities.data!) : null, [realities.data]);
  const executionId = executions.data?.[0]?.id ?? null;

  const metrics = useAsyncResource(executionId ? () => listMetrics(executionId) : null, [executionId]);
  const activeMetric = metrics.data?.find((m) => m.id === metricId) ?? metrics.data?.[0] ?? null;
  const series = useAsyncResource(
    executionId && activeMetric ? () => getMetricSeries(executionId, activeMetric.id) : null,
    [executionId, activeMetric?.id],
  );

  const lastValue = series.data && series.data.samples.length > 0 ? series.data.samples[series.data.samples.length - 1]?.value ?? null : null;

  return (
    <div className="page">
      <PageHeader title="Observation" subtitle="Charts show recorded observations only. Limits come from engineering constraints supplied by the backend." />
      <ResourceGate phase={metrics.phase} error={metrics.error} data={metrics.data} feature="Observation service (metrics/probes)" onRetry={metrics.reload} isEmpty={(m) => m.length === 0} emptyTitle="No probes defined for this execution">
        {(list) => (
          <div className="obs-layout">
            <section className="hq-panel" aria-label="Available metrics">
              <div className="hq-panel__header"><span className="label">Probes</span></div>
              <div className="hq-panel__body probe-list">
                {list.map((m) => (
                  <button key={m.id} className="probe" aria-selected={activeMetric?.id === m.id} onClick={() => setMetricId(m.id)}>
                    {m.name} <span className="mono" style={{ float: "right", opacity: 0.6 }}>{m.unit}</span>
                  </button>
                ))}
              </div>
            </section>
            <section className="hq-panel" aria-label="Primary chart">
              <div className="hq-panel__header">
                <span className="label">{activeMetric?.name ?? "Metric"} vs Time</span>
                {activeMetric?.limit ? <Badge tone="warning">constraint {activeMetric.limit.constraintId.slice(0, 8)}</Badge> : null}
              </div>
              <div className="hq-panel__body">
                <ResourceGate phase={series.phase} error={series.error} data={series.data} feature="Metric series endpoint" onRetry={series.reload} isEmpty={(s) => s.samples.length === 0} emptyTitle="No samples recorded yet">
                  {(s) => <MetricChart series={s} limitMax={activeMetric?.limit?.max ?? null} />}
                </ResourceGate>
              </div>
            </section>
            <section className="hq-panel" aria-label="Current value">
              <div className="hq-panel__header"><span className="label">Current</span></div>
              <div className="hq-panel__body">
                {lastValue === null ? <p className="page__subtitle">Awaiting observation stream.</p> : (
                  <>
                    <div className={`current-value${activeMetric?.limit && lastValue > activeMetric.limit.max ? " current-value--over" : ""}`}>
                      {trimFloat(lastValue)} <span style={{ fontSize: "1rem" }}>{activeMetric?.unit}</span>
                    </div>
                    {activeMetric?.limit ? <p className="page__subtitle mono" style={{ marginTop: 6 }}>Limit: {trimFloat(activeMetric.limit.max)} {activeMetric.unit}</p> : null}
                  </>
                )}
              </div>
            </section>
          </div>
        )}
      </ResourceGate>
      {series.data ? (
        <p className="page__subtitle mono" style={{ marginTop: "var(--hq-sp-3)" }}>
          window: {formatLogicalTime(series.data.samples[0]?.time.nanos)} → {formatLogicalTime(series.data.samples[series.data.samples.length - 1]?.time.nanos)} · {series.data.samples.length} samples
        </p>
      ) : null}
    </div>
  );
}
