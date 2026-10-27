// EXAMPLE content for the Define steps (plan rule 2: domain content lives only in src/mock and is
// labelled Example). The steps show these until the engine derives briefs, architecture and
// component candidates from the user's own description.

export const EXAMPLE_BRIEFS = [
  { id: 'b-sys', title: 'System', status: 'Ready', desc: 'Closed fluid loop actuator driven by DC electrical supply.' },
  { id: 'b-cmp', title: 'Components', status: '4 proposed', desc: 'Power source, electric drive assembly, sensor unit, fluid displacement unit.' },
  { id: 'b-con', title: 'Connections', status: 'Ready', desc: '48 V DC link, mechanical shaft coupling, sensor feedback wire.' },
  { id: 'b-env', title: 'Environment', status: 'Draft', desc: 'Ambient 25.0 °C, atmospheric pressure 101.3 kPa, convection cooling.' },
  { id: 'b-scn', title: 'Scenario', status: '1 proposed', desc: 'High load dynamic ramp step at t=4.20s to evaluate thermal margin.' },
  { id: 'b-req', title: 'Requirements', status: 'Ready', desc: 'Stator winding temperature limit < 120.0 °C; deterministic bitwise replay.' }
];

export const EXAMPLE_QUESTIONS = [
  {
    id: 'q1',
    text: 'Specify continuous hydraulic flow rate requirement:',
    defaultVal: '4.5 L/min at 3.5 bar',
    answered: false,
    value: ''
  },
  {
    id: 'q2',
    text: 'Acceptable rotor angular speed tolerance:',
    defaultVal: '±50 RPM (Tolerance tier)',
    answered: false,
    value: ''
  }
];

export const EXAMPLE_SUBSYSTEMS = [
  {
    id: 'sub-elec',
    name: 'Electrical Subsystem',
    blocks: [
      { id: 'blk-ps', name: 'Power Supply Unit', ports: ['DC Out (48 V)'] }
    ]
  },
  {
    id: 'sub-drive',
    name: 'Drive Subsystem',
    blocks: [
      { id: 'blk-mot', name: 'Actuator Unit', ports: ['DC In', 'Shaft Coupling'] },
      { id: 'blk-sens', name: 'Thermal Sensor Probe', ports: ['Analog Out'] }
    ]
  },
  {
    id: 'sub-fluid',
    name: 'Fluid Subsystem',
    blocks: [
      { id: 'blk-pump', name: 'Fluid Displacement Assembly', ports: ['Shaft Input', 'Inlet', 'Outlet'] }
    ]
  }
];

export const EXAMPLE_COMPONENT_CATEGORIES = [
  {
    id: 'block-ps',
    subsystem: 'Power Supply Unit',
    candidates: [
      {
        id: 'c-ps-1',
        name: 'Power supply (Example)',
        specs: '48.0 V DC, 3.2 kW rating',
        source: 'Internal Component CAD Vault',
        asOf: '2026-10-25',
        maturity: 'Validated model',
        reason: 'Matches voltage (48 V) and continuous power demand',
        selected: true
      }
    ]
  },
  {
    id: 'block-act',
    subsystem: 'Drive Assembly',
    candidates: [
      {
        id: 'c-act-1',
        name: 'Electric motor (Example)',
        specs: '48 V DC nominal, 3000 RPM, 3.0 kW',
        source: 'Internal Component CAD Vault',
        asOf: '2026-10-25',
        maturity: 'Validated model',
        reason: 'Matches 48 V electrical port and rotational drive requirement',
        selected: true
      }
    ]
  },
  {
    id: 'block-sens',
    subsystem: 'Thermal Sensor Probe',
    candidates: [
      {
        id: 'c-sens-1',
        name: 'Temperature sensor (Example)',
        specs: '-40 to 150 °C range, RTD Pt100',
        source: 'Power Lab Telemetry Ingest',
        asOf: '2026-10-25',
        maturity: 'Validated model',
        reason: 'Operational limit covers 120.0 °C threshold requirement',
        selected: true
      }
    ]
  },
  {
    id: 'block-disp',
    subsystem: 'Fluid Displacement Assembly',
    candidates: [
      {
        id: 'c-disp-placeholder',
        name: 'Pump (no model) (Example)',
        specs: 'Displacement placeholder, 3.5 bar fluid head',
        source: 'Inferred Specification',
        asOf: '2026-10-25',
        maturity: 'Placeholder',
        reason: 'No geometric CAD source found in connected vaults; using generic placeholder geometry',
        selected: true
      }
    ]
  }
];
