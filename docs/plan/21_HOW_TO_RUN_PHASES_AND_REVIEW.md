# 21. How to Run Each Phase and How to Review It (for a non-coding founder)

## The loop (same for every phase)

1. Check the previous gate is PASSED in 12_STATUS_BOARD.md. If not, do not start.
2. Open Claude Code in the repo. Paste 13_COMMON_PREAMBLE.md, then the phase prompt. Fill any placeholders only with decisions you actually made (see 08). Never let Claude Code invent them.
3. Let it work. It must stop at the gate and give the report format from 13.
4. Review with the checklist below. Do not accept "done" without raw test output.
5. Update 12_STATUS_BOARD.md and the decision log in 08. Only then start the next phase.

## Phase order and what you must decide first

| Phase | File | Needs from you before starting |
|---|---|---|
| 0 Hygiene | 09 | D7 approve Phase 0 |
| 1 Trust core | 10 | D1 first domain, D2 Context of Use and threshold X, D4 data protocol |
| 2 Replay | 14 | D5 runtime owner (from Phase 0 trace) |
| 3 Causality | 15 | none new |
| 4 Branches | 16 | none new |
| 5 Signed evidence | 17 | key custody decision (dev keys only until decided) |
| 6 Adapters | 18 | tool licence check result; which solvers you can install |
| 7A Edge | 19 | rig BOM, safety plan, real data |
| 7B Agent | 19 | provider choice and budget |
| 7C Quantum | 19 | only if you want it; hardware needs account and budget |
| 8A Build Spec | 20 | pilot feedback on what lineage they need |
| 8B Research | 20 | time-box approval |

## Review checklist after any phase (you do not need to read code)

Ask Claude Code to answer these in plain English and paste raw output:

1. Which gate criteria were met? Show the exact command and its output for each.
2. Test counts before and after. Did any count go down? If yes, why (it should be no).
3. Did you modify, skip or loosen any existing test? (must be no, or show evidence the test was wrong)
4. Show one test failing on purpose (mutation check) and then passing.
5. What did you NOT verify? What is still a stub?
6. Which of your claims depend on assumptions I should confirm?
7. List every file outside the phase folders that you changed, and why.
8. Is anything unpushed, uncommitted, or only on your machine?

## Red flags (stop and push back)

- "All tests pass" without pasted output.
- Test count dropped, or tests marked ignore/skip appeared.
- New thresholds or numbers appeared that you did not provide.
- Words like "verified", "proven", "root cause" in UI or docs for things that have no test.
- Big refactors or renames "while I was there".
- A phase finished suspiciously fast with large new code but no new tests.
- Claims about cross-platform bitwise reproducibility, quantum hardware, or independent validation with no logs.

## Independent second check (recommended for Phases 1, 2, 5)

Use a fresh Claude Code session, give it only the repo and the gate criteria, and ask it to try to BREAK the claim (for example, find a nondeterminism source, forge a package that passes the verifier, or find a case where an invalid result is labeled Verified). The builder should not grade its own work. A human domain expert reviewing Phase 1 thresholds is better still (D12).

## Time and cost estimate (ASSUMPTION, judgement; re-estimate after Phase 0)

Phase 0: days to about 2 weeks. Phases 1 to 5 (the wedge MVP): about 3 to 4 months part-time with AI assistance. Phase 6: 3 to 6 weeks. Phase 7: depends on hardware and data, 1 to 3 months. Phase 8: open-ended. These are not measurements; replace with actuals after Phase 0 and 1.

## If a gate fails

Do not weaken the gate. Record why in the decision log, apply the kill trigger from 06 if one applies, and decide: fix, redesign, or drop the claim. Dropping an overclaim is a valid and good outcome.
