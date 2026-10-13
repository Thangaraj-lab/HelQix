/**
 * HELQIX Engineering Model Runtime
 *
 * Turns an EngineeringReality document into an executable, validated model
 * contract. This layer owns topology, model bindings, parameter normalization,
 * constraint checks and deterministic model revisions. It deliberately does
 * not solve physics; physics providers consume the runtime snapshot produced
 * here.
 */

import type {
  EngineeringReality,
  EngineeringRequirement,
  RealityComponent,
  RealityConnection,
  RealityInterface,
  RealityProperty,
} from '../api';
import {
  assertValidReality,
  cloneReality,
  normalizeReality,
  validateReality,
  type ComponentDefinition,
  type EngineeringComponent,
  type EngineeringModelContract,
} from './engineeringReality';
import {
  assertQuantity,
  quantity,
  validateQuantityRange,
  type PhysicalQuantity,
  type QuantityRange,
} from './physicalQuantity';

export const ENGINEERING_MODEL_RUNTIME_SCHEMA = 'helqix.engineering-model-runtime.v1' as const;
export const ENGINEERING_MODEL_SNAPSHOT_SCHEMA = 'helqix.engineering-model-snapshot.v1' as const;

export interface RuntimeParameter {
  componentId: string;
  propertyName: string;
  value: string | number | boolean;
  unit?: string;
  physicalQuantity?: PhysicalQuantity;
  source: RealityProperty['origin'] | 'runtime';
  state: 'valid' | 'warning' | 'invalid';
}

export interface RuntimeInterface {
  id: string;
  componentId: string;
  name: string;
  direction: RealityInterface['direction'];
  domain: string;
  connectedInterfaceIds: string[];
}

export interface RuntimeConnection {
  id: string;
  sourceInterfaceId: string;
  targetInterfaceId: string;
  connectionType: string;
  sourceComponentId: string;
  targetComponentId: string;
}

export interface RuntimeComponent {
  id: string;
  name: string;
  componentType: string;
  category: EngineeringComponent['category'];
  parameters: RuntimeParameter[];
  interfaces: RuntimeInterface[];
  modelContracts: EngineeringModelContract[];
  constraints: EngineeringComponent['constraints'];
  behaviors: EngineeringComponent['behaviors'];
  tags: string[];
}

export interface ModelConstraintResult {
  constraintId: string;
  componentId: string;
  valid: boolean;
  severity: 'error' | 'warning' | 'info';
  expression: string;
  message: string;
}

export interface ModelValidationIssue {
  path: string;
  code:
    | 'reality_invalid'
    | 'missing_model'
    | 'missing_parameter'
    | 'invalid_parameter'
    | 'invalid_unit'
    | 'constraint_failed'
    | 'topology_invalid'
    | 'requirement_invalid';
  message: string;
  severity: 'error' | 'warning';
}

export interface EngineeringModelSnapshot {
  schema: typeof ENGINEERING_MODEL_SNAPSHOT_SCHEMA;
  runtimeSchema: typeof ENGINEERING_MODEL_RUNTIME_SCHEMA;
  realityRevision: number;
  modelRevision: number;
  capturedAt: string;
  components: RuntimeComponent[];
  connections: RuntimeConnection[];
  requirements: EngineeringRequirement[];
  parameters: RuntimeParameter[];
  constraints: ModelConstraintResult[];
}

export interface EngineeringModelRuntime {
  readonly schema: typeof ENGINEERING_MODEL_RUNTIME_SCHEMA;
  readonly reality: EngineeringReality;
  readonly modelRevision: number;
  validate(): ModelValidationIssue[];
  snapshot(): EngineeringModelSnapshot;
  getComponent(componentId: string): RuntimeComponent | undefined;
  getParameter(componentId: string, propertyName: string): RuntimeParameter | undefined;
  setParameter(componentId: string, propertyName: string, value: string | number | boolean, unit?: string): void;
  evaluateConstraints(): ModelConstraintResult[];
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function finite(value: number, label: string): void {
  if (!Number.isFinite(value)) throw new Error(`${label} must be finite.`);
}

function clean(value: string): string {
  return value.trim();
}

function parameterKey(componentId: string, propertyName: string): string {
  return `${componentId}::${propertyName}`;
}

function componentDefinitionFor(
  component: EngineeringComponent,
  definitions: ComponentDefinition[],
): ComponentDefinition | undefined {
  return definitions.find((definition) => definition.componentType === component.componentType);
}

function buildRuntimeComponent(
  component: EngineeringComponent,
  reality: EngineeringReality,
  contracts: EngineeringModelContract[],
): RuntimeComponent {
  const componentInterfaces = reality.interfaces.filter((item) => item.componentId === component.id);
  const interfaces = componentInterfaces.map((item) => ({
    id: item.id,
    componentId: item.componentId,
    name: item.name,
    direction: item.direction,
    domain: item.domain,
    connectedInterfaceIds: reality.connections
      .filter((connection) => connection.sourceInterfaceId === item.id || connection.targetInterfaceId === item.id)
      .map((connection) => connection.sourceInterfaceId === item.id ? connection.targetInterfaceId : connection.sourceInterfaceId),
  }));

  return {
    id: component.id,
    name: component.name,
    componentType: component.componentType,
    category: component.category,
    parameters: component.properties.map((property) => {
      const parameter: RuntimeParameter = {
        componentId: component.id,
        propertyName: property.name,
        value: clone(property.value),
        ...(property.unit ? { unit: property.unit } : {}),
        source: property.origin,
        state: 'valid',
      };
      if (typeof property.value === 'number' && property.unit) {
        try {
          parameter.physicalQuantity = quantity(property.value, property.unit, {
            source: property.origin,
            ...(property.sourceRef ? { sourceRef: property.sourceRef } : {}),
          });
        } catch {
          parameter.state = 'invalid';
        }
      }
      return parameter;
    }),
    interfaces,
    modelContracts: contracts.filter((contract) => component.componentType === contract.componentType),
    constraints: clone(component.constraints),
    behaviors: clone(component.behaviors),
    tags: [...component.tags],
  };
}

function connectedComponents(reality: EngineeringReality, componentId: string): string[] {
  const interfaces = new Set(reality.interfaces.filter((item) => item.componentId === componentId).map((item) => item.id));
  const result = new Set<string>();
  for (const connection of reality.connections) {
    if (interfaces.has(connection.sourceInterfaceId)) {
      const target = reality.interfaces.find((item) => item.id === connection.targetInterfaceId);
      if (target) result.add(target.componentId);
    }
    if (interfaces.has(connection.targetInterfaceId)) {
      const source = reality.interfaces.find((item) => item.id === connection.sourceInterfaceId);
      if (source) result.add(source.componentId);
    }
  }
  result.delete(componentId);
  return [...result].sort();
}

function parseConstraintExpression(expression: string): { left: string; operator: string; right: number } | undefined {
  const match = expression.trim().match(/^([a-zA-Z0-9_.:-]+)\s*(<=|>=|==|!=|<|>)\s*(-?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)$/);
  if (!match) return undefined;
  return { left: match[1], operator: match[2], right: Number(match[3]) };
}

function compare(actual: number, operator: string, expected: number): boolean {
  switch (operator) {
    case '<': return actual < expected;
    case '<=': return actual <= expected;
    case '>': return actual > expected;
    case '>=': return actual >= expected;
    case '==': return Math.abs(actual - expected) <= Math.max(1e-12, Math.abs(expected) * 1e-12);
    case '!=': return Math.abs(actual - expected) > Math.max(1e-12, Math.abs(expected) * 1e-12);
    default: return false;
  }
}

export class EngineeringModelRuntimeImpl implements EngineeringModelRuntime {
  readonly schema = ENGINEERING_MODEL_RUNTIME_SCHEMA;
  private readonly parameterIndex = new Map<string, RuntimeParameter>();
  private readonly components = new Map<string, RuntimeComponent>();
  private modelRevisionValue: number;
  private realityValue: EngineeringReality;

  constructor(
    reality: EngineeringReality,
    private readonly modelContracts: EngineeringModelContract[] = [],
    private readonly componentDefinitions: ComponentDefinition[] = [],
  ) {
    const normalized = normalizeReality(reality);
    assertValidReality(normalized);
    this.realityValue = cloneReality(normalized);
    this.modelRevisionValue = normalized.revision;
    for (const component of normalized.components as EngineeringComponent[]) {
      const runtimeComponent = buildRuntimeComponent(component, normalized, modelContracts);
      this.components.set(component.id, runtimeComponent);
      for (const parameter of runtimeComponent.parameters) this.parameterIndex.set(parameterKey(parameter.componentId, parameter.propertyName), parameter);
    }
    this.refreshParameterValidity();
  }

  get reality(): EngineeringReality {
    return clone(this.realityValue);
  }

  get modelRevision(): number {
    return this.modelRevisionValue;
  }

  validate(): ModelValidationIssue[] {
    const issues: ModelValidationIssue[] = [];
    const realityResult = validateReality(this.realityValue);
    for (const issue of realityResult.issues.filter((item) => item.severity === 'error')) {
      issues.push({ code: 'reality_invalid', path: issue.path, message: issue.message, severity: 'error' });
    }

    for (const component of this.components.values()) {
      const matchingContracts = component.modelContracts;
      if (!matchingContracts.length && component.componentType !== 'generic_component') {
        issues.push({ code: 'missing_model', path: `components.${component.id}.model`, message: `No executable model contract is bound to ${component.componentType}.`, severity: 'warning' });
      }
      for (const parameter of component.parameters) {
        if (parameter.state === 'invalid') issues.push({ code: 'invalid_parameter', path: `components.${component.id}.properties.${parameter.propertyName}`, message: `Parameter ${parameter.propertyName} has an invalid value or unit.`, severity: 'error' });
      }
      for (const contract of matchingContracts) {
        for (const required of contract.requiredProperties.filter((item) => item.required)) {
          const parameter = component.parameters.find((item) => item.propertyName === required.name);
          if (!parameter) issues.push({ code: 'missing_parameter', path: `components.${component.id}.properties.${required.name}`, message: `Required model parameter ${required.name} is missing.`, severity: 'error' });
          else if (required.kind !== typeof parameter.value) issues.push({ code: 'invalid_parameter', path: `components.${component.id}.properties.${required.name}`, message: `Parameter ${required.name} must be ${required.kind}.`, severity: 'error' });
          else if (required.unit && parameter.unit !== required.unit && typeof parameter.value === 'number') {
            try { parameter.physicalQuantity = quantity(parameter.value, parameter.unit ?? required.unit); } catch { issues.push({ code: 'invalid_unit', path: `components.${component.id}.properties.${required.name}.unit`, message: `Parameter ${required.name} uses an unsupported or incompatible unit.`, severity: 'error' }); }
          }
        }
      }
    }

    for (const connection of this.realityValue.connections) {
      const source = this.realityValue.interfaces.find((item) => item.id === connection.sourceInterfaceId);
      const target = this.realityValue.interfaces.find((item) => item.id === connection.targetInterfaceId);
      if (!source || !target) issues.push({ code: 'topology_invalid', path: `connections.${connection.id}`, message: 'Connection topology references a missing interface.', severity: 'error' });
      else if (source.direction === 'in' && target.direction === 'in') issues.push({ code: 'topology_invalid', path: `connections.${connection.id}`, message: 'Two input-only interfaces cannot be directly connected.', severity: 'error' });
      else if (source.direction === 'out' && target.direction === 'out') issues.push({ code: 'topology_invalid', path: `connections.${connection.id}`, message: 'Two output-only interfaces cannot be directly connected.', severity: 'warning' });
    }

    for (const requirement of this.realityValue.requirements) {
      if (requirement.reviewStatus === 'accepted' && !requirement.verificationRule) {
        issues.push({ code: 'requirement_invalid', path: `requirements.${requirement.id}`, message: 'Accepted requirements should carry a verification rule before execution.', severity: 'warning' });
      }
    }

    for (const result of this.evaluateConstraints()) {
      if (!result.valid && result.severity === 'error') issues.push({ code: 'constraint_failed', path: `components.${result.componentId}.constraints.${result.constraintId}`, message: result.message, severity: 'error' });
      else if (!result.valid) issues.push({ code: 'constraint_failed', path: `components.${result.componentId}.constraints.${result.constraintId}`, message: result.message, severity: 'warning' });
    }
    return issues;
  }

  snapshot(): EngineeringModelSnapshot {
    const components = [...this.components.values()].map(clone);
    const parameters = components.flatMap((component) => component.parameters);
    return {
      schema: ENGINEERING_MODEL_SNAPSHOT_SCHEMA,
      runtimeSchema: ENGINEERING_MODEL_RUNTIME_SCHEMA,
      realityRevision: this.realityValue.revision,
      modelRevision: this.modelRevisionValue,
      capturedAt: new Date().toISOString(),
      components,
      connections: this.realityValue.connections.map((connection) => this.runtimeConnection(connection)),
      requirements: clone(this.realityValue.requirements),
      parameters,
      constraints: this.evaluateConstraints(),
    };
  }

  getComponent(componentId: string): RuntimeComponent | undefined {
    const component = this.components.get(componentId);
    return component ? clone(component) : undefined;
  }

  getParameter(componentId: string, propertyName: string): RuntimeParameter | undefined {
    const parameter = this.parameterIndex.get(parameterKey(componentId, propertyName));
    return parameter ? clone(parameter) : undefined;
  }

  setParameter(componentId: string, propertyName: string, value: string | number | boolean, unit?: string): void {
    const parameter = this.parameterIndex.get(parameterKey(componentId, propertyName));
    if (!parameter) throw new Error(`Unknown engineering parameter: ${componentId}.${propertyName}`);
    if (typeof value === 'number') finite(value, `${componentId}.${propertyName}`);
    const nextUnit = unit ?? parameter.unit;
    if (typeof value === 'number' && nextUnit) {
      try {
        parameter.physicalQuantity = quantity(value, nextUnit, { source: 'derived', sourceRef: `${componentId}.${propertyName}` });
      } catch (error) {
        parameter.state = 'invalid';
        throw error;
      }
    } else parameter.physicalQuantity = undefined;
    parameter.value = value;
    parameter.unit = nextUnit;
    parameter.source = 'runtime';
    parameter.state = 'valid';
    this.modelRevisionValue += 1;
    const realityComponent = this.realityValue.components.find((component) => component.id === componentId);
    if (!realityComponent) throw new Error(`Unknown reality component: ${componentId}`);
    const property = realityComponent.properties.find((item) => item.name === propertyName);
    if (property) {
      property.value = value;
      if (nextUnit) property.unit = nextUnit;
      property.origin = 'derived';
    }
    this.refreshParameterValidity();
  }

  evaluateConstraints(): ModelConstraintResult[] {
    const results: ModelConstraintResult[] = [];
    for (const component of this.components.values()) {
      for (const constraint of component.constraints.filter((item) => item.enabled)) {
        const parsed = parseConstraintExpression(constraint.expression);
        if (!parsed) {
          results.push({ constraintId: constraint.id, componentId: component.id, valid: false, severity: constraint.severity, expression: constraint.expression, message: 'Constraint expression is not supported by the deterministic runtime evaluator.' });
          continue;
        }
        const local = component.parameters.find((parameter) => parameter.propertyName === parsed.left);
        if (!local || typeof local.value !== 'number') {
          results.push({ constraintId: constraint.id, componentId: component.id, valid: false, severity: constraint.severity, expression: constraint.expression, message: `Constraint parameter ${parsed.left} is missing or non-numeric.` });
          continue;
        }
        const valid = compare(local.value, parsed.operator, parsed.right);
        results.push({ constraintId: constraint.id, componentId: component.id, valid, severity: constraint.severity, expression: constraint.expression, message: valid ? 'Constraint satisfied.' : `Constraint failed: ${local.value} ${parsed.operator} ${parsed.right}.` });
      }
    }
    return results;
  }

  private runtimeConnection(connection: RealityConnection): RuntimeConnection {
    const source = this.realityValue.interfaces.find((item) => item.id === connection.sourceInterfaceId);
    const target = this.realityValue.interfaces.find((item) => item.id === connection.targetInterfaceId);
    if (!source || !target) throw new Error(`Connection ${connection.id} references missing interface.`);
    return { id: connection.id, sourceInterfaceId: source.id, targetInterfaceId: target.id, connectionType: connection.connectionType, sourceComponentId: source.componentId, targetComponentId: target.componentId };
  }

  private refreshParameterValidity(): void {
    for (const parameter of this.parameterIndex.values()) {
      if (typeof parameter.value !== 'number' || !parameter.unit) continue;
      try {
        parameter.physicalQuantity = quantity(parameter.value, parameter.unit, { source: parameter.source === 'runtime' ? 'derived' : parameter.source });
        assertQuantity(parameter.physicalQuantity);
        parameter.state = 'valid';
      } catch {
        parameter.state = 'invalid';
      }
    }
  }

  connectedComponentIds(componentId: string): string[] {
    return connectedComponents(this.realityValue, componentId);
  }
}

export function createEngineeringModelRuntime(
  reality: EngineeringReality,
  modelContracts: EngineeringModelContract[] = [],
  componentDefinitions: ComponentDefinition[] = [],
): EngineeringModelRuntimeImpl {
  return new EngineeringModelRuntimeImpl(reality, modelContracts, componentDefinitions);
}

export function validateEngineeringModelRuntime(runtime: EngineeringModelRuntime): ModelValidationIssue[] {
  return runtime.validate();
}

export function assertEngineeringModelRuntime(runtime: EngineeringModelRuntime): void {
  const issues = runtime.validate();
  const errors = issues.filter((issue) => issue.severity === 'error');
  if (errors.length) throw new Error(`Engineering model runtime is invalid: ${errors.slice(0, 8).map((issue) => `${issue.path}: ${issue.message}`).join('; ')}`);
}

export function quantityRangeCheck(value: PhysicalQuantity, range: QuantityRange): boolean {
  return validateQuantityRange(value, range).valid;
}
