import type { EngineeringReality } from '../api';
import { validateState, type AuthoritativeEngineeringState } from './authoritativeState';
import type { EngineeringBranch, BranchResult } from './branchEngine';
import type { ComputeObservation } from './computeIntegration';
import type { ExecutableScenario } from './scenarioEngine';
import type { ReplayResult } from './replayEngine';
import type { StateFreeze } from './stateFreeze';

export const EVIDENCE_PACKAGE_SCHEMA = 'helqix.evidence-package.v1' as const;

export interface EvidenceArtifactRef {
  kind: 'reality' | 'scenario' | 'state' | 'freeze' | 'replay' | 'branch' | 'compute' | 'optimization';
  id: string;
  digest: string;
}

export interface EvidencePackage {
  schema: typeof EVIDENCE_PACKAGE_SCHEMA;
  id: string;
  createdAt: string;
  realityRevision: number;
  scenarioId: string;
  scenarioVersion?: number;
  authoritativeStateRevision: number;
  artifacts: EvidenceArtifactRef[];
  traceDigest: string;
  deterministic: boolean;
}

function canonicalize(value: unknown, volatileKeys = new Set<string>()): unknown {
  if (Array.isArray(value)) return value.map((item) => canonicalize(item, volatileKeys));
  if (value && typeof value === 'object') {
    return Object.keys(value as Record<string, unknown>)
      .filter((key) => !volatileKeys.has(key))
      .sort()
      .reduce<Record<string, unknown>>((result, key) => {
        result[key] = canonicalize((value as Record<string, unknown>)[key], volatileKeys);
        return result;
      }, {});
  }
  return value;
}

function digest(value: unknown): string {
  const serialized = JSON.stringify(canonicalize(value, new Set(['createdAt', 'updatedAt'])));
  let hash = 2166136261;
  for (let index = 0; index < serialized.length; index += 1) {
    hash ^= serialized.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

function sortedArtifacts(artifacts: EvidenceArtifactRef[]): EvidenceArtifactRef[] {
  return [...artifacts].sort((a, b) => `${a.kind}:${a.id}:${a.digest}`.localeCompare(`${b.kind}:${b.id}:${b.digest}`));
}

function assertUniqueArtifacts(artifacts: EvidenceArtifactRef[]): void {
  const identities = new Set<string>();
  for (const artifact of artifacts) {
    if (!artifact.id.trim() || !artifact.digest.trim()) throw new Error('Evidence artifacts require non-empty identities and digests.');
    const identity = `${artifact.kind}:${artifact.id}`;
    if (identities.has(identity)) throw new Error(`Duplicate evidence artifact: ${identity}.`);
    identities.add(identity);
  }
}

export function createEvidencePackage(input: {
  reality: EngineeringReality;
  scenario: ExecutableScenario;
  state: AuthoritativeEngineeringState;
  freeze?: StateFreeze;
  replay?: ReplayResult;
  branch?: { definition: EngineeringBranch; result: BranchResult };
  compute?: ComputeObservation[];
  optimization?: { id: string; digest: string };
  deterministic?: boolean;
}): EvidencePackage {
  const validation = validateState(input.state, input.reality);
  if (!validation.valid) throw new Error('Cannot create evidence package from invalid authoritative state.');
  if (input.state.realityRevision !== input.reality.revision) throw new Error('Evidence state does not belong to the current reality revision.');
  if (!input.scenario.id.trim()) throw new Error('Evidence package requires a scenario identity.');
  if (input.freeze) {
    if (input.freeze.realityRevision !== input.reality.revision || input.freeze.scenarioId !== input.scenario.id) throw new Error('State freeze does not belong to the evidence package reality or scenario.');
    if (input.freeze.stateRevision !== input.state.stateRevision || input.freeze.logicalTimeMs !== input.state.logicalTimeMs) throw new Error('State freeze does not match the authoritative evidence state.');
  }
  if (input.replay && (input.replay.realityRevision !== input.reality.revision || input.replay.scenarioId !== input.scenario.id)) throw new Error('Replay does not belong to the evidence package reality or scenario.');
  if (input.branch) {
    if (input.branch.result.status !== 'completed') throw new Error('Failed branch results cannot be attached as a completed evidence package.');
    if (input.branch.result.baseRealityRevision !== input.reality.revision || input.branch.result.baseStateRevision !== input.state.stateRevision) throw new Error('Branch result does not belong to the evidence package authoritative base.');
    if (input.branch.definition.id !== input.branch.result.branchId) throw new Error('Branch definition and result identities do not match.');
  }

  const artifacts: EvidenceArtifactRef[] = [
    { kind: 'reality', id: `reality-${input.reality.revision}`, digest: digest(input.reality) },
    { kind: 'scenario', id: input.scenario.id, digest: digest(input.scenario) },
    { kind: 'state', id: `state-${input.state.stateRevision}`, digest: digest(input.state) },
  ];
  if (input.freeze) artifacts.push({ kind: 'freeze', id: input.freeze.id, digest: input.freeze.freezeDigest });
  if (input.replay) artifacts.push({ kind: 'replay', id: `replay-${input.replay.traceDigest}`, digest: input.replay.traceDigest });
  if (input.branch) artifacts.push({ kind: 'branch', id: input.branch.definition.id, digest: digest(input.branch.result) });
  const computeIds = new Set<string>();
  for (const observation of input.compute ?? []) {
    if (computeIds.has(observation.executionId)) throw new Error(`Duplicate compute observation: ${observation.executionId}.`);
    computeIds.add(observation.executionId);
    artifacts.push({ kind: 'compute', id: observation.executionId, digest: digest({ output: observation.output, provenance: observation.provenance, logicalTimeMs: observation.logicalTimeMs, status: observation.status }) });
  }
  if (input.optimization) {
    if (!input.optimization.id.trim() || !input.optimization.digest.trim()) throw new Error('Optimization evidence requires an id and digest.');
    artifacts.push({ kind: 'optimization', id: input.optimization.id, digest: input.optimization.digest });
  }

  assertUniqueArtifacts(artifacts);
  const canonicalArtifacts = sortedArtifacts(artifacts);
  const traceDigest = digest({ schema: EVIDENCE_PACKAGE_SCHEMA, realityRevision: input.reality.revision, scenarioId: input.scenario.id, scenarioVersion: input.scenario.version, artifacts: canonicalArtifacts });
  const deterministic = (input.deterministic ?? true) && (input.freeze?.deterministic ?? true) && (input.replay?.deterministic ?? true);
  return {
    schema: EVIDENCE_PACKAGE_SCHEMA,
    id: `evidence-${traceDigest}`,
    createdAt: new Date().toISOString(),
    realityRevision: input.reality.revision,
    scenarioId: input.scenario.id,
    scenarioVersion: input.scenario.version,
    authoritativeStateRevision: input.state.stateRevision,
    artifacts: canonicalArtifacts,
    traceDigest,
    deterministic,
  };
}

export function verifyEvidencePackage(pkg: EvidencePackage, reality: EngineeringReality, scenario: ExecutableScenario, state: AuthoritativeEngineeringState): boolean {
  if (pkg.schema !== EVIDENCE_PACKAGE_SCHEMA || !pkg.id.startsWith('evidence-')) return false;
  if (pkg.realityRevision !== reality.revision || pkg.scenarioId !== scenario.id || pkg.scenarioVersion !== scenario.version) return false;
  if (pkg.authoritativeStateRevision !== state.stateRevision || state.realityRevision !== reality.revision) return false;
  try { assertUniqueArtifacts(pkg.artifacts); } catch { return false; }
  const realityArtifact = pkg.artifacts.find((artifact) => artifact.kind === 'reality');
  const scenarioArtifact = pkg.artifacts.find((artifact) => artifact.kind === 'scenario');
  const stateArtifact = pkg.artifacts.find((artifact) => artifact.kind === 'state');
  if (!realityArtifact || realityArtifact.digest !== digest(reality)) return false;
  if (!scenarioArtifact || scenarioArtifact.digest !== digest(scenario)) return false;
  if (!stateArtifact || stateArtifact.digest !== digest(state)) return false;
  if (pkg.artifacts.some((artifact) => !artifact.id.trim() || !artifact.digest.trim())) return false;
  const artifacts = sortedArtifacts(pkg.artifacts);
  const traceDigest = digest({ schema: EVIDENCE_PACKAGE_SCHEMA, realityRevision: pkg.realityRevision, scenarioId: pkg.scenarioId, scenarioVersion: pkg.scenarioVersion, artifacts });
  return pkg.traceDigest === traceDigest && pkg.id === `evidence-${traceDigest}`;
}
