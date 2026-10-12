import type { EngineeringReality } from '../api';
import { captureSnapshot, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import type { ExecutableScenario } from './scenarioEngine';

export const STATE_FREEZE_SCHEMA = 'helqix.state-freeze.v1' as const;

export interface StateFreeze {
  schema: typeof STATE_FREEZE_SCHEMA;
  id: string;
  createdAt: string;
  realityRevision: number;
  stateRevision: number;
  logicalTimeMs: number;
  scenarioId: string;
  scenarioVersion?: string;
  snapshot: StateSnapshot;
  realityDigest: string;
  scenarioDigest: string;
  executionConfiguration?: Record<string, unknown>;
  solver?: { name: string; version?: string };
  externalTools?: Array<{ name: string; version?: string }>;
  deterministic: boolean;
}

function digest(value: unknown): string {
  const serialized = JSON.stringify(value);
  let hash = 2166136261;
  for (let index = 0; index < serialized.length; index += 1) {
    hash ^= serialized.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

export function createStateFreeze(
  reality: EngineeringReality,
  state: AuthoritativeEngineeringState,
  scenario: ExecutableScenario,
  options: {
    executionConfiguration?: Record<string, unknown>;
    solver?: { name: string; version?: string };
    externalTools?: Array<{ name: string; version?: string }>;
    deterministic?: boolean;
  } = {},
): StateFreeze {
  const snapshot = captureSnapshot(state, 'manual', `freeze-${scenario.id}-${state.stateRevision}`);
  return {
    schema: STATE_FREEZE_SCHEMA,
    id: `freeze-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    createdAt: new Date().toISOString(),
    realityRevision: reality.revision,
    stateRevision: state.stateRevision,
    logicalTimeMs: state.logicalTimeMs,
    scenarioId: scenario.id,
    ...(scenario.version === undefined ? {} : { scenarioVersion: scenario.version }),
    snapshot,
    realityDigest: digest(reality),
    scenarioDigest: digest(scenario),
    ...(options.executionConfiguration ? { executionConfiguration: structuredClone(options.executionConfiguration) } : {}),
    ...(options.solver ? { solver: { ...options.solver } } : {}),
    ...(options.externalTools ? { externalTools: structuredClone(options.externalTools) } : {}),
    deterministic: options.deterministic ?? true,
  };
}

export function verifyStateFreeze(freeze: StateFreeze, reality: EngineeringReality, scenario: ExecutableScenario): boolean {
  return freeze.schema === STATE_FREEZE_SCHEMA
    && freeze.realityRevision === reality.revision
    && freeze.scenarioId === scenario.id
    && freeze.realityDigest === digest(reality)
    && freeze.scenarioDigest === digest(scenario)
    && freeze.snapshot.stateRevision === freeze.stateRevision
    && freeze.snapshot.logicalTimeMs === freeze.logicalTimeMs;
}
