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

function digest(value: unknown): string {
  const serialized = JSON.stringify(value);
  let hash = 2166136261;
  for (let index = 0; index < serialized.length; index += 1) { hash ^= serialized.charCodeAt(index); hash = Math.imul(hash, 16777619); }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

function sortedArtifacts(artifacts: EvidenceArtifactRef[]): EvidenceArtifactRef[] {
  return [...artifacts].sort((a, b) => `${a.kind}:${a.id}`.localeCompare(`${b.kind}:${b.id}`));
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
  if (input.freeze && (input.freeze.realityRevision !== input.reality.revision || input.freeze.scenarioId !== input.scenario.id)) throw new Error('State freeze does not belong to the evidence package reality or scenario.');
  if (input.replay && (input.replay.realityRevision !== input.reality.revision || input.replay.scenarioId !== input.scenario.id)) throw new Error('Replay does not belong to the evidence package reality or scenario.');
  if (input.branch && input.branch.result.status !== 'completed') throw new Error('Failed branch results cannot be attached as a completed evidence package.');

  const artifacts: EvidenceArtifactRef[] = [
    { kind: 'reality', id: `reality-${input.reality.revision}`, digest: digest(input.reality) },
    { kind: 'scenario', id: input.scenario.id, digest: digest(input.scenario) },
    { kind: 'state', id: `state-${input.state.stateRevision}`, digest: digest(input.state) },
  ];
  if (input.freeze) artifacts.push({ kind: 'freeze', id: input.freeze.id, digest: input.freeze.freezeDigest });
  if (input.replay) artifacts.push({ kind: 'replay', id: `replay-${input.replay.traceDigest}`, digest: input.replay.traceDigest });
  if (input.branch) artifacts.push({ kind: 'branch', id: input.branch.definition.id, digest: digest(input.branch.result) });
  for (const observation of input.compute ?? []) artifacts.push({ kind: 'compute', id: observation.executionId, digest: digest({ output: observation.output, provenance: observation.provenance, logicalTimeMs: observation.logicalTimeMs }) });
  if (input.optimization) artifacts.push({ kind: 'optimization', id: input.optimization.id, digest: input.optimization.digest });

  const canonicalArtifacts = sortedArtifacts(artifacts);
  const traceDigest = digest({ schema: EVIDENCE_PACKAGE_SCHEMA, realityRevision: input.reality.revision, scenarioId: input.scenario.id, scenarioVersion: input.scenario.version, artifacts: canonicalArtifacts });
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
    deterministic: input.deterministic ?? true,
  };
}

export function verifyEvidencePackage(pkg: EvidencePackage, reality: EngineeringReality, scenario: ExecutableScenario, state: AuthoritativeEngineeringState): boolean {
  if (pkg.schema !== EVIDENCE_PACKAGE_SCHEMA || !pkg.id.startsWith('evidence-')) return false;
  if (pkg.realityRevision !== reality.revision || pkg.scenarioId !== scenario.id || pkg.scenarioVersion !== scenario.version) return false;
  if (pkg.authoritativeStateRevision !== state.stateRevision || state.realityRevision !== reality.revision) return false;
  const realityArtifact = pkg.artifacts.find((artifact) => artifact.kind === 'reality');
  const scenarioArtifact = pkg.artifacts.find((artifact) => artifact.kind === 'scenario');
  const stateArtifact = pkg.artifacts.find((artifact) => artifact.kind === 'state');
  if (!realityArtifact || realityArtifact.digest !== digest(reality)) return false;
  if (!scenarioArtifact || scenarioArtifact.digest !== digest(scenario)) return false;
  if (!stateArtifact || stateArtifact.digest !== digest(state)) return false;
  const artifacts = sortedArtifacts(pkg.artifacts);
  const traceDigest = digest({ schema: EVIDENCE_PACKAGE_SCHEMA, realityRevision: pkg.realityRevision, scenarioId: pkg.scenarioId, scenarioVersion: pkg.scenarioVersion, artifacts });
  return pkg.traceDigest === traceDigest && pkg.id === `evidence-${traceDigest}`;
}
