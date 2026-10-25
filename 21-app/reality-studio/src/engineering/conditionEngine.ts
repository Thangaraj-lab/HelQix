import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState, ScalarValue } from './authoritativeState';

export const CONDITION_SCHEMA = 'helqix.condition.v1' as const;

export type ConditionOperator = 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte' | 'exists' | 'in';

export interface StateCondition {
  schema: typeof CONDITION_SCHEMA;
  id: string;
  path: string;
  operator: ConditionOperator;
  value?: ScalarValue | ScalarValue[];
}

export interface ConditionResult {
  conditionId: string;
  matched: boolean;
  path: string;
  actual?: ScalarValue;
  expected?: ScalarValue | ScalarValue[];
  reason: string;
}

function readPath(state: AuthoritativeEngineeringState, path: string): ScalarValue | undefined {
  const parts = path.split('.').filter(Boolean);
  if (!parts.length) return undefined;
  let current: unknown = state;
  for (const part of parts) {
    if (current === null || typeof current !== 'object') return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return ['string', 'number', 'boolean'].includes(typeof current) ? current as ScalarValue : undefined;
}

export function evaluateCondition(state: AuthoritativeEngineeringState, condition: StateCondition): ConditionResult {
  const actual = readPath(state, condition.path);
  const expected = condition.value;
  let matched = false;
  switch (condition.operator) {
    case 'exists': matched = actual !== undefined; break;
    case 'eq': matched = actual !== undefined && actual === expected; break;
    case 'neq': matched = actual === undefined || actual !== expected; break;
    case 'gt': matched = typeof actual === 'number' && typeof expected === 'number' && actual > expected; break;
    case 'gte': matched = typeof actual === 'number' && typeof expected === 'number' && actual >= expected; break;
    case 'lt': matched = typeof actual === 'number' && typeof expected === 'number' && actual < expected; break;
    case 'lte': matched = typeof actual === 'number' && typeof expected === 'number' && actual <= expected; break;
    case 'in': matched = Array.isArray(expected) && expected.some((item) => item === actual); break;
  }
  return { conditionId: condition.id, matched, path: condition.path, ...(actual === undefined ? {} : { actual }), ...(expected === undefined ? {} : { expected }), reason: matched ? 'Condition matched.' : 'Condition did not match.' };
}

export function validateCondition(condition: StateCondition, reality?: EngineeringReality): string[] {
  const errors: string[] = [];
  if (condition.schema !== CONDITION_SCHEMA) errors.push(`Expected ${CONDITION_SCHEMA}.`);
  if (!condition.id.trim()) errors.push('Condition id is required.');
  if (!condition.path.trim()) errors.push('Condition path is required.');
  if (condition.operator !== 'exists' && condition.value === undefined) errors.push(`${condition.operator} requires a value.`);
  if (condition.operator === 'in' && !Array.isArray(condition.value)) errors.push('in requires an array value.');
  if (condition.path.startsWith('components.') && reality) {
    const componentId = condition.path.split('.')[1];
    if (!reality.components.some((component) => component.id === componentId)) errors.push(`Unknown component ${componentId}.`);
  }
  return errors;
}

export function evaluateAllConditions(state: AuthoritativeEngineeringState, conditions: StateCondition[]): { matched: boolean; results: ConditionResult[] } {
  const results = conditions.map((condition) => evaluateCondition(state, condition));
  return { matched: results.every((result) => result.matched), results };
}
