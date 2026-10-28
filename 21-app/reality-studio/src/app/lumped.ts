import { ModelRevision, Value } from '../contracts';

// The one honest way the engine can run a Studio project today: treat the assembly as a single lumped body
// (the engine's model is a point mass with air drag). Mass is the sum of volume x density over the parts, and
// the frontal area is the footprint of the parts' bounding box seen from above. Nothing is invented: a part with
// no stated density, or with geometry that has no volume, makes the model unrunnable and is named.

export const DENSITY_KEY = 'density_kg_m3';

export type Lumped =
  | { ok: true; massKg: number; areaM2: number; parts: number }
  | { ok: false; missing: { id: string; name: string; reason: string }[] };

const mm = (v: Value | undefined) => (v && Number.isFinite(v.v) && v.v > 0 ? v.v / 1000 : null);

/** Volume in m3 of a primitive from its dimensions (mm), or null when it cannot be known. */
export function volumeM3(shape: string, dims: Value[]): number | null {
  const a = mm(dims[0]), b = mm(dims[1]), c = mm(dims[2]);
  if (shape === 'sphere') return a === null ? null : (4 / 3) * Math.PI * (a / 2) ** 3;
  if (shape === 'cylinder') return a === null || b === null ? null : Math.PI * (a / 2) ** 2 * b; // diameter, height
  if (shape === 'box' || shape === 'plate') return a === null || b === null || c === null ? null : a * b * c;
  return null;
}

/** Half-extents (m) along x, y, z, as the scene draws the part. */
function extents(shape: string, dims: Value[]): [number, number, number] | null {
  const a = mm(dims[0]), b = mm(dims[1]), c = mm(dims[2]);
  if (a === null) return null;
  if (shape === 'sphere') return [a / 2, a / 2, a / 2];
  if (b === null) return null;
  if (shape === 'cylinder') return [a / 2, b / 2, a / 2];
  if (c === null) return null;
  return [a / 2, b / 2, c / 2];
}

export function lumpedModel(rev: ModelRevision): Lumped {
  const missing: { id: string; name: string; reason: string }[] = [];
  let mass = 0;
  let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
  const all = Object.values(rev.parts);
  for (const p of all) {
    if (p.geometry.kind !== 'primitive') { missing.push({ id: p.id, name: p.name, reason: 'imported geometry has no known volume' }); continue; }
    const density = p.params?.[DENSITY_KEY];
    if (!density || !Number.isFinite(density.v) || density.v <= 0) { missing.push({ id: p.id, name: p.name, reason: 'no density stated' }); continue; }
    const vol = volumeM3(p.geometry.shape, p.geometry.dims);
    const ext = extents(p.geometry.shape, p.geometry.dims);
    if (vol === null || !ext) { missing.push({ id: p.id, name: p.name, reason: 'dimensions are missing' }); continue; }
    mass += vol * density.v;
    const [x, , z] = p.transform.pos.map(q => q.v);
    minX = Math.min(minX, x - ext[0]); maxX = Math.max(maxX, x + ext[0]);
    minZ = Math.min(minZ, z - ext[2]); maxZ = Math.max(maxZ, z + ext[2]);
  }
  if (all.length === 0) return { ok: false, missing: [{ id: '', name: 'This model', reason: 'has no parts' }] };
  if (missing.length) return { ok: false, missing };
  return { ok: true, massKg: mass, areaM2: (maxX - minX) * (maxZ - minZ), parts: all.length };
}
