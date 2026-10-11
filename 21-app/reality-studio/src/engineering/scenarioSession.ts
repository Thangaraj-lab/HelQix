import type { EngineeringReality } from '../api';
import {
  captureSnapshot,
  createInitialState,
  type AuthoritativeEngineeringState,
  type StateSnapshot,
} from './authoritativeState';
import {
  createScenario,
  executeScenario,
  type ExecutableScenario,
  type ScenarioAction,
  type ScenarioExecutionResult,
} from './scenarioEngine';

export const LOCAL_SCENARIO_SESSION_SCHEMA = 'helqix.local-scenario-session.v1' as const;
const STORAGE_PREFIX = 'helqix:scenario-session:v1:';

export interface ScenarioSessionRecord {
  schema: typeof LOCAL_SCENARIO_SESSION_SCHEMA;
  projectId: string;
  realityRevision: number;
  scenario: ExecutableScenario;
  initialState: AuthoritativeEngineeringState;
  result?: ScenarioExecutionResult;
  snapshots: StateSnapshot[];
  updatedAt: string;
}

export interface ScenarioSessionSummary {
  scenarioId: string;
  name: string;
  actionCount: number;
  status: 'draft' | 'completed' | 'failed';
  updatedAt: string;
  logicalTimeMs: number;
  eventCount: number;
}

function storageKey(projectId: string): string {
  return `${STORAGE_PREFIX}${projectId}`;
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function now(): string {
  return new Date().toISOString();
}

/**
 * A browser-local workbench session for the generic scenario engine.
 * The session is explicitly local and is never presented as a server run,
 * physical simulation, or external-solver result.
 */
export function createScenarioSession(
  projectId: string,
  reality: EngineeringReality,
  name = 'Untitled scenario',
): ScenarioSessionRecord {
  if (!projectId.trim()) throw new Error('A project id is required for a scenario session.');
  const state = createInitialState(reality, { scenarioId: undefined });
  const scenario = createScenario(name, reality, [], 'Generic executable scenario on the saved engineering reality.');
  return {
    schema: LOCAL_SCENARIO_SESSION_SCHEMA,
    projectId,
    realityRevision: reality.revision,
    scenario,
    initialState: state,
    snapshots: [captureSnapshot(state, 'initial', `scenario-session-${scenario.id}-initial`)],
    updatedAt: now(),
  };
}

export function validateScenarioSession(
  session: ScenarioSessionRecord,
  projectId: string,
  reality: EngineeringReality,
): string[] {
  const issues: string[] = [];
  if (session.schema !== LOCAL_SCENARIO_SESSION_SCHEMA) issues.push('Unsupported scenario session schema.');
  if (session.projectId !== projectId) issues.push('Scenario session belongs to a different project.');
  if (session.realityRevision !== reality.revision) issues.push(`Scenario session is based on reality revision ${session.realityRevision}; current revision is ${reality.revision}. Start a new scenario after model changes.`);
  return issues;
}

export function appendSessionAction(
  session: ScenarioSessionRecord,
  action: ScenarioAction,
): ScenarioSessionRecord {
  const next = clone(session);
  if (next.scenario.actions.some((item) => item.id === action.id)) {
    throw new Error(`Action id already exists: ${action.id}`);
  }
  next.scenario.actions = [...next.scenario.actions, clone(action)]
    .sort((a, b) => a.atMs - b.atMs || (a.order ?? 0) - (b.order ?? 0) || a.id.localeCompare(b.id));
  next.scenario.version += 1;
  next.scenario.metadata.updatedAt = now();
  next.result = undefined;
  next.updatedAt = now();
  return next;
}

export function removeSessionAction(session: ScenarioSessionRecord, actionId: string): ScenarioSessionRecord {
  const next = clone(session);
  const originalCount = next.scenario.actions.length;
  next.scenario.actions = next.scenario.actions.filter((action) => action.id !== actionId);
  if (next.scenario.actions.length === originalCount) return session;
  next.scenario.version += 1;
  next.scenario.metadata.updatedAt = now();
  next.result = undefined;
  next.updatedAt = now();
  return next;
}

export function setSessionResult(
  session: ScenarioSessionRecord,
  result: ScenarioExecutionResult,
): ScenarioSessionRecord {
  if (result.scenarioId !== session.scenario.id) throw new Error('Cannot attach a result from a different scenario.');
  if (result.realityRevision !== session.realityRevision) throw new Error('Result revision does not match the saved scenario session.');
  const next = clone(session);
  next.result = clone(result);
  next.snapshots.push(captureSnapshot(result.state, result.status === 'completed' ? 'run-complete' : 'failure', `scenario-session-${session.scenario.id}-result-${Date.now()}`));
  next.updatedAt = now();
  return next;
}

export function currentSessionState(session: ScenarioSessionRecord): AuthoritativeEngineeringState {
  return clone(session.result?.state ?? session.initialState);
}

export function summarizeScenarioSession(session: ScenarioSessionRecord): ScenarioSessionSummary {
  const state = session.result?.state ?? session.initialState;
  return {
    scenarioId: session.scenario.id,
    name: session.scenario.name,
    actionCount: session.scenario.actions.filter((action) => action.enabled !== false).length,
    status: !session.result ? 'draft' : session.result.status === 'completed' ? 'completed' : 'failed',
    updatedAt: session.updatedAt,
    logicalTimeMs: state.logicalTimeMs,
    eventCount: state.events.length,
  };
}

export function saveScenarioSession(session: ScenarioSessionRecord): void {
  if (typeof window === 'undefined' || !window.localStorage) {
    throw new Error('Browser storage is not available; this session could not be saved locally.');
  }
  try {
    window.localStorage.setItem(storageKey(session.projectId), JSON.stringify(session));
  } catch (error) {
    const reason = error instanceof Error ? error.message : 'Browser storage quota exceeded.';
    throw new Error(`Could not save scenario session locally: ${reason}`);
  }
}

export function loadScenarioSession(projectId: string): ScenarioSessionRecord | undefined {
  if (typeof window === 'undefined' || !window.localStorage) return undefined;
  try {
    const raw = window.localStorage.getItem(storageKey(projectId));
    if (!raw) return undefined;
    const parsed = JSON.parse(raw) as Partial<ScenarioSessionRecord>;
    if (parsed.schema !== LOCAL_SCENARIO_SESSION_SCHEMA || parsed.projectId !== projectId
      || typeof parsed.realityRevision !== 'number' || !parsed.scenario || !parsed.initialState
      || !Array.isArray(parsed.snapshots)) return undefined;
    return parsed as ScenarioSessionRecord;
  } catch {
    return undefined;
  }
}

export function runScenarioSession(session: ScenarioSessionRecord, reality: EngineeringReality): ScenarioSessionRecord {
  const sessionIssues = validateScenarioSession(session, session.projectId, reality);
  if (sessionIssues.length) throw new Error(sessionIssues.join(' '));
  const result = executeScenario(session.scenario, reality, session.initialState);
  return setSessionResult(session, result);
}

export function restoreSessionSnapshot(
  session: ScenarioSessionRecord,
  snapshotId: string,
): AuthoritativeEngineeringState {
  const snapshot = session.snapshots.find((item) => item.id === snapshotId);
  if (!snapshot) throw new Error(`Unknown scenario snapshot: ${snapshotId}`);
  return clone(snapshot.state);
}
