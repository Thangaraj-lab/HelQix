import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, StateSnapshot } from './authoritativeState';
import { applyBranch, compareBranches, createBranch, type BranchChange, type BranchResult } from './branchEngine';

export const BRANCH_EXPERIMENT_SCHEMA = 'helqix.branch-experiment.v1' as const;

export interface BranchExperiment {
  schema: typeof BRANCH_EXPERIMENT_SCHEMA;
  id: string;
  baseSnapshotId: string;
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

export function runBranchExperiment(
  reality: EngineeringReality,
  baseState: AuthoritativeEngineeringState,
  baseSnapshot: StateSnapshot,
  candidates: Array<{ name: string; description?: string; changes: BranchChange[] }>,
): BranchExperiment {
  const baselineBranch = createBranch(reality, 'Baseline', 'Unmodified state at the experiment freeze.', [], baseState.stateRevision);
  const baseline = applyBranch(baselineBranch, reality, baseState);
  const results = candidates.map((candidate) => {
    const branch = createBranch(reality, candidate.name, candidate.description ?? '', candidate.changes, baseState.stateRevision);
    return applyBranch(branch, reality, baseState);
  });
  return {
    schema: BRANCH_EXPERIMENT_SCHEMA,
    id: `experiment-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    baseSnapshotId: baseSnapshot.id,
    baseline,
    candidates: results,
    createdAt: new Date().toISOString(),
  };
}

export function compareExperiment(experiment: BranchExperiment): BranchComparison[] {
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
  return experiment.candidates
    .filter((candidate) => candidate.status === 'completed')
    .map((candidate) => ({ candidate, score: score(experiment.baseline.resultingState, candidate.resultingState) }))
    .sort((a, b) => b.score - a.score)[0]?.candidate;
}
