/** Search / optimize — defines the search space and runs it through Layer-12 services. */
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getSearchStatus, listCandidates, startSearch } from "../../api/resources";
import { ResourceGate, PageHeader, Badge, EmptyState } from "../../components/common";
import { ApiError } from "../../api/errors";
import { formatPercent, trimFloat } from "../../utils/format";
import type { CandidateRow, SearchRunStatus } from "../../api/types";
import "./search.css";

interface VarDraft { name: string; min: string; max: string; unit: string }

export function SearchPage() {
  const { projectId = "" } = useParams();
  const [vars, setVars] = useState<VarDraft[]>([{ name: "", min: "", max: "", unit: "" }]);
  const [sense, setSense] = useState<"minimize" | "maximize">("minimize");
  const [objective, setObjective] = useState("");
  const [constraints, setConstraints] = useState("");
  const [run, setRun] = useState<SearchRunStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedCandidate, setSelectedCandidate] = useState<CandidateRow | null>(null);

  const status = useAsyncResource(run ? () => getSearchStatus(run.runId) : null, [run?.runId]);
  const candidates = useAsyncResource(run ? () => listCandidates(run.runId) : null, [run?.runId]);

  // Poll run status only while a search is running (backend has no push channel yet).
  useEffect(() => {
    if (!run || status.data?.state !== "running") return;
    const timer = setInterval(() => status.reload(), 2000);
    return () => clearInterval(timer);
  }, [run, status]);

  async function onRun() {
    setBusy(true); setError(null); setRun(null); setSelectedCandidate(null);
    try {
      const definition = {
        variables: vars.filter((v) => v.name.trim()).map((v) => ({ name: v.name.trim(), min: Number(v.min), max: Number(v.max), unit: v.unit.trim() })),
        objective: { sense, expression: objective.trim() },
        constraints: constraints.split("\n").map((c) => c.trim()).filter(Boolean),
      };
      const started = await startSearch(projectId, definition);
      setRun(started);
      status.reload(); candidates.reload();
    } catch (err) {
      setError(err instanceof ApiError ? err.userMessage : "Search could not start.");
    } finally {
      setBusy(false);
    }
  }

  const live = status.data ?? run;

  return (
    <div className="page">
      <PageHeader title="Search & Optimize" subtitle="Define the search space; optimization executes on the Layer-12 service. No optimizer logic runs in the frontend." actions={live ? <Badge tone={live.state === "completed" ? "success" : live.state === "failed" ? "failure" : "active"}>{live.state.toUpperCase()}</Badge> : undefined} />
      <div className="search-layout">
        <section className="hq-panel" aria-label="Search definition">
          <div className="hq-panel__header"><span className="label">Search definition</span></div>
          <div className="hq-panel__body">
            <div className="label" style={{ marginBottom: 6 }}>Variables</div>
            {vars.map((v, i) => (
              <div className="var-row" key={i}>
                <input className="hq-input" placeholder="name" aria-label={`Variable ${i + 1} name`} value={v.name} onChange={(e) => setVars(vars.map((x, j) => (j === i ? { ...x, name: e.target.value } : x)))} />
                <input className="hq-input mono" placeholder="min" aria-label="min" value={v.min} onChange={(e) => setVars(vars.map((x, j) => (j === i ? { ...x, min: e.target.value } : x)))} />
                <input className="hq-input mono" placeholder="max" aria-label="max" value={v.max} onChange={(e) => setVars(vars.map((x, j) => (j === i ? { ...x, max: e.target.value } : x)))} />
                <input className="hq-input" placeholder="unit" aria-label="unit" value={v.unit} onChange={(e) => setVars(vars.map((x, j) => (j === i ? { ...x, unit: e.target.value } : x)))} />
              </div>
            ))}
            <button className="hq-button hq-button--ghost hq-button--sm" onClick={() => setVars([...vars, { name: "", min: "", max: "", unit: "" }])}>+ variable</button>
            <label className="hq-field" style={{ marginTop: "var(--hq-sp-4)" }}><span className="label">Objective ({sense})</span>
              <input className="hq-input mono" value={objective} onChange={(e) => setObjective(e.target.value)} placeholder="e.g. peak_temperature" /></label>
            <label className="hq-field"><span className="label">Sense</span>
              <select className="hq-select" value={sense} onChange={(e) => setSense(e.target.value as "minimize" | "maximize")}><option value="minimize">minimize</option><option value="maximize">maximize</option></select></label>
            <label className="hq-field"><span className="label">Constraints (one per line)</span>
              <textarea className="hq-textarea" value={constraints} onChange={(e) => setConstraints(e.target.value)} placeholder={"thrust >= 90\nmass <= 1200"} /></label>
            {error ? <div className="hq-banner hq-banner--error" role="alert" style={{ marginBottom: "var(--hq-sp-3)" }}>{error}</div> : null}
            <button className="hq-button hq-button--primary" disabled={busy || !objective.trim()} onClick={() => void onRun()}>{busy ? "STARTING…" : "RUN SEARCH"}</button>
          </div>
        </section>

        <div style={{ display: "grid", gap: "var(--hq-sp-3)" }}>
          {live ? (
            <section className="hq-panel" aria-label="Search progress">
              <div className="hq-panel__header"><span className="label">Progress</span><span className="mono page__subtitle">{live.evaluated} evaluated</span></div>
              <div className="hq-panel__body">
                <div className="progress-bar"><div className="progress-bar__fill" style={{ width: formatPercent(Math.max(0, Math.min(1, live.progress))) }} /></div>
              </div>
            </section>
          ) : null}
          <section className="hq-panel" aria-label="Candidates">
            <div className="hq-panel__header"><span className="label">Candidates & ranking</span></div>
            {run ? (
              <ResourceGate phase={candidates.phase} error={candidates.error} data={candidates.data} feature="Search candidate service" onRetry={candidates.reload} isEmpty={(c) => c.length === 0} emptyTitle="No candidates yet" emptyBody="The optimizer reports candidates as evaluations complete.">
                {(list) => (
                  <table className="hq-table">
                    <thead><tr><th>#</th><th>Parameters</th><th>Objective</th><th>Feasible</th><th>Status</th></tr></thead>
                    <tbody>
                      {list.map((c) => (
                        <tr key={c.id} style={{ cursor: "pointer" }} onClick={() => setSelectedCandidate(c)}>
                          <td className="mono">{c.rank}</td>
                          <td className="mono">{Object.entries(c.parameters).map(([k, v]) => `${k}=${v}`).join(", ")}</td>
                          <td className="mono">{c.objectiveValue}</td>
                          <td>{c.feasible ? <Badge tone="success">yes</Badge> : <Badge tone="failure">no</Badge>}</td>
                          <td><Badge tone={c.status === "verified" ? "success" : c.status === "violated" ? "failure" : "normal"}>{c.status}</Badge></td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </ResourceGate>
            ) : (
              <EmptyState title="No search run" body="Configure the search space and press RUN SEARCH." />
            )}
          </section>
          {selectedCandidate ? (
            <section className="hq-panel" aria-label="Candidate detail">
              <div className="hq-panel__header"><span className="label">Candidate #{selectedCandidate.rank} detail</span><Badge tone={selectedCandidate.status === "verified" ? "success" : "normal"}>{selectedCandidate.status}</Badge></div>
              <div className="hq-panel__body">
                <table className="prop-table"><tbody>
                  {Object.entries(selectedCandidate.parameters).map(([k, v]) => <tr key={k}><td>{k}</td><td className="mono">{v}</td></tr>)}
                  <tr><td>Objective</td><td className="mono">{trimFloat(Number(selectedCandidate.objectiveValue)) }</td></tr>
                </tbody></table>
                <p className="page__subtitle" style={{ marginTop: "var(--hq-sp-3)", fontSize: "var(--hq-text-xs)" }}>
                  Execution results, metrics, verification and evidence for this candidate link into Verify/Evidence once the backend associates them.
                </p>
              </div>
            </section>
          ) : null}
        </div>
      </div>
    </div>
  );
}
