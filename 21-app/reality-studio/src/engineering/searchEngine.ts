export const SEARCH_ENGINE_SCHEMA = 'helqix.search-engine.v1' as const;

export interface SearchVariable {
  name: string;
  min: number;
  max: number;
  step?: number;
  integer?: boolean;
}

export interface SearchConstraint {
  id: string;
  evaluate: (candidate: Record<string, number>) => boolean;
}

export interface SearchEvaluation {
  index: number;
  candidate: Record<string, number>;
  feasible: boolean;
  score?: number;
  output?: Record<string, unknown>;
  error?: string;
}

export interface SearchRequest {
  schema: typeof SEARCH_ENGINE_SCHEMA;
  id: string;
  variables: SearchVariable[];
  constraints: SearchConstraint[];
  maximize: boolean;
  maxEvaluations: number;
  seed: number;
}

export interface SearchResult {
  schema: typeof SEARCH_ENGINE_SCHEMA;
  request: SearchRequest;
  evaluations: SearchEvaluation[];
  best?: SearchEvaluation;
  completed: boolean;
}

function normalize(variable: SearchVariable, value: number): number {
  const bounded = Math.min(variable.max, Math.max(variable.min, value));
  if (variable.integer) return Math.round(bounded);
  if (variable.step && variable.step > 0) return Math.min(variable.max, variable.min + Math.round((bounded - variable.min) / variable.step) * variable.step);
  return bounded;
}

function random(seed: number): () => number {
  let state = Math.abs(Math.floor(seed)) >>> 0;
  return () => {
    state = (1664525 * state + 1013904223) >>> 0;
    return state / 4294967296;
  };
}

function validateVariable(variable: SearchVariable): void {
  if (!variable.name.trim()) throw new Error('Search variable name is required.');
  if (!Number.isFinite(variable.min) || !Number.isFinite(variable.max) || variable.max < variable.min) throw new Error(`Invalid bounds for ${variable.name}.`);
  if (variable.step !== undefined && (!Number.isFinite(variable.step) || variable.step <= 0)) throw new Error(`Invalid step for ${variable.name}.`);
}

export function createSearchRequest(variables: SearchVariable[], constraints: SearchConstraint[], maximize: boolean, maxEvaluations: number, seed = 1, id = 'search'): SearchRequest {
  if (!variables.length) throw new Error('At least one search variable is required.');
  if (!Number.isInteger(maxEvaluations) || maxEvaluations <= 0) throw new Error('maxEvaluations must be a positive integer.');
  variables.forEach(validateVariable);
  if (!Number.isFinite(seed)) throw new Error('Search seed must be finite.');
  if (!id.trim()) throw new Error('Search id must not be blank.');
  return { schema: SEARCH_ENGINE_SCHEMA, id: id.trim(), variables: variables.map((v) => ({ ...v })), constraints: constraints.map((c) => ({ ...c })), maximize, maxEvaluations, seed };
}

export async function search(
  request: SearchRequest,
  evaluate: (candidate: Record<string, number>) => Promise<{ score: number; output?: Record<string, unknown> }> | { score: number; output?: Record<string, unknown> },
): Promise<SearchResult> {
  const rng = random(request.seed);
  const evaluations: SearchEvaluation[] = [];
  let best: SearchEvaluation | undefined;

  for (let index = 0; index < request.maxEvaluations; index += 1) {
    const candidate: Record<string, number> = {};
    for (const variable of request.variables) candidate[variable.name] = normalize(variable, variable.min + rng() * (variable.max - variable.min));
    if (!request.constraints.every((constraint) => constraint.evaluate(candidate))) {
      evaluations.push({ index, candidate, feasible: false });
      continue;
    }
    try {
      const result = await evaluate({ ...candidate });
      if (!Number.isFinite(result.score)) throw new Error('Search score must be finite.');
      const evaluation: SearchEvaluation = { index, candidate, feasible: true, score: result.score, output: result.output };
      evaluations.push(evaluation);
      if (!best || (request.maximize ? result.score > (best.score as number) : result.score < (best.score as number))) best = evaluation;
    } catch (error) {
      evaluations.push({ index, candidate, feasible: true, error: error instanceof Error ? error.message : 'Search evaluation failed.' });
    }
  }

  return { schema: SEARCH_ENGINE_SCHEMA, request: { ...request, variables: request.variables.map((v) => ({ ...v })), constraints: request.constraints.map((c) => ({ ...c })) }, evaluations, best, completed: evaluations.length === request.maxEvaluations };
}
