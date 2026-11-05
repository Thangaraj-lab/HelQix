# E4.4: Studio walkthrough with a real FMU (2026-11-05)

Setup (MEASURED): API built in WSL Ubuntu with PostgreSQL 18 and an FMU library holding the Modelica Reference FMU **BouncingBall** (BSD-2, Example data) under its SHA-256; FMPy 0.3.32 and Pint 0.26.2 in a venv; Studio dev server in real mode, driven in the built-in browser.

| Step | Result |
|---|---|
| New project, add a part | Works. No density warning for a part that will carry an FMU |
| Part inspector: Behaviour panel | Lists the library (`1c7a40ab…30b7`, 213 KB), read from the real API |
| Choose the FMU | The FMU describes itself: BouncingBall (FMI 2.0); parameters `g` (m/s2, start -9.81) and `e` (no unit, start 0.7); outputs `h` (m) and `v` (m/s) ticked, `der(h)` and `der(v)` offered as internal |
| Set `e` to 0.5, Attach FMU | The part shows "FMU 1c7a40ab…30b7, records h, v, sets e"; the build is Ready |
| Create model r1, open Simulate | The run form says the part runs the FMU through FMPy; duration 3 s and step 0.01 s come from the FMU's own default experiment |
| Run my model | A run with 301 frames, determinism **Tolerance**; the graph of `h` shows the ball bouncing lower each time (restitution 0.5) |
| Verify | "Replay matches the original: 50 of 301 frames re-read"; the evidence package lists the run frames |

## Found and fixed during the work

1. A late answer for an FMU the user had already moved away from replaced the current one (Behaviour panel). A late description overwrote a duration the user had typed (run form). Both have tests.
2. The dev setup: Windows' relay to WSL stalled on port 8090; the Studio proxy was pointed at the WSL address instead. A local setup detail, not a product defect.

## Not done / limits

- A parameter whose FMU unit is not in the HELQIX unit registry (for example `1/s` or `N.m`) is offered but becomes a blocking issue at commit ("not a registered unit"); the panel does not warn first.
- The run's note ("Ran the FMU ...") disappears when the page switches to the new run; the Events panel still shows the run id and determinism.
- Only one FMU part per model runs. Freeze and branch of an FMU run are refused by design.
- CI does not run any of this yet (E4.5). Screenshots are in the session, not in the repository.
