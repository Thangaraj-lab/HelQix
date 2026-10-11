import { describe, expect, it } from 'vitest';
import type { EngineeringReality } from '../api';
import { createInitialState } from './authoritativeState';
import { createScenario, executeScenario, validateScenario, type ExecutableScenario } from './scenarioEngine';

const reality: EngineeringReality = {
  schema: 'helqix.engineering-reality.v1',
  name: 'Test system', domain: 'generic', description: '', revision: 1, unitsSystem: 'SI',
  components: [{ id: 'motor', name: 'Motor', componentType: 'motor', properties: [{ name: 'speed_rpm', value: 0, unit: 'rpm', origin: 'user' }] }],
  interfaces: [], connections: [], requirements: [], modelBindings: [],
};

describe('executable scenario engine', () => {
  it('executes ordered actions against authoritative state', () => {
    const scenario = createScenario('Start motor', reality, [
      { id: 'start', type: 'set_status', atMs: 0, componentId: 'motor', status: 'running' },
      { id: 'speed', type: 'set_property', atMs: 100, componentId: 'motor', propertyName: 'speed_rpm', value: 1500, unit: 'rpm' },
      { id: 'fault', type: 'inject_fault', atMs: 200, componentId: 'motor', faultId: 'overheat' },
    ]);
    const result = executeScenario(scenario, reality, createInitialState(reality));
    expect(result.status).toBe('completed');
    expect(result.state.logicalTimeMs).toBe(200);
    expect(result.state.components.motor.status).toBe('faulted');
    expect(result.state.components.motor.properties[0].value).toBe(1500);
    expect(result.events).toHaveLength(3);
  });

  it('rejects actions that reference unknown components', () => {
    const scenario: ExecutableScenario = {
      schema: 'helqix.executable-scenario.v1', id: 'bad', name: 'Bad', description: '', createdFromRealityRevision: 1,
      actions: [{ id: 'bad-action', type: 'set_status', atMs: 0, componentId: 'missing', status: 'running' }],
    };
    expect(validateScenario(scenario, reality)).toHaveLength(1);
    expect(executeScenario(scenario, reality, createInitialState(reality)).status).toBe('failed');
  });
});
