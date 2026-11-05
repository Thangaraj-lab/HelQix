import type { ModelRevision } from '../../contracts';

/** The part of a committed model that carries an FMU behaviour (the engine runs one FMU per model today), or null. */
export function fmuPartOf(revision: ModelRevision | null | undefined) {
  if (!revision) return null;
  return Object.values(revision.parts).find(p => p.behavior) ?? null;
}
