import type {
  EngineeringReality,
  EngineeringRequirement,
  ModelBinding,
  RealityComponent,
  RealityConnection,
  RealityInterface,
  RealityProperty,
} from '../api';

/**
 * Generic engineering-reality utilities.
 *
 * This module is deliberately domain-neutral. A battery, drone, robot, vehicle,
 * pump, spacecraft, or any other engineered system enters HELQIX through the
 * same model contract. Domain-specific solvers belong behind model bindings and
 * adapters; this layer only describes and validates the engineering reality.
 */

export const ENGINEERING_REALITY_SCHEMA = 'helqix.engineering-reality.v1' as const;
export const ENGINEERING_MODEL_CONTRACT_SCHEMA = 'helqix.engineering-model-contract.v1' as const;

export type ScalarValue = string | number | boolean;
export type ComponentId = string;
export type InterfaceId = string;
export type ConnectionId = string;
export type RequirementId = string;
export type BindingId = string;

export type ComponentCategory =
  | 'assembly'
  | 'structure'
  | 'mechanical'
  | 'electrical'
  | 'electronics'
  | 'thermal'
  | 'fluid'
  | 'control'
  | 'software'
  | 'sensor'
  | 'actuator'
  | 'energy'
  | 'environment'
  | 'unknown';

export type PropertyValueKind = 'string' | 'number' | 'boolean';

export interface ComponentConstraint {
  id: string;
  name: string;
  expression: string;
  severity: 'error' | 'warning' | 'info';
  source: 'user' | 'catalog' | 'import' | 'model';
  enabled: boolean;
}

export interface ComponentBehavior {
  id: string;
  name: string;
  inputInterfaceIds: string[];
  outputInterfaceIds: string[];
  description: string;
  source: 'user' | 'catalog' | 'import' | 'model';
}

export interface ComponentPlacement {
  x: number;
  y: number;
  z: number;
  rotationX: number;
  rotationY: number;
  rotationZ: number;
  coordinateSystem: string;
}

export interface EngineeringComponent extends RealityComponent {
  category: ComponentCategory;
  constraints: ComponentConstraint[];
  behaviors: ComponentBehavior[];
  placement?: ComponentPlacement;
  tags: string[];
}

export interface EngineeringModelContract {
  schema: typeof ENGINEERING_MODEL_CONTRACT_SCHEMA;
  id: string;
  name: string;
  version: string;
  domain: string;
  componentType: string;
  description: string;
  requiredProperties: Array<{
    name: string;
    kind: PropertyValueKind;
    unit?: string;
    required: boolean;
  }>;
  acceptedInterfaceDomains: string[];
  fidelity: string;
  assumptions: string[];
  limitations: string[];
  source: 'catalog' | 'import' | 'user' | 'adapter';
}

export interface EngineeringRealityDocument {
  reality: EngineeringReality;
  components: EngineeringComponent[];
  modelContracts: EngineeringModelContract[];
  revision: number;
}

export interface RealityValidationIssue {
  code:
  | 'invalid_schema'
  | 'invalid_revision'
  | 'duplicate_component'
  | 'empty_component_id'
  | 'empty_component_name'
  | 'invalid_component_type'
  | 'duplicate_interface'
  | 'unknown_interface_component'
  | 'duplicate_connection'
  | 'unknown_connection_source'
  | 'unknown_connection_target'
  | 'self_connection'
  | 'invalid_requirement'
  | 'invalid_binding'
  | 'binding_component_missing'
  | 'property_invalid'
  | 'constraint_invalid'
  | 'behavior_invalid';
  path: string;
  message: string;
  severity: 'error' | 'warning';
}

export interface RealityValidationResult {
  valid: boolean;
  issues: RealityValidationIssue[];
  errorCount: number;
  warningCount: number;
}

export interface RealityDiffEntry {
  path: string;
  kind: 'added' | 'removed' | 'changed';
  before?: unknown;
  after?: unknown;
}

export interface RealityDiff {
  fromRevision: number;
  toRevision: number;
  changed: boolean;
  entries: RealityDiffEntry[];
}

export interface RealityLineage {
  realityRevision: number;
  projectVersion?: number;
  componentIds: string[];
  modelBindingIds: string[];
  requirementIds: string[];
  generatedAt: string;
}

export interface RealityMutationResult {
  reality: EngineeringReality;
  changedPaths: string[];
  nextRevision: number;
}

export interface PropertyDefinition {
  name: string;
  kind: PropertyValueKind;
  unit?: string;
  required?: boolean;
  min?: number;
  max?: number;
  allowedValues?: ScalarValue[];
}

export interface ComponentDefinition {
  componentType: string;
  label: string;
  category: ComponentCategory;
  description: string;
  propertyDefinitions: PropertyDefinition[];
  interfaceDomains: string[];
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function normalizeText(value: string): string {
  return value.trim().replace(/\s+/g, ' ');
}

function normalizeId(value: string, fallback: string): string {
  const normalized = normalizeText(value)
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 120);
  return normalized || fallback;
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values.map(normalizeText).filter(Boolean))];
}

export function createId(prefix: string, source = ''): string {
  const base = normalizeId(source, prefix);
  const suffix = typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? crypto.randomUUID().replaceAll('-', '').slice(0, 12)
    : Math.random().toString(36).slice(2, 14);
  return `${base}_${suffix}`;
}

export function createEmptyReality(
  name: string,
  domain = 'general',
  description = '',
): EngineeringReality {
  const normalizedName = normalizeText(name) || 'Untitled engineering reality';
  const normalizedDomain = normalizeId(domain, 'general');
  return {
    schema: ENGINEERING_REALITY_SCHEMA,
    name: normalizedName,
    domain: normalizedDomain,
    description: normalizeText(description),
    revision: 1,
    unitsSystem: 'SI',
    components: [],
    interfaces: [],
    connections: [],
    requirements: [],
    modelBindings: [],
  };
}

export function cloneReality(reality: EngineeringReality): EngineeringReality {
  return clone(reality);
}

export function normalizeReality(reality: EngineeringReality): EngineeringReality {
  const next = clone(reality);
  next.schema = ENGINEERING_REALITY_SCHEMA;
  next.name = normalizeText(next.name) || 'Untitled engineering reality';
  next.domain = normalizeId(next.domain, 'general');
  next.description = normalizeText(next.description);
  next.unitsSystem = 'SI';
  next.components = next.components.map((component) => ({
    ...component,
    id: normalizeId(component.id, createId('component')),
    name: normalizeText(component.name) || 'Unnamed component',
    componentType: normalizeText(component.componentType) || 'generic_component',
    properties: component.properties.map((property) => ({
      ...property,
      name: normalizeText(property.name),
      ...(typeof property.value === 'string' ? { value: normalizeText(property.value) } : {}),
    })),
  }));
  next.interfaces = next.interfaces.map((item) => ({
    ...item,
    id: normalizeId(item.id, createId('interface')),
    componentId: normalizeText(item.componentId),
    name: normalizeText(item.name),
    domain: normalizeText(item.domain) || 'generic',
  }));
  next.connections = next.connections.map((item) => ({
    ...item,
    id: normalizeId(item.id, createId('connection')),
    sourceInterfaceId: normalizeText(item.sourceInterfaceId),
    targetInterfaceId: normalizeText(item.targetInterfaceId),
    connectionType: normalizeText(item.connectionType) || 'direct',
  }));
  next.requirements = next.requirements.map((item) => ({
    ...item,
    id: normalizeId(item.id, createId('requirement')),
    statement: normalizeText(item.statement),
  }));
  next.modelBindings = next.modelBindings.map((item) => ({
    ...item,
    id: normalizeId(item.id, createId('binding')),
    modelId: normalizeText(item.modelId),
    modelVersion: normalizeText(item.modelVersion),
    scopeComponentId: normalizeText(item.scopeComponentId),
    declaredFidelity: normalizeText(item.declaredFidelity),
  }));
  return next;
}

function propertyKind(value: ScalarValue): PropertyValueKind {
  if (typeof value === 'boolean') return 'boolean';
  if (typeof value === 'number') return 'number';
  return 'string';
}

function propertyAt(component: RealityComponent, name: string): RealityProperty | undefined {
  return component.properties.find((property) => property.name === name);
}

function validateProperty(
  property: RealityProperty,
  path: string,
  issues: RealityValidationIssue[],
): void {
  if (!property.name.trim()) {
    issues.push({ code: 'property_invalid', path, message: 'Property name is required.', severity: 'error' });
  }
  if (typeof property.value === 'number' && !Number.isFinite(property.value)) {
    issues.push({ code: 'property_invalid', path: `${path}.value`, message: 'Numeric property values must be finite.', severity: 'error' });
  }
}

export function validateReality(reality: EngineeringReality): RealityValidationResult {
  const issues: RealityValidationIssue[] = [];
  if (reality.schema !== ENGINEERING_REALITY_SCHEMA) {
    issues.push({ code: 'invalid_schema', path: 'schema', message: `Expected ${ENGINEERING_REALITY_SCHEMA}.`, severity: 'error' });
  }
  if (!Number.isInteger(reality.revision) || reality.revision < 1) {
    issues.push({ code: 'invalid_revision', path: 'revision', message: 'Reality revision must be a positive integer.', severity: 'error' });
  }

  const components = new Set<string>();
  for (const component of reality.components) {
    if (!component.id.trim()) issues.push({ code: 'empty_component_id', path: 'components', message: 'Every component requires an id.', severity: 'error' });
    if (components.has(component.id)) issues.push({ code: 'duplicate_component', path: `components.${component.id}`, message: 'Component ids must be unique.', severity: 'error' });
    components.add(component.id);
    if (!component.name.trim()) issues.push({ code: 'empty_component_name', path: `components.${component.id}.name`, message: 'Component name is required.', severity: 'error' });
    if (!component.componentType.trim()) issues.push({ code: 'invalid_component_type', path: `components.${component.id}.componentType`, message: 'Component type is required.', severity: 'error' });
    const properties = new Set<string>();
    component.properties.forEach((property, index) => {
      validateProperty(property, `components.${component.id}.properties.${index}`, issues);
      if (properties.has(property.name)) issues.push({ code: 'property_invalid', path: `components.${component.id}.properties.${property.name}`, message: 'Property names must be unique within a component.', severity: 'error' });
      properties.add(property.name);
    });
  }

  const interfaces = new Map<string, RealityInterface>();
  for (const item of reality.interfaces) {
    if (interfaces.has(item.id)) issues.push({ code: 'duplicate_interface', path: `interfaces.${item.id}`, message: 'Interface ids must be unique.', severity: 'error' });
    interfaces.set(item.id, item);
    if (!components.has(item.componentId)) issues.push({ code: 'unknown_interface_component', path: `interfaces.${item.id}.componentId`, message: `Interface references unknown component ${item.componentId}.`, severity: 'error' });
    if (!item.name.trim() || !item.domain.trim()) issues.push({ code: 'invalid_component_type', path: `interfaces.${item.id}`, message: 'Interface name and domain are required.', severity: 'error' });
  }

  const connections = new Set<string>();
  for (const connection of reality.connections) {
    if (connections.has(connection.id)) issues.push({ code: 'duplicate_connection', path: `connections.${connection.id}`, message: 'Connection ids must be unique.', severity: 'error' });
    connections.add(connection.id);
    const source = interfaces.get(connection.sourceInterfaceId);
    const target = interfaces.get(connection.targetInterfaceId);
    if (!source) issues.push({ code: 'unknown_connection_source', path: `connections.${connection.id}.sourceInterfaceId`, message: `Unknown source interface ${connection.sourceInterfaceId}.`, severity: 'error' });
    if (!target) issues.push({ code: 'unknown_connection_target', path: `connections.${connection.id}.targetInterfaceId`, message: `Unknown target interface ${connection.targetInterfaceId}.`, severity: 'error' });
    if (source && target && source.componentId === target.componentId) issues.push({ code: 'self_connection', path: `connections.${connection.id}`, message: 'A connection cannot connect two interfaces on the same component.', severity: 'warning' });
  }

  for (const requirement of reality.requirements) {
    validateRequirement(requirement, issues);
  }

  for (const binding of reality.modelBindings) {
    validateBinding(binding, components, issues);
  }

  const errorCount = issues.filter((issue) => issue.severity === 'error').length;
  const warningCount = issues.length - errorCount;
  return { valid: errorCount === 0, issues, errorCount, warningCount };
}

function validateRequirement(requirement: EngineeringRequirement, issues: RealityValidationIssue[]): void {
  if (!requirement.id.trim()) issues.push({ code: 'invalid_requirement', path: 'requirements', message: 'Requirement id is required.', severity: 'error' });
  if (!requirement.statement.trim()) issues.push({ code: 'invalid_requirement', path: `requirements.${requirement.id}.statement`, message: 'Requirement statement is required.', severity: 'error' });
  if (!['must', 'should', 'could'].includes(requirement.priority)) issues.push({ code: 'invalid_requirement', path: `requirements.${requirement.id}.priority`, message: 'Requirement priority must be must, should, or could.', severity: 'error' });
  if (!['draft', 'accepted'].includes(requirement.reviewStatus)) issues.push({ code: 'invalid_requirement', path: `requirements.${requirement.id}.reviewStatus`, message: 'Requirement review status must be draft or accepted.', severity: 'error' });
  if (requirement.verificationRule) {
    const rule = requirement.verificationRule;
    if (!Number.isFinite(rule.threshold)) issues.push({ code: 'invalid_requirement', path: `requirements.${requirement.id}.verificationRule.threshold`, message: 'Verification threshold must be finite.', severity: 'error' });
  }
}

function validateBinding(binding: ModelBinding, components: Set<string>, issues: RealityValidationIssue[]): void {
  if (!binding.id.trim() || !binding.modelId.trim() || !binding.modelVersion.trim()) issues.push({ code: 'invalid_binding', path: `modelBindings.${binding.id}`, message: 'Model binding id, model id, and version are required.', severity: 'error' });
  if (!components.has(binding.scopeComponentId)) issues.push({ code: 'binding_component_missing', path: `modelBindings.${binding.id}.scopeComponentId`, message: `Model binding references unknown component ${binding.scopeComponentId}.`, severity: 'error' });
}

export function assertValidReality(reality: EngineeringReality): void {
  const result = validateReality(reality);
  if (!result.valid) {
    const summary = result.issues.filter((issue) => issue.severity === 'error').slice(0, 8).map((issue) => `${issue.path}: ${issue.message}`).join('; ');
    throw new Error(`Engineering reality is invalid. ${summary}`);
  }
}

export function createComponent(
  name: string,
  componentType: string,
  options: Partial<Pick<EngineeringComponent, 'category' | 'tags' | 'constraints' | 'behaviors' | 'placement'>> & { properties?: RealityProperty[] } = {},
): EngineeringComponent {
  const cleanName = normalizeText(name) || 'Unnamed component';
  const cleanType = normalizeText(componentType) || 'generic_component';
  return {
    id: createId('component', cleanName),
    name: cleanName,
    componentType: cleanType,
    properties: clone(options.properties ?? []),
    category: options.category ?? 'unknown',
    constraints: clone(options.constraints ?? []),
    behaviors: clone(options.behaviors ?? []),
    placement: options.placement ? clone(options.placement) : undefined,
    tags: uniqueStrings(options.tags ?? []),
  };
}

export function addComponent(
  reality: EngineeringReality,
  component: RealityComponent,
): RealityMutationResult {
  if (reality.components.some((item) => item.id === component.id)) throw new Error(`Component already exists: ${component.id}`);
  const next = cloneReality(reality);
  next.components.push(clone(component));
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`components.${component.id}`], nextRevision: next.revision };
}

export function removeComponent(reality: EngineeringReality, componentId: string): RealityMutationResult {
  if (!reality.components.some((item) => item.id === componentId)) throw new Error(`Unknown component: ${componentId}`);
  const interfaceIds = new Set(reality.interfaces.filter((item) => item.componentId === componentId).map((item) => item.id));
  const next = cloneReality(reality);
  next.components = next.components.filter((item) => item.id !== componentId);
  next.interfaces = next.interfaces.filter((item) => item.componentId !== componentId);
  next.connections = next.connections.filter((item) => !interfaceIds.has(item.sourceInterfaceId) && !interfaceIds.has(item.targetInterfaceId));
  next.modelBindings = next.modelBindings.filter((item) => item.scopeComponentId !== componentId);
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`components.${componentId}`, 'interfaces', 'connections', 'modelBindings'], nextRevision: next.revision };
}

export function updateComponent(
  reality: EngineeringReality,
  componentId: string,
  patch: Partial<Pick<RealityComponent, 'name' | 'componentType' | 'properties'>>,
): RealityMutationResult {
  const next = cloneReality(reality);
  const component = next.components.find((item) => item.id === componentId);
  if (!component) throw new Error(`Unknown component: ${componentId}`);
  if (patch.name !== undefined) component.name = normalizeText(patch.name);
  if (patch.componentType !== undefined) component.componentType = normalizeText(patch.componentType);
  if (patch.properties !== undefined) component.properties = clone(patch.properties);
  next.revision = reality.revision + 1;
  const changedPaths = Object.keys(patch).map((key) => `components.${componentId}.${key}`);
  return { reality: next, changedPaths, nextRevision: next.revision };
}

export function setComponentProperty(
  reality: EngineeringReality,
  componentId: string,
  name: string,
  value: ScalarValue,
  unit?: string,
  origin: RealityProperty['origin'] = 'user',
): RealityMutationResult {
  const next = cloneReality(reality);
  const component = next.components.find((item) => item.id === componentId);
  if (!component) throw new Error(`Unknown component: ${componentId}`);
  const propertyName = normalizeText(name);
  if (!propertyName) throw new Error('Property name is required.');
  const existing = component.properties.find((item) => item.name === propertyName);
  if (existing) {
    existing.value = value;
    existing.origin = origin;
    if (unit !== undefined) existing.unit = unit;
  } else {
    component.properties.push({ name: propertyName, value, ...(unit ? { unit } : {}), origin });
  }
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`components.${componentId}.properties.${propertyName}`], nextRevision: next.revision };
}

export function addInterface(
  reality: EngineeringReality,
  componentId: string,
  name: string,
  direction: RealityInterface['direction'],
  domain: string,
): RealityMutationResult {
  if (!reality.components.some((item) => item.id === componentId)) throw new Error(`Unknown component: ${componentId}`);
  const cleanName = normalizeText(name);
  const cleanDomain = normalizeText(domain);
  if (!cleanName || !cleanDomain) throw new Error('Interface name and domain are required.');
  const duplicate = reality.interfaces.some((item) => item.componentId === componentId && item.name.toLowerCase() === cleanName.toLowerCase());
  if (duplicate) throw new Error(`Interface already exists on component: ${cleanName}`);
  const next = cloneReality(reality);
  const item: RealityInterface = { id: createId('interface', cleanName), componentId, name: cleanName, direction, domain: cleanDomain };
  next.interfaces.push(item);
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`interfaces.${item.id}`], nextRevision: next.revision };
}

export function addConnection(
  reality: EngineeringReality,
  sourceInterfaceId: string,
  targetInterfaceId: string,
  connectionType = 'direct',
): RealityMutationResult {
  const source = reality.interfaces.find((item) => item.id === sourceInterfaceId);
  const target = reality.interfaces.find((item) => item.id === targetInterfaceId);
  if (!source || !target) throw new Error('Both connection endpoints must reference existing interfaces.');
  if (source.componentId === target.componentId) throw new Error('A connection must cross component boundaries.');
  if (reality.connections.some((item) => item.sourceInterfaceId === sourceInterfaceId && item.targetInterfaceId === targetInterfaceId)) throw new Error('That interface connection already exists.');
  const next = cloneReality(reality);
  const item: RealityConnection = { id: createId('connection'), sourceInterfaceId, targetInterfaceId, connectionType: normalizeText(connectionType) || 'direct' };
  next.connections.push(item);
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`connections.${item.id}`], nextRevision: next.revision };
}

export function removeConnection(reality: EngineeringReality, connectionId: string): RealityMutationResult {
  if (!reality.connections.some((item) => item.id === connectionId)) throw new Error(`Unknown connection: ${connectionId}`);
  const next = cloneReality(reality);
  next.connections = next.connections.filter((item) => item.id !== connectionId);
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`connections.${connectionId}`], nextRevision: next.revision };
}

export function addModelBinding(reality: EngineeringReality, binding: ModelBinding): RealityMutationResult {
  if (!reality.components.some((item) => item.id === binding.scopeComponentId)) throw new Error(`Unknown binding component: ${binding.scopeComponentId}`);
  if (reality.modelBindings.some((item) => item.id === binding.id)) throw new Error(`Model binding already exists: ${binding.id}`);
  const next = cloneReality(reality);
  next.modelBindings.push(clone(binding));
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`modelBindings.${binding.id}`], nextRevision: next.revision };
}

export function addRequirement(reality: EngineeringReality, requirement: EngineeringRequirement): RealityMutationResult {
  if (reality.requirements.some((item) => item.id === requirement.id)) throw new Error(`Requirement already exists: ${requirement.id}`);
  if (!requirement.statement.trim()) throw new Error('Requirement statement is required.');
  const next = cloneReality(reality);
  next.requirements.push(clone(requirement));
  next.revision = reality.revision + 1;
  return { reality: next, changedPaths: [`requirements.${requirement.id}`], nextRevision: next.revision };
}

export function componentCategory(componentType: string): ComponentCategory {
  const type = componentType.toLowerCase();
  if (/battery|cell|pack|power|fuel|energy/.test(type)) return 'energy';
  if (/motor|engine|pump|gear|actuator|wheel|bearing/.test(type)) return 'mechanical';
  if (/sensor|camera|imu|gps|lidar|radar/.test(type)) return 'sensor';
  if (/controller|ecu|bms|flight_control|control/.test(type)) return 'control';
  if (/wire|bus|cable|connector|inverter|converter|electrical/.test(type)) return 'electrical';
  if (/thermal|heater|cooler|radiator/.test(type)) return 'thermal';
  if (/fluid|valve|pipe|tank|hydraulic|pneumatic/.test(type)) return 'fluid';
  if (/frame|body|chassis|structure|beam|bracket/.test(type)) return 'structure';
  if (/software|service|agent|algorithm/.test(type)) return 'software';
  if (/assembly|system|subsystem/.test(type)) return 'assembly';
  return 'unknown';
}

export function inferComponentDefinition(componentType: string): ComponentDefinition {
  const type = normalizeText(componentType) || 'generic_component';
  const category = componentCategory(type);
  return {
    componentType: type,
    label: type.replace(/[_-]+/g, ' ').replace(/\b\w/g, (letter) => letter.toUpperCase()),
    category,
    description: `Generic ${type} engineering component.`,
    propertyDefinitions: [],
    interfaceDomains: ['generic'],
  };
}

export function realityLineage(reality: EngineeringReality, projectVersion?: number): RealityLineage {
  return {
    realityRevision: reality.revision,
    ...(projectVersion === undefined ? {} : { projectVersion }),
    componentIds: reality.components.map((item) => item.id),
    modelBindingIds: reality.modelBindings.map((item) => item.id),
    requirementIds: reality.requirements.map((item) => item.id),
    generatedAt: new Date().toISOString(),
  };
}

function stableValue(value: unknown): string {
  if (value === null || value === undefined) return String(value);
  if (typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stableValue).join(',')}]`;
  const object = value as Record<string, unknown>;
  return `{${Object.keys(object).sort().map((key) => `${JSON.stringify(key)}:${stableValue(object[key])}`).join(',')}}`;
}

export function hashableReality(reality: EngineeringReality): string {
  return stableValue({ ...reality, revision: undefined });
}

export function diffReality(before: EngineeringReality, after: EngineeringReality): RealityDiff {
  const entries: RealityDiffEntry[] = [];
  const compareMaps = <T extends { id: string }>(
    beforeItems: T[],
    afterItems: T[],
    path: string,
  ) => {
    const beforeMap = new Map(beforeItems.map((item) => [item.id, item]));
    const afterMap = new Map(afterItems.map((item) => [item.id, item]));
    for (const [id, item] of afterMap) {
      if (!beforeMap.has(id)) entries.push({ path: `${path}.${id}`, kind: 'added', after: clone(item) });
      else if (stableValue(beforeMap.get(id)) !== stableValue(item)) entries.push({ path: `${path}.${id}`, kind: 'changed', before: clone(beforeMap.get(id)), after: clone(item) });
    }
    for (const [id, item] of beforeMap) {
      if (!afterMap.has(id)) entries.push({ path: `${path}.${id}`, kind: 'removed', before: clone(item) });
    }
  };
  if (before.name !== after.name) entries.push({ path: 'name', kind: 'changed', before: before.name, after: after.name });
  if (before.domain !== after.domain) entries.push({ path: 'domain', kind: 'changed', before: before.domain, after: after.domain });
  if (before.description !== after.description) entries.push({ path: 'description', kind: 'changed', before: before.description, after: after.description });
  compareMaps(before.components, after.components, 'components');
  compareMaps(before.interfaces, after.interfaces, 'interfaces');
  compareMaps(before.connections, after.connections, 'connections');
  compareMaps(before.requirements, after.requirements, 'requirements');
  compareMaps(before.modelBindings, after.modelBindings, 'modelBindings');
  return {
    fromRevision: before.revision,
    toRevision: after.revision,
    changed: entries.length > 0,
    entries,
  };
}

export function applyPropertyDefinitions(
  component: RealityComponent,
  definitions: PropertyDefinition[],
): RealityValidationIssue[] {
  const issues: RealityValidationIssue[] = [];
  for (const definition of definitions) {
    const property = propertyAt(component, definition.name);
    if (!property) {
      if (definition.required) issues.push({ code: 'property_invalid', path: `components.${component.id}.properties.${definition.name}`, message: `Required property ${definition.name} is missing.`, severity: 'error' });
      continue;
    }
    if (propertyKind(property.value) !== definition.kind) issues.push({ code: 'property_invalid', path: `components.${component.id}.properties.${definition.name}`, message: `Property ${definition.name} must be a ${definition.kind}.`, severity: 'error' });
    if (definition.kind === 'number' && typeof property.value === 'number') {
      if (definition.min !== undefined && property.value < definition.min) issues.push({ code: 'property_invalid', path: `components.${component.id}.properties.${definition.name}`, message: `${definition.name} must be >= ${definition.min}.`, severity: 'error' });
      if (definition.max !== undefined && property.value > definition.max) issues.push({ code: 'property_invalid', path: `components.${component.id}.properties.${definition.name}`, message: `${definition.name} must be <= ${definition.max}.`, severity: 'error' });
    }
    if (definition.allowedValues && !definition.allowedValues.some((allowed) => stableValue(allowed) === stableValue(property.value))) issues.push({ code: 'property_invalid', path: `components.${component.id}.properties.${definition.name}`, message: `${definition.name} is outside its declared allowed values.`, severity: 'error' });
  }
  return issues;
}

export function ensureRealityRevision(reality: EngineeringReality, expectedRevision: number): void {
  if (reality.revision !== expectedRevision) throw new Error(`Reality revision conflict: expected ${expectedRevision}, received ${reality.revision}. Reload the project before applying changes.`);
}

export function isEngineeringReality(value: unknown): value is EngineeringReality {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<EngineeringReality>;
  return candidate.schema === ENGINEERING_REALITY_SCHEMA
    && typeof candidate.name === 'string'
    && typeof candidate.domain === 'string'
    && Number.isInteger(candidate.revision)
    && Array.isArray(candidate.components)
    && Array.isArray(candidate.interfaces)
    && Array.isArray(candidate.connections)
    && Array.isArray(candidate.requirements)
    && Array.isArray(candidate.modelBindings);
}
