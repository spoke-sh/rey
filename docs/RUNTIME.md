# Runtime Transitions And Reasoning Surfaces

This document defines Rey's formal runtime lifecycle and the bounded input
surface presented to policy. ADRs 0012 and 0013 fix the architecture and v1
contracts. The repository implements the pure state reducer and surface
validation/projection, but it does not yet implement generic frontier
scheduling, provider retrieval, action execution, or an agent loop.

## State Dimensions

Rey does not use one status to represent unrelated facts. Runtime evidence
keeps these dimensions separate:

| Dimension | V1 facts |
| --- | --- |
| Rey lifecycle | bootstrapping, ready, orienting, awaiting proposal, admitting, executing, observing, evaluating, committing, stopped |
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
| `ready` | `begin_orientation` | `orienting` | retained baseline and frontier; new exact transition id |
| `orienting` | `reasoning_surface_ready` | `awaiting_proposal` | matching transition id and verified surface identity |
| `awaiting_proposal` | `proposal_received` | `admitting` | proposal cites the active surface and transition |
| `admitting` | `proposal_admitted` | `executing` | matching transition and frozen preconditions |
| `executing` | `execution_finished` | `observing` | provider reports a terminal outcome |
| `observing` | `observation_completed` | `evaluating` | complete/partial observations cite frames; unavailable/failed do not fabricate them |
| `evaluating` | `evaluation_completed` | `committing` | typed semantic outcome and compatible next-frontier presence |
| `committing` | `transition_committed` | `ready` or `stopped` | stop semantics agree and continuation evidence is retained or verified |

Three additional events preserve non-happy-path evidence:

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

Every event after `begin_orientation` must cite the active transition id.
Retry or replay cannot substitute a new id midway through the transition. Each
accepted event increments the trace-local sequence and produces a new semantic
runtime-state identity. V1 bounds the total accepted event count and retained
observation, transition-delta, and residual-delta references; those effective
limits participate in state identity. A deserialized state must pass bounds,
phase-shape, canonical-order, schema, and digest verification before use.

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

The runtime state preserves both sets. A later progress relation will compare
successive residual/frontier states. V1 records the semantic assessment but
does not implement a generic progress comparator or scalar score.

## Reasoning Surface Envelope

`rey.reasoning-surface.v1` is the content-identified policy input constructed
from a committed frontier. It contains:

```text
schema · surface_id
application · component · space
trace · committed_transition · active_transition
frontier_frame · capability_snapshot · projection_contract
effective_limits · retrieval_iterations · completeness
projected_rows · evidence_references · admissible_actions · omissions
```

Application, component, space, projection, provider, and action references bind
stable ids, revisions, and semantic contract digests. Trace, transition,
frontier, capability, delta, evidence-content, and surface identities are
semantic digests. Evidence also retains its provider contract, provider-owned
source id and immutable revision, media type, and logical byte length.

The canonical `rey.reasoning-surface-rows` version `1` relation is:

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

The array fields use canonical compact JSON strings in this first Polars/Arrow
schema, matching the capability-frame precedent. The semantic document keeps
the arrays typed. A later Arrow list/struct representation requires a schema
revision and parity evidence.

Rows are sorted by unique `frontier_row_id`. Every nested reference array is
sorted and unique. A row is delta-directed or claim-directed, and all evidence
and action references resolve against the exact surface envelope. Source bytes
and native artifacts remain outside the DataFrame and addressable through
their evidence references.

## Bounds And Completeness

V1 enforces nonzero maxima for:

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

- frontier derivation, invalidation, ranking, or scheduling;
- provider read, query, or retrieval implementation;
- policy request transport or proposal parser;
- action admission or execution;
- generic frame comparator or progress calculation; or
- trace persistence service.

Those are later end-to-end slices. They must use these contracts rather than
introducing a second state machine or provider-specific context envelope.

## Required Contract Fixtures

The v1 contract fixtures cover:

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

Future runtime slices still need timeout, budget, retry, stale-input,
partial-failure, replay, and provider-specific retention fixtures around real
effects.
