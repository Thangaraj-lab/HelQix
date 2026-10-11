import { describe, expect, it } from 'vitest';
import type { EngineeringReality } from '../api';
import { addFault, advanceLogicalTime, captureSnapshot, createInitialState, restoreSnapshot, setComponentProperty, validateState } from './authoritativeState';

const reality: EngineeringReality = {
  schema: 'helqix.engineering-reality.v1',
  name: 'Test system', domain: 'generic', description: '', revision: 1, unitsSystem: 'SI',
  components: [{ id: 'motor', name: 'Motor', componentType: 'motor', properties: [{ name: 'speed_rpm', value: 0, unit: 'rpm', origin: 'user' }] }],
  interfaces: [], connections: [], requirements: [], modelBindings: [],
};

describe('authoritative engineering state', () => {
  it('derives state only from the persisted reality', () => {
    const state = createInitialState(reality);
    expect(state.components.motor.properties[0].value).toBe(0);
    expect(validateState(state, reality)).toEqual([]);
  });

  it('applies deterministic property and time transitions', () => {
    let state = createInitialState(reality);
    state = setComponentProperty(state, 'motor', 'speed_rpm', 1200, 'rpm', 'action-1');
    state = advanceLogicalTime(state, 500);
    expect(state.components.motor.properties[0].value).toBe(1200);
    expect(state.logicalTimeMs).toBe(500);
    expect(() => advanceLogicalTime(state, 400)).toThrow();
  });

  it('captures and restores exact state snapshots', () => {
    let state = addFault(createInitialState(reality), 'motor', 'overheat');
    const snapshot = captureSnapshot(state, 'snap-1');
    state = setComponentProperty(state, 'motor', 'speed_rpm', 900);
    const restored = restoreSnapshot(snapshot);
    expect(restored.components.motor.faultIds).toEqual(['overheat']);
    expect(restored.components.motor.properties[0].value).toBe(0);
  });
});
