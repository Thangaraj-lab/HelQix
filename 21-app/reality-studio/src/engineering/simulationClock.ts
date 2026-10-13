export const SIMULATION_CLOCK_SCHEMA = 'helqix.simulation-clock.v1' as const;

export interface SimulationClockState {
  readonly schema: typeof SIMULATION_CLOCK_SCHEMA;
  readonly startTimeMs: number;
  readonly logicalTimeMs: number;
  readonly stepIndex: number;
  readonly fixedStepMs: number;
  readonly maxTimeMs?: number;
}

export interface SimulationClockOptions {
  startTimeMs?: number;
  fixedStepMs: number;
  maxTimeMs?: number;
}

export class DeterministicSimulationClock {
  private state: SimulationClockState;

  constructor(options: SimulationClockOptions) {
    if (!Number.isFinite(options.fixedStepMs) || options.fixedStepMs <= 0) throw new Error('fixedStepMs must be finite and greater than zero.');
    if (options.maxTimeMs !== undefined && options.maxTimeMs < (options.startTimeMs ?? 0)) throw new Error('maxTimeMs must not precede startTimeMs.');
    this.state = {
      schema: SIMULATION_CLOCK_SCHEMA,
      startTimeMs: options.startTimeMs ?? 0,
      logicalTimeMs: options.startTimeMs ?? 0,
      stepIndex: 0,
      fixedStepMs: options.fixedStepMs,
      maxTimeMs: options.maxTimeMs,
    };
  }

  snapshot(): SimulationClockState {
    return { ...this.state };
  }

  reset(): SimulationClockState {
    this.state = { ...this.state, logicalTimeMs: this.state.startTimeMs, stepIndex: 0 };
    return this.snapshot();
  }

  step(): SimulationClockState {
    const nextTime = this.state.logicalTimeMs + this.state.fixedStepMs;
    if (this.state.maxTimeMs !== undefined && nextTime > this.state.maxTimeMs) {
      this.state = { ...this.state, logicalTimeMs: this.state.maxTimeMs, stepIndex: this.state.stepIndex + 1 };
    } else {
      this.state = { ...this.state, logicalTimeMs: nextTime, stepIndex: this.state.stepIndex + 1 };
    }
    return this.snapshot();
  }

  advance(steps: number): SimulationClockState {
    if (!Number.isInteger(steps) || steps < 0) throw new Error('steps must be a non-negative integer.');
    for (let index = 0; index < steps; index += 1) this.step();
    return this.snapshot();
  }

  seek(logicalTimeMs: number): SimulationClockState {
    if (!Number.isFinite(logicalTimeMs)) throw new Error('logicalTimeMs must be finite.');
    if (logicalTimeMs < this.state.startTimeMs) throw new Error('Cannot seek before simulation start.');
    if (this.state.maxTimeMs !== undefined && logicalTimeMs > this.state.maxTimeMs) throw new Error('Cannot seek beyond simulation maximum.');
    const elapsed = logicalTimeMs - this.state.startTimeMs;
    const steps = elapsed / this.state.fixedStepMs;
    if (!Number.isInteger(steps)) throw new Error('Seek target must align to the fixed simulation timestep.');
    this.state = { ...this.state, logicalTimeMs, stepIndex: steps };
    return this.snapshot();
  }

  isAtEnd(): boolean {
    return this.state.maxTimeMs !== undefined && this.state.logicalTimeMs >= this.state.maxTimeMs;
  }
}

export function deterministicStepCount(durationMs: number, fixedStepMs: number): number {
  if (!Number.isFinite(durationMs) || durationMs < 0) throw new Error('durationMs must be finite and non-negative.');
  if (!Number.isFinite(fixedStepMs) || fixedStepMs <= 0) throw new Error('fixedStepMs must be finite and greater than zero.');
  if (!Number.isInteger(durationMs / fixedStepMs)) throw new Error('Duration must align to the fixed timestep.');
  return durationMs / fixedStepMs;
}
