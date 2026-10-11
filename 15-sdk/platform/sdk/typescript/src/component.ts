import {
  assertNonEmptyToken,
  cloneJson,
  deepFreeze,
  isJsonValue,
  isNonEmptyToken,
  isRecord,
  type JsonValue,
} from "./reality.js";

/** A versioned, typed piece of state attached to a reality or entity. */
export interface Component<TData extends JsonValue = JsonValue> {
  readonly id: string;
  readonly type: string;
  readonly version: number;
  readonly data: TData;
  readonly metadata?: Readonly<Record<string, JsonValue>>;
}

/** Stable identity for updating/removing one component instance. */
export interface ComponentKey {
  readonly ownerId: string;
  readonly componentId: string;
}

export class ComponentValidationError extends TypeError {
  constructor(message: string) {
    super(message);
    this.name = "ComponentValidationError";
  }
}

export function createComponent<TData extends JsonValue>(component: Component<TData>): Component<TData> {
  requireToken(component.id, "id");
  requireToken(component.type, "type");
  if (!Number.isSafeInteger(component.version) || component.version < 1) {
    throw new ComponentValidationError("version must be a positive safe integer");
  }
  if (!isJsonValue(component.data)) throw new ComponentValidationError("data must contain only finite JSON values");
  if (component.metadata !== undefined && (!isRecord(component.metadata) || !isJsonValue(component.metadata))) {
    throw new ComponentValidationError("metadata must be an object containing only finite JSON values");
  }
  return deepFreeze(cloneJson(component)) as Component<TData>;
}

export function serializeComponent<TData extends JsonValue>(component: Component<TData>): string {
  return JSON.stringify(createComponent(component));
}

export function parseComponent(serialized: string): Component {
  let value: unknown;
  try {
    value = JSON.parse(serialized) as unknown;
  } catch (cause) {
    throw new SyntaxError("Component payload is not valid JSON", { cause });
  }
  if (!isRecord(value)) throw new ComponentValidationError("component payload must be an object");
  const allowed = new Set(["id", "type", "version", "data", "metadata"]);
  const unknown = Object.keys(value).filter((key) => !allowed.has(key));
  if (unknown.length > 0) throw new ComponentValidationError(`Unsupported component fields: ${unknown.join(", ")}`);
  return createComponent(value as unknown as Component);
}

export function componentKey(ownerId: string, componentId: string): ComponentKey {
  requireToken(ownerId, "ownerId");
  requireToken(componentId, "componentId");
  return Object.freeze({ ownerId, componentId });
}

export function isComponent(value: unknown): value is Component {
  if (!isRecord(value)) return false;
  return isNonEmptyToken(value.id) && isNonEmptyToken(value.type) && Number.isSafeInteger(value.version) &&
    (value.version as number) >= 1 && isJsonValue(value.data) &&
    (value.metadata === undefined || (isRecord(value.metadata) && isJsonValue(value.metadata)));
}

function requireToken(value: unknown, name: string): asserts value is string {
  try {
    assertNonEmptyToken(value, name);
  } catch (error) {
    throw new ComponentValidationError(error instanceof Error ? error.message : `${name} is invalid`);
  }
}
