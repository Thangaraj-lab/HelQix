import { useMemo, useState } from 'react';
import { Activity, Check, CircleAlert, FastForward, Pause, Play, RotateCcw, SkipForward } from 'lucide-react';
import type { EngineeringReality } from '../api';
import { createInitialState } from './authoritativeState';
import { createExecutionController, pauseExecution, resetExecution, restoreExecutionCheckpoint, runToCompletion, startExecution, stepExecution, type ExecutionController } from './executionController';
import { createScenario, validateScenario, type ExecutableScenario, type ScenarioAction } from './scenarioEngine';
import { buildStateTimeline } from './stateTimeline';

type ScenarioDockProps = {
  reality: EngineeringReality;
  selectedComponentId?: string | null;
  mode: string;
  onStateChange?: (state: ExecutionController['state'] | null) => void;
};

const shortId = () => `act-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;

function defaultAction(reality: EngineeringReality, componentId: string, type: ScenarioAction['type'], atMs: number): ScenarioAction {
  const action: ScenarioAction = { id: shortId(), type, atMs };
  if (type === 'set_status') return { ...action, componentId, status: 'running' };
  if (type === 'inject_fault') return { ...action, componentId, faultId: `fault-${shortId()}` };
  if (type === 'set_property') {
    const component = reality.components.find((item) => item.id === componentId);
    const property = component?.properties.find((item) => typeof item.value === 'number') ?? component?.properties[0];
    if (!property) throw new Error('The selected component has no declared property to change. Add a property in Build first.');
    const value = typeof property.value === 'number' ? property.value : typeof property.value === 'boolean' ? !property.value : `${property.value}-scenario`;
    return { ...action, componentId, propertyName: property.name, value, ...(property.unit ? { unit: property.unit } : {}) };
  }
  return action;
}

function makeSeedScenario(reality: EngineeringReality, componentId: string) {
  const actions: ScenarioAction[] = [
    { id: shortId(), type: 'set_status', atMs: 0, componentId, status: 'running' },
    { id: shortId(), type: 'advance_time', atMs: 1000 },
    { id: shortId(), type: 'set_status', atMs: 1000, componentId, status: 'completed' },
    { id: shortId(), type: 'checkpoint', atMs: 1000, checkpointId: 'completion' },
  ];
  return createScenario('Component lifecycle', reality, actions, 'Generic state-transition demonstration; it does not invoke a physics solver.');
}

export function ScenarioDock({ reality, selectedComponentId, mode, onStateChange }: ScenarioDockProps) {
  const firstComponentId = selectedComponentId && reality.components.some((item) => item.id === selectedComponentId) ? selectedComponentId : reality.components[0]?.id ?? '';
  const [scenario, setScenario] = useState<ExecutableScenario | null>(() => firstComponentId ? makeSeedScenario(reality, firstComponentId) : null);
  const [controller, setController] = useState<ExecutionController | null>(null);
  const [actionType, setActionType] = useState<ScenarioAction['type']>('set_status');
  const [actionTime, setActionTime] = useState('0');
  const [actionComponent, setActionComponent] = useState(firstComponentId);
  const [faultText, setFaultText] = useState('test-fault');
  const [notice, setNotice] = useState('');
  const state = controller?.state ?? null;
  const timeline = useMemo(() => state && scenario ? buildStateTimeline(reality, state) : null, [reality, state, scenario]);
  const issues = useMemo(() => scenario ? validateScenario(scenario, reality) : [], [scenario, reality]);
  const busy = controller?.phase === 'running';
  const active = controller && !['completed', 'failed'].includes(controller.phase);
  const currentAction = scenario?.actions.filter((a) => a.enabled !== false).sort((a, b) => a.atMs - b.atMs || (a.order ?? 0) - (b.order ?? 0))[controller?.cursor ?? 0];

  // Keep the local scenario revision-aligned and ensure a changed reality never reuses stale state.
  useState(() => {
    if (scenario?.createdFromRealityRevision !== reality.revision) setScenario(firstComponentId ? makeSeedScenario(reality, firstComponentId) : null);
    setController(null);
  });
  useState(() => { setActionComponent(firstComponentId); });

  const emit = (next: ExecutionController | null) => {
    setController(next);
    onStateChange?.(next?.state ?? null);
    setNotice('');
  };
  const resetToCurrentReality = () => {
    if (!scenario) return;
    const nextScenario = makeSeedScenario(reality, actionComponent || firstComponentId);
    setScenario(nextScenario);
    const initial = createInitialState(reality);
    const next = createExecutionController(reality, nextScenario, initial);
    emit(next);
    setActionTime('0');
  };
  const ensureController = () => {
    if (!scenario) return null;
    if (controller && controller.realityRevision === reality.revision && controller.scenarioId === scenario.id) return controller;
    const next = createExecutionController(reality, scenario, createInitialState(reality));
    setController(next);
    onStateChange?.(next.state);
    return next;
  };
  const applyStep = () => {
    const current = ensureController();
    if (!current || !scenario) return;
    const next = current.phase === 'running' ? current : startExecution(current);
    const result = stepExecution(reality, scenario, next);
    emit(result.controller);
  };
  const runAll = () => {
    const current = ensureController();
    if (!current || !scenario) return;
    emit(runToCompletion(reality, scenario, current));
  };
  const pause = () => { if (controller) emit(pauseExecution(controller)); };
  const reset = () => { if (controller) emit(resetExecution(controller)); };
  const restore = (id: string) => {
    if (!controller) return;
    try { emit(restoreExecutionCheckpoint(controller, id)); }
    catch (e) { setNotice(e instanceof Error ? e.message : 'Could not restore checkpoint.'); }
  };
  const addAction = () => {
    if (!scenario || !actionComponent) { setNotice('Add a component in Build before creating component actions.'); return; }
    const time = Number(actionTime);
    if (!Number.isFinite(time) || time < 0) { setNotice('Action time must be a finite, non-negative number of milliseconds.'); return; }
    try {
      let action = defaultAction(reality, actionComponent, actionType, time);
      if (actionType === 'inject_fault') action = { ...action, faultId: faultText.trim() || `fault-${shortId()}` };
      const next = createScenario(scenario.name, reality, [...scenario.actions, action], scenario.description);
      next.version = scenario.version + 1;
      setScenario(next);
      setController(null);
      setNotice('Action added. Reset execution to evaluate the edited scenario.');
    } catch (e) { setNotice(e instanceof Error ? e.message : 'Could not add action.'); }
  };

  if (!reality.components.length) return <section className="scenario-dock"><div className="scenario-dock-head"><div className="scenario-dock-title"><b>SCENARIO TIMELINE</b><small>BUILD needs at least one component</small></div></div><p className="scenario-dock-footer">Create an engineering component before defining executable component actions.</p></section>;

  const ordered = [...(scenario?.actions ?? [])].filter((a) => a.enabled !== false).sort((a, b) => a.atMs - b.atMs || (a.order ?? 0) - (b.order ?? 0));
  const visibleActions = ordered;
  const status = controller?.phase ?? 'not started';
  const durationMs = ordered.reduce((max, action) => Math.max(max, action.atMs), 0);

  return <section className={`scenario-dock scenario-dock-${mode.toLowerCase()}`} aria-label="Executable scenario timeline">
    <header className="scenario-dock-head">
      <div className="scenario-dock-title"><b>EXECUTABLE SCENARIO</b><small>{scenario?.name ?? 'No scenario'} · {scenario?.actions.length ?? 0} actions · reality R{reality.revision}</small></div>
      <span className="scenario-dock-spacer"/>
      <span className={`scenario-dock-status ${status === 'failed' ? 'is-failed' : status === 'completed' ? 'is-completed' : ''}`}>{status}</span>
      <span className="scenario-dock-clock">T+{((controller?.logicalTimeMs ?? 0) / 1000).toFixed(3)} s</span>
    </header>
    <div className="scenario-dock-actions">
      <button type="button" className="primary" onClick={runAll} disabled={!scenario || !ordered.length || issues.some((issue) => issue.severity === 'error') || busy}><Play size={12}/> Run all</button>
      <button type="button" onClick={applyStep} disabled={!scenario || !ordered.length || issues.some((issue) => issue.severity === 'error') || status === 'completed' || status === 'failed'}><SkipForward size={12}/> Step</button>
      <button type="button" onClick={pause} disabled={!busy}><Pause size={12}/> Pause</button>
      <button type="button" onClick={resetToCurrentReality}><RotateCcw size={12}/> New baseline</button>
      <label>Restore checkpoint <select value="" onChange={(e) => restore(e.target.value)} disabled={!controller?.checkpoints.length}><option value="">Choose…</option>{(controller?.checkpoints ?? []).map((snap) => <option key={snap.id} value={snap.id}>{snap.id} · T+{(snap.logicalTimeMs / 1000).toFixed(2)}s</option>)}</select></label>
    </div>
    <div className="scenario-builder">
      <select aria-label="Action type" value={actionType} onChange={(e) => setActionType(e.target.value as ScenarioAction['type'])}>
        <option value="set_status">Change component status</option>
        <option value="set_property">Change declared property</option>
        <option value="advance_time">Advance logical time</option>
        <option value="inject_fault">Inject test fault</option>
        <option value="checkpoint">Capture checkpoint</option>
      </select>
      <select aria-label="Target component" value={actionComponent} onChange={(e) => setActionComponent(e.target.value)}>
        {reality.components.map((component) => <option key={component.id} value={component.id}>{component.name}</option>)}
      </select>
      <input aria-label="Action time in milliseconds" type="number" min="0" step="any" value={actionTime} onChange={(e) => setActionTime(e.target.value)} placeholder="Time (ms)"/>
      <button type="button" onClick={addAction}><Check size={12}/> Add action</button>
    </div>
    {actionType === 'inject_fault' && <div className="scenario-dock-footer"><label>Fault identifier <input value={faultText} onChange={(e) => setFaultText(e.target.value)} /></label><small>Fault injection changes declared runtime state only; it does not simulate physical failure propagation.</small></div>}
    <div className="scenario-dock-track" role="list" aria-label="Scenario actions">
      {visibleActions.map((action, index) => <button key={action.id} type="button" className="scenario-action-chip" role="listitem" aria-current={index === controller?.cursor ? 'step' : undefined} title={`${action.type} at ${action.atMs} ms`}><small>T+{(action.atMs / 1000).toFixed(3)}s · #{index + 1}</small><b>{action.type.replaceAll('_', ' ')}</b></button>)}
    </div>
    {notice && <p className="scenario-notice" role="status"><CircleAlert size={12}/>{notice}</p>}
    {issues.some((issue) => issue.severity === 'error') && <p className="scenario-notice" role="alert"><CircleAlert size={12}/>{issues.filter((issue) => issue.severity === 'error').length} scenario validation error(s): {issues.filter((issue) => issue.severity === 'error').map((issue) => issue.message).join(' ')}</p>}
    <footer className="scenario-dock-footer">
      <span>{timeline?.entries.length ?? 0} state events · {timeline?.failures.length ?? 0} failure observation(s) · {controller?.checkpoints.length ?? 0} checkpoint(s)</span>
      <span>{durationMs ? `Duration ${(durationMs / 1000).toFixed(3)} s` : 'Zero-time scenario'} · {controller ? `State S${controller.state.stateRevision}` : 'State not initialized'}</span>
    </footer>
    {!!timeline?.entries.length && <div className="scenario-timeline-events" aria-label="State event timeline">{timeline.entries.slice(-12).map((entry) => <div key={`${entry.sequence}:${entry.logicalTimeMs}:${entry.type}`} className={`scenario-timeline-event ${entry.type === 'fault_injected' ? 'is-failure' : ''}`}><b>{entry.type.replaceAll('_', ' ')}</b> · T+{(entry.logicalTimeMs / 1000).toFixed(3)}s {entry.componentId ? `· ${entry.componentId}` : ''}</div>)}</div>}
  </section>;
}
