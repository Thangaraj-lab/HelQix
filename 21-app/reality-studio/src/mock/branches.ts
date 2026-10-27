// MOCK branch result. The mock engine has no solver, so an intervention branch is a scripted
// scaling of the original run's winding temperature rise: more cooling lowers the rise, more load
// raises it. It demonstrates the branch and compare flow with labelled data. It is not physics and
// must never be shown as a computed result.

const AMBIENT_C = 25;

export interface BranchParams {
  /** Added cooling, percent (0 to 40) */
  coolingPct: number;
  /** Load change, percent (-30 to 0) */
  loadPct: number;
}

export function scriptedBranchTemperature(original: [number, number][], p: BranchParams): [number, number][] {
  const factor = (1 + p.loadPct / 100) / (1 + p.coolingPct / 100);
  return original.map(([t, v]) => [t, +(AMBIENT_C + (v - AMBIENT_C) * factor).toFixed(1)] as [number, number]);
}
