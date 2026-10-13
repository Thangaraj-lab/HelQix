import { describe, expect, it } from 'vitest';
import { engineeringVisualMotion } from './engineeringVisualMotion';
import type { RuntimeEntityState } from './visualStateBinding';

function entity(overrides: Partial<RuntimeEntityState> = {}): RuntimeEntityState {
  return {
    id: 'motor-1',
    name: 'Drive Motor',
    componentType: 'motor',
    status: 'running',
    propertyValues: {},
    faults: [],
    signals: {},
    selected: false,
    stateRevision: 1,
    logicalTimeMs: 0,
    ...overrides,
  };
}

describe('engineeringVisualMotion', () => {
  it('maps RPM to deterministic angular rotation from logical time', () => {
    const result = engineeringVisualMotion(entity({ logicalTimeMs: 1000, signals: { rpm: 60 } }));
    expect(result.rotationRad).toBeCloseTo(Math.PI * 2);
    expect(result.hasMotionSignal).toBe(true);
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
