import {
  assertNonEmptyToken,
  cloneJson,
  deepFreeze,
  isJsonValue,
  isNonEmptyToken,
  isRecord,
  isValidDateTime,
  isoDateTime,
  realityId,
  type JsonValue,
  type IsoDateTime,
  type RealityId,
} from "./reality.js";

/** A declared operation that may be requested against a reality. */
export interface ActionDefinition {
  readonly id: string;
  readonly type: string;
  readonly description?: string;
  readonly inputSchema?: JsonValue;
  readonly outputSchema?: JsonValue;
  readonly metadata?: Readonly<Record<string, JsonValue>>;
}

/** A single client request to execute a declared action. */
export interface ActionInvocation<TInput extends JsonValue = JsonValue> {
  readonly invocationId: string;
  readonly realityId: RealityId;
  readonly actionId: string;
  readonly input: TInput;
  /** Stable key used by servers to deduplicate retried requests. */
  readonly idempotencyKey: string;
  readonly requestedAt?: IsoDateTime;
  readonly traceId?: string;
}

export type ActionOutcome<TOutput extends JsonValue = JsonValue> =
  | { readonly status: "succeeded"; readonly output: TOutput; readonly completedAt: IsoDateTime }
  | { readonly status: "accepted"; readonly operationId: string; readonly acceptedAt: IsoDateTime }
  | { readonly status: "failed"; readonly error: ActionFailure; readonly completedAt: IsoDateTime };

export interface ActionFailure {
  readonly code: string;
  readonly message: string;
  readonly retryable: boolean;
  readonly details?: JsonValue;
}

export class ActionValidationError extends TypeError {
  constructor(message: string) {
    super(message);
    this.name = "ActionValidationError";
  }
}

export function createActionDefinition(value: ActionDefinition): ActionDefinition {
  requireToken(value.id, "id");
  requireToken(value.type, "type");
  if (value.description !== undefined && !isNonEmptyToken(value.description)) {
    throw new ActionValidationError("description must be non-empty and trimmed when provided");
  }
  for (const field of ["inputSchema", "outputSchema"] as const) {
    if (value[field] !== undefined && !isJsonValue(value[field])) {
      throw new ActionValidationError(`${field} must contain only finite JSON values`);
    }
  }
  if (value.metadata !== undefined && (!isRecord(value.metadata) || !isJsonValue(value.metadata))) {
    throw new ActionValidationError("metadata must be an object containing only finite JSON values");
  }
  return deepFreeze(cloneJson(value));
}

export function createActionInvocation<TInput extends JsonValue>(
  value: ActionInvocation<TInput>,
): ActionInvocation<TInput> {
  requireToken(value.invocationId, "invocationId");
  requireToken(value.actionId, "actionId");
  requireToken(value.idempotencyKey, "idempotencyKey");
  requireToken(value.realityId, "realityId");
  if (!isJsonValue(value.input)) throw new ActionValidationError("input must contain only finite JSON values");
  if (value.requestedAt !== undefined) requireTimestamp(value.requestedAt, "requestedAt");
  if (value.traceId !== undefined) requireToken(value.traceId, "traceId");
  return deepFreeze(cloneJson(value)) as ActionInvocation<TInput>;
}

export function serializeActionInvocation(value: ActionInvocation): string {
  return JSON.stringify(createActionInvocation(value));
}

export function parseActionInvocation(serialized: string): ActionInvocation {
  let value: unknown;
  try {
    value = JSON.parse(serialized) as unknown;
  } catch (cause) {
    throw new SyntaxError("Action invocation is not valid JSON", { cause });
  }
  if (!isRecord(value)) throw new ActionValidationError("Action invocation must be an object");
  const allowed = new Set(["invocationId", "realityId", "actionId", "input", "idempotencyKey", "requestedAt", "traceId"]);
  const unknown = Object.keys(value).filter((key) => !allowed.has(key));
  if (unknown.length > 0) throw new ActionValidationError(`Unsupported action invocation fields: ${unknown.join(", ")}`);
  requireToken(value.realityId, "realityId");
  return createActionInvocation({
    invocationId: value.invocationId as string,
    realityId: realityId(value.realityId as string),
    actionId: value.actionId as string,
    input: value.input as JsonValue,
    idempotencyKey: value.idempotencyKey as string,
    ...(value.requestedAt === undefined ? {} : { requestedAt: isoDateTime(value.requestedAt as string) }),
    ...(value.traceId === undefined ? {} : { traceId: value.traceId as string }),
  });
}

export function validateActionOutcome(value: unknown): value is ActionOutcome {
  if (!isRecord(value)) return false;
  if (value.status === "succeeded") {
    return isJsonValue(value.output) && isValidDateTime(value.completedAt);
  }
  if (value.status === "accepted") {
    return isNonEmptyToken(value.operationId) && isValidDateTime(value.acceptedAt);
  }
  if (value.status === "failed") {
    const error = value.error;
    return isRecord(error) && isNonEmptyToken(error.code) && isNonEmptyToken(error.message) &&
      typeof error.retryable === "boolean" &&
      (error.details === undefined || isJsonValue(error.details)) && isValidDateTime(value.completedAt);
  }
  return false;
}

export function parseActionOutcome(serialized: string): ActionOutcome {
  let value: unknown;
  try {
    value = JSON.parse(serialized) as unknown;
  } catch (cause) {
    throw new SyntaxError("Action outcome is not valid JSON", { cause });
  }
  if (!validateActionOutcome(value)) throw new ActionValidationError("Invalid action outcome payload");
  const allowed = value.status === "succeeded"
    ? new Set(["status", "output", "completedAt"])
    : value.status === "accepted"
      ? new Set(["status", "operationId", "acceptedAt"])
      : new Set(["status", "error", "completedAt"]);
  const unknown = Object.keys(value).filter((key) => !allowed.has(key));
  if (unknown.length > 0) throw new ActionValidationError(`Unsupported action outcome fields: ${unknown.join(", ")}`);
  if (value.status === "failed" && isRecord(value.error)) {
    const errorFields = new Set(["code", "message", "retryable", "details"]);
    const unknownErrorFields = Object.keys(value.error).filter((key) => !errorFields.has(key));
    if (unknownErrorFields.length > 0) {
      throw new ActionValidationError(`Unsupported action error fields: ${unknownErrorFields.join(", ")}`);
    }
  }
  return deepFreeze(cloneJson(value));
}

export function serializeActionOutcome(value: ActionOutcome): string {
  if (!validateActionOutcome(value)) throw new ActionValidationError("Invalid action outcome");
  return JSON.stringify(value);
}

function requireToken(value: unknown, name: string): asserts value is string {
  try {
    assertNonEmptyToken(value, name);
  } catch (error) {
    throw new ActionValidationError(error instanceof Error ? error.message : `${name} is invalid`);
  }
}

function requireTimestamp(value: string, name: string): void {
  if (!isValidDateTime(value)) throw new ActionValidationError(`${name} must be an ISO-8601 timestamp with timezone`);
}
