import type { EngineMode } from '../../engine';

/** Which stored run a model shows. The real engine's runs are data it produced, shown whatever the bindings are
 *  (bindings only decide what moves in the 3D view). The mock engine has one scripted run that means nothing
 *  to a model without bindings, so it is hidden there. */
export function runForModel<T>(rawRun: T | null, bindingCount: number, mode: EngineMode): T | null {
  if (!rawRun) return null;
  return mode === 'real' || bindingCount > 0 ? rawRun : null;
}
