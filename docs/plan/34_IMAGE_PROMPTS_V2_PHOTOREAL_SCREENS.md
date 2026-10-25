# 34. Image Prompts v2 (photoreal, matches your taste) for Every Key Screen

Use in your image generator. Generate each screen twice with a different [SUBJECT] to avoid locking the product to one domain. These are visual references for Antigravity; text will be re-typed in code.

[SUBJECT] examples: "a quadcopter drone" or "a pump driven by an electric motor on a test bench" or "a small electric vehicle". [SCENE] examples: "mountain valley at sunset", "clean hangar with soft studio light", "engineering test lab".

## STYLE BLOCK (paste first, every time)

Photorealistic professional engineering software UI, desktop web app, 16:9, flat front-on screenshot, no device mockup. Layout: left navigation 176 px with hex-arrow HELQIX logomark and "Engineering studio"; header card with project thumbnail, project name, green Active chip, version and "Saved 2 min ago", count chips, and a large blue "Run simulation" split button on the right; dark navy glass panels (#0E1B2E) with 14 px radii and hairline borders; one bright blue (#2F7BFF) for primary and selection; violet Assistant button bottom-left; light and dark toggle, bell, avatar. The hero is a photoreal, physically accurate [SUBJECT] in a cinematic [SCENE], with realistic materials (brushed metal, carbon fibre, rubber, copper), soft shadows, filmic color. Panels are readable and restrained, not neon. Every number has a unit. Small colored status dots, tidy tabular figures, Inter or IBM Plex Sans typography. Sharp legible small text, no gibberish, no watermarks.

## SCREENS

### 1 Hero / Welcome
Full-bleed cinematic [SUBJECT] in [SCENE] with a soft dark vignette at the bottom. Headline "Engineering truth, with proof." Subtext one line. A large glass prompt box "Describe what you want to engineer…" with buttons Attach files, Start from scratch, Import from tools, and a blue Build model button. Beneath it a thin "Proof strip": Run, Replay matches, Requirement checked, Signed, Verified by anyone. Small row "Connects to the tools you already trust" with tool chips. Top bar: logo, Product, Integrations, Docs, Pricing, Sign in, Start free.

### 2 Home
Greeting and a big prompt box with three options. Grid of project cards, each with a real rendered thumbnail of the object in its scene, project name, last run state and a trust chip. Recent runs list. Right column: Needs attention, Connected sources, Trust summary.

### 3 Define: Intent
Centered panel "What do you want to engineer?" with mode tabs Text, Reference, Voice (with a small preview tag), Blank. Text area with a realistic example prompt about [SUBJECT], suggestion chips, a character counter, and a blue "Interpret" button. Stepper across the top: Intent (active), Requirements, Architecture, Components, Assemble. Faint photoreal [SUBJECT] scene blurred behind.

### 4 Define: Requirements and Architecture
Left: the brief and six brief cards (System, Components, Connections, Environment, Scenario, Requirements) with status chips. Center: an architecture block diagram of the [SUBJECT] subsystems over a translucent render of the object, blocks grouped in dashed subsystem boxes, links colored by type with a legend (electrical, mechanical, thermal, data). Right: "Extracted requirements" list with values, units and source tags (stated, assumed, calculated). Bottom: three question cards with Use default, Answer, Skip. Blue "Continue to components".

### 5 Define: Components
Two columns: left category list for the chosen block; right "Select component" cards from connected sources, each with a product-style render thumbnail, name, key specs with units, a source label with "as of" date, a maturity tag (Validated model, Unvalidated model, Placeholder), and Select. Top shows ranking reasons ("matches voltage and port types"). A card for "Define from scratch" and "Import file".

### 6 Define: Assemble
Center: the object assembling in 3D, parts flying into place over a clean studio floor. Left: checklist of real steps with ticks (Resolving sources, Importing geometry, Placing parts, Connecting ports, Checking units) and a percentage. Right: list of warnings (for example "Pump has no model") with Fix buttons. Button "Create model r1".

### 7 Studio: Model (Engineering view)
Neutral studio background with subtle grid, the [SUBJECT] shown realistic with one component selected (blue outline, gizmo). Left: component list with render thumbnails. Right: Component details with photo, tabs Parameters, Behavior, Ports, Source, sliders with units, and Material card showing a real PBR swatch. Bottom four cards: Issues, Connections, History, Sources. Toggle Realistic or Wireframe, camera presets.

### 8 Studio: Simulate (Realistic view)
[SUBJECT] in motion in [SCENE] with the follow camera: propellers or rotor spinning with motion blur, a floating HUD card (four live values with units and badges Simulated, Unverified), a small compass. Left: Scenario cards numbered 01 to 05 with tags (Current, Baseline) and play buttons, then Simulation controls (Pause, Stop, speed 0.5x 1x 2x 4x, time 32.4 s of 120 s). Right: Environment card with weather chips and sliders; Scenario parameters. Bottom four cards: Live telemetry tiles with sparklines, Real-time graphs with a cursor tooltip, Event stream, State inspector with a top-down schematic of the object.

### 9 Studio: Failure event
Same layout; the failing part glows red with a heat gradient, a red banner "Winding temperature above limit at 8.42 s" with buttons Investigate failure and Replay. Primary button top right reads "Investigate failure". Graphs show the threshold line crossing with a marker.

### 10 Studio: Causal analysis
Left list of failures and observations. Center: the object with the candidate cause highlighted and thin leader lines. Right: vertical "Candidate chain" of five rounded nodes with arrows and a node detail card (relation, previous value, value at failure, Origin and Status). Label "Candidate: structural and temporal". Button "Test this cause". Bottom: replay check card "Replay matches original".

### 11 Studio: What-if and Compare
Top: tabs Original, Cooling +20 %, Load -10 %, New branch. Left: intervention sliders with units and "Forked from freeze at 8.0 s". Center: two photoreal views side by side (original with red hot spot, branch cooler) with a divergence marker. Right: metric deltas, a result card "Meets R1 in the model. Not yet validated." Bottom: overlay curves and a branch strip of thumbnails R0 to R3. Primary button "Compare".

### 12 Studio: Verify
Left requirements list with pass, warning, not run states. Center: the object with small requirement markers. Right: credibility checklist (validation error against rig data, uncertainty, determinism, numerical checks, independent review) with the missing items clearly listed and a card "Not yet verified for this use". Bottom: evidence package composer with six included items and "Sign and export".

### 13 Evidence sheet
A tall slide-over sheet on the right of the Studio: package contents tree, root hash, signature status, verifier command with a Copy button, a note "Signing proves origin and integrity, not correct physics", buttons Export evidence and Generate report.

### 14 Observe
The real [SUBJECT] on a test bench photo-blended with its digital twin overlay (solid simulated, dashed observed). Left: device list with status. Right: residual cards, anomaly events, a note "Observed data never overwrites the model", button "Propose model update". Bottom: observed vs simulated chart.

### 15 Project workspace (Home project page)
Project header, tabs Overview, Variants, Simulations, Reports, Settings. Variants strip R0 to R3 with real renders and colored chips (Failed, Recommended, Valid). Activity list and a trust summary.

## Rules for every prompt
No invented vendor data: if a product name appears, label the card "Example". Every value has a unit. No "Verified" or "Calibrated" badge unless the screen explicitly shows its criteria. Keep text short and legible.
