import type { EngineeringReality } from '../api';
import { captureSnapshot, type AuthoritativeEngineeringState, type StateSnapshot } from './authoritativeState';
import type { ExecutableScenario } from './scenarioEngine';

export const STATE_FREEZE_SCHEMA = 'helqix.state-freeze.v1' as const;

export interface ExecutionConfigFingerprint {
  solver?: string;
  solverVersion?: string;
  timestepMs?: number;
  seed?: number;
  worker?: string;
  extra?: Record<string, string | number | boolean>;
}

export interface StateFreezeManifest {
  schema: typeof STATE_FREEZE_SCHEMA;
  id: string;
  capturedAt: string;
  realityRevision: number;
  stateRevision: number;
  logicalTimeMs: number;
  scenario: {
    id: string;
    version: number;
    name: string;
    actionCount: number;
  };
  snapshot: StateSnapshot;
  realityDigest: string;
  scenarioDigest: string;
  executionConfig: ExecutionConfigFingerprint;
  externalVersions: Record<string, string>;
  deterministic: boolean;
}

function stable(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stable).join(',')}]`;
  return `{${Object.keys(value as Record<string, unknown>).sort().map((key) => `${JSON.stringify(key)}:${stable((value as Record<string, unknown>)[key])}`).join(',')}}`;
}

export function stableDigest(value: unknown): string {
  // Portable deterministic digest for manifests. Cryptographic hashing belongs at the persistence boundary.
  let hash = 2166136261;
  for (const char of stable(value)) {
    hash ^= char.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return `fnv1a32-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

export function createStateFreezeManifest(
  reality: EngineeringReality,
  scenario: ExecutableScenario,
  state: AuthoritativeEngineeringState,
  options: { executionConfig?: ExecutionConfigFingerprint; externalVersions?: Record<string, string>; deterministic?: boolean } = {},
): StateFreezeManifest {
  const snapshot = captureSnapshot(state, 'manual', `freeze-${scenario.id}-${state.stateRevision}`);
  return {
    schema: STATE_FREEZE_SCHEMA,
    id: `freeze-${reality.revision}-${state.stateRevision}-${Date.now()}`,
    capturedAt: snapshot.capturedAt,
    realityRevision: reality.revision,
    stateRevision: state.stateRevision,
    logicalTimeMs: state.logicalTimeMs,
    scenario: { id: scenario.id, version: scenario.version, name: scenario.name, actionCount: scenario.actions.length },
    snapshot,
    realityDigest: stableDigest(reality),
    scenarioDigest: stableDigest(scenario),
    executionConfig: options.executionConfig ?? {},
    externalVersions: options.externalVersions ?? {},
    deterministic: options.deterministic ?? true,
  };
}

export function verifyStateFreezeManifest(manifest: StateFreezeManifest, reality: EngineeringReality, scenario: ExecutableScenario): string[] {
  const errors: string[] = [];
  if (manifest.schema !== STATE_FREEZE_SCHEMA) errors.push(`Expected ${STATE_FREEZE_SCHEMA}.`);
  if (manifest.realityRevision !== reality.revision) errors.push(`Reality revision mismatch: freeze ${manifest.realityRevision}, current ${reality.revision}.`);
  if (manifest.scenario.id !== scenario.id || manifest.scenario.version !== scenario.version) errors.push('Scenario identity/version does not match the freeze.');
  if (manifest.realityDigest !== stableDigest(reality)) errors.push('Engineering reality digest does not match the freeze.');
  if (manifest.scenarioDigest !== stableDigest(scenario)) errors.push('Scenario digest does not match the freeze.');
  if (manifest.snapshot.stateRevision !== manifest.stateRevision || manifest.snapshot.logicalTimeMs !== manifest.logicalTimeMs) errors.push('Snapshot metadata does not match the freeze manifest.');
  return errors;
}
