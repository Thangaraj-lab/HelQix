/**
 * Public, transport-safe representation of a HELQIX reality.
 *
 * A reality is the versioned root document exchanged by SDK clients and the
 * HELQIX API. It deliberately contains data only: execution, scheduling and
 * persistence are responsibilities of higher layers.
 */

export const REALITY_SCHEMA_VERSION = 1 as const;

export type RealityId = string & { readonly __brand: "RealityId" };
export type IsoDateTime = string & { readonly __brand: "IsoDateTime" };

export interface RealityMetadata {
  readonly name: string;
  readonly description?: string;
  readonly tags?: readonly string[];
  readonly createdAt?: IsoDateTime;
  readonly updatedAt?: IsoDateTime;
  readonly extensions?: Readonly<Record<string, JsonValue>>;
}

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | readonly JsonValue[] | { readonly [key: string]: JsonValue };

/** Root resource exchanged by the SDK. Extension data must remain JSON-safe. */
export interface RealityDocument {
  readonly schemaVersion: typeof REALITY_SCHEMA_VERSION;
  readonly id: RealityId;
  readonly metadata: RealityMetadata;
  readonly components: readonly RealityComponentRef[];
  readonly behaviours?: readonly RealityBehaviourRef[];
  readonly actions?: readonly RealityActionRef[];
  readonly extensions?: Readonly<Record<string, JsonValue>>;
}

/** Minimal references keep the root document stable as component schemas evolve. */
export interface RealityComponentRef {
  readonly id: string;
  readonly type: string;
  readonly data?: JsonValue;
}

export interface RealityBehaviourRef {
  readonly id: string;
  readonly type: string;
  readonly config?: JsonValue;
}

export interface RealityActionRef {
  readonly id: string;
  readonly type: string;
  readonly input?: JsonValue;
}

export interface RealityValidationIssue {
  readonly path: string;
  readonly message: string;
}

export class RealityValidationError extends Error {
  readonly issues: readonly RealityValidationIssue[];

  constructor(issues: readonly RealityValidationIssue[]) {
    super(`Invalid reality document: ${issues.map(({ path, message }) => `${path}: ${message}`).join("; ")}`);
    this.name = "RealityValidationError";
    this.issues = Object.freeze([...issues]);
  }
}

export function realityId(value: string): RealityId {
  assertNonEmptyToken(value, "Reality id");
  return value as RealityId;
}

export function isoDateTime(value: string): IsoDateTime {
  if (!isValidDateTime(value)) throw new TypeError("Expected an ISO-8601 date-time with an explicit timezone");
  return value as IsoDateTime;
}

/** Validate untrusted API or persisted data without throwing. */
export function validateReality(value: unknown): readonly RealityValidationIssue[] {
  const issues: RealityValidationIssue[] = [];
  if (!isRecord(value)) return Object.freeze([{ path: "$", message: "must be an object" }]);
  collectUnknownKeys(value, ["schemaVersion", "id", "metadata", "components", "behaviours", "actions", "extensions"], "$", issues);

  if (value.schemaVersion !== REALITY_SCHEMA_VERSION) {
    issues.push({ path: "$.schemaVersion", message: `must equal ${REALITY_SCHEMA_VERSION}` });
  }
  if (!isNonEmptyToken(value.id)) {
    issues.push({ path: "$.id", message: "must be a non-empty string without surrounding whitespace" });
  }
  if (!isRecord(value.metadata)) {
    issues.push({ path: "$.metadata", message: "must be an object" });
  } else {
    collectUnknownKeys(value.metadata, ["name", "description", "tags", "createdAt", "updatedAt", "extensions"], "$.metadata", issues);
    if (!isNonEmptyToken(value.metadata.name)) {
      issues.push({ path: "$.metadata.name", message: "must be a non-empty, trimmed string" });
    }
    if (value.metadata.description !== undefined && typeof value.metadata.description !== "string") {
      issues.push({ path: "$.metadata.description", message: "must be a string when provided" });
    }
    if (value.metadata.tags !== undefined && (!Array.isArray(value.metadata.tags) || !value.metadata.tags.every(isNonEmptyToken))) {
      issues.push({ path: "$.metadata.tags", message: "must be an array of non-empty strings" });
    }
    for (const key of ["createdAt", "updatedAt"] as const) {
      const timestamp = value.metadata[key];
      if (timestamp !== undefined && (typeof timestamp !== "string" || !isValidDateTime(timestamp))) {
        issues.push({ path: `$.metadata.${key}`, message: "must be an ISO-8601 date-time with an explicit timezone" });
      }
    }
    if (value.metadata.extensions !== undefined &&
        (!isRecord(value.metadata.extensions) || !isJsonValue(value.metadata.extensions))) {
      issues.push({ path: "$.metadata.extensions", message: "must be an object containing only JSON values" });
    }
  }

  validateReferences(value.components, "$.components", issues, "data");
  if (value.behaviours !== undefined) validateReferences(value.behaviours, "$.behaviours", issues, "config");
  if (value.actions !== undefined) validateReferences(value.actions, "$.actions", issues, "input");
  if (value.extensions !== undefined && (!isRecord(value.extensions) || !isJsonValue(value.extensions))) {
    issues.push({ path: "$.extensions", message: "must be an object containing only JSON values" });
  }
  return Object.freeze(issues.map((issue) => Object.freeze(issue)));
}

/** Validate and return a detached, deeply frozen document safe to retain. */
export function createReality(value: unknown): RealityDocument {
  const issues = validateReality(value);
  if (issues.length > 0) throw new RealityValidationError(issues);
  return deepFreeze(cloneJson(value)) as RealityDocument;
}

/** Serialize a validated reality as compact JSON. */
export function serializeReality(value: RealityDocument): string {
  const issues = validateReality(value);
  if (issues.length > 0) throw new RealityValidationError(issues);
  return JSON.stringify(value);
}

/** Parse and validate a serialized reality from an API or storage boundary. */
export function parseReality(serialized: string): RealityDocument {
  let parsed: unknown;
  try {
    parsed = JSON.parse(serialized) as unknown;
  } catch (cause) {
    throw new SyntaxError("Reality payload is not valid JSON", { cause });
  }
  return createReality(parsed);
}

function validateReferences(
  value: unknown,
  path: string,
  issues: RealityValidationIssue[],
  payloadKey: "data" | "config" | "input",
): void {
  if (!Array.isArray(value)) {
    issues.push({ path, message: "must be an array" });
    return;
  }
  const seenIds = new Set<string>();
  value.forEach((entry: unknown, index: number) => {
    const entryPath = `${path}[${index}]`;
    if (!isRecord(entry)) {
      issues.push({ path: entryPath, message: "must be an object" });
      return;
    }
    collectUnknownKeys(entry, ["id", "type", payloadKey], entryPath, issues);
    if (!isNonEmptyToken(entry.id)) {
      issues.push({ path: `${entryPath}.id`, message: "must be a non-empty, trimmed string" });
    } else if (seenIds.has(entry.id)) {
      issues.push({ path: `${entryPath}.id`, message: `duplicate id ${JSON.stringify(entry.id)} in ${path}` });
    } else {
      seenIds.add(entry.id);
    }
    if (!isNonEmptyToken(entry.type)) {
      issues.push({ path: `${entryPath}.type`, message: "must be a non-empty, trimmed string" });
    }
    if (entry[payloadKey] !== undefined && !isJsonValue(entry[payloadKey])) {
      issues.push({ path: `${entryPath}.${payloadKey}`, message: "must contain only JSON values" });
    }
  });
}

function collectUnknownKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
  path: string,
  issues: RealityValidationIssue[],
): void {
  const accepted = new Set(allowed);
  for (const key of Object.keys(value)) {
    if (!accepted.has(key)) issues.push({ path: `${path}.${key}`, message: "is not a supported field" });
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isNonEmptyToken(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0 && value.trim() === value;
}

export function assertNonEmptyToken(value: unknown, name: string): asserts value is string {
  if (!isNonEmptyToken(value)) throw new TypeError(`${name} must be a non-empty trimmed string`);
}

export function isValidDateTime(value: unknown): value is IsoDateTime {
  if (typeof value !== "string") return false;
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(Z|([+-])(\d{2}):(\d{2}))$/.exec(value);
  if (!match) return false;
  const [, year, month, day, hour, minute, second, , , offsetHour, offsetMinute] = match;
  const y = Number(year);
  const m = Number(month);
  const d = Number(day);
  const daysInMonth = m === 2 ? (isLeapYear(y) ? 29 : 28) : [4, 6, 9, 11].includes(m) ? 30 : 31;
  return m >= 1 && m <= 12 && d >= 1 && d <= daysInMonth &&
    Number(hour) <= 23 && Number(minute) <= 59 && Number(second) <= 59 &&
    (offsetHour === undefined || (Number(offsetHour) <= 23 && Number(offsetMinute) <= 59)) &&
    Number.isFinite(Date.parse(value));
}

function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

export function isJsonValue(value: unknown, ancestors = new Set<object>()): value is JsonValue {
  if (value === null || typeof value === "string" || typeof value === "boolean") return true;
  if (typeof value === "number") return Number.isFinite(value);
  if (typeof value !== "object") return false;
  if (ancestors.has(value)) return false;
  ancestors.add(value);
  const valid = Array.isArray(value)
    ? value.every((entry) => isJsonValue(entry, ancestors))
    : Object.getPrototypeOf(value) === Object.prototype
      ? Object.values(value).every((entry) => isJsonValue(entry, ancestors))
      : false;
  ancestors.delete(value);
  return valid;
}

export function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  Object.freeze(value);
  for (const child of Object.values(value)) deepFreeze(child);
  return value;
}
