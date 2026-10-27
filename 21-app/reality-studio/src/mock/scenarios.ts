// EXAMPLE scenarios and requirement for the scripted mock run (plan rule 2: domain content lives only
// in src/mock and is labelled Example). The run itself is MockEngineClient.computeFrame, a fixed script.

export interface ExampleScenario {
  id: string;
  num: string;
  title: string;
  tag?: 'Current' | 'Baseline';
  duration: string;
  note: string;
}

export const EXAMPLE_SCENARIOS: ExampleScenario[] = [
  { id: 'scen-01', num: '01', title: 'High-load dynamic cycle', tag: 'Current', duration: '12.0 s', note: 'Load steps up at 4.2 s' },
  { id: 'scen-02', num: '02', title: 'Nominal continuous rated', tag: 'Baseline', duration: '120.0 s', note: 'Constant rated load' },
  { id: 'scen-03', num: '03', title: 'Thermal soak and cool-down', duration: '60.0 s', note: 'No load' },
  { id: 'scen-04', num: '04', title: 'Harmonic torque pulsing', duration: '30.0 s', note: 'Pulsed load' },
  { id: 'scen-05', num: '05', title: 'Cold-start rapid ramp', duration: '15.0 s', note: 'Fast ramp from rest' }
];

/** Only the first scenario has a scripted run; the others are listed but not runnable yet. */
export const RUNNABLE_SCENARIO = 'scen-01';

export const EXAMPLE_REQUIREMENT = {
  id: 'R1',
  text: 'Winding temperature below 120 °C',
  variable: 'stator_winding_temperature',
  limit: 120,
  unit: '°C'
};

export const MOCK_RUN_ID = 'mock-run-1';
export const MOCK_RUN_DURATION_S = 12;
