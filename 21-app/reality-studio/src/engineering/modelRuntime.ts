import type { EngineeringReality, EngineeringRequirement, RealityInterface, RealityProperty } from '../api';
import {
  assertValidReality,
  cloneReality,
  normalizeReality,
  validateReality,
  type ComponentDefinition,
  type EngineeringComponent,
  type EngineeringModelContract,
} from './engineeringReality';
import { assertQuantity, quantity, SI_UNITS, type PhysicalQuantity, type DimensionVector } from './physicalQuantity';

export const ENGINEERING_MODEL_RUNTIME_SCHEMA = 'helqix.engineering-model-runtime.v1' as const;
export const ENGINEERING_MODEL_SNAPSHOT_SCHEMA = 'helqix.engineering-model-snapshot.v1' as const;

export interface RuntimeParameter { componentId: string; propertyName: string; value: string | number | boolean; unit?: string; physicalQuantity?: PhysicalQuantity; source: RealityProperty['origin'] | 'runtime'; state: 'valid' | 'warning' | 'invalid'; }
export interface RuntimeInterface { id: string; componentId: string; name: string; direction: RealityInterface['direction']; domain: string; connectedInterfaceIds: string[]; }
export interface RuntimeConnection { id: string; sourceInterfaceId: string; targetInterfaceId: string; connectionType: string; sourceComponentId: string; targetComponentId: string; }
export interface RuntimeComponent { id: string; name: string; componentType: string; category: EngineeringComponent['category']; parameters: RuntimeParameter[]; interfaces: RuntimeInterface[]; modelContracts: EngineeringModelContract[]; constraints: EngineeringComponent['constraints']; behaviors: EngineeringComponent['behaviors']; tags: string[]; }
export interface ModelConstraintResult { constraintId: string; componentId: string; valid: boolean; severity: 'error' | 'warning' | 'info'; expression: string; message: string; }
export interface ModelValidationIssue { path: string; code: 'reality_invalid' | 'missing_model' | 'missing_parameter' | 'invalid_parameter' | 'invalid_unit' | 'constraint_failed' | 'topology_invalid' | 'requirement_invalid'; message: string; severity: 'error' | 'warning'; }
export interface EngineeringModelSnapshot { schema: typeof ENGINEERING_MODEL_SNAPSHOT_SCHEMA; runtimeSchema: typeof ENGINEERING_MODEL_RUNTIME_SCHEMA; realityRevision: number; modelRevision: number; capturedAt: string; components: RuntimeComponent[]; connections: RuntimeConnection[]; requirements: EngineeringRequirement[]; parameters: RuntimeParameter[]; constraints: ModelConstraintResult[]; }
export interface EngineeringModelRuntime { readonly schema: typeof ENGINEERING_MODEL_RUNTIME_SCHEMA; readonly reality: EngineeringReality; readonly modelRevision: number; validate(): ModelValidationIssue[]; snapshot(): EngineeringModelSnapshot; getComponent(componentId: string): RuntimeComponent | undefined; getParameter(componentId: string, propertyName: string): RuntimeParameter | undefined; setParameter(componentId: string, propertyName: string, value: string | number | boolean, unit?: string): void; evaluateConstraints(): ModelConstraintResult[]; connectedComponentIds(componentId: string): string[]; }

function clone<T>(value: T): T { return structuredClone(value); }
function finite(value: number, label: string): void { if (!Number.isFinite(value)) throw new Error(`${label} must be finite.`); }
function parameterKey(componentId: string, propertyName: string): string { return `${componentId}::${propertyName}`; }
function sameDimension(a: DimensionVector, b: DimensionVector): boolean { const keys = ['length', 'mass', 'time', 'temperature', 'current', 'amount', 'luminousIntensity'] as const; return keys.every((key) => a[key] === b[key]); }
function componentMetadata(component: EngineeringComponent): Pick<RuntimeComponent, 'category' | 'constraints' | 'behaviors' | 'tags'> {
  return { category: component.category ?? 'unknown', constraints: clone(component.constraints ?? []), behaviors: clone(component.behaviors ?? []), tags: [...(component.tags ?? [])] };
}
function parameterSource(origin: RealityProperty['origin']): RuntimeParameter['source'] { return origin === 'model' ? 'derived' : origin; }
function runtimeComponent(component: EngineeringComponent, reality: EngineeringReality, contracts: EngineeringModelContract[]): RuntimeComponent {
  const metadata = componentMetadata(component);
  const interfaces = reality.interfaces.filter((item) => item.componentId === component.id).map((item) => ({
    id: item.id, componentId: item.componentId, name: item.name, direction: item.direction, domain: item.domain,
    connectedInterfaceIds: reality.connections.filter((connection) => connection.sourceInterfaceId === item.id || connection.targetInterfaceId === item.id).map((connection) => connection.sourceInterfaceId === item.id ? connection.targetInterfaceId : connection.sourceInterfaceId).sort(),
  }));
  return { id: component.id, name: component.name, componentType: component.componentType, ...metadata,
    parameters: component.properties.map((property) => {
      const result: RuntimeParameter = { componentId: component.id, propertyName: property.name, value: clone(property.value), ...(property.unit ? { unit: property.unit } : {}), source: parameterSource(property.origin), state: 'valid' };
      if (typeof property.value === 'number' && property.unit) { try { result.physicalQuantity = quantity(property.value, property.unit, { source: parameterSource(property.origin) === 'runtime' ? 'derived' : parameterSource(property.origin) }); } catch { result.state = 'invalid'; } }
      return result;
    }),
    interfaces, modelContracts: contracts.filter((contract) => contract.componentType === component.componentType),
  };
}
function parseConstraint(expression: string): { left: string; operator: '<' | '<=' | '>' | '>=' | '==' | '!='; right: number } | undefined {
  const match = expression.trim().match(/^([a-zA-Z0-9_.:-]+)\s*(<=|>=|==|!=|<|>)\s*(-?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)$/);
  if (!match) return undefined;
  return { left: match[1], operator: match[2] as '<' | '<=' | '>' | '>=' | '==' | '!=', right: Number(match[3]) };
}
function satisfies(actual: number, operator: string, expected: number): boolean { switch (operator) { case '<': return actual < expected; case '<=': return actual <= expected; case '>': return actual > expected; case '>=': return actual >= expected; case '==': return Math.abs(actual - expected) <= Math.max(1e-12, Math.abs(expected) * 1e-12); case '!=': return Math.abs(actual - expected) > Math.max(1e-12, Math.abs(expected) * 1e-12); default: return false; } }

export class EngineeringModelRuntimeImpl implements EngineeringModelRuntime {
  readonly schema = ENGINEERING_MODEL_RUNTIME_SCHEMA;
  private readonly components = new Map<string, RuntimeComponent>();
  private readonly parameterIndex = new Map<string, RuntimeParameter>();
  private modelRevisionValue: number;
  private realityValue: EngineeringReality;

  constructor(reality: EngineeringReality, modelContracts: EngineeringModelContract[] = [], componentDefinitions: ComponentDefinition[] = []) {
    const normalized = normalizeReality(reality); assertValidReality(normalized); this.realityValue = cloneReality(normalized); this.modelRevisionValue = normalized.revision;
    const definitions = new Map(componentDefinitions.map((definition) => [definition.componentType, definition]));
    for (const raw of normalized.components) {
      const component = raw as EngineeringComponent;
      const definition = definitions.get(component.componentType);
      const merged: EngineeringComponent = { ...component, category: component.category ?? definition?.category ?? 'unknown', constraints: component.constraints ?? [], behaviors: component.behaviors ?? [], tags: component.tags ?? [] };
      const runtime = runtimeComponent(merged, normalized, modelContracts); this.components.set(runtime.id, runtime); for (const parameter of runtime.parameters) this.parameterIndex.set(parameterKey(parameter.componentId, parameter.propertyName), parameter);
    }
    this.refreshParameterValidity();
  }

  get reality(): EngineeringReality { return clone(this.realityValue); }
  get modelRevision(): number { return this.modelRevisionValue; }

  validate(): ModelValidationIssue[] {
    const issues: ModelValidationIssue[] = [];
    for (const issue of validateReality(this.realityValue).issues.filter((item) => item.severity === 'error')) issues.push({ code: 'reality_invalid', path: issue.path, message: issue.message, severity: 'error' });
    for (const component of this.components.values()) {
      if (!component.modelContracts.length && component.componentType !== 'generic_component') issues.push({ code: 'missing_model', path: `components.${component.id}.model`, message: `No executable model contract is bound to ${component.componentType}.`, severity: 'warning' });
      for (const parameter of component.parameters) if (parameter.state === 'invalid') issues.push({ code: 'invalid_parameter', path: `components.${component.id}.properties.${parameter.propertyName}`, message: `Parameter ${parameter.propertyName} has an invalid value or unit.`, severity: 'error' });
      for (const contract of component.modelContracts) for (const required of contract.requiredProperties.filter((item) => item.required)) {
        const parameter = component.parameters.find((item) => item.propertyName === required.name);
        if (!parameter) issues.push({ code: 'missing_parameter', path: `components.${component.id}.properties.${required.name}`, message: `Required model parameter ${required.name} is missing.`, severity: 'error' });
        else if (typeof parameter.value !== required.kind) issues.push({ code: 'invalid_parameter', path: `components.${component.id}.properties.${required.name}`, message: `Parameter ${required.name} must be ${required.kind}.`, severity: 'error' });
        else if (required.unit && parameter.unit && typeof parameter.value === 'number') {
          try { if (!sameDimension(SI_UNITS.get(parameter.unit).dimension, SI_UNITS.get(required.unit).dimension)) throw new Error('dimension mismatch'); }
          catch { issues.push({ code: 'invalid_unit', path: `components.${component.id}.properties.${required.name}.unit`, message: `Parameter ${required.name} is not dimensionally compatible with ${required.unit}.`, severity: 'error' }); }
        }
      }
    }
    for (const connection of this.realityValue.connections) {
      const source = this.realityValue.interfaces.find((item) => item.id === connection.sourceInterfaceId); const target = this.realityValue.interfaces.find((item) => item.id === connection.targetInterfaceId);
      if (!source || !target) issues.push({ code: 'topology_invalid', path: `connections.${connection.id}`, message: 'Connection references a missing interface.', severity: 'error' });
      else if (source.direction === 'in' && target.direction === 'in') issues.push({ code: 'topology_invalid', path: `connections.${connection.id}`, message: 'Two input-only interfaces cannot be directly connected.', severity: 'error' });
    }
    for (const requirement of this.realityValue.requirements) if (requirement.reviewStatus === 'accepted' && !requirement.verificationRule) issues.push({ code: 'requirement_invalid', path: `requirements.${requirement.id}`, message: 'Accepted requirements should have a verification rule before execution.', severity: 'warning' });
    for (const result of this.evaluateConstraints()) if (!result.valid) issues.push({ code: 'constraint_failed', path: `components.${result.componentId}.constraints.${result.constraintId}`, message: result.message, severity: result.severity === 'error' ? 'error' : 'warning' });
    return issues;
  }

  snapshot(): EngineeringModelSnapshot { const components = [...this.components.values()].map(clone); return { schema: ENGINEERING_MODEL_SNAPSHOT_SCHEMA, runtimeSchema: ENGINEERING_MODEL_RUNTIME_SCHEMA, realityRevision: this.realityValue.revision, modelRevision: this.modelRevisionValue, capturedAt: new Date().toISOString(), components, connections: this.realityValue.connections.map((connection) => this.runtimeConnection(connection)), requirements: clone(this.realityValue.requirements), parameters: components.flatMap((component) => component.parameters), constraints: this.evaluateConstraints() }; }
  getComponent(componentId: string): RuntimeComponent | undefined { const value = this.components.get(componentId); return value ? clone(value) : undefined; }
  getParameter(componentId: string, propertyName: string): RuntimeParameter | undefined { const value = this.parameterIndex.get(parameterKey(componentId, propertyName)); return value ? clone(value) : undefined; }

  setParameter(componentId: string, propertyName: string, value: string | number | boolean, unit?: string): void {
    const parameter = this.parameterIndex.get(parameterKey(componentId, propertyName)); if (!parameter) throw new Error(`Unknown engineering parameter: ${componentId}.${propertyName}`); if (typeof value === 'number') finite(value, `${componentId}.${propertyName}`);
    const nextUnit = unit ?? parameter.unit; if (typeof value === 'number' && nextUnit) { parameter.physicalQuantity = quantity(value, nextUnit, { source: 'derived', sourceRef: `${componentId}.${propertyName}` }); assertQuantity(parameter.physicalQuantity); }
    parameter.value = value; parameter.unit = nextUnit; parameter.source = 'runtime'; parameter.state = 'valid'; const component = this.realityValue.components.find((item) => item.id === componentId); if (!component) throw new Error(`Unknown reality component: ${componentId}`); const property = component.properties.find((item) => item.name === propertyName); if (property) { property.value = value; if (nextUnit) property.unit = nextUnit; property.origin = 'derived'; } this.modelRevisionValue += 1;
  }

  evaluateConstraints(): ModelConstraintResult[] {
    const results: ModelConstraintResult[] = [];
    for (const component of this.components.values()) for (const constraint of component.constraints.filter((item) => item.enabled)) {
      const parsed = parseConstraint(constraint.expression); if (!parsed) { results.push({ constraintId: constraint.id, componentId: component.id, valid: false, severity: constraint.severity, expression: constraint.expression, message: 'Constraint expression is not supported by the deterministic evaluator.' }); continue; }
      const parameter = component.parameters.find((item) => item.propertyName === parsed.left); if (!parameter || typeof parameter.value !== 'number') { results.push({ constraintId: constraint.id, componentId: component.id, valid: false, severity: constraint.severity, expression: constraint.expression, message: `Constraint parameter ${parsed.left} is missing or non-numeric.` }); continue; }
      const valid = satisfies(parameter.value, parsed.operator, parsed.right); results.push({ constraintId: constraint.id, componentId: component.id, valid, severity: constraint.severity, expression: constraint.expression, message: valid ? 'Constraint satisfied.' : `Constraint failed: ${parameter.value} ${parsed.operator} ${parsed.right}.` });
    }
    return results;
  }

  connectedComponentIds(componentId: string): string[] {
    const interfaceIds = new Set(this.realityValue.interfaces.filter((item) => item.componentId === componentId).map((item) => item.id)); const connected = new Set<string>();
    for (const connection of this.realityValue.connections) { if (interfaceIds.has(connection.sourceInterfaceId)) { const target = this.realityValue.interfaces.find((item) => item.id === connection.targetInterfaceId); if (target) connected.add(target.componentId); } if (interfaceIds.has(connection.targetInterfaceId)) { const source = this.realityValue.interfaces.find((item) => item.id === connection.sourceInterfaceId); if (source) connected.add(source.componentId); } }
    connected.delete(componentId); return [...connected].sort();
  }

  private runtimeConnection(connection: EngineeringReality['connections'][number]): RuntimeConnection { const source = this.realityValue.interfaces.find((item) => item.id === connection.sourceInterfaceId); const target = this.realityValue.interfaces.find((item) => item.id === connection.targetInterfaceId); if (!source || !target) throw new Error(`Connection ${connection.id} references a missing interface.`); return { id: connection.id, sourceInterfaceId: source.id, targetInterfaceId: target.id, connectionType: connection.connectionType, sourceComponentId: source.componentId, targetComponentId: target.componentId }; }
  private refreshParameterValidity(): void { for (const parameter of this.parameterIndex.values()) { if (typeof parameter.value !== 'number' || !parameter.unit) continue; try { parameter.physicalQuantity = quantity(parameter.value, parameter.unit, { source: parameter.source === 'runtime' ? 'derived' : parameterSource(parameter.source) }); assertQuantity(parameter.physicalQuantity); parameter.state = 'valid'; } catch { parameter.state = 'invalid'; } } }
}

export function createEngineeringModelRuntime(reality: EngineeringReality, modelContracts: EngineeringModelContract[] = [], componentDefinitions: ComponentDefinition[] = []): EngineeringModelRuntimeImpl { return new EngineeringModelRuntimeImpl(reality, modelContracts, componentDefinitions); }
export function validateEngineeringModelRuntime(runtime: EngineeringModelRuntime): ModelValidationIssue[] { return runtime.validate(); }
export function assertEngineeringModelRuntime(runtime: EngineeringModelRuntime): void { const errors = runtime.validate().filter((issue) => issue.severity === 'error'); if (errors.length) throw new Error(`Engineering model runtime is invalid: ${errors.map((issue) => `${issue.path}: ${issue.message}`).join('; ')}`); }
