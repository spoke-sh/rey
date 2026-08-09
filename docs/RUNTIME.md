# Runtime Transitions And Reasoning Surfaces

This document defines Rey's formal runtime lifecycle and the bounded input
surface presented to policy. ADRs 0012–0014 fix the architecture and current
contracts. The repository implements the pure state reducer through scheduling,
canonical frontier/progress/selection contracts, and surface
validation/projection. It also implements one bounded source-mining workload
fixture that retrieves through a built-in provider, derives and schedules one
failure row, and projects a reasoning surface. It does not execute the proposed
revision action or run an agent loop.

ADR 0015 places these mechanisms inside a workload test campaign. ADR 0016
implements the first deterministic workload/graph/scenario/qualification
slice and makes the public identity cutover. Runtime state remains v2 because
it had no application/component envelope; the reasoning surface is now v3 and
binds exact workload, graph, scenario-suite, and campaign identities.
ADR 0017 names the orientation capability layer as mining. ADR 0018 implements
the first provider-neutral request/result execution path and its
workload-specific orientation fixture without turning the reducer into a
generic recurring scheduler.

ADR 0022 adds the outer portfolio campaign. The runtime now has a deterministic
portfolio-snapshot to workload-attention derivation and an executable system
workload, but it does not yet feed those rows into recurring generic
scheduling or policy.
ADR 0023 moves product workloads to bounded workspace packages with exact
proposal provenance and frozen scenario admission. The runtime still does not
invoke a coding harness; it deterministically receives and evaluates admitted
packages.

## Nested Campaigns

The phase machine governs one admitted unit of work. It can be used inside two
nested campaigns:

```text
inner  workload graph → scenarios → deltas → graph attention
outer  portfolio snapshot → workload attention → selected workload work
```

The outer loop does not add a second scheduler. It derives typed attention
facts from exact catalog/result/environment/coverage inputs. The existing
scheduler may later select ready rows; orientation then mines only the evidence
needed for those rows. Re-evaluation after an admitted action decides whether
the portfolio delta changed.

## State Dimensions

Rey does not use one status to represent unrelated facts. Runtime evidence
keeps these dimensions separate:

| Dimension | V2 facts |
| --- | --- |
| Rey lifecycle | bootstrapping, ready, scheduling, orienting, awaiting proposal, admitting, executing, observing, evaluating, committing, stopped |
| Provider execution | succeeded, failed, cancelled, timed out, or lost |
| Observation | complete, partial, unavailable, or failed |
| Semantic outcome | unresolved, progressing, unchanged, regressing, converged, or inconclusive |
| Evidence | pending, retained, verified, missing, or stale under a local or Spoke-backed profile |
| Stop reason | converged, budget, cancellation, timeout, evidence, eligibility, capability, inconclusive, or failure |

A successful provider process changes only the provider-execution dimension.
Rey must still observe, calculate transition and residual deltas, evaluate the
next frontier, and commit evidence before it can continue or stop as converged.

## Bootstrap

Bootstrap has no imaginary predecessor. Rey discovers a bounded capability
snapshot, materializes declared initial observations, and commits a baseline.
It then either:

- retains an unresolved frontier and enters `ready`; or
- records an explicit converged, inconclusive, evidence, capability, budget,
  or failure stop and enters `stopped`.

The baseline record is evidence, but it is not described as a transition delta
from an unobserved prior world.

## Recurring Machine

The implemented reducer accepts only these normal phase events:

| Current phase | Event | Next phase | Required guard |
| --- | --- | --- | --- |
| `ready` | `begin_scheduling` | `scheduling` | retained baseline and frontier; new exact transition id |
| `scheduling` | `scheduling_completed` | `orienting` | matching transition and verified decision identity |
| `orienting` | `reasoning_surface_ready` | `awaiting_proposal` | matching transition; surface binds the decision |
| `awaiting_proposal` | `proposal_received` | `admitting` | proposal cites the active surface and transition |
| `admitting` | `proposal_admitted` | `executing` | matching transition and frozen preconditions |
| `executing` | `execution_finished` | `observing` | provider reports a terminal outcome |
| `observing` | `observation_completed` | `evaluating` | complete/partial observations cite frames; unavailable/failed do not fabricate them |
| `evaluating` | `evaluation_completed` | `committing` | typed semantic outcome and compatible next-frontier presence |
| `committing` | `transition_committed` | `ready` or `stopped` | stop semantics agree and continuation evidence is retained or verified |

Four additional events preserve non-happy-path evidence:

- `scheduling_stopped` moves `scheduling` to `committing` with an explicit
  decision, unresolved or inconclusive work, and a non-converged stop reason;
- `orientation_failed` moves `orienting` to `committing` with an inconclusive
  semantic outcome and an explicit pending stop reason;
- `proposal_rejected` moves `admitting` to `committing` with unresolved work
  and the current frontier retained; and
- `cancellation_requested` records cancellation while remaining in
  `executing`; provider termination, observation, and evaluation still follow.

Unavailable or failed post-action observation can evaluate only as
inconclusive. Bootstrap can establish unresolved, converged, or inconclusive
state, but cannot report progressing, unchanged, or regressing without a prior
residual state to compare.

Every event after `begin_scheduling` must cite the active transition id.
Retry or replay cannot substitute a new id midway through the transition. Each
accepted event increments the trace-local sequence and produces a new semantic
runtime-state identity. V2 bounds the total accepted event count and retained
observation, transition-delta, and residual-delta references; those effective
limits participate in state identity. A deserialized state must pass bounds,
phase-shape, canonical-order, schema, and digest verification before use.

Orientation is illegal until `scheduling_completed` records a decision
identity. Scheduling can preserve an unresolved frontier and stop for an
explicit budget, capability, eligibility, cancellation, timeout, evidence, or
failure condition. It cannot declare convergence: convergence must already
have been derived and committed during bootstrap or post-action evaluation.
The pure reducer validates identity shape and phase lineage; a composition
layer must replay-verify the scheduling artifact against its frontier before
submitting the event.

## Commit And Stop Guards

Continuation is legal only when:

- the semantic outcome is nonterminal;
- a next frontier identity exists;
- no stop condition is pending; and
- evidence is retained or verified at the selected profile.

Convergence is legal only when the evaluated semantic outcome is `converged`,
the post-action observation is complete, there is no next frontier, and the
committed stop reason is also `converged`. Inconclusive state must stop.
Missing, stale, or pending transition evidence cannot direct another action.
Convergence itself also requires retained or verified evidence; it cannot be
committed over missing or stale evidence.

The local profile means evidence reached the declared local boundary; it does
not claim process-crash durability, remote durability, or Spoke semantics. The
Spoke-backed profile can be selected only when public Spoke contracts provide
the claimed retention evidence.

## Delta Roles And Progress

A transition can cite two independent delta sets:

- transition deltas compare pre-action to post-action observations and state
  what changed; and
- residual deltas compare declared expected/baseline observations to current
  observations and state what remains.

The runtime state preserves both sets. `rey.frontier-progress.v2` compares
successive compatible frontier states by stable work identity and reports
resolved, introduced, updated, and unchanged facts. The runtime evaluator still
owns its semantic outcome; the generic relation does not implement a scalar
score or guess the direction of updated work. See
[Frontier, Progress, and Scheduling](FRONTIER.md).

Relational, text, and structural mining deltas may all fill transition or
residual roles when their comparison contracts are explicit. A source patch,
syntax change, grouped metric delta, or claim fact is not flattened into a
frame delta merely to enter runtime state; the frontier retains its typed
artifact identity and role.

## Reasoning Surface Envelope

`rey.reasoning-surface.v3` is the content-identified policy input constructed
from scheduled work in a committed frontier. It contains:

```text
schema · surface_id
workload · graph · scenario suite · campaign · space
trace · committed_transition · active_transition · scheduling_decision
frontier_frame · capability_snapshot · projection_contract
effective_limits · retrieval_iterations · completeness
projected_rows · evidence_references · admissible_actions · omissions
```

Workload, graph, scenario-suite, space, projection, provider, and action references bind
stable ids, revisions, and semantic contract digests. Trace, transition,
frontier, capability, delta, evidence-content, and surface identities are
semantic digests. Evidence also retains its provider contract, provider-owned
source id and immutable revision, media type, and logical byte length.

The canonical `rey.reasoning-surface-rows` version `3` relation is:

```text
frontier_row_id
entity_kind
entity_id
transition_delta_ids
residual_delta_ids
claim_ids
evidence_ids
admissible_action_ids
```

The row columns remain the same as earlier versions; v2 added scheduling
decision lineage and v3 replaces the legacy application/component scope with
exact workload/graph/scenario-suite/campaign identities in the envelope and
Arrow metadata. Array fields use canonical compact JSON strings. The semantic document keeps
the arrays typed. A later Arrow list/struct representation requires a schema
revision and parity evidence.

Rows are sorted by unique `frontier_row_id`. Every nested reference array is
sorted and unique. A row is delta-directed or claim-directed, and all evidence
and action references resolve against the exact surface envelope. Source bytes
and native artifacts remain outside the DataFrame and addressable through
their evidence references.

Those evidence references may cite mining-result manifests and native,
relational, tree, graph, metric, delta, or visualization artifacts. The surface
binds exact source, operation, provider, completeness, derivation, omission,
and effective-limit lineage. It does not copy an ambient repository or promote
a visualization to authoritative evidence.

The pre-alpha cutover has no compatibility alias or decoder for the superseded
application/component envelope.

## Placement In A Workload Campaign

One workload test pass freezes an exact graph revision, executes selected
scenarios, and compares `EXPECTED` to `OBSERVED`. Failing deltas and unresolved
claim facts derive the next workload frontier. The scheduling and orientation
phases then select bounded unresolved work and construct the reasoning surface
by mining exact relevant relational and source evidence. A policy may then
propose another immutable graph revision.

Graph edges determine node dependency order inside one execution. The frontier
scheduler determines which unresolved scenario evidence receives the next unit
of attention between graph revisions. These are separate mechanisms; the
runtime reducer does not become a generic graph-task scheduler.

During orientation, exact immutable retrieval and pure projection may build
the surface directly within its existing retrieval-iteration and evidence-byte
bounds. A mutable read or external mining tool invocation is a probe and must
cross the full proposal/admission/execution/observation boundary. Plan 0006
proves the local source distinction with a declared bounded read-only graph
operation; external search or parser processes still require the full probe
boundary.

A graph proposal is an untrusted policy proposal and must pass graph,
operation, capability, effect, precondition, and limit validation. Only fresh
scenario results produced by deterministic evaluation can qualify it. See
[Workloads, Compute Graphs, and Scenarios](WORKLOADS.md).

## Bounds And Completeness

V3 enforces nonzero maxima for:

- projected rows;
- total transition and residual delta references;
- evidence references;
- admissible action references;
- omissions;
- total selected evidence bytes;
- semantic string bytes; and
- retrieval iterations.

The surface records the effective limits and actual retrieval-iteration count,
and all affect surface identity. Count and byte arithmetic fails closed on
overflow.

A complete surface has no omissions. Partial or truncated surfaces identify
omission kind, optional affected subject, omitted count, and reason. Initial
omission classes cover row, delta, evidence, action, and byte limits plus
provider unavailability, unsupported retrieval, and retrieval failure.

An unavailable or failed orientation does not emit a misleading empty surface.
It follows the runtime's `orientation_failed` path. Reaching a bound is not
readiness, progress, or convergence.

## Ownership And Non-Goals

Rey owns the delta/frontier rationale, projection contract, bounds, identity,
and trace edges. The selected provider owns retrieval, query, source revision,
and durability semantics. A reasoning surface does not grant authority to an
action it cites.

The implemented crates deliberately contain no:

- generic workload-specific frontier derivation or dependency invalidation
  beyond the source-search fixture and portfolio-attention derivation;
- coding-harness proposal request/response campaign beyond the bounded
  workspace package receiving contract;
- recurring, fair, parallel, or multi-user scheduling;
- external tool, query, parser, index, or Spoke retrieval implementation;
- general visualization specification beyond the source workload's terminal
  table/patch projection;
- policy request transport or proposal parser;
- action admission or execution;
- domain-specific interpretation of updated work or scalar progress score; or
- trace persistence service.

Those are later end-to-end slices. They must use these contracts rather than
introducing a second state machine or provider-specific context envelope.

## Required Contract Fixtures

The current contract fixtures cover:

- scheduling required before orientation and explicit scheduling stops;
- a complete transition returning to a committed frontier;
- convergence only after observation, evaluation, and commit;
- process success unable to skip semantic phases;
- bootstrap unable to fabricate progress without a predecessor residual;
- cancellation unable to skip terminal provider observation;
- unavailable observation unable to claim progress or convergence;
- missing evidence unable to continue;
- missing convergence evidence, malformed identities, mismatched transition
  identity, and tampered state rejection;
- runtime-state JSON round-trip with stable replay identity;
- canonical reasoning-surface ordering and identity;
- reasoning-surface JSON round-trip with stable identity;
- DataFrame and Arrow metadata round-trip;
- row and evidence-byte bounds;
- unresolved evidence/action citations;
- completeness/omission mismatch;
- rows with neither delta nor claim direction; and
- tampered surface identity.

Frontier fixtures additionally cover canonical work identity, bounds,
readiness/blockers, completeness-derived convergence, directional progress,
incompatible inputs, deterministic priority/cost selection, stale scheduling
preconditions, deferral, replay, Arrow metadata, and tampering.

Future runtime slices still need timeout, execution-budget, retry,
action-precondition staleness, partial-failure, replay, and provider-specific
retention fixtures around real effects.
