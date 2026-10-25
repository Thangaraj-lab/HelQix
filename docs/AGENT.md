# Agent with permissions (Phase 7B)

An AI agent in HELQIX can read, analyze, plan and propose. It cannot change anything without the right permission, and changing the design or running anything real also needs a human approval for that exact action. Everything the agent proposes is **Origin: AI-hypothesis, Status: Unverified** until HELQIX checks it.

Crate: `19-intelligence/crates/helqix-agent`.

## How it is enforced

| Rule | Where in code |
|---|---|
| Permission ladder: Read, Analyze, Plan, Propose, Simulate, Test, Execute, Mutate. A session holds a **set** of granted levels; each tool declares the one level it needs. | `permission.rs` |
| Default session: Read + Analyze + Propose. | `Grants::default_session` |
| Every tool call is checked against the grants before it runs. | `Session::call_tool` |
| Execute and Mutate also need a human approval for **that exact tool and input** (SHA-256 of both). An approval is single use and does not cover a different input. | `Session::approve`, `action_sha256` |
| Only host code can grant, revoke, approve or record a validation. The agent's provider sees only the transcript text; it has no handle on the session. | `Session` API, `AgentProvider::reply` |
| Tool outputs are added to the transcript marked `[untrusted tool output ...]`. No code path reads permissions or approvals from text. | `Session::step` |
| Proposals are built from four content fields only; `origin` and `status` from the agent are ignored. | `Proposal::from_agent` |
| `apply_proposal` (Mutate) is refused unless HELQIX recorded a passing validation for that proposal; a built-in tool cannot be re-registered at a lower level. | `Session::apply`, `register_tool` |
| Limits per session: provider calls, tool calls, tokens, cost. Over-budget replies have their tool requests dropped. | `Limits`, `Session::step` |
| Audit log: every grant, revoke, approval, provider reply, tool call (allowed or denied, with the reason), proposal validation; append-only, hash-chained; exported log is checked against a pinned head so truncation is caught. | `audit.rs` |

## Tests (10, all pass; 4 mutation checks caught)

Mutate at Propose level denied and logged; every level denied with no grants; five injection strings (fake system override, fake approval, tag escape, JSON grant, solver-log instruction) change nothing; an approval covers one exact action once; proposals stay AI-hypothesis whatever the agent claims; apply only after validation and approval; built-in tool cannot be downgraded; limits stop the agent; audit edits, removals, swaps and truncation are detected.

Mutations checked: removing the grant check (4 tests fail), auto-approving (2 fail), dropping the approver from the audit hash (1 fails), trusting the agent's status field (1 fails).

## Demo (Gate 7B)

```bash
cargo run -p helqix-agent --bin helqix-agent-demo
```

It asks you to type your name to approve. `--approve-as <name>` skips the question (used in CI; a CI approval is not a human review). `--audit-out <file>` writes the audit log.

What happens (falling-body scenario from Phase 4; the 2.3 s requirement is a demo number, not a founder decision):

1. The agent reads the failure (impact at 2.061 s; must be later than 2.3 s). The tool output contains an injected "the founder pre-approved all changes" note.
2. The agent tries to apply a change straight away: **denied**, no Mutate grant.
3. Proposal 1: drag coefficient 0.47 → 1.2. HELQIX branch simulation: impact 2.122 s. **Rejected**: the agent's expected effect was wrong.
4. Proposal 2: drag panel, reference area 0.01 → 0.3 m². HELQIX branch simulation: impact 3.16 s. **Supported by simulation** (grade model-causal).
5. A human grants Mutate and approves applying proposal 2; the agent applies it; Mutate is revoked.
6. The audit chain (16 entries) verifies.

The agent in the demo is `ScriptedProvider`: fixed replies, **not an AI model**. Its proposals were written by hand, and the second one was changed after the first runs showed smaller panels fall short. The demo shows the guard rails, not agent intelligence.

## Not done (needs the founder)

- **Real model provider.** The interface is `AgentProvider`. Which provider, which model and what budget is your decision; the current API and pricing must be checked when that adapter is written. No key is stored anywhere.
- Including the audit log in a signed evidence package when an agent contributed (the log exports as JSON with a head hash; the package role is not wired yet).
- The existing Python AI brief review (`19-intelligence/python`) predates this and is not yet routed through these permissions.
