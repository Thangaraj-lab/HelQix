import { describe, expect, it } from 'vitest';
import type { EngineeringReality } from '../api';
import { captureSnapshot, createInitialState } from './authoritativeState';
import { applyBranch, compareBranches, createBranch } from './branchEngine';
import { createStateFreeze, verifyStateFreeze } from './stateFreeze';
import { replayExecution, verifyReplayDeterminism } from './replayEngine';
import { createMotionScenario } from './scenarioTemplates';
import { createExecutionController, runToCompletion } from './executionController';

function reality(): EngineeringReality {
  return {
    schema: 'helqix.engineering-reality.v1',
    name: 'Execution Trace Fixture',
    domain: 'mechanical',
    description: 'Deterministic test fixture for execution trace, freeze, replay and branch behavior.',
    revision: 7,
    unitsSystem: 'SI',
    components: [{ id: 'motor-1', name: 'Reference Motor', componentType: 'motor', properties: [] }],
    interfaces: [],
    connections: [],
    requirements: [],
    modelBindings: [],
  };
}

describe('HELQIX execution trace', () => {
  it('freezes, replays, and compares deterministic branches from one authoritative state', () => {
    const currentReality = reality();
    const scenario = createMotionScenario(currentReality, 'motor-1');
    const initial = createInitialState(currentReality, { scenarioId: scenario.id, executionId: 'trace-test' });
    const initialSnapshot = captureSnapshot(initial, 'initial', 'trace-test-initial');
    const controller = runToCompletion(currentReality, scenario, createExecutionController(currentReality, scenario, initial));

    const freeze = createStateFreeze(currentReality, controller.state, scenario, { deterministic: true });
    expect(verifyStateFreeze(freeze, currentReality, scenario)).toBe(true);
    expect(freeze.logicalTimeMs).toBe(5000);

    const firstReplay = replayExecution(currentReality, scenario, initialSnapshot);
    const secondReplay = replayExecution(currentReality, scenario, initialSnapshot);
    const replayCheck = verifyReplayDeterminism(firstReplay, secondReplay);
    expect(replayCheck.deterministic).toBe(true);
    expect(firstReplay.finalState.events).toEqual(secondReplay.finalState.events);

    const branchA = applyBranch(
      createBranch(currentReality, 'A', 'Lower RPM', [{ componentId: 'motor-1', propertyName: 'rpm', value: 2500, unit: 'rpm' }], controller.state.stateRevision),
      currentReality,
      controller.state,
    );
    const branchB = applyBranch(
      createBranch(currentReality, 'B', 'Higher RPM', [{ componentId: 'motor-1', propertyName: 'rpm', value: 4500, unit: 'rpm' }], controller.state.stateRevision),
      currentReality,
      controller.state,
    );

    expect(branchA.status).toBe('completed');
    expect(branchB.status).toBe('completed');
    const comparison = compareBranches(branchA, branchB);
    expect(comparison.changedProperties).toEqual([
      { componentId: 'motor-1', propertyName: 'rpm', baseline: 2500, candidate: 4500 },
    ]);
  });
});
