# 24. HELQIX UI Screen Image Prompts (16 screens, same look as the uploaded set)

How to use: paste STYLE BLOCK first in the image generator, then one SCREEN prompt. Generate at 16:9, 1600x900. Keep the same seed or style reference image across screens so they match. Image generators garble small text, so treat the images as VISUAL REFERENCES; the real UI is rebuilt in code (file 23) with real text and real data. Pass the images plus file 23 and file 25 (below) to Antigravity.

## STYLE BLOCK (paste before every screen)

Ultra-premium dark engineering software UI, desktop web app, 16:9, flat front-on screenshot (not a device mockup, no perspective, no hands). Background deep navy-black (#050B18) with subtle blue glow. Glass panels with 1px cool-blue borders, 12-16px rounded corners, soft inner glow. Accent electric blue (#2F7BFF) and cyan (#22D3EE), secondary violet (#8B5CF6); status colors green (#22C55E), amber (#F59E0B), red (#EF4444). Typography: clean geometric sans (Inter style), wide letter-spaced wordmark "HELQIX" with the Q in glowing blue at top-left. Left sidebar (collapsed icons plus labels): Overview, Model, Simulate, Understand, Improve, Build, Compute, Verify, Edge; active item highlighted with blue bar; bottom-left small glass card with a glowing planet-arc and text "HELQIX ENGINEERING INTELLIGENCE". Top bar: breadcrumb (Project > Workspace > ...), search icon, theme toggle, notification bell with dot, round avatar ring. Central hero is a translucent glowing blue/violet 3D engineering model on a circular reflective platform with thin grid lines. Charts are crisp with thin glowing lines, units labelled on axes. Legible small text, consistent spacing, no lorem ipsum gibberish, no watermarks. Mood: precise, trustworthy, futuristic but calm.

## HONESTY RULES (add to every prompt that shows results)

Every number shows a unit. Every value or result carries a small badge pair: Origin (Observed / Simulated / Derived / AI-hypothesis) and Status (Verified / Unverified). A chip near the run name shows determinism level (Bitwise / Tolerance / Statistical). Use the wording "Candidate root cause" (never "Root cause" for unproven chains). Never show a green Verified badge on a failing requirement. Show a "PREVIEW" tag on features not built yet. Spell "Verified" correctly.

---

## SCREEN PROMPTS

### 01 Landing / Entry
Center: huge thin glowing circular portal ring over a dark mountain horizon with reflective circular platform. Wordmark HELQIX with blue Q, tagline "Engineering truth, with proof." Below, small caption "Simulate. Understand. Improve." Two buttons: "Get Started" (filled blue) and "Watch Demo" (outline). Bottom row of four small icons: Model, Simulate, Analyze, Deploy. Minimal, cinematic.

### 02 Home / Overview
Sidebar active: Overview. Top KPI cards: Active Projects, Runs This Week, Requirements Passing (percent with count, e.g. 18 of 22), Compute Hours. Left: Recent Projects list with status chips (Running, Completed, Needs Review) and determinism chip. Right: "Trust Summary" panel (requirements passed, validation status, last evidence package exported). Bottom quick actions: New Project, Import Model, Run Simulation, View Reports. Do not show a fake global world map.

### 03 Engineering Workspace / Reality Canvas
Large central 3D glowing model on the platform. Top mode tabs: Reality (active), Time, Causality, Branch, Verification. Right panels: System State (labelled bars with units), Components (list with status dots), Layers (Structure, Physics, State, Flow, Environment toggles). Bottom timeline with play/stop/step, time 00:04.20 / 00:12.00, event markers (red failure marker), phase chips Init, Run, Event, Result. Header chip: run id, model revision hash (short), determinism level.

### 04 Model Editor
Left Model Explorer tree (System > Structure > Component A/B, Thermal, Electrical, Control, Environment, Sensors) with colored icons. Center exploded view of a mechanical assembly (gearbox/motor housing) in translucent blue with violet internals and axis gizmo. Right: Component Properties (ID, Type, Material, Mass in kg, Dimensions in mm) with Properties/Parameters/Relations tabs; Visual Settings (view mode, transparency slider, highlight, show connections/sensors toggles). Bottom: Component Library grid with search and filter, and Component Details card.

### 05 Simulation Setup
Top tabs: Setup (active), Run, Results, Compare. Left: Scenario selector with Initial Condition, Environment, Inputs, Constraints, Events. Center: 3D model with glowing field lines. Right: Parameters sliders with units, and Simulation Settings (Solver, Time step, Duration, Precision dropdowns). Bottom-left Run Configuration radio: Deterministic (selected, with "Bitwise" sub-label), Performance, Balanced, Exploration. Bottom: timeline preview. Primary button "Next".

### 06 Simulation Run
Tab Run active, green "Running..." pill, pause and stop buttons. Left Run Configuration (Scenario, Environment, Duration slider in min, Resolution, Seed 8421). Center live 3D with flowing particle streams and a hot spot. Right: Live Metrics bars with units and Run Status steps (Initializing, Loading Model, Simulating, Processing, Finalizing with timestamps). Bottom: Simulation Timeline chart with multi-color curves and playhead 00:06.14 / 00:20.00; Snapshots carousel of three thumbnails. Add a "Freeze here" button near the timeline.

### 07 Simulation Results
Tab Results active. Left Runs list (Run 01..04 with color dots). Center 3D/Slice View toggle with translucent slices and an orange-red heat region. Right: Key Metrics (temperature in degC, power in kW, etc.) and State Timeline (Init, Stable, Transition, Critical, Recovered with times). Bottom: Parameter Trends line chart and Distribution histogram with units. Each metric row has Origin x Status badges.

### 08 Failure Investigation / Causality
Breadcrumb ends "Failure Investigation". Tabs: Causality (active), Timeline, Evidence, Report. Center-left: 3D rotor/motor assembly with a thermal heatmap and callout "Thermal Overload"; component card (Rotor Assembly, 92.4 degC, status Critical); color scale 20 to 120 degC. Center: "Causal Chain" vertical nodes: Thermal Failure, Temperature up, Copper Loss up, Current up, Load up, RPM down; header says "Candidate chain (structural + temporal)". Right: "Selected Cause Details" with description, Previous Value, Current Value, Change, an Evidence list (Simulation Result Verified, Sensor Data Matching, Causal Link Candidate, Model Equation Validated) and a blue button "Run Counterfactual Simulation". Bottom-left Event Timeline list (0.00s Normal Operation ... 8.42s Thermal Failure ... 9.10s System Shutdown); bottom-center chart with red failure marker at 8.42 s and playback controls; bottom-right Simulation State card (Temperature, Current, Load, RPM, Power with units).

### 09 Compare
Tab Compare active. Two side-by-side 3D panels: Scenario A (blue) and Scenario B (violet with red hot spot), each with a thumbnail strip. Right: Comparison Metrics with paired bars and signed deltas, and an Insight list with green/red arrows (statements must be neutral wording, for example "Peak temperature 8 percent lower"). Bottom: overlay timeline with both curves, speed dropdown 1.0x, first-divergence marker, phase chips.

### 10 Scenario Builder
Step bar: 1 Scenario, 2 Environment, 3 Inputs, 4 Constraints, 5 Events; Save and Continue buttons. Left scenario list (Base, High Load, Low Temperature, Stress Test, Custom 01). Center glowing 3D system with a flowing signal line. Right: Scenario Parameters sliders and Condition Preview mini chart with three readouts. Bottom: node flow builder (Initial State, Environment, Inputs, Constraints, Events connected by colored links) and Quick Presets (Normal, Extreme, Cold, Hot).

### 11 Timeline / Replay
Large 3D model with floating event callouts (Load Increase, Temperature Rise, Anomaly Detected) in colored boxes. Right: Event Details card (event, component, parameter, threshold, severity, possible cause as "Candidate") and "View Analysis" button. Bottom: multi-line parameter timeline with colored event markers and a dense playback bar; add "Replay from freeze" and a hash-match indicator "Replay matches original: PASS" with the determinism level chip.

### 12 Build / Optimization
Left: Optimization Goals (Maximize efficiency, Minimize weight, Improve stability) and Algorithm dropdown, Iterations field, button "Start Optimization". Center: 3D assembly with a circular progress ring (72 percent, "Running... Iteration 366/500"). Right: Results Preview with deltas and units. Label proposals as "Proposed change (Unverified)" with a "Validate by branch simulation" button.

### 13 Compute / Resources
Segmented control: Local, Edge, Cloud, Quantum (Quantum marked "PREVIEW, simulator only"). Resource cards: CPU, GPU, RAM, Storage with percent and units. Cluster Nodes cards: Node-01 Local, Node-02 Edge, Node-03 Cloud, QPU-01 Quantum with status chips. Show routing reason line, for example "Routed to GPU: batch of 200 branches".

### 14 Verify / Validation
Left checklist with check marks: Model Validation, Physics Consistency, Boundary Conditions, Result Verification, Safety Compliance, Performance Check. Each row states pass or fail with a count. Center: large shield with check on a 3D model. Right: Validation Report card showing Context of Use, requirement outcomes, model error vs threshold (value with unit), uncertainty, credibility level, and status "PASS" only if all criteria pass. Button "Generate evidence package" and a small "Signed: yes/no, verifier command" line. Include one failing row in red to show honesty.

### 15 Deploy / Export
Export Model (STL, OBJ, FBX, STEP marked PREVIEW), Export Results (CSV, JSON, Video), Evidence Package (signed archive, manifest, root hash). Center cloud-and-device illustration. Right: Deploy to Edge Device, Integrate API, IoT Gateway, Webhook. Primary button "Export evidence package".

### 16 Edge / Real-time Monitoring
Left stats: Active Devices, Error Devices, Signal Quality. Center: abstract device map (no real-world claims). Right Live Feed list of devices with Online/Warning/Offline chips. Bottom: Sensor Data line charts (Temperature, Vibration, Load, Power) with units and a badge "Origin: Observed". Add a "Simulated vs Observed" overlay toggle and residual readout.

---

## Notes for the founder

1. The uploaded set shows a motor assembly. The wedge domain (battery vs motor) is still undecided (D1). Keep screens domain-neutral or generate both variants after D1.
2. Uploaded images contain issues to fix in the real build: "Verifed" typo, "Root Cause" wording for unproven chains, "Failure Prediction 87 percent" with no basis (remove or label HYPOTHESIS), a global map and "Success Rate 96.3" with no definition, and Quantum shown as active compute. All fixed in the prompts above.
3. Image prompts only give looks. Behavior, data and labels come from files 01, 02, 23 and the phase prompts.
