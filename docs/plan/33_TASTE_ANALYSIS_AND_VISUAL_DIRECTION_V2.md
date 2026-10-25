# 33. Your Taste, Read from the Images + Visual Direction v2 + Correct Navigation

Images analyzed: the first set (abstract neon cube and motor screens), the 16-screen EV journey sheet (shown twice), and the three drone screens (mountain sim, project view, Drone-X1 hangar). Diagrams: wireframes 12, 13, 14.

## 1. What I see, and what it says about your taste (my reading; confirm or correct)

The two sets are different design philosophies, and you moved from the first to the second.
- Set 1 (cube, hologram, motor wireframe): abstract, neon, atmospheric. Beautiful but the hero object is a symbol, not a thing.
- Set 2 (EV journey, drone screens): a real object in a real environment, data attached to the object, a clear guided story. This is where your taste is going. I read the last six images as your actual target.

Taste profile (T1 to T10):
1. **Real objects, real places.** A photoreal drone over mountains at sunset, a car on a road, a drone in a hangar. The object is the hero, the scene gives it truth.
2. **Dark glass panels around a bright scene.** Deep navy panels, 12 to 16 px radii, hairline borders, white text, one bright blue for primary actions and selection. Neon is used sparingly.
3. **Dense but organized: five zones.** Left lists with thumbnails (components, scenarios), center scene, right details plus environment, bottom row of four cards (live telemetry, graphs, event log, map or state inspector).
4. **Data lives on the object.** Floating HUD card over the scene, telemetry tiles with sparklines, gauges for the vehicle, event log with colored dots, a route map.
5. **A guided journey.** "Engineer Any Reality" welcome, intent box with modes (Text, Voice, Reference, Blank), suggestion chips, extracted requirements, an architecture block diagram, source cards to choose from, a generation progress screen, then the studio. You like the feeling that the system builds it with you.
6. **Product feel.** Hex-arrow logomark with HELQIX and "Engineering Studio", project header with version and "Saved 2 min ago", counts (8 components, 3 scenarios, 14 simulations, 2 branches), light and dark toggle, notification bell, avatar with name.
7. **Clear primary action.** A big blue Run button with a dropdown, top right. Violet AI Assistant button bottom left.
8. **Control of the view.** Perspective, Top, Front, Right; Follow, Orbit, FPV; Realistic or Wireframe; weather chips (Clear, Cloudy, Rain, Fog, Night) with wind, temperature, humidity, pressure sliders.
9. **Story of change.** What-if sliders, side-by-side realities with PASS or FAIL overlays, a green "Recommended design" card, a Design Iteration strip R0 to R3 with thumbnails.
10. **Scenario cards** with numbers and tags (Current, Baseline, Wind 15 m/s) and play buttons.

## 2. Adopt, adapt, drop

Adopt as is: T1 to T10 visually, including the five-zone layout, 4-card bottom row, HUD card, source cards, branch strip, hex logomark, project header, light and dark toggle, view controls, weather chips.

Adapt (to keep the product honest; each is a place where the images would mislead a real user):
| In the images | Problem | What we do |
|---|---|---|
| "Model status: CALIBRATED, confidence High", "Mark model as Verified/Calibrated" button | Blanket claim, manual toggle, generic confidence | Remove the button. Show credibility per Context of Use, computed by the Trust layer, with missing items listed. Button becomes "Run validation". |
| Green "Recommended design: meets requirements" | Overclaim | "Meets R1 to R3 in the model. Not yet validated." Both facts shown. |
| "Root cause analysis", "Why did this happen?" with confident chain | Unproven causes | "Candidate chain"; "Test this cause" creates an intervention branch. |
| Vendor battery cards with names and specs | Invented or unlicensed data, trademarks | Source cards show data only from a connected catalog, with source and as-of date. Brand names only if the catalog supplies them. Examples labeled Example. |
| "Generate 3D geometry: creating chassis, placing battery…" | Text-to-engineering-geometry is not reliable | The progress screen shows real steps: resolving sources, importing, assembling, flagging placeholders. Generated proxy shapes are tagged PLACEHOLDER MODEL. No claim of generated engineering CAD. |
| Car dashboard (speed, P R N D, Accelerate, Brake) | Vehicle logic would be built into the core | Core has instrument widgets (gauge, number, bar, graph) bound to variables. A domain pack or source supplies presets such as a vehicle cluster. Interactive inputs (accelerate, brake) only if the backend supports live input; otherwise scripted inputs. |
| Wind streaks and glowing airflow | Decoration may look like solved flow | Show flow only from a solved field. Otherwise show an "Environment preview, not solved" label. |
| Scenic backdrop everywhere | Distracts from measurement | Two looks: Realistic (HDRI scene, default for Simulate) and Engineering (neutral studio with grid, default for Model and Verify). One toggle. |
| Telemetry numbers, event logs, maps | Mock | Real variables or tagged MOCK; unit and Origin x Status on every value. |
| Voice input | Not built | Shown as PREVIEW. |

Drop: the abstract cube as the hero object (keep it only as the empty-state illustration), floating decorative particles, all-caps taglines except inside the brand lockup.

## 3. Visual Direction v2 (replaces the look sections of files 31 and 32 where they differ)

Palette: Void #050B14, Deck #0A1422, Panel #0E1B2E, Panel raised #13253D, Line #22395A, Ink #D6E2F3, Muted #7F93AF, Primary blue #2F7BFF (buttons, selection), Trace cyan #22D3EE (time, replay, determinism), Assistant violet #8B5CF6, green #22C55E, amber #F59E0B, red #EF4444. Light theme with the same roles.
Type: IBM Plex Sans (UI), IBM Plex Mono tabular (values). The hero headline and wordmark in a light, wide-tracked geometric face (Sora Light proposed). Sentence case.
Scenes: real glTF objects, PBR materials, HDRI lighting and background (mountain, hangar, road, lab; choose CC0 HDRIs and verify the license), soft contact shadows, filmic tone mapping, depth of field only in Hero. Terrain from open elevation data optional later.
Panels: dark glass with slight blur only where they overlap the scene; solid elsewhere (readability first).
Motion: object motion bound to simulation; UI motion 150 to 220 ms; the single orchestrated moment is Hero load (object settles, proof strip fills in).
Brand: hex-arrow mark plus HELQIX plus "Engineering studio".

## 4. Studio shell v2 (wireframe 12)

- Left nav 176 px (collapses to icons): Studio, Define, Model, Simulate, Understand, Improve, Verify, Observe, Build (PREVIEW); Assistant button (violet), Settings, Help at the bottom.
- Header card: thumbnail, project name, Active chip, "Model r3 · Saved 2 min ago", count chips, state chips (determinism, signature), the **primary action split button** and the avatar.
- Left panel: lists with thumbnails (Components, or Scenarios with numbered cards and tags, or Branches).
- Center: scene with tool strip (Select, Move, Rotate, Scale, Connect, Measure), HUD card, camera presets, Realistic or Wireframe toggle.
- Right: Component details (photo, tabs, parameter sliders with units) and Environment (weather chips and sliders).
- Bottom: four cards (Live telemetry, Graphs, Event log, Map and state). The dock tabs of file 31 become these cards; a card can be expanded, replaced (Events to Jobs) or collapsed.
Everything in file 31 about stages, overlays and feature coverage stays; only the chrome changes.

## 5. Navigation: the right screen at the right time (wireframes 13 and 14)

Your 16-screen sheet is the canonical journey. It maps to the Studio like this (full table in wireframe 14):
Welcome = Hero. Intent, Requirements, Architecture, Components, Assemble = the five steps of Define (a stepper across the top of the Studio the first time; the **architecture block diagram** is the center view of step 3, before any 3D). Reality studio = Studio. Real-time simulation = Simulate. Failure event, Causal analysis = Understand. What-if, Comparative = Improve. Design iteration = branch strip. Validation and calibration = Verify. Export = Evidence sheet. Project workspace = Home project card.

Rules that make the flow impossible to get lost in:
1. **One primary action, top right, always the next best step:** Run simulation, then Investigate failure, then Test this cause, then Compare, then Verify, then Export evidence. The label and target change with the state of the work; other choices live in the dropdown.
2. **Automatic focus:** when a run fails the camera flies to the failed part, the part glows red, a banner says what failed and when, and the primary action becomes "Investigate failure".
3. **Stepper only while defining.** After model r1 exists the stepper disappears; the left nav is the map.
4. **Stage state is visible:** each nav item shows a small status (for example Simulate: Run 19 running, Verify: 2 gaps, Improve: 2 branches).
5. **Any stage opens any time.** Missing prerequisites show as issues, not walls.
6. **The URL holds project, stage, selection and time**, so back and forward behave.
7. **Breadcrumbs are replaced** by the header card; where you are is always the highlighted nav item plus the selected object.

## 6. Screen quality checklist (use on every screen before accepting)

1. The object is the hero (largest, brightest thing).
2. Every value has a unit and Origin x Status; mock is tagged.
3. One primary button per screen state.
4. No decorative data; no number without a source.
5. Panels readable at 1440 x 900; text at least 11 px; contrast AA.
6. Empty, loading and error states designed.
7. Looks right in Realistic and Engineering view, dark and light.
8. The next step is obvious within 3 seconds.
