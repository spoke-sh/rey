# ADR 0034: Agent Runtime Inventory And Derived Task Plane

- Status: Accepted
- Date: 2026-08-10
- Extends: [ADR 0032](0032-seed-discovery-survey-and-live-communications.md)
- Supersedes: the `/agents` registry projection in [ADR 0030](0030-operator-cadence-agents-and-explorer-coordinates.md)
- Superseded: the `/agents` runtime-inventory presentation is replaced by [ADR
  0035](0035-agent-recommendations-and-observed-work.md); runtime discovery and
  task/operation concepts remain current.

## Context

The first `/agents` route treated exact workload generator provenance as an
agent registry. That evidence remains necessary on a workload package, but it
answers who or what proposed an artifact at a particular revision. It does not
answer which collaboration runtimes Rey can currently find, which one may be
assigned, or what bounded work needs attention now.

The collaboration language also needs an organizing unit above individual
artifacts without creating another retained collection that grows beside
workloads, attention, evidence, and cadence. A journey is useful as a human
projection over workflow progress, but retaining journey objects would
duplicate state already carried by exact operations and deltas.

## Decision

Rey distinguishes four concepts:

- an **agent runtime** is a desired application that might participate in
  collaboration;
- an **operation** is one named bounded action within a known workflow;
- a **task** is the current coordination envelope for an intent, operation,
  bounded artifact references, desired delta, readiness, and eventual agent
  assignment; and
- a **journey** is a derived human projection over operation state, never a
  second authoritative or durable object.

Process-owned discovery now declares bounded identity adapters for `agy`,
`claude`, `codex`, `copilot`, `droid`, and `opencode` alongside the existing
`git` and `rg` adapters. Rey resolves each exact executable name through the
already bounded `PATH` search. Agent-runtime adapters record exact executable
presence without starting the agent CLI; `git` and `rg` retain their declared
non-interactive identity probes. The result records desired, found, not-found, and error evidence in the same
environment application inventory and search record as every other known
application.

Finding an agent runtime proves only availability and identity-probe evidence.
It grants no task assignment, process execution, write authority, session,
message transport, or Explorer location. Those require their own admitted
contracts and locator evidence.

`/agents` consumes both the current environment status and workload portfolio.
Its task plane is derived from current non-excluded attention plus workload
creation requests. A draft and its matching attention row collapse into one
task instead of appearing twice. No independent task store or API is added.
The page shows two workflow grammars as organizing projections:

```text
CONTEXT   DISCOVER → REASON → SURVEY → PROCESS
WORKLOAD  ORIENT → AUTHOR → TEST → REFINE → RUN
```

Each current task enters one bounded operation. The initial action mapping is
`create → AUTHOR`, `refine → REFINE`, `retest → TEST`, and `block → RESOLVE`.
The current UI leaves every agent assignment explicit as `UNASSIGNED`; it does
not infer assignment from generator provenance or executable presence.

Workload generation tuples remain exact provenance and continue to support the
existing v1 Explorer agent coordinates until a surveyed runtime locator and
task-assignment contract supersede that coordinate source. `/agents` does not
present those tuples as the available runtime inventory.

## Consequences

- Humans can see major collaboration options whether found or missing without
  inspecting workload packages or source code.
- Current work is organized by task and operation while workload, attention,
  evidence, and environment documents remain authoritative.
- Workflow journeys can improve as projections without creating a parallel
  lifecycle or retention problem.
- Discovery cannot silently become execution authority, and a past generator
  cannot be mistaken for a currently available or assigned agent.
- Exact agent assignment, invocation, task persistence, and surveyed Explorer
  locations remain enabling work.
