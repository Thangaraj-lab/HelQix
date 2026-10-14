import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, StateSnapshot } from './authoritativeState';
import { applyBranch, compareBranches, createBranch, type BranchChange, type BranchResult } from './branchEngine';

export const BRANCH_EXPERIMENT_SCHEMA = 'helqix.branch-experiment.v1' as const;

export interface BranchExperiment {
  schema: typeof BRANCH_EXPERIMENT_SCHEMA;
  id: string;
  baseSnapshotId: string;
  baseStateDigest: string;
  baseline: BranchResult;
  candidates: BranchResult[];
  createdAt: string;
}

export interface BranchComparison {
  branchId: string;
  status: BranchResult['status'];
  changedProperties: ReturnType<typeof compareBranches>['changedProperties'];
  stateRevisionDelta: number;
  logicalTimeDeltaMs: number;
}

function digest(value: unknown): string {
  const serialized = JSON.stringify(value);
  let hash = 2166136261;
  for (let index = 0; index < serialized.length; index += 1) { hash ^= serialized.charCodeAt(index); hash = Math.imul(hash, 16777619); }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

export function runBranchExperiment(
  reality: EngineeringReality,
  baseState: AuthoritativeEngineeringState,
  baseSnapshot: StateSnapshot,
  candidates: Array<{ name: string; description?: string; changes: BranchChange[] }>,
): BranchExperiment {
  if (baseSnapshot.schema !== 'helqix.state-snapshot.v1') throw new Error('Unsupported experiment snapshot schema.');
  if (baseSnapshot.stateRevision !== baseState.stateRevision || baseSnapshot.logicalTimeMs !== baseState.logicalTimeMs) throw new Error('Branch experiment snapshot does not match the supplied authoritative state.');
  if (baseSnapshot.state.realityRevision !== reality.revision || baseState.realityRevision !== reality.revision) throw new Error('Branch experiment requires an authoritative state from the current reality revision.');
  if (digest(baseSnapshot.state) !== digest(baseState)) throw new Error('Branch experiment snapshot state digest does not match the authoritative state.');
  if (!baseSnapshot.id.trim()) throw new Error('Branch experiment requires an exact base snapshot identity.');
  const baselineBranch = createBranch(reality, 'Baseline', 'Unmodified state at the experiment freeze.', [], baseState.stateRevision);
  const baseline = applyBranch(baselineBranch, reality, baseState);
  const results = candidates.map((candidate) => {
    if (!candidate.name.trim()) throw new Error('Branch experiment candidates require names.');
    const branch = createBranch(reality, candidate.name, candidate.description ?? '', candidate.changes, baseState.stateRevision);
    return applyBranch(branch, reality, baseState);
  });
  if (baseline.status !== 'completed') throw new Error(`Baseline branch failed: ${baseline.errors.join('; ')}`);
  return {
    schema: BRANCH_EXPERIMENT_SCHEMA,
    id: `experiment-${baseSnapshot.id}-${baseState.stateRevision}`,
    baseSnapshotId: baseSnapshot.id,
    baseStateDigest: digest(baseState),
    baseline,
    candidates: results,
    createdAt: new Date().toISOString(),
  };
}

export function compareExperiment(experiment: BranchExperiment): BranchComparison[] {
  if (experiment.schema !== BRANCH_EXPERIMENT_SCHEMA) throw new Error('Unsupported branch experiment schema.');
  if (!experiment.baseSnapshotId.trim() || !experiment.baseStateDigest.trim()) throw new Error('Branch experiment provenance is incomplete.');
  return experiment.candidates.map((candidate) => {
    const comparison = compareBranches(experiment.baseline, candidate);
    return {
      branchId: candidate.branchId,
      status: candidate.status,
      changedProperties: comparison.changedProperties,
      stateRevisionDelta: comparison.stateRevisionDelta,
      logicalTimeDeltaMs: comparison.logicalTimeDeltaMs,
    };
  });
}

export function bestExperimentCandidate(
  experiment: BranchExperiment,
  score: (baseline: AuthoritativeEngineeringState, candidate: AuthoritativeEngineeringState) => number,
): BranchResult | undefined {
  if (experiment.schema !== BRANCH_EXPERIMENT_SCHEMA) throw new Error('Unsupported branch experiment schema.');
  const ranked = experiment.candidates
    .filter((candidate) => candidate.status === 'completed')
    .map((candidate) => {
      const candidateScore = score(experiment.baseline.resultingState, candidate.resultingState);
      if (!Number.isFinite(candidateScore)) throw new Error(`Branch ${candidate.branchId} produced a non-finite score.`);
      return { candidate, score: candidateScore };
    })
    .sort((a, b) => b.score - a.score || a.candidate.branchId.localeCompare(b.candidate.branchId));
  return ranked[0]?.candidate;
}
