import { describe, expect, it } from 'vitest';
import type { EngineeringReality } from '../api';
import { addFault, createInitialState } from './authoritativeState';
import { CausalityEngine } from './causalityEngine';
import { ObservationFabric } from './observationFabric';

const reality: EngineeringReality = {
  schema: 'helqix.engineering-reality.v1', name: 'Observation fixture', domain: 'mechanical', description: '', revision: 1, unitsSystem: 'SI',
  components: [{ id: 'motor-1', name: 'Motor', componentType: 'motor', properties: [] }], interfaces: [], connections: [], requirements: [], modelBindings: [],
};

describe('observation and causal evidence', () => {
  it('turns an authoritative fault into an observation and avoids inventing a root cause', () => {
    const initial = createInitialState(reality);
    const faulted = addFault(initial, 'motor-1', 'winding-overheat');
    const observations = new ObservationFabric();
    observations.registerProbe({ id: 'motor-status', name: 'Motor status', source: 'state', componentId: 'motor-1', enabled: true });

    const frame = observations.observeState(faulted);
    expect(frame.samples).toMatchObject([{ value: 'faulted', severity: 'critical', source: 'state' }]);
    expect(frame.anomalies).toMatchObject([{ kind: 'state_fault', observedValue: 'faulted' }]);

    const causalFailure = new CausalityEngine().ingestAnomaly(frame.anomalies[0]);
    expect(causalFailure.rootCauseNodeId).toBeUndefined();
    expect(causalFailure.explanation).toContain('additional instrumentation or simulation evidence');
  });
});
