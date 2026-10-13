import type { EngineeringReality } from '../api';
import { captureSnapshot, validateState, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
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
  scenarioVersion?: number;
  snapshot: StateSnapshot;
  realityDigest: string;
  scenarioDigest: string;
  stateDigest: string;
  freezeDigest: string;
  executionConfiguration?: Record<string, unknown>;
  solver?: { name: string; version?: string };
  externalTools?: Array<{ name: string; version?: string }>;
  deterministic: boolean;
}

function canonicalize(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === 'object') {
    return Object.keys(value as Record<string, unknown>)
      .filter((key) => key !== 'createdAt' && key !== 'updatedAt')
      .sort()
      .reduce<Record<string, unknown>>((result, key) => {
        result[key] = canonicalize((value as Record<string, unknown>)[key]);
        return result;
      }, {});
  }
  return value;
}

function digest(value: unknown): string {
  const serialized = JSON.stringify(canonicalize(value));
  let hash = 2166136261;
  for (let index = 0; index < serialized.length; index += 1) { hash ^= serialized.charCodeAt(index); hash = Math.imul(hash, 16777619); }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

function freezeDigestInput(freeze: Omit<StateFreeze, 'freezeDigest'>): unknown {
  return {
    schema: freeze.schema,
    id: freeze.id,
    realityRevision: freeze.realityRevision,
    stateRevision: freeze.stateRevision,
    logicalTimeMs: freeze.logicalTimeMs,
    scenarioId: freeze.scenarioId,
    scenarioVersion: freeze.scenarioVersion,
    realityDigest: freeze.realityDigest,
    scenarioDigest: freeze.scenarioDigest,
    stateDigest: freeze.stateDigest,
    executionConfiguration: freeze.executionConfiguration,
    solver: freeze.solver,
    externalTools: freeze.externalTools,
    deterministic: freeze.deterministic,
  };
}

function freezeIdentityInput(freeze: Omit<StateFreeze, 'id' | 'createdAt' | 'freezeDigest'>): unknown {
  return {
    schema: freeze.schema,
    realityRevision: freeze.realityRevision,
    stateRevision: freeze.stateRevision,
    logicalTimeMs: freeze.logicalTimeMs,
    scenarioId: freeze.scenarioId,
    scenarioVersion: freeze.scenarioVersion,
    realityDigest: freeze.realityDigest,
    scenarioDigest: freeze.scenarioDigest,
    stateDigest: freeze.stateDigest,
    executionConfiguration: freeze.executionConfiguration,
    solver: freeze.solver,
    externalTools: freeze.externalTools,
    deterministic: freeze.deterministic,
  };
}

export function createStateFreeze(reality: EngineeringReality, state: AuthoritativeEngineeringState, scenario: ExecutableScenario, options: { executionConfiguration?: Record<string, unknown>; solver?: { name: string; version?: string }; externalTools?: Array<{ name: string; version?: string }>; deterministic?: boolean } = {}): StateFreeze {
  const validation = validateState(state, reality);
  if (!validation.valid) throw new Error(`Cannot freeze invalid authoritative state: ${validation.issues.filter((issue) => issue.severity === 'error').map((issue) => issue.message).join('; ')}`);
  if (state.realityRevision !== reality.revision) throw new Error('Cannot freeze state from a different reality revision.');
  if (state.logicalTimeMs < 0 || !Number.isFinite(state.logicalTimeMs)) throw new Error('Cannot freeze state with invalid logical time.');
  if (!scenario.id.trim()) throw new Error('Cannot freeze state without a scenario identity.');
  const snapshot = captureSnapshot(state, 'manual', `freeze-${scenario.id}-${state.stateRevision}`);
  const baseWithoutIdentity = {
    schema: STATE_FREEZE_SCHEMA,
    realityRevision: reality.revision,
    stateRevision: state.stateRevision,
    logicalTimeMs: state.logicalTimeMs,
    scenarioId: scenario.id,
    scenarioVersion: scenario.version,
    snapshot,
    realityDigest: digest(reality),
    scenarioDigest: digest(scenario),
    stateDigest: digest(snapshot.state),
    ...(options.executionConfiguration ? { executionConfiguration: structuredClone(options.executionConfiguration) } : {}),
    ...(options.solver ? { solver: { ...options.solver } } : {}),
    ...(options.externalTools ? { externalTools: structuredClone(options.externalTools) } : {}),
    deterministic: options.deterministic ?? true,
  } satisfies Omit<StateFreeze, 'id' | 'createdAt' | 'freezeDigest'>;
  const identityDigest = digest(freezeIdentityInput(baseWithoutIdentity));
  const base = {
    ...baseWithoutIdentity,
    id: `freeze-${identityDigest}`,
    createdAt: new Date().toISOString(),
  } satisfies Omit<StateFreeze, 'freezeDigest'>;
  return { ...base, freezeDigest: digest(freezeDigestInput(base)) };
}

export function verifyStateFreeze(freeze: StateFreeze, reality: EngineeringReality, scenario: ExecutableScenario): boolean {
  if (freeze.schema !== STATE_FREEZE_SCHEMA) return false;
  if (freeze.realityRevision !== reality.revision || freeze.scenarioId !== scenario.id || freeze.scenarioVersion !== scenario.version) return false;
  if (freeze.realityDigest !== digest(reality) || freeze.scenarioDigest !== digest(scenario)) return false;
  if (freeze.snapshot.schema !== 'helqix.state-snapshot.v1') return false;
  if (freeze.snapshot.stateRevision !== freeze.stateRevision || freeze.snapshot.logicalTimeMs !== freeze.logicalTimeMs) return false;
  if (freeze.snapshot.state.realityRevision !== freeze.realityRevision) return false;
  if (freeze.snapshot.state.stateRevision !== freeze.stateRevision || freeze.snapshot.state.logicalTimeMs !== freeze.logicalTimeMs) return false;
  if (freeze.stateDigest !== digest(freeze.snapshot.state)) return false;
  const expectedId = `freeze-${digest(freezeIdentityInput(freeze))}`;
  if (freeze.id !== expectedId) return false;
  if (freeze.freezeDigest !== digest(freezeDigestInput(freeze))) return false;
  return validateState(freeze.snapshot.state, reality).valid;
}
