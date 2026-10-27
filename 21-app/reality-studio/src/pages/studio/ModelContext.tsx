import React, { createContext, useContext, useMemo } from 'react';
import { Binding, ModelRevision } from '../../contracts';
import { Requirement, requirementsOf, violates } from '../../app/requirements';
import { SceneConnection, ScenePart, Series } from '../../scene/types';
import { RunData, RunSelection, useRunSelection } from '../simulate/useRun';
import { revisionToScene } from '../model/useRevision';

// Everything the stages share about the committed model: its parts and bindings, its requirements,
// and the stored run. One place reads the revision and the run, so every stage and the 3D canvas
// agree about which part is bound to what and which requirement failed.

export interface FailureInfo {
  /** The part whose bound variable broke the requirement */
  partId: string | null;
  partName: string;
  t: number;
  requirement: Requirement;
  value: number;
}

export interface ModelData {
  projectId: string;
  revision: ModelRevision | null;
  loading: boolean;
  parts: ScenePart[];
  connections: SceneConnection[];
  bindings: Binding[];
  requirements: Requirement[];
  /** The stored run, or null when the model has no bindings (nothing in the mock engine to run) */
  run: RunData | null;
  /** The stored runs on offer and which one is selected (the URL's run) */
  runChoice: Pick<RunSelection, 'runs' | 'runId' | 'runsError'>;
  series: Series;
  failure: FailureInfo | null;
}

const Ctx = createContext<ModelData | null>(null);

export const ModelProvider: React.FC<{ projectId: string; revision: ModelRevision | null; loading: boolean; children: React.ReactNode }> = ({ projectId, revision, loading, children }) => {
  const selection = useRunSelection();
  const rawRun = selection.run;
  const value = useMemo<ModelData>(() => {
    const bindings = revision ? Object.values(revision.bindings) : [];
    const requirements = requirementsOf(revision);
    const run = bindings.length > 0 ? rawRun : null;
    const series: Series = {};
    if (run) for (const key of Object.keys(run.frames[0]?.values ?? {})) series[key] = run.series(key);

    let failure: FailureInfo | null = null;
    if (run) {
      for (const r of requirements) {
        const hit = (series[r.variable] ?? []).find(p => violates(r, p[1]));
        if (hit && (!failure || hit[0] < failure.t)) {
          const node = bindings.find(b => b.variable === r.variable)?.node ?? null;
          failure = { partId: node, partName: node ? revision!.parts[node]?.name ?? node : 'Unknown part', t: hit[0], requirement: r, value: hit[1] };
        }
      }
    }
    return {
      projectId,
      revision,
      loading,
      parts: revisionToScene(revision) ?? [],
      connections: revision ? Object.values(revision.connections).map(c => ({ id: c.id, from: c.from, to: c.to, kind: c.kind })) : [],
      bindings,
      requirements,
      run,
      runChoice: { runs: selection.runs, runId: selection.runId, runsError: selection.runsError },
      series,
      failure
    };
  }, [projectId, revision, loading, rawRun, selection.runs, selection.runId, selection.runsError]);

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
};

export function useModelData(): ModelData {
  const v = useContext(Ctx);
  if (!v) throw new Error('useModelData must be used inside ModelProvider');
  return v;
}
