import type { AuthoritativeEngineeringState, ScalarValue } from './authoritativeState';

export const CONDITION_ENGINE_SCHEMA = 'helqix.condition-engine.v1' as const;
export type ConditionOperator = 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte' | 'exists' | 'in';

export interface StateCondition {
  path: string;
  operator: ConditionOperator;
  value?: ScalarValue | ScalarValue[];
}

export interface ConditionResult {
  schema: typeof CONDITION_ENGINE_SCHEMA;
  condition: StateCondition;
  matched: boolean;
  actualValue: unknown;
  reason: string;
}

function readPath(state: AuthoritativeEngineeringState, path: string): unknown {
  return path.split('.').reduce<unknown>((current, segment) => {
    if (current === null || current === undefined || typeof current !== 'object') return undefined;
    return (current as Record<string, unknown>)[segment];
  }, state);
}

export function evaluateCondition(state: AuthoritativeEngineeringState, condition: StateCondition): ConditionResult {
  const actualValue = readPath(state, condition.path);
  const expected = condition.value;
  let matched = false;
  switch (condition.operator) {
    case 'exists': matched = actualValue !== undefined && actualValue !== null; break;
    case 'eq': matched = JSON.stringify(actualValue) === JSON.stringify(expected); break;
    case 'neq': matched = JSON.stringify(actualValue) !== JSON.stringify(expected); break;
    case 'gt': matched = typeof actualValue === 'number' && typeof expected === 'number' && actualValue > expected; break;
    case 'gte': matched = typeof actualValue === 'number' && typeof expected === 'number' && actualValue >= expected; break;
    case 'lt': matched = typeof actualValue === 'number' && typeof expected === 'number' && actualValue < expected; break;
    case 'lte': matched = typeof actualValue === 'number' && typeof expected === 'number' && actualValue <= expected; break;
    case 'in': matched = Array.isArray(expected) && expected.some((item) => JSON.stringify(item) === JSON.stringify(actualValue)); break;
  }
  return {
    schema: CONDITION_ENGINE_SCHEMA,
    condition: structuredClone(condition),
    matched,
    actualValue,
    reason: matched ? `Condition matched at ${condition.path}.` : `Condition did not match at ${condition.path}.`,
  };
}

export function evaluateAllConditions(state: AuthoritativeEngineeringState, conditions: StateCondition[]): ConditionResult[] {
  return conditions.map((condition) => evaluateCondition(state, condition));
}

export function conditionsSatisfied(state: AuthoritativeEngineeringState, conditions: StateCondition[]): boolean {
  return evaluateAllConditions(state, conditions).every((result) => result.matched);
}
