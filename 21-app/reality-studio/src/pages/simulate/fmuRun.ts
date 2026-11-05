import type { ModelRevision } from '../../contracts';

/** The part of a committed model that carries an FMU behaviour (the engine runs one FMU per model today), or null. */
export function fmuPartOf(revision: ModelRevision | null | undefined) {
  if (!revision) return null;
  const found = Object.values(revision.parts).find(p => p.behavior?.kind === 'fmu');
  return found && found.behavior?.kind === 'fmu' ? { ...found, behavior: found.behavior } : null;
}
