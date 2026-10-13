export const PHYSICS_FABRIC_SCHEMA = 'helqix.physics-fabric.v1' as const;

export type PhysicsDomain = 'mechanical' | 'electrical' | 'thermal' | 'fluid' | 'structural' | 'control' | 'electromagnetic' | 'cfd' | 'custom';

export interface PhysicsState {
  readonly values: Readonly<Record<string, number>>;
  readonly units: Readonly<Record<string, string>>;
  readonly metadata?: Readonly<Record<string, string | number | boolean>>;
}

export interface PhysicsEvent {
  readonly type: string;
  readonly timeMs: number;
  readonly componentId?: string;
  readonly severity: 'info' | 'warning' | 'error' | 'critical';
  readonly payload: Readonly<Record<string, unknown>>;
}

export interface PhysicsStepContext {
  readonly logicalTimeMs: number;
  readonly deltaTimeMs: number;
  readonly state: PhysicsState;
  readonly environment: Readonly<Record<string, number>>;
  readonly inputs: Readonly<Record<string, number>>;
}

export interface PhysicsStepResult {
  readonly state: PhysicsState;
  readonly events: readonly PhysicsEvent[];
  readonly converged: boolean;
  readonly iterations: number;
  readonly residual?: number;
}

export interface PhysicsProviderCapabilities {
  readonly domains: readonly PhysicsDomain[];
  readonly supportsDeterministicStep: boolean;
  readonly supportsPause: boolean;
  readonly supportsCheckpoint: boolean;
  readonly supportsBatch: boolean;
}

export interface PhysicsProvider {
  readonly id: string;
  readonly version: string;
  readonly capabilities: PhysicsProviderCapabilities;
  prepare(context: PhysicsPrepareContext): Promise<void> | void;
  initialize(context: PhysicsInitializeContext): Promise<PhysicsState> | PhysicsState;
  step(context: PhysicsStepContext): Promise<PhysicsStepResult> | PhysicsStepResult;
  solve(context: PhysicsSolveContext): Promise<PhysicsStepResult> | PhysicsStepResult;
  getState(): PhysicsState;
  getEvents(): readonly PhysicsEvent[];
  validate(context: PhysicsValidationContext): PhysicsValidationResult;
  dispose?(): Promise<void> | void;
}

export interface PhysicsPrepareContext {
  readonly projectId: string;
  readonly modelRevision: string;
  readonly scenarioId: string;
  readonly parameters: Readonly<Record<string, number | string | boolean>>;
}

export interface PhysicsInitializeContext extends PhysicsPrepareContext {
  readonly initialState: PhysicsState;
  readonly environment: Readonly<Record<string, number>>;
}

export interface PhysicsSolveContext extends PhysicsStepContext {
  readonly tolerance: number;
  readonly maxIterations: number;
}

export interface PhysicsValidationContext {
  readonly state: PhysicsState;
  readonly previousState?: PhysicsState;
}

export interface PhysicsValidationResult {
  readonly valid: boolean;
  readonly issues: readonly { code: string; message: string; path?: string }[];
}

export interface PhysicsRegistration {
  readonly provider: PhysicsProvider;
  readonly priority: number;
  readonly domains: readonly PhysicsDomain[];
}

export class PhysicsFabric {
  private readonly registrations = new Map<string, PhysicsRegistration>();

  register(registration: PhysicsRegistration): void {
    if (this.registrations.has(registration.provider.id)) throw new Error(`Physics provider already registered: ${registration.provider.id}`);
    if (!Number.isInteger(registration.priority)) throw new Error('Physics provider priority must be an integer.');
    if (registration.domains.length === 0) throw new Error('Physics provider must declare at least one domain.');
    this.registrations.set(registration.provider.id, { ...registration, domains: [...registration.domains] });
  }

  unregister(providerId: string): boolean {
    return this.registrations.delete(providerId);
  }

  get(providerId: string): PhysicsProvider | undefined {
    return this.registrations.get(providerId)?.provider;
  }

  list(domain?: PhysicsDomain): readonly PhysicsRegistration[] {
    return [...this.registrations.values()]
      .filter(item => domain === undefined || item.domains.includes(domain))
      .sort((a, b) => b.priority - a.priority || a.provider.id.localeCompare(b.provider.id));
  }

  resolve(domain: PhysicsDomain, providerId?: string): PhysicsProvider {
    if (providerId) {
      const provider = this.get(providerId);
      if (!provider) throw new Error(`Unknown physics provider: ${providerId}`);
      if (!provider.capabilities.domains.includes(domain)) throw new Error(`Provider ${providerId} does not support ${domain}.`);
      return provider;
    }
    const registration = this.list(domain)[0];
    if (!registration) throw new Error(`No physics provider registered for ${domain}.`);
    return registration.provider;
  }
}

export function createPhysicsState(values: Record<string, number>, units: Record<string, string> = {}): PhysicsState {
  for (const [key, value] of Object.entries(values)) {
    if (!Number.isFinite(value)) throw new Error(`Physics state value must be finite: ${key}.`);
  }
  return { values: { ...values }, units: { ...units } };
}

export function mergePhysicsState(base: PhysicsState, update: PhysicsState): PhysicsState {
  return {
    values: { ...base.values, ...update.values },
    units: { ...base.units, ...update.units },
    metadata: { ...base.metadata, ...update.metadata },
  };
}

export function validatePhysicsResult(result: PhysicsStepResult): PhysicsValidationResult {
  const issues: { code: string; message: string; path?: string }[] = [];
  if (!Number.isInteger(result.iterations) || result.iterations < 0) issues.push({ code: 'INVALID_ITERATIONS', message: 'Iterations must be a non-negative integer.', path: 'iterations' });
  if (result.residual !== undefined && (!Number.isFinite(result.residual) || result.residual < 0)) issues.push({ code: 'INVALID_RESIDUAL', message: 'Residual must be finite and non-negative.', path: 'residual' });
  for (const [key, value] of Object.entries(result.state.values)) {
    if (!Number.isFinite(value)) issues.push({ code: 'NON_FINITE_STATE', message: `Non-finite state value: ${key}.`, path: `state.values.${key}` });
  }
  return { valid: issues.length === 0, issues };
}
