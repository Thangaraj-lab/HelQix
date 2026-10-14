export const OPTIMIZATION_ENGINE_SCHEMA = 'helqix.optimization-engine.v1' as const;
export type OptimizationDirection = 'minimize' | 'maximize';
export type OptimizationStrategy = 'grid' | 'random';
export interface OptimizationVariable { name: string; min: number; max: number; step?: number; integer?: boolean; }
export interface OptimizationConstraint { id: string; description: string; evaluate: (variables: Record<string, number>) => boolean; }
export interface OptimizationObjective { id: string; direction: OptimizationDirection; evaluate: (variables: Record<string, number>, output: Record<string, unknown>) => number; }
export interface OptimizationRequest { schema: typeof OPTIMIZATION_ENGINE_SCHEMA; id: string; strategy: OptimizationStrategy; variables: OptimizationVariable[]; constraints: OptimizationConstraint[]; objectives: OptimizationObjective[]; maxEvaluations: number; seed: number; }
export interface OptimizationEvaluation { index: number; variables: Record<string, number>; feasible: boolean; outputs?: Record<string, unknown>; objectiveValues?: Record<string, number>; error?: string; }
export interface OptimizationResult { schema: typeof OPTIMIZATION_ENGINE_SCHEMA; request: OptimizationRequest; evaluations: OptimizationEvaluation[]; best?: OptimizationEvaluation; completed: boolean; }
function clone<T>(value: T): T { return structuredClone(value); }
function validateVariable(variable: OptimizationVariable): void { if (!variable.name.trim()) throw new Error('Optimization variable name is required.'); if (!Number.isFinite(variable.min) || !Number.isFinite(variable.max) || variable.max < variable.min) throw new Error(`Invalid bounds for ${variable.name}.`); if (variable.step !== undefined && (!Number.isFinite(variable.step) || variable.step <= 0)) throw new Error(`Invalid step for ${variable.name}.`); }
export function createOptimizationRequest(strategy: OptimizationStrategy, variables: OptimizationVariable[], constraints: OptimizationConstraint[], objectives: OptimizationObjective[], maxEvaluations: number, seed = 1): OptimizationRequest {
  if (!['grid', 'random'].includes(strategy)) throw new Error(`Unsupported optimization strategy: ${strategy}.`);
  if (!variables.length) throw new Error('At least one optimization variable is required.');
  if (!objectives.length) throw new Error('At least one optimization objective is required.');
  if (!Number.isInteger(maxEvaluations) || maxEvaluations <= 0) throw new Error('maxEvaluations must be a positive integer.');
  variables.forEach(validateVariable);
  if (!Number.isFinite(seed)) throw new Error('Optimization seed must be finite.');
  return { schema: OPTIMIZATION_ENGINE_SCHEMA, id: `opt-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, strategy, variables: clone(variables), constraints: clone(constraints), objectives: clone(objectives), maxEvaluations, seed };
}
function lcg(seed: number): () => number { let state = Math.abs(Math.floor(seed)) >>> 0; return () => { state = (1664525 * state + 1013904223) >>> 0; return state / 4294967296; }; }
function normalizeValue(variable: OptimizationVariable, value: number): number { const clamped = Math.min(variable.max, Math.max(variable.min, value)); if (variable.integer) return Math.round(clamped); if (variable.step) return variable.min + Math.round((clamped - variable.min) / variable.step) * variable.step; return clamped; }
function gridCandidates(request: OptimizationRequest): Record<string, number>[] {
  const result: Record<string, number>[] = [];
  const walk = (index: number, current: Record<string, number>) => { if (result.length >= request.maxEvaluations) return; if (index === request.variables.length) { result.push({ ...current }); return; } const variable = request.variables[index]; const step = variable.step ?? (variable.integer ? 1 : variable.max - variable.min || 1); const count = Math.floor((variable.max - variable.min) / step + 1e-9); for (let i = 0; i <= count && result.length < request.maxEvaluations; i++) { current[variable.name] = normalizeValue(variable, variable.min + i * step); walk(index + 1, current); } delete current[variable.name]; };
  walk(0, {}); return result;
}
function randomCandidates(request: OptimizationRequest): Record<string, number>[] { const random = lcg(request.seed); const result: Record<string, number>[] = []; for (let i = 0; i < request.maxEvaluations; i++) { const candidate: Record<string, number> = {}; for (const variable of request.variables) candidate[variable.name] = normalizeValue(variable, variable.min + random() * (variable.max - variable.min)); result.push(candidate); } return result; }
function feasible(candidate: Record<string, number>, constraints: OptimizationConstraint[]): boolean { return constraints.every((constraint) => constraint.evaluate(candidate)); }
function better(a: OptimizationEvaluation, b: OptimizationEvaluation, objectives: OptimizationObjective[]): boolean { if (!a.objectiveValues || !b.objectiveValues) return false; let strictlyBetter = false; for (const objective of objectives) { const left = a.objectiveValues[objective.id]; const right = b.objectiveValues[objective.id]; if (objective.direction === 'minimize') { if (left > right) return false; if (left < right) strictlyBetter = true; } else { if (left < right) return false; if (left > right) strictlyBetter = true; } } return strictlyBetter; }
export async function optimize(request: OptimizationRequest, execute: (variables: Record<string, number>) => Promise<Record<string, unknown>> | Record<string, unknown>): Promise<OptimizationResult> {
  const candidates = request.strategy === 'grid' ? gridCandidates(request) : randomCandidates(request); const evaluations: OptimizationEvaluation[] = []; let best: OptimizationEvaluation | undefined;
  for (let index = 0; index < candidates.length && evaluations.length < request.maxEvaluations; index++) { const variables = candidates[index]; if (!feasible(variables, request.constraints)) { evaluations.push({ index, variables, feasible: false }); continue; } try { const outputs = await execute(clone(variables)); const objectiveValues: Record<string, number> = {}; for (const objective of request.objectives) { const value = objective.evaluate(variables, outputs); if (!Number.isFinite(value)) throw new Error(`Objective ${objective.id} returned a non-finite value.`); objectiveValues[objective.id] = value; } const evaluation: OptimizationEvaluation = { index, variables, feasible: true, outputs, objectiveValues }; evaluations.push(evaluation); if (!best || better(evaluation, best, request.objectives)) best = evaluation; } catch (error) { evaluations.push({ index, variables, feasible: true, error: error instanceof Error ? error.message : 'Optimization evaluation failed.' }); } }
  return { schema: OPTIMIZATION_ENGINE_SCHEMA, request: clone(request), evaluations, best, completed: evaluations.length >= Math.min(request.maxEvaluations, candidates.length) };
}
