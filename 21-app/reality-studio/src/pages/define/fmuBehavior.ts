import type { FmuDescription, FmuVariable, ModelOp, Value } from '../../contracts';

// What an FMU offers a part, and the `part.setBehavior` op built from the user's choices. Pure functions: the checks the user sees here
// are only a convenience; the model build and the FMI tool check the same things again (a unit the registry does not know is a blocking issue,
// a parameter that measures another dimension is refused before the run).

export interface ParameterCandidate { name: string; unit: string; start: number | null; description: string }
export interface OutputCandidate { name: string; unit: string; isOutput: boolean; description: string }

/** The unit symbol the HELQIX registry uses for what the FMU declares: no declared unit is dimensionless, `one`. */
export const paramUnit = (v: Pick<FmuVariable, 'unit'>): string => (v.unit === '' ? 'one' : v.unit);

/** Real parameters the user can set (fixed or tunable). Constants and non-real parameters are not offered. */
export function parameterCandidates(d: FmuDescription): ParameterCandidate[] {
  return d.variables
    .filter(v => v.causality === 'parameter' && v.type === 'Real' && (v.variability === 'fixed' || v.variability === 'tunable'))
    .map(v => ({ name: v.name, unit: paramUnit(v), start: v.start, description: v.description }));
}

/** Real variables that can be recorded: outputs first, then the other continuous variables. Time, parameters and constants are not offered. */
export function outputCandidates(d: FmuDescription): OutputCandidate[] {
  const real = d.variables.filter(v => v.type === 'Real' && v.causality !== 'independent' && v.causality !== 'parameter' && v.variability !== 'constant');
  const rank = (v: FmuVariable) => (v.causality === 'output' ? 0 : 1);
  return real.map((v, i) => ({ v, i })).sort((a, b) => rank(a.v) - rank(b.v) || a.i - b.i)
    .map(({ v }) => ({ name: v.name, unit: paramUnit(v), isOutput: v.causality === 'output', description: v.description }));
}

/** The op for the user's choices. `typed` holds only what the user typed per parameter; an empty text or the FMU's own start value changes nothing. */
export function buildBehaviorOp(partId: string, sha256: string, d: FmuDescription, typed: Record<string, string>, outputs: string[]): { op: ModelOp } | { error: string } {
  if (outputs.length === 0) return { error: 'Choose at least one output to record.' };
  const known = new Map(d.variables.map(v => [v.name, v]));
  for (const o of outputs) if (!known.has(o)) return { error: `The FMU has no variable ${o} to record.` };
  const settable = new Map(parameterCandidates(d).map(p => [p.name, p]));
  const parameters: Record<string, Value> = {};
  for (const [name, text] of Object.entries(typed)) {
    const p = settable.get(name);
    if (!p) return { error: `The FMU has no settable parameter ${name}.` };
    if (text.trim() === '') continue;
    const n = Number(text);
    if (!Number.isFinite(n)) return { error: `${name} must be a number.` };
    if (p.start !== null && n === p.start) continue;
    parameters[name] = { v: n, unit: p.unit, origin: 'user_stated', status: 'unverified' };
  }
  return { op: { op: 'part.setBehavior', id: partId, behavior: { kind: 'fmu', fmuSha256: sha256, ...(Object.keys(parameters).length ? { parameters } : {}), outputs } } };
}
