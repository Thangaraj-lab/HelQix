import { useMemo, useState, type ReactNode } from 'react';
import { Check, CircleAlert, GitBranch, Pause, Play, RotateCcw, SkipForward, TriangleAlert } from 'lucide-react';
import type { ProjectPayload, Resource } from './api';
import { createInitialState } from './engineering/authoritativeState';
import { createExecutionController, pauseExecution, restoreExecutionCheckpoint, startExecution, stepExecution, runToCompletion, type ExecutionController } from './engineering/executionController';
import { buildStateTimeline } from './engineering/stateTimeline';
import { createFaultInjectionScenario, createInspectionScenario } from './engineering/scenarioTemplates';
import type { ExecutableScenario } from './engineering/scenarioEngine';

interface Props { project: Resource<ProjectPayload>; realityViewport: (state?: import('./engineering/authoritativeState').AuthoritativeEngineeringState) => ReactNode; selectedComponentId?: string | null; onSelectComponent?: (id: string | null) => void; onBackToRuns: () => void; }

export function ScenarioExecutionBoard({ project, realityViewport, selectedComponentId, onSelectComponent, onBackToRuns }: Props) {
  const reality = project.payload.engineering_reality;
  const [scenarioKind, setScenarioKind] = useState<'inspection' | 'fault'>('inspection');
  const [faultComponentId, setFaultComponentId] = useState(selectedComponentId ?? reality?.components[0]?.id ?? '');
  const [controller, setController] = useState<ExecutionController | null>(null);
  const [scenario, setScenario] = useState<ExecutableScenario | null>(null);
  const [error, setError] = useState('');
  const timeline = useMemo(() => controller && reality ? buildStateTimeline(reality, controller.state) : null, [controller, reality]);

  if (!reality) return <div className="canvas-stage stage-run">{realityViewport()}<section className="execution-empty"><h2>No engineering reality</h2><p>Build a saved engineering reality before executing a scenario.</p><button className="secondary-button" onClick={onBackToRuns}>Back to model runs</button></section></div>;
  const prepare = () => { try { setError(''); const chosen = scenarioKind === 'inspection' ? createInspectionScenario(reality) : createFaultInjectionScenario(reality, faultComponentId || reality.components[0]?.id || ''); const initial = createInitialState(reality, { scenarioId: chosen.id, executionId: `studio-${Date.now()}` }); setScenario(chosen); setController(createExecutionController(reality, chosen, initial)); } catch (cause) { setError(cause instanceof Error ? cause.message : 'Could not prepare scenario.'); } };
  const apply = (fn: (current: ExecutionController) => ExecutionController) => { if (!controller || !scenario) return; try { setError(''); setController(fn(controller)); } catch (cause) { setError(cause instanceof Error ? cause.message : 'Execution action failed.'); } };
  const step = () => { const currentScenario = scenario; if (!currentScenario) return; apply((current) => stepExecution(reality, currentScenario, current).controller); };
  const run = () => { const currentScenario = scenario; if (!currentScenario) return; apply((current) => runToCompletion(reality, currentScenario, current)); };
  const phase = controller?.phase ?? 'idle';
  const selectedComponent = reality.components.find((component) => component.id === (selectedComponentId ?? faultComponentId));

  return <div className="canvas-stage stage-run">{realityViewport(controller?.state)}<div className="stage-grid"/><section className="scenario-board">
    <header className="scenario-header"><div><button type="button" className="scenario-back" onClick={onBackToRuns}>← Model runs</button><span className="stage-kicker">EXECUTABLE REALITY <i/> AUTHORITATIVE STATE</span><h1>Scenario execution</h1><p>Run deterministic engineering actions against the saved reality. The 3D scene follows authoritative runtime state.</p></div><span className={`scenario-phase phase-${phase}`}><i/>{phase.toUpperCase()}</span></header>
    <div className="scenario-toolbar">
      <label>Scenario<select value={scenarioKind} onChange={(event) => { setScenarioKind(event.target.value as 'inspection' | 'fault'); setController(null); setScenario(null); }}><option value="inspection">Inspection · start → checkpoint → complete</option><option value="fault">Fault injection · start → fault → checkpoint</option></select></label>
      {scenarioKind === 'fault' && <label>Target component<select value={faultComponentId} onChange={(event) => { setFaultComponentId(event.target.value); onSelectComponent?.(event.target.value); setController(null); }}>{reality.components.map((component) => <option key={component.id} value={component.id}>{component.name}</option>)}</select></label>}
      <button className="secondary-button" onClick={prepare}><RotateCcw size={13}/> Prepare</button><button className="primary-button" disabled={!controller || phase === 'completed' || phase === 'failed'} onClick={() => apply(startExecution)}><Play size={13}/> Run</button><button className="secondary-button" disabled={phase !== 'running'} onClick={() => apply(pauseExecution)}><Pause size={13}/> Pause</button><button className="secondary-button" disabled={!controller || phase === 'completed' || phase === 'failed'} onClick={step}><SkipForward size={13}/> Step</button><button className="secondary-button" disabled={!controller || phase === 'completed' || phase === 'failed'} onClick={run}><Play size={13}/> Run all</button>
    </div>
    {error && <div className="scenario-error"><CircleAlert size={14}/>{error}</div>}
    <div className="scenario-grid">
      <section className="scenario-panel scenario-actions"><div className="panel-title"><span>ACTION TIMELINE</span><small>{scenario?.actions.length ?? 0} actions</small></div>{scenario ? <div className="action-list">{scenario.actions.map((action, index) => <div key={action.id} className={`action-row ${controller && index < controller.cursor ? 'action-complete' : ''} ${controller && index === controller.cursor && phase === 'running' ? 'action-current' : ''}`}><span className="action-index">{String(index + 1).padStart(2, '0')}</span><div><b>{action.type.replaceAll('_', ' ')}</b><small>{action.componentId ? reality.components.find((item) => item.id === action.componentId)?.name : 'global'} · t={action.atMs} ms</small></div>{controller && index < controller.cursor && <Check size={14}/>}</div>)}</div> : <div className="scenario-placeholder"><GitBranch size={19}/><span>Prepare a scenario to create executable actions.</span></div>}</section>
      <section className="scenario-panel scenario-state"><div className="panel-title"><span>AUTHORITATIVE STATE</span><small>REV {controller?.state.stateRevision ?? '—'}</small></div><div className="state-metrics"><Metric label="STATUS" value={controller?.state.status ?? 'READY'}/><Metric label="LOGICAL TIME" value={`${controller?.logicalTimeMs ?? 0} ms`}/><Metric label="EVENTS" value={String(controller?.events.length ?? 0)}/><Metric label="CHECKPOINTS" value={String(controller?.checkpoints.length ?? 0)}/></div>{selectedComponent && <div className="state-component"><b>{selectedComponent.name}</b><span>{controller?.state.components[selectedComponent.id]?.status ?? 'ready'}</span>{(controller?.state.components[selectedComponent.id]?.faultIds ?? []).map((fault) => <small key={fault}><TriangleAlert size={12}/> {fault}</small>)}</div>}</section>
    </div>
    <section className="scenario-panel scenario-events"><div className="panel-title"><span>OBSERVATION / EVENTS</span><small>{timeline?.entries.length ?? 0} events</small></div>{timeline?.entries.length ? <div className="event-list">{timeline.entries.map((entry) => <div className="event-row" key={`${entry.sequence}-${entry.actionId ?? entry.type}`}><time>{entry.logicalTimeMs} ms</time><b>{entry.type.replaceAll('_', ' ')}</b><span>{entry.componentId ? reality.components.find((item) => item.id === entry.componentId)?.name : 'runtime'}{entry.faultId ? ` · ${entry.faultId}` : ''}</span></div>)}</div> : <div className="scenario-placeholder">Execution events will appear here.</div>}</section>
    {controller && controller.checkpoints.length > 1 ? <section className="scenario-checkpoints"><div className="panel-title"><span>EXACT-STATE CHECKPOINTS</span><small>Restore creates a new authoritative state revision</small></div><div className="checkpoint-list">{controller.checkpoints.map((checkpoint) => <button key={checkpoint.id} className="checkpoint" onClick={() => apply((current) => restoreExecutionCheckpoint(current, checkpoint.id))}><span>{checkpoint.reason}</span><b>{checkpoint.logicalTimeMs} ms</b><small>{checkpoint.id}</small></button>)}</div></section> : null}
  </section></div>;
}
function Metric({ label, value }: { label: string; value: string }) { return <div className="state-metric"><small>{label}</small><b>{value}</b></div>; }
