import { useEffect, useMemo, useState } from 'react';
import { RunFrame } from '../../contracts';
import { engine } from '../../engine';
import { MOCK_RUN_ID } from '../../mock/scenarios';

// The scripted run, loaded once through the engine interface and shared by Simulate, Understand,
// Improve and Verify. Frames are read-only; scrubbing reads the stored frame at t, so the same t
// always gives the same values.

let cache: RunFrame[] | null = null;
let pending: Promise<RunFrame[]> | null = null;

function load(): Promise<RunFrame[]> {
  if (cache) return Promise.resolve(cache);
  if (!pending) {
    pending = (async () => {
      const out: RunFrame[] = [];
      for await (const f of engine.runFrames(MOCK_RUN_ID)) out.push(f);
      cache = out;
      return out;
    })();
  }
  return pending;
}

export interface RunData {
  frames: RunFrame[];
  duration: number;
  at: (t: number) => RunFrame | null;
  series: (key: string) => [number, number][];
}

export function useRun(): RunData | null {
  const [frames, setFrames] = useState<RunFrame[] | null>(cache);
  useEffect(() => {
    let live = true;
    load().then(f => live && setFrames(f)).catch(() => { pending = null; if (live) setFrames([]); });
    return () => { live = false; };
  }, []);

  // One stable object per frame set, so effects that depend on the run do not loop.
  return useMemo<RunData | null>(() => {
    if (!frames || frames.length === 0) return null;
    const last = frames[frames.length - 1].t;
    return {
      frames,
      duration: last,
      at: t => frames[Math.max(0, Math.min(frames.length - 1, Math.round((t / last) * (frames.length - 1))))] ?? null,
      series: key => frames.filter(f => f.values[key]).map(f => [f.t, f.values[key].v] as [number, number])
    };
  }, [frames]);
}
