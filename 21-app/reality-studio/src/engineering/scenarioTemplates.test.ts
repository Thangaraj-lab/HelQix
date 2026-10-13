import { describe, expect, it } from 'vitest';
import { createInitialState } from './authoritativeState';
import { createExecutionController, runToCompletion, stepExecution } from './executionController';
import { createMotionScenario } from './scenarioTemplates';
import type { EngineeringReality } from '../api';

function reality(): EngineeringReality {
  return {
    revision: 1,
    components: [{ id: 'motor-1', name: 'Drive Motor', componentType: 'motor', properties: [], behaviors: [], tags: [] }],
    interfaces: [],
    connections: [],
    requirements: [],
    environment: {},
  } as EngineeringReality;
}

describe('createMotionScenario', () => {
  it('executes deterministic RPM transitions through authoritative state', () => {
    const currentReality = reality();
    const scenario = createMotionScenario(currentReality, 'motor-1');
    const initial = createInitialState(currentReality, { scenarioId: scenario.id, executionId: 'test-motion' });
    const controller = createExecutionController(currentReality, scenario, initial);

    const first = stepExecution(currentReality, scenario, controller).controller;
    expect(first.state.components['motor-1'].status).toBe('running');

    const completed = runToCompletion(currentReality, scenario, first);
    expect(completed.phase).toBe('completed');
    expect(completed.logicalTimeMs).toBe(5000);
    expect(completed.state.components['motor-1'].signals.rpm.value).toBe(0);
    expect(completed.state.events.some((event) => event.type === 'signal_updated' && event.after === 1500)).toBe(true);
    expect(completed.state.events.some((event) => event.type === 'signal_updated' && event.after === 4000)).toBe(true);
  });
});
