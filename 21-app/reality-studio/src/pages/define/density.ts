export const DENSITY_KEY = 'density_kg_m3'; // the part parameter the engine reads to find mass (volume x density)

/** Names of the parts that state no usable density (missing, zero, negative or not a number). */
/** A part with an FMU behaviour is not counted: its FMU is what runs, not a lumped body from volume and density. */
export function partsMissingDensity(parts: Iterable<{ name: string; params?: Record<string, { v: number }>; behavior?: unknown }>): string[] {
  const out: string[] = [];
  for (const p of parts) {
    if (p.behavior) continue;
    const d = p.params?.[DENSITY_KEY]?.v;
    if (!(typeof d === 'number' && Number.isFinite(d) && d > 0)) out.push(p.name);
  }
  return out;
}
