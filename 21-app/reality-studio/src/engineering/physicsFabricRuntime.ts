import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState } from './authoritativeState';
import type { EnvironmentDefinition } from './environmentFabric';

export const PHYSICS_FABRIC_RUNTIME_SCHEMA = 'helqix.physics-fabric-runtime.v1' as const;

export type PhysicsDomain = 'mechanical' | 'electrical' | 'thermal' | 'fluid' | 'structural' | 'control' | 'electromagnetic' | 'cfd';
export type PhysicsProviderPhase = 'created' | 'prepared' | 'initialized' | 'stepped' | 'solved' | 'failed' | 'disposed';

export interface PhysicsProviderCapabilities {
  domains: readonly PhysicsDomain[];
  deterministic: boolean;
  supportsVariableStep: boolean;
  supportsCoupling: boolean;
}

export interface PhysicsStepContext {
  reality: EngineeringReality;
  state: AuthoritativeEngineeringState;
  environment: EnvironmentDefinition;
  logicalTimeMs: number;
  deltaTimeMs: number;
  inputs: Readonly<Record<string, number>>;
}

export interface PhysicsEvent {
  type: 'warning' | 'failure' | 'threshold' | 'convergence';
  domain: PhysicsDomain;
  componentId?: string;
  message: string;
  logicalTimeMs: number;
  severity: 'info' | 'warning' | 'error';
}

export interface PhysicsProviderResult {
  providerId: string;
  domain: PhysicsDomain;
  phase: PhysicsProviderPhase;
  stateUpdates: Readonly<Record<string, number>>;
  events: readonly PhysicsEvent[];
  residual?: number;
  iterations?: number;
  metadata?: Readonly<Record<string, string | number | boolean>>;
}

export interface PhysicsProvider {
  readonly id: string;
  readonly version: string;
  readonly domain: PhysicsDomain;
  readonly capabilities: PhysicsProviderCapabilities;
  prepare(context: PhysicsStepContext): void;
  initialize(context: PhysicsStepContext): void;
  step(context: PhysicsStepContext): PhysicsProviderResult;
  solve(context: PhysicsStepContext): PhysicsProviderResult;
  dispose(): void;
}

export interface PhysicsExecutionResult {
  schema: typeof PHYSICS_FABRIC_RUNTIME_SCHEMA;
  logicalTimeMs: number;
  deltaTimeMs: number;
  providers: readonly PhysicsProviderResult[];
  stateUpdates: Readonly<Record<string, number>>;
  events: readonly PhysicsEvent[];
  converged: boolean;
}

function clone<T>(value: T): T { return structuredClone(value); }
function finite(value: number, label: string): void { if (!Number.isFinite(value)) throw new Error(`${label} must be finite.`); }

export class PhysicsFabricRuntime {
  private readonly providers = new Map<string, PhysicsProvider>();
  private readonly phases = new Map<string, PhysicsProviderPhase>();

  register(provider: PhysicsProvider): void {
    if (!provider.id.trim()) throw new Error('Physics provider id is required.');
    if (this.providers.has(provider.id)) throw new Error(`Physics provider already registered: ${provider.id}`);
    if (!provider.capabilities.domains.includes(provider.domain)) throw new Error(`Provider ${provider.id} does not declare its own domain capability.`);
    this.providers.set(provider.id, provider);
    this.phases.set(provider.id, 'created');
  }

  unregister(providerId: string): void {
    const provider = this.providers.get(providerId);
    if (!provider) return;
    provider.dispose();
    this.providers.delete(providerId);
    this.phases.delete(providerId);
  }

  listProviders(): PhysicsProvider[] { return [...this.providers.values()]; }

  prepare(context: PhysicsStepContext): void {
    for (const provider of this.providers.values()) {
      provider.prepare(context);
      this.phases.set(provider.id, 'prepared');
    }
  }

  initialize(context: PhysicsStepContext): void {
    for (const provider of this.providers.values()) {
      if (this.phases.get(provider.id) !== 'prepared') throw new Error(`Provider ${provider.id} must be prepared before initialization.`);
      provider.initialize(context);
      this.phases.set(provider.id, 'initialized');
    }
  }

  step(context: PhysicsStepContext): PhysicsExecutionResult {
    finite(context.logicalTimeMs, 'logicalTimeMs');
    finite(context.deltaTimeMs, 'deltaTimeMs');
    if (context.deltaTimeMs <= 0) throw new Error('deltaTimeMs must be greater than zero.');
    const results: PhysicsProviderResult[] = [];
    const updates: Record<string, number> = {};
    const events: PhysicsEvent[] = [];
    for (const provider of this.providers.values()) {
      const phase = this.phases.get(provider.id);
      if (phase !== 'initialized' && phase !== 'stepped' && phase !== 'solved') throw new Error(`Provider ${provider.id} is not initialized.`);
      try {
        const result = clone(provider.step(context));
        if (result.providerId !== provider.id || result.domain !== provider.domain) throw new Error(`Provider ${provider.id} returned an inconsistent result identity.`);
        for (const [key, value] of Object.entries(result.stateUpdates)) finite(value, `${provider.id}.${key}`);
        results.push(result);
        Object.assign(updates, result.stateUpdates);
        events.push(...result.events);
        this.phases.set(provider.id, 'stepped');
      } catch (error) {
        this.phases.set(provider.id, 'failed');
        throw new Error(`Physics provider ${provider.id} failed: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
    return { schema: PHYSICS_FABRIC_RUNTIME_SCHEMA, logicalTimeMs: context.logicalTimeMs, deltaTimeMs: context.deltaTimeMs, providers: results, stateUpdates: updates, events, converged: results.every((result) => result.residual === undefined || result.residual <= 1e-8) };
  }

  solve(context: PhysicsStepContext): PhysicsExecutionResult {
    const results: PhysicsProviderResult[] = [];
    const updates: Record<string, number> = {};
    const events: PhysicsEvent[] = [];
    for (const provider of this.providers.values()) {
      if (!['initialized', 'stepped', 'solved'].includes(this.phases.get(provider.id) ?? '')) throw new Error(`Provider ${provider.id} is not ready to solve.`);
      const result = clone(provider.solve(context));
      if (result.providerId !== provider.id) throw new Error(`Provider ${provider.id} returned an invalid solve identity.`);
      for (const [key, value] of Object.entries(result.stateUpdates)) finite(value, `${provider.id}.${key}`);
      results.push(result);
      Object.assign(updates, result.stateUpdates);
      events.push(...result.events);
      this.phases.set(provider.id, 'solved');
    }
    return { schema: PHYSICS_FABRIC_RUNTIME_SCHEMA, logicalTimeMs: context.logicalTimeMs, deltaTimeMs: context.deltaTimeMs, providers: results, stateUpdates: updates, events, converged: results.every((result) => result.residual === undefined || result.residual <= 1e-8) };
  }

  phase(providerId: string): PhysicsProviderPhase | undefined { return this.phases.get(providerId); }
}
