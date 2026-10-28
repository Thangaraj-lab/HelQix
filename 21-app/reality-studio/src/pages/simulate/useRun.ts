import { useEffect, useMemo, useState } from 'react';
import { RunFrame, StoredRunInfo } from '../../contracts';
import { engine } from '../../engine';
import { useLocation } from '../../app/router';

// Stored runs, loaded through the engine interface and shared by Simulate, Understand, Improve and
// Verify. Frames are read-only; scrubbing reads the stored frame at t, so the same t always gives
// the same values. The mock engine has one scripted run; the real engine has whatever its frame
// store holds. A run is not tied to a model: it is data the user binds model parts to.

const cache = new Map<string, RunFrame[]>();
const pending = new Map<string, Promise<RunFrame[]>>();

function load(runId: string): Promise<RunFrame[]> {
  const hit = cache.get(runId);
  if (hit) return Promise.resolve(hit);
  let p = pending.get(runId);
  if (!p) {
    p = (async () => {
      const out: RunFrame[] = [];
      for await (const f of engine.runFrames(runId)) out.push(f);
      cache.set(runId, out);
      return out;
    })().finally(() => pending.delete(runId));
    pending.set(runId, p);
  }
  return p;
}

export interface RunData {
  id: string;
  frames: RunFrame[];
  duration: number;
  at: (t: number) => RunFrame | null;
  series: (key: string) => [number, number][];
}

function useStoredRuns(): { runs: StoredRunInfo[] | null; error: string | null } {
  const [runs, setRuns] = useState<StoredRunInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    const load = () => engine.listStoredRuns().then(r => live && setRuns(r)).catch(e => { if (live) { setRuns([]); setError(e instanceof Error ? e.message : String(e)); } });
    void load();
    runsChanged.addEventListener('change', load);
    return () => { live = false; runsChanged.removeEventListener('change', load); };
  }, []);
  return { runs, error };
}

/** Fired after a run is started so every list of stored runs reloads. */
export const runsChanged = new EventTarget();

export interface RunSelection {
  /** Stored runs the engine offers (null while loading) */
  runs: StoredRunInfo[] | null;
  runsError: string | null;
  runId: string | null;
  run: RunData | null;
}

/** The run named by the URL's run, else the engine's first stored run. */
export function useRunSelection(): RunSelection {
  const { query } = useLocation();
  const { runs, error } = useStoredRuns();
  const wanted = query.get('run');
  const runId = runs ? (runs.find(r => r.id === wanted)?.id ?? runs[0]?.id ?? null) : null;

  const [frames, setFrames] = useState<{ id: string; f: RunFrame[] } | null>(null);
  useEffect(() => {
    if (!runId) { setFrames(null); return; }
    let live = true;
    load(runId).then(f => live && setFrames({ id: runId, f })).catch(() => live && setFrames({ id: runId, f: [] }));
    return () => { live = false; };
  }, [runId]);

  // One stable object per frame set, so effects that depend on the run do not loop.
  const run = useMemo<RunData | null>(() => {
    if (!frames || frames.id !== runId || frames.f.length === 0) return null;
    const f = frames.f;
    const first = f[0].t;
    const last = f[f.length - 1].t;
    return {
      id: frames.id,
      frames: f,
      duration: last,
      at: t => f[Math.max(0, Math.min(f.length - 1, Math.round(((t - first) / ((last - first) || 1)) * (f.length - 1))))] ?? null,
      series: key => f.filter(x => x.values[key]).map(x => [x.t, x.values[key].v] as [number, number])
    };
  }, [frames, runId]);

  return { runs, runsError: error, runId, run };
}

/** The selected run's data only (most stages need nothing else). */
export function useRun(): RunData | null {
  return useRunSelection().run;
}
