export const ENVIRONMENT_FABRIC_SCHEMA = 'helqix.environment-fabric.v1' as const;

export type EnvironmentMode = 'controlled' | 'recorded' | 'live' | 'synthetic' | 'hybrid';

export interface EnvironmentSource {
  id: string;
  kind: 'user' | 'sensor' | 'dataset' | 'provider' | 'simulation' | 'derived';
  label: string;
  reference?: string;
  confidence: number;
}

export interface EnvironmentValue {
  key: string;
  value: number;
  unit: string;
  timestampMs: number;
  source: EnvironmentSource;
  spatialReference?: string;
  validityStartMs?: number;
  validityEndMs?: number;
  metadata?: Readonly<Record<string, string | number | boolean>>;
}

export interface EnvironmentCondition {
  key: string;
  value: number;
  unit: string;
  min?: number;
  max?: number;
  source?: EnvironmentSource;
}

export interface EnvironmentDefinition {
  schema: typeof ENVIRONMENT_FABRIC_SCHEMA;
  id: string;
  revision: string;
  mode: EnvironmentMode;
  conditions: readonly EnvironmentCondition[];
  values: readonly EnvironmentValue[];
  coordinateFrame: string;
  createdAtMs: number;
}

export interface EnvironmentValidationIssue {
  path: string;
  code: string;
  message: string;
}

export interface EnvironmentValidationResult {
  valid: boolean;
  issues: readonly EnvironmentValidationIssue[];
}

export interface EnvironmentSample {
  key: string;
  value: number;
  unit: string;
  timestampMs: number;
  sourceId: string;
}

function finite(value: number, path: string, issues: EnvironmentValidationIssue[]): void {
  if (!Number.isFinite(value)) issues.push({ path, code: 'NON_FINITE', message: 'Value must be finite.' });
}

function assertConfidence(confidence: number, path: string, issues: EnvironmentValidationIssue[]): void {
  finite(confidence, path, issues);
  if (confidence < 0 || confidence > 1) issues.push({ path, code: 'INVALID_CONFIDENCE', message: 'Confidence must be between 0 and 1.' });
}

export function validateEnvironment(environment: EnvironmentDefinition): EnvironmentValidationResult {
  const issues: EnvironmentValidationIssue[] = [];
  if (environment.schema !== ENVIRONMENT_FABRIC_SCHEMA) {
    issues.push({ path: 'schema', code: 'SCHEMA_MISMATCH', message: `Expected ${ENVIRONMENT_FABRIC_SCHEMA}.` });
  }
  if (!environment.id.trim()) issues.push({ path: 'id', code: 'REQUIRED', message: 'Environment id is required.' });
  if (!environment.revision.trim()) issues.push({ path: 'revision', code: 'REQUIRED', message: 'Environment revision is required.' });
  if (!environment.coordinateFrame.trim()) issues.push({ path: 'coordinateFrame', code: 'REQUIRED', message: 'Coordinate frame is required.' });
  finite(environment.createdAtMs, 'createdAtMs', issues);

  const seen = new Set<string>();
  environment.values.forEach((value, index) => {
    const path = `values[${index}]`;
    if (seen.has(value.key)) issues.push({ path: `${path}.key`, code: 'DUPLICATE_KEY', message: `Duplicate environment key: ${value.key}.` });
    seen.add(value.key);
    if (!value.key.trim()) issues.push({ path: `${path}.key`, code: 'REQUIRED', message: 'Environment key is required.' });
    if (!value.unit.trim()) issues.push({ path: `${path}.unit`, code: 'REQUIRED', message: 'Environment unit is required.' });
    finite(value.value, `${path}.value`, issues);
    finite(value.timestampMs, `${path}.timestampMs`, issues);
    assertConfidence(value.source.confidence, `${path}.source.confidence`, issues);
    if (value.validityStartMs !== undefined) finite(value.validityStartMs, `${path}.validityStartMs`, issues);
    if (value.validityEndMs !== undefined) finite(value.validityEndMs, `${path}.validityEndMs`, issues);
    if (value.validityStartMs !== undefined && value.validityEndMs !== undefined && value.validityEndMs < value.validityStartMs) {
      issues.push({ path, code: 'INVALID_VALIDITY_WINDOW', message: 'Validity end must not precede validity start.' });
    }
  });

  environment.conditions.forEach((condition, index) => {
    const path = `conditions[${index}]`;
    finite(condition.value, `${path}.value`, issues);
    if (condition.min !== undefined) finite(condition.min, `${path}.min`, issues);
    if (condition.max !== undefined) finite(condition.max, `${path}.max`, issues);
    if (condition.min !== undefined && condition.max !== undefined && condition.max < condition.min) {
      issues.push({ path, code: 'INVALID_RANGE', message: 'Maximum must not be less than minimum.' });
    }
    if (condition.min !== undefined && condition.value < condition.min) issues.push({ path, code: 'BELOW_RANGE', message: 'Condition value is below its minimum.' });
    if (condition.max !== undefined && condition.value > condition.max) issues.push({ path, code: 'ABOVE_RANGE', message: 'Condition value is above its maximum.' });
  });

  return { valid: issues.length === 0, issues };
}

export function assertValidEnvironment(environment: EnvironmentDefinition): void {
  const result = validateEnvironment(environment);
  if (!result.valid) throw new Error(`Invalid environment: ${result.issues.map(issue => `${issue.path}: ${issue.message}`).join('; ')}`);
}

export function createEnvironment(
  id: string,
  revision: string,
  mode: EnvironmentMode,
  conditions: readonly EnvironmentCondition[] = [],
  values: readonly EnvironmentValue[] = [],
  coordinateFrame = 'global',
  createdAtMs = 0,
): EnvironmentDefinition {
  const environment: EnvironmentDefinition = {
    schema: ENVIRONMENT_FABRIC_SCHEMA,
    id,
    revision,
    mode,
    conditions: conditions.map(condition => ({ ...condition })),
    values: values.map(value => ({ ...value, source: { ...value.source } })),
    coordinateFrame,
    createdAtMs,
  };
  assertValidEnvironment(environment);
  return environment;
}

export function upsertEnvironmentValue(environment: EnvironmentDefinition, value: EnvironmentValue): EnvironmentDefinition {
  const values = environment.values.filter(existing => existing.key !== value.key);
  values.push({ ...value, source: { ...value.source } });
  values.sort((a, b) => a.key.localeCompare(b.key));
  return { ...environment, values };
}

export function sampleEnvironment(environment: EnvironmentDefinition, timestampMs: number): readonly EnvironmentSample[] {
  const candidates = environment.values.filter(value => {
    if (value.timestampMs > timestampMs) return false;
    if (value.validityStartMs !== undefined && timestampMs < value.validityStartMs) return false;
    if (value.validityEndMs !== undefined && timestampMs > value.validityEndMs) return false;
    return true;
  });
  const latest = new Map<string, EnvironmentValue>();
  for (const value of candidates) {
    const current = latest.get(value.key);
    if (!current || value.timestampMs > current.timestampMs || (value.timestampMs === current.timestampMs && value.source.id < current.source.id)) {
      latest.set(value.key, value);
    }
  }
  return [...latest.values()]
    .sort((a, b) => a.key.localeCompare(b.key))
    .map(value => ({ key: value.key, value: value.value, unit: value.unit, timestampMs: value.timestampMs, sourceId: value.source.id }));
}

export function environmentValue(environment: EnvironmentDefinition, key: string, timestampMs: number): EnvironmentValue | undefined {
  const samples = sampleEnvironment(environment, timestampMs);
  const sample = samples.find(item => item.key === key);
  if (!sample) return undefined;
  return environment.values.find(value => value.key === key && value.timestampMs === sample.timestampMs && value.source.id === sample.sourceId);
}

export function mergeEnvironment(base: EnvironmentDefinition, overlay: EnvironmentDefinition): EnvironmentDefinition {
  if (base.coordinateFrame !== overlay.coordinateFrame) throw new Error('Cannot merge environments with different coordinate frames.');
  const values = [...base.values];
  for (const value of overlay.values) {
    const index = values.findIndex(existing => existing.key === value.key && existing.timestampMs === value.timestampMs);
    if (index >= 0) values[index] = { ...value, source: { ...value.source } };
    else values.push({ ...value, source: { ...value.source } });
  }
  return {
    ...base,
    revision: overlay.revision,
    mode: overlay.mode,
    conditions: [...base.conditions, ...overlay.conditions],
    values: values.sort((a, b) => a.key.localeCompare(b.key) || a.timestampMs - b.timestampMs),
  };
}
