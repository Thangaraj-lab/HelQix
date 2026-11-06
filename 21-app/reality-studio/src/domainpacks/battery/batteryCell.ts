import type { ModelOp, ModelRevision, Value } from '../../contracts';
import type { PackInfo } from './types';

export const OCV_RANGE_V: [number, number] = [2.5, 4.25];
export const RESISTANCE_RANGE_OHM: [number, number] = [0, 1];

const stated = (v: number, unit: string): Value => ({ v, unit, origin: 'user_stated', status: 'unverified' });

/** The part of a committed model that is a battery cell, or null. */
export function cellPartOf(revision: ModelRevision | null | undefined) {
  if (!revision) return null;
  return Object.values(revision.parts).find(p => p.behavior?.kind === 'battery_cell') ?? null;
}

/** The `part.setBehavior` op for the user's values. The ranges are the ones the pack accepts; the engine checks them again. An empty resistance is left
 *  out (the pack then runs with 0 Ohm: the calibrated value is offered to the user, never applied for them). */
export function buildCellOp(partId: string, pack: string, t: { ocv: string; resistance: string; current: string }): { op: ModelOp } | { error: string } {
  const num = (s: string) => (s.trim() === '' ? NaN : Number(s));
  const ocv = num(t.ocv);
  if (!Number.isFinite(ocv)) return { error: 'Open-circuit voltage must be a number.' };
  if (ocv < OCV_RANGE_V[0] || ocv > OCV_RANGE_V[1]) return { error: `Open-circuit voltage must be between ${OCV_RANGE_V[0]} and ${OCV_RANGE_V[1]} V.` };
  const current = num(t.current);
  if (!Number.isFinite(current)) return { error: 'Discharge current must be a number.' };
  let resistance: Value | undefined;
  if (t.resistance.trim() !== '') {
    const r = num(t.resistance);
    if (!Number.isFinite(r)) return { error: 'Contact resistance must be a number.' };
    if (r < RESISTANCE_RANGE_OHM[0] || r > RESISTANCE_RANGE_OHM[1]) return { error: `Contact resistance must be between ${RESISTANCE_RANGE_OHM[0]} and ${RESISTANCE_RANGE_OHM[1]} Ohm.` };
    resistance = stated(r, 'ohm');
  }
  return { op: { op: 'part.setBehavior', id: partId, behavior: { kind: 'battery_cell', pack, initialOcv: stated(ocv, 'V'), ...(resistance ? { contactResistance: resistance } : {}), load: { kind: 'constant_current', current: stated(current, 'A') } } } };
}

/** The label sentence that goes with every battery result. It states the label first and the published error after it. */
export function packNotice(p: PackInfo): string {
  const lo = Math.min(...p.heldOutRmseMv);
  const hi = Math.max(...p.heldOutRmseMv);
  const err = p.heldOutRmseMv.length ? `${lo.toFixed(1)} to ${hi.toFixed(1)} mV` : 'not published';
  const verdict = p.label === 'Verified' ? 'It met' : 'It did not meet';
  return `${p.label}: on held-out discharges the published error of this model is ${err}. ${verdict} the ${p.thresholdMv} mV limit, so its results are simulated, not validated.`;
}
