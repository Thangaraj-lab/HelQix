import { describe, expect, it } from 'vitest';
import { engineeringVisualMotion } from './engineeringVisualMotion';
import type { RuntimeEntityState } from './visualStateBinding';

function entity(overrides: Partial<RuntimeEntityState> = {}): RuntimeEntityState {
  return {
    id: 'motor-1', name: 'Drive Motor', componentType: 'motor', status: 'running',
    propertyValues: {}, faults: [], signals: {}, selected: false, stateRevision: 1,
    logicalTimeMs: 0, motionEvents: [], ...overrides,
  };
}

describe('engineeringVisualMotion', () => {
  it('maps RPM to deterministic angular rotation from logical time when no history exists', () => {
    const result = engineeringVisualMotion(entity({ logicalTimeMs: 1000, signals: { rpm: 60 } }));
    expect(result.rotationRad).toBeCloseTo(Math.PI * 2);
    expect(result.hasMotionSignal).toBe(true);
  });

  it('integrates piecewise RPM history instead of multiplying final RPM by all elapsed time', () => {
    const result = engineeringVisualMotion(entity({
      logicalTimeMs: 5000,
      signals: { rpm: 0 },
      motionEvents: [
        { logicalTimeMs: 1000, name: 'rpm', value: 1500, source: 'scenario' },
        { logicalTimeMs: 3000, name: 'rpm', value: 4000, source: 'scenario' },
        { logicalTimeMs: 4000, name: 'rpm', value: 0, source: 'scenario' },
      ],
    }));
    const expected = (2 * 1500 + 1 * 4000) * (Math.PI * 2 / 60);
    expect(result.rotationRad).toBeCloseTo(expected);
  });

  it('maps linear velocity to deterministic displacement', () => {
    const result = engineeringVisualMotion(entity({ logicalTimeMs: 2500, signals: { velocity_m_s: 4 } }));
    expect(result.translationMeters).toBeCloseTo(10);
    expect(result.hasMotionSignal).toBe(true);
  });

  it('normalizes temperature without inventing heat when absent', () => {
    expect(engineeringVisualMotion(entity()).hasThermalSignal).toBe(false);
    expect(engineeringVisualMotion(entity({ signals: { temperature_c: 70 } })).thermalIntensity).toBeCloseTo(0.5);
    expect(engineeringVisualMotion(entity({ signals: { temperature_c: 200 } })).thermalIntensity).toBe(1);
  });
});
