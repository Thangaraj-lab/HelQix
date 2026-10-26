import { BuildEvent, ModelOp, Value, Source } from '../contracts';

export const EXAMPLE_PROMPT =
  'A pump driven by an electric motor from a 48 V supply; temperature sensor on the stator; winding below 120 degC.';

export function createMockValue(v: number, unit: string, origin: Value['origin'] = 'simulated', status: Value['status'] = 'unverified'): Value {
  return {
    v,
    unit,
    origin,
    status,
    mock: true
  };
}

export const MOCK_SOURCES: Source[] = [
  { id: 'src-1', name: 'Internal Component CAD Vault (Example)', kind: 'cad_repository', status: 'connected', lastSync: '10 min ago' },
  { id: 'src-2', name: 'Power Lab Telemetry Ingest (Example)', kind: 'connected_tool', status: 'connected', lastSync: '1 min ago' },
  { id: 'src-3', name: 'Standard Fasteners & Housings (Example)', kind: 'catalog', status: 'connected', lastSync: '1 hour ago' }
];

export function getMockBuildScriptEvents(buildId: string): BuildEvent[] {
  const ts = new Date().toISOString();
  
  const opPart1: ModelOp = {
    op: 'part.add',
    id: 'part-power-supply',
    name: 'Power supply (Example)',
    geometry: { kind: 'primitive', shape: 'box', dims: [createMockValue(120, 'mm'), createMockValue(80, 'mm'), createMockValue(40, 'mm')] },
    maturity: 'validated_model',
    prov: { kind: 'from_source', sourceId: 'src-1', asOf: '2026-10-25' }
  };

  const opPart2: ModelOp = {
    op: 'part.add',
    id: 'part-electric-motor',
    name: 'Electric motor (Example)',
    geometry: { kind: 'primitive', shape: 'cylinder', dims: [createMockValue(90, 'mm'), createMockValue(140, 'mm'), createMockValue(90, 'mm')] },
    maturity: 'validated_model',
    prov: { kind: 'from_source', sourceId: 'src-1', asOf: '2026-10-25' }
  };

  const opPart3: ModelOp = {
    op: 'part.add',
    id: 'part-temp-sensor',
    name: 'Temperature sensor (Example)',
    geometry: { kind: 'primitive', shape: 'sphere', dims: [createMockValue(15, 'mm'), createMockValue(15, 'mm'), createMockValue(15, 'mm')] },
    maturity: 'validated_model',
    prov: { kind: 'from_source', sourceId: 'src-2', asOf: '2026-10-25' }
  };

  const opPart4: ModelOp = {
    op: 'part.add',
    id: 'part-pump',
    name: 'Pump (no model) (Example)',
    geometry: { kind: 'primitive', shape: 'box', dims: [createMockValue(110, 'mm'), createMockValue(110, 'mm'), createMockValue(90, 'mm')] },
    maturity: 'placeholder',
    prov: { kind: 'inferred', note: 'No CAD source found; using generic placeholder geometry' }
  };

  return [
    { seq: 1, buildId, ts, kind: 'step.start', step: 'interpret' },
    {
      seq: 2,
      buildId,
      ts,
      kind: 'question.asked',
      question: {
        id: 'q-load-profile',
        text: 'What is the anticipated hydraulic backpressure load curve?',
        defaultVal: 'Constant 3.5 bar fluid head',
        choices: ['Constant 3.5 bar fluid head', 'Cyclic pressure pulses (0-6 bar)', 'Unknown / unconstrained']
      }
    },
    { seq: 3, buildId, ts, kind: 'step.done', step: 'interpret' },
    
    { seq: 4, buildId, ts, kind: 'step.start', step: 'resolve' },
    { seq: 5, buildId, ts, kind: 'op.applied', op: opPart1 },
    { seq: 6, buildId, ts, kind: 'op.applied', op: opPart2 },
    { seq: 7, buildId, ts, kind: 'op.applied', op: opPart3 },
    { seq: 8, buildId, ts, kind: 'op.applied', op: opPart4 },
    { seq: 9, buildId, ts, kind: 'step.done', step: 'resolve' },

    { seq: 10, buildId, ts, kind: 'step.start', step: 'place' },
    {
      seq: 11,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'part.place',
        id: 'part-power-supply',
        transform: {
          pos: [createMockValue(-2.2, 'm'), createMockValue(0.5, 'm'), createMockValue(0, 'm')],
          rot: [createMockValue(0, 'deg'), createMockValue(0, 'deg'), createMockValue(0, 'deg')],
          scale: [1, 1, 1],
          frame: 'world'
        }
      }
    },
    {
      seq: 12,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'part.place',
        id: 'part-electric-motor',
        transform: {
          pos: [createMockValue(-0.5, 'm'), createMockValue(0.8, 'm'), createMockValue(0, 'm')],
          rot: [createMockValue(0, 'deg'), createMockValue(0, 'deg'), createMockValue(0, 'deg')],
          scale: [1, 1, 1],
          frame: 'world'
        }
      }
    },
    {
      seq: 13,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'part.place',
        id: 'part-temp-sensor',
        transform: {
          pos: [createMockValue(-0.5, 'm'), createMockValue(1.2, 'm'), createMockValue(0.4, 'm')],
          rot: [createMockValue(0, 'deg'), createMockValue(0, 'deg'), createMockValue(0, 'deg')],
          scale: [1, 1, 1],
          frame: 'part-electric-motor'
        }
      }
    },
    {
      seq: 14,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'part.place',
        id: 'part-pump',
        transform: {
          pos: [createMockValue(1.4, 'm'), createMockValue(0.8, 'm'), createMockValue(0, 'm')],
          rot: [createMockValue(0, 'deg'), createMockValue(0, 'deg'), createMockValue(0, 'deg')],
          scale: [1, 1, 1],
          frame: 'world'
        }
      }
    },
    { seq: 15, buildId, ts, kind: 'step.done', step: 'place' },

    { seq: 16, buildId, ts, kind: 'step.start', step: 'connect' },
    {
      seq: 17,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'connection.add',
        id: 'conn-elec-1',
        from: { partId: 'part-power-supply', portId: 'dc-out' },
        to: { partId: 'part-electric-motor', portId: 'dc-in' },
        kind: 'electrical_dc'
      }
    },
    {
      seq: 18,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'connection.add',
        id: 'conn-mech-1',
        from: { partId: 'part-electric-motor', portId: 'shaft-out' },
        to: { partId: 'part-pump', portId: 'shaft-in' },
        kind: 'mechanical_torque'
      }
    },
    {
      seq: 19,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'connection.add',
        id: 'conn-sensor-1',
        from: { partId: 'part-temp-sensor', portId: 'analog-out' },
        to: { partId: 'part-power-supply', portId: 'thermal-feedback-in' },
        kind: 'telemetry_signal'
      }
    },
    {
      seq: 20,
      buildId,
      ts,
      kind: 'issue.raised',
      issue: {
        id: 'iss-voltage-unit-mismatch',
        partId: 'part-power-supply',
        portId: 'dc-out',
        message: 'Port output is rated 48 V while motor specification is 48.0 V DC nominal. Verified compatible.',
        severity: 'warning',
        fixOp: {
          op: 'part.setParam',
          id: 'part-power-supply',
          key: 'rated_voltage',
          value: createMockValue(48.0, 'V', 'derived', 'verified')
        }
      }
    },
    { seq: 21, buildId, ts, kind: 'step.done', step: 'connect' },

    { seq: 22, buildId, ts, kind: 'step.start', step: 'check' },
    {
      seq: 23,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'requirement.add',
        id: 'req-thermal-limit',
        text: 'Stator winding temperature must remain below 120.0 °C under all dynamic operating cycles.',
        expr: 'winding_temp < 120.0 degC',
        prov: { kind: 'user_stated' }
      }
    },
    {
      seq: 24,
      buildId,
      ts,
      kind: 'op.applied',
      op: {
        op: 'binding.add',
        binding: {
          id: 'bind-rotor-spin',
          variable: 'rotor_angular_velocity',
          node: 'part-electric-motor',
          channel: 'rotation',
          axis: 'y',
          mapping: 'angle = integral(angular_velocity * dt)',
          units: 'rad/s'
        }
      }
    },
    { seq: 25, buildId, ts, kind: 'step.done', step: 'check' },
    { seq: 26, buildId, ts, kind: 'staged.ready', note: 'All components staged. Ready to commit Model revision r1.' }
  ];
}
