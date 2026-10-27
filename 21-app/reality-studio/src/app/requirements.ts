import { Binding, ModelRevision } from '../contracts';

// A requirement is stored on the model as text plus an expression such as
// "stator_winding_temperature < 120". Evaluating it needs only the stored series.

export interface Requirement {
  id: string;
  text: string;
  variable: string;
  /** The comparison that must hold, e.g. "<" for "stay below" */
  op: '<' | '<=' | '>' | '>=';
  limit: number;
  unit: string;
}

const EXPR = /^\s*([A-Za-z_][\w]*)\s*(<=|>=|<|>)\s*(-?\d+(?:\.\d+)?)\s*$/;

export function parseRequirement(id: string, text: string, expr: string | undefined, bindings: Binding[]): Requirement | null {
  const m = expr ? EXPR.exec(expr) : null;
  if (!m) return null;
  const unit = bindings.find(b => b.variable === m[1])?.units ?? '';
  return { id, text, variable: m[1], op: m[2] as Requirement['op'], limit: Number(m[3]), unit };
}

export function requirementsOf(revision: ModelRevision | null): Requirement[] {
  if (!revision) return [];
  const bindings = Object.values(revision.bindings);
  return Object.values(revision.requirements)
    .map(r => parseRequirement(r.id, r.text, r.expr, bindings))
    .filter((r): r is Requirement => r !== null);
}

/** True when the value breaks the requirement (the opposite of the stated comparison). */
export function violates(r: Requirement, v: number): boolean {
  switch (r.op) {
    case '<': return !(v < r.limit);
    case '<=': return !(v <= r.limit);
    case '>': return !(v > r.limit);
    default: return !(v >= r.limit);
  }
}

export const limitLabel = (r: Requirement) => `${r.limit}${r.unit ? ' ' + r.unit : ''}`;
