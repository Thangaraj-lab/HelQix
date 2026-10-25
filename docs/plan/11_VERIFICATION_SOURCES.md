# 11. Verification Sources (what was checked on the web, 25 Oct 2026)

## VERIFIED

| Claim | Source |
|---|---|
| FMI has save and restore of FMU state, but implementation is not mandatory and exists mainly in some commercial tools | https://arxiv.org/abs/2106.01343 (search result excerpt) and https://github.com/modelica-tools/FMUComplianceChecker/issues/50 (compliance check for state store and restore) |
| FMU state restore can be subtly wrong in practice (example issue with a Simulink FMU; root cause was an uninitialized model variable) | https://github.com/modelica/fmi-standard/discussions/2189 |
| ASME V&V 40:2018 gives a risk-based credibility framework for computational models in medical devices; the subcommittee considers it general enough for other disciplines | https://webstore.ansi.org/standards/asme/asme402018 |
| FDA recognizes V&V 40; the standard applies to physics models, not to statistical or AI models; relies on Context of Use and model risk | https://www.accessdata.fda.gov/scripts/cdrh/cfdocs/cfstandards/detail.cfm?standard__identification_no=38534 |
| V&V 40 foundations include Sandia PCMM and NASA-STD-7009; NASA-STD-7009 prescribes required levels per risk | https://nafems.org/blog/posts/credibility-is-everything-or-how-to-get-the-most-from-verification-and-validation |
| Ansys Minerva SPDM gives requirement tracking, version control, traceability and audit trails; powered by Aras | https://www.ansys.com/blog/how-to-meet-the-top-5-spdm-challenges and https://aras.com/en/capabilities/simulation |
| Three.js WebGPURenderer uses WebGPU by default and falls back to WebGL 2; it is described as experimental; WebGLRenderer remains maintained and recommended for pure WebGL 2 apps | https://threejs.org/manual/en/webgpurenderer |
| INA219 modules: bus voltage up to 26 V, stock 0.1 ohm shunt gives about plus or minus 3.2 A, 12-bit ADC | https://docs.soldered.com/ina219/overview/ and https://components101.com/modules/1na219-current-sensor-module |

## VERIFIED by running code (25 Oct 2026)

Build and test results in 04. The README statements about what the repo contains were read directly from README.md and docs/PRODUCT_DEFINITION.md in the repo.

## UNVERIFIED (do not use publicly)

- ISO 26262 and DO-178C tool-qualification routes for simulation tools (from memory).
- That PyBaMM SPM / Chen2020 supports the thermal failure analysis needed for the wedge (H3).
- That public battery datasets match the chosen cell (H6).
- Teamcenter capabilities.
- That incumbents lack a native freeze, branch, causal-attribution and signed-evidence workflow (competitor check not done).
- Any market size, price, CAC, LTV, customer count (none computed; no data).
- Whether the repo's Python gateway, UI, PyBaMM run and branch compare actually work (not run).

## Source-use rules

Prefer primary sources (standards bodies, official docs, filings, papers). Do not quote numbers without opening the source. Mark memory-based statements "approximate" or "UNVERIFIED".
