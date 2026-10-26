import { OriginBadge, StatusBadge } from '../engineering/stateStore';

export interface ComponentItem {
  id: string;
  name: string;
  type: string;
  material: string;
  mass: number;
  massUnit: string;
  dimensions: string;
  dimUnit: string;
  connectedTo: number;
  status: 'Active' | 'Critical' | 'Warning' | 'Standby';
  origin: OriginBadge;
  validationStatus: StatusBadge;
  temperature: number;
  load: number;
  vibration: number;
  efficiency: number;
}

export const MOCK_COMPONENTS: ComponentItem[] = [
  {
    id: 'CMP-A-001',
    name: 'Component A (Rotor Assembly)',
    type: 'Mechanical / Electro-magnetic',
    material: 'Titanium Alloy Ti-6Al-4V',
    mass: 2.48,
    massUnit: 'kg',
    dimensions: '120 x 80 x 65',
    dimUnit: 'mm',
    connectedTo: 3,
    status: 'Critical',
    origin: 'Simulated',
    validationStatus: 'Verified',
    temperature: 92.4,
    load: 62.0,
    vibration: 21.4,
    efficiency: 87.2
  },
  {
    id: 'CMP-B-002',
    name: 'Component B (Stator Housing)',
    type: 'Structural Casing',
    material: 'Anodized Aluminum 6061-T6',
    mass: 4.15,
    massUnit: 'kg',
    dimensions: '210 x 180 x 140',
    dimUnit: 'mm',
    connectedTo: 5,
    status: 'Active',
    origin: 'Observed',
    validationStatus: 'Verified',
    temperature: 58.1,
    load: 44.0,
    vibration: 12.0,
    efficiency: 94.5
  },
  {
    id: 'CMP-C-003',
    name: 'High-Speed Planetary Gearbox',
    type: 'Kinematic Transmission',
    material: 'Case-Hardened Steel 16MnCr5',
    mass: 3.82,
    massUnit: 'kg',
    dimensions: '150 x 150 x 90',
    dimUnit: 'mm',
    connectedTo: 4,
    status: 'Warning',
    origin: 'Derived',
    validationStatus: 'Unverified',
    temperature: 78.6,
    load: 76.5,
    vibration: 38.2,
    efficiency: 91.0
  },
  {
    id: 'CMP-D-004',
    name: 'Direct Drive Inverter Module',
    type: 'Power Electronics / SiC MOSFET',
    material: 'Silicon Carbide / Copper Substrate',
    mass: 1.12,
    massUnit: 'kg',
    dimensions: '95 x 65 x 30',
    dimUnit: 'mm',
    connectedTo: 2,
    status: 'Active',
    origin: 'Simulated',
    validationStatus: 'Verified',
    temperature: 64.2,
    load: 55.0,
    vibration: 5.1,
    efficiency: 98.4
  }
];

// Causality Investigation Nodes
export interface CausalNode {
  id: string;
  label: string;
  type: 'event' | 'condition' | 'root-candidate';
  parameter: string;
  currentVal: string;
  previousVal: string;
  change: string;
  evidence: string;
  isCandidateRoot: boolean;
}

export const MOCK_CAUSAL_CHAIN: CausalNode[] = [
  {
    id: 'c1',
    label: 'Thermal Overload Failure',
    type: 'event',
    parameter: 'Temperature',
    currentVal: '92.4 °C',
    previousVal: '65.0 °C',
    change: '+27.4 °C (+42%)',
    evidence: 'Simulation Result Verified',
    isCandidateRoot: false
  },
  {
    id: 'c2',
    label: 'Temperature Surge in Rotor Core',
    type: 'condition',
    parameter: 'Heat Flux',
    currentVal: '4.8 kW/m²',
    previousVal: '2.1 kW/m²',
    change: '+2.7 kW/m²',
    evidence: 'Model Equation Validated',
    isCandidateRoot: false
  },
  {
    id: 'c3',
    label: 'Copper Loss & Joule Dissipation',
    type: 'condition',
    parameter: 'Ohmic Loss',
    currentVal: '320 W',
    previousVal: '110 W',
    change: '+210 W (+190%)',
    evidence: 'Sensor Data Matching',
    isCandidateRoot: false
  },
  {
    id: 'c4',
    label: 'Phase Current Saturation',
    type: 'condition',
    parameter: 'RMS Current',
    currentVal: '48.2 A',
    previousVal: '24.0 A',
    change: '+24.2 A (+101%)',
    evidence: 'Observed Telemetry',
    isCandidateRoot: false
  },
  {
    id: 'c5',
    label: 'Candidate root cause: Excessive Mechanical Load',
    type: 'root-candidate',
    parameter: 'Shaft Torque Load',
    currentVal: '85.4 N·m',
    previousVal: '32.0 N·m',
    change: '+53.4 N·m (+167%)',
    evidence: 'Causal Link Candidate (Unverified Counterfactual Required)',
    isCandidateRoot: true
  }
];

// Validation Checklist (Screen 14)
export interface ValidationRow {
  id: string;
  name: string;
  context: string;
  count: string;
  errorVsThreshold: string;
  uncertainty: string;
  credibility: string;
  passed: boolean;
}

export const MOCK_VALIDATION_RULES: ValidationRow[] = [
  {
    id: 'V1',
    name: 'Model Mathematical & Geometric Validation',
    context: 'CAD geometry manifold check & Euler equation integrity',
    count: '14 of 14 checks passed',
    errorVsThreshold: '0.04% < 0.50%',
    uncertainty: '±0.02 mm',
    credibility: 'Tier 1 (Bitwise)',
    passed: true
  },
  {
    id: 'V2',
    name: 'Physics & Conservation Consistency',
    context: 'Energy, momentum, mass conservation across dynamic steps',
    count: '32 of 32 checks passed',
    errorVsThreshold: '0.008% < 0.100%',
    uncertainty: '±0.005 J',
    credibility: 'Tier 1 (Bitwise)',
    passed: true
  },
  {
    id: 'V3',
    name: 'Boundary Conditions Feasibility',
    context: 'Thermal conduction flux and mechanical stress constraints',
    count: '18 of 18 checks passed',
    errorVsThreshold: '0.12% < 1.00%',
    uncertainty: '±0.4 °C',
    credibility: 'Tier 2 (Tolerance)',
    passed: true
  },
  {
    id: 'V4',
    name: 'Result Verification & Gateway Hash',
    context: 'Deterministic bitwise seed verification across solver cores',
    count: '8 of 8 checks passed',
    errorVsThreshold: '0.00% == 0.00%',
    uncertainty: '0.000 (Bitwise)',
    credibility: 'Tier 1 (Bitwise)',
    passed: true
  },
  {
    id: 'V5',
    name: 'Safety Compliance (ISO 26262 ASIL-D)',
    context: 'Thermal limit trip timing under high-load contingency',
    count: '4 of 5 checks passed (1 FAILING)',
    errorVsThreshold: 'Trip latency 124 ms > 100 ms limit',
    uncertainty: '±4.2 ms',
    credibility: 'Tier 2 (Tolerance)',
    passed: false // HONESTY: One failing row in red! Never marked verified!
  },
  {
    id: 'V6',
    name: 'Real-time Performance Latency Check',
    context: 'Scrubber execution at 60 fps under 100k datapoints',
    count: '6 of 6 checks passed',
    errorVsThreshold: '14.2 ms < 16.6 ms budget',
    uncertainty: '±0.8 ms',
    credibility: 'Tier 1 (High)',
    passed: true
  }
];

// Compute Cluster Nodes (Screen 13)
export interface ComputeNode {
  id: string;
  name: string;
  type: 'Local' | 'Edge' | 'Cloud' | 'Quantum';
  status: 'Ready' | 'Active' | 'Standby' | 'Preview';
  loadPercent: number;
  specs: string;
  routingReason: string;
  isPreview?: boolean;
}

export const MOCK_COMPUTE_NODES: ComputeNode[] = [
  {
    id: 'node-01',
    name: 'Node-01 Local Workstation',
    type: 'Local',
    status: 'Active',
    loadPercent: 68,
    specs: '16 Cores CPU, 64 GB RAM, RTX 4090',
    routingReason: 'Routed to Local GPU: interactive viewport & real-time scrubbing'
  },
  {
    id: 'node-02',
    name: 'Node-02 Edge Device Jetson-AGX',
    type: 'Edge',
    status: 'Ready',
    loadPercent: 24,
    specs: 'NVIDIA Orin 64GB, 275 TOPS INT8',
    routingReason: 'Routed to Edge: hardware-in-the-loop sensor telemetry ingest'
  },
  {
    id: 'node-03',
    name: 'Node-03 Cloud HPC Cluster',
    type: 'Cloud',
    status: 'Active',
    loadPercent: 88,
    specs: '64x H100 80GB SXM5, 3.2 Tbps InfiniBand',
    routingReason: 'Routed to Cloud GPU: batch of 200 counterfactual branches'
  },
  {
    id: 'qpu-01',
    name: 'QPU-01 Quantum Coprocessor',
    type: 'Quantum',
    status: 'Preview',
    loadPercent: 0,
    specs: '127 Qubit Transmon Array (Simulator Backend)',
    routingReason: 'Topology map exploration (PREVIEW, simulator only)',
    isPreview: true
  }
];
