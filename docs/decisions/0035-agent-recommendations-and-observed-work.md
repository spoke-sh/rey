# ADR 0035: Agent Recommendations And Observed Work

- Status: Accepted
- Date: 2026-08-10
- Extends: [ADR 0034](0034-agent-runtime-inventory-and-derived-task-plane.md)
- Supersedes: the `/agents` runtime-inventory presentation in ADR 0034

## Context

The first task-oriented `/agents` revision put the process-owned agent-runtime
inventory beside current tasks. That inventory is useful and remains required,
but desired/found/not-found executable evidence is an environment dimension.
Repeating it on `/agents` forces the human operator down to mechanism when the
route should answer higher-order collaboration questions:

1. What work does Rey recommend an agent perform next, and why?
2. What work has actually been observed, with what result and artifacts?

The implementation must not manufacture agent intelligence, assignment, or
live activity. Current Rey has typed portfolio attention, creation requests,
qualification and run summaries, and retained mining/test identifiers. It does
not have an admitted agent session, process telemetry, or task execution log.

## Decision

Agent runtime discovery remains process-owned environment evidence and is
shown on `/environment`. `/agents` no longer loads or renders environment
status.

`/agents` is a higher-order collaboration-intelligence projection with two
dense tables:

- **System recommendations** rank current non-excluded attention and creation
  requests by readiness, priority, estimated cost, and stable identity. A
  matching creation request and attention row collapse into one recommendation.
  Each row shows the recommended operation, capability profile, exact reason,
  source evidence class, evidence/dependency counts, bounds, readiness, and
  subject location.
- **Work ledger** summarizes the current bounded portfolio. Each row shows the
  last operation Rey can actually support from retained evidence—request,
  admission, test, or run—plus result, scenario progress, mined artifact/delta/
  surface counts, current journey projection, attention count, revision, and
  evidence identity.

Recommendations are deterministic projections of authoritative typed inputs,
not LLM opinions. The initial mapping remains narrow:

```text
create  → AUTHOR  / CODING HARNESS
refine  → REFINE  / CODING HARNESS
retest  → TEST    / QUALIFICATION RUNNER
block   → RESOLVE / SURVEY OR OPERATOR
```

A recommendation is not scheduling, assignment, invocation, or proof of
progress. The work ledger is retained-result insight, not live agent telemetry.
The task concept from ADR 0034 remains the bounded coordination envelope that
future assignment can admit, while journeys remain derived projections rather
than stored objects.

## Consequences

- `/environment` is the single human surface for discovered agent applications.
- `/agents` operates at the recommendation and work-insight level instead of
  duplicating low-level capability inventory.
- Humans can inspect why a recommendation exists and what evidence bounds it
  without dropping to the CLI immediately.
- The UI says when agent identity and live work attribution are unavailable
  instead of inferring them from workload generator provenance.
- The next contract is still recommendation-to-task assignment: bind one ready
  recommendation to one admitted runtime, exact locator, and returned delta.
