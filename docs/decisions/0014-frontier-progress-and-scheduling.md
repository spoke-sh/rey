# ADR 0014: Frontier, Progress, And Deterministic Scheduling

- Status: Accepted; public identity cutover required by ADR 0015
- Date: 2026-08-07
- Followed by: [ADR 0015](0015-workload-centered-product.md)

## Context

ADRs 0012 and 0013 established a delta-directed lifecycle, a pure runtime
reducer, and a bounded reasoning surface, but deliberately stopped before
frontier and scheduling behavior. The runtime therefore moved directly from a
committed frontier to orientation, without an artifact stating which work had
been selected, under which limits, or against which capability revision.

Scheduling before canonical frontier identity would let the first policy or
provider adapter invent work keys, convergence, ranking meaning, stale-input
behavior, and progress scoring. Those are deterministic runtime contracts and
must remain usable without an LLM or external service.

## Decision

Rey adds `rey-frontier`, a deterministic contract crate using only the existing
identity, Serde, Polars, Arrow, and error dependencies. It owns:

- `rey.frontier.v1` and the `rey.frontier-rows` version `1` relation;
- `rey.frontier-progress.v1` and the
  `rey.frontier-progress-changes` version `1` relation; and
- `rey.scheduling-decision.v1` and the `rey.scheduled-work` version `1`
  relation.

### Frontier

A frontier binds exact application, component, space, trace, committed-record,
capability-snapshot, derivation-contract, prioritization-contract, coverage,
and effective-limit inputs. A row has a stable `work_id` and a derived
content-sensitive `row_id`; typed transition/residual delta citations; claim,
lens, and action citations; explicit readiness and blockers; priority; and a
nonzero estimated cost.

The relation is bounded and canonical. A row is delta- or claim-directed, and
one delta cannot silently serve both transition and residual roles. `ready`
rows have no blockers; `blocked` and `inconclusive` rows have explicit blockers.

Frontier assessment is derived. Any nonempty valid frontier is `open`. An empty
frontier is `converged` only when delta and claim coverage are complete and all
required claims are satisfied. Otherwise it is `inconclusive`. A known claim
violation without corresponding work is invalid.

### Progress

Progress compares source to target frontiers under an exact comparator contract
and stable `work_id` alignment. It reports resolved, introduced, updated, and
unchanged counts with source/target row identities. The first comparator
classifies resolved-only change as progressing, introduced-only change as
regressing, updates or opposing changes as mixed, equal rows as unchanged,
conclusive empty targets as converged, and incomplete comparisons as
inconclusive.

No scalar score is authoritative. An updated row has changed semantics but no
generic direction. Reopened work requires prior history and is deferred rather
than guessed from two frontiers.

### Scheduling

The v1 scheduler selects work units, not actions. It rejects stale expected
committed-record, frontier, or capability-snapshot identities before producing
a decision. Frontier size is rejected when it exceeds the scheduler's
consideration bound.

Ready rows use a fixed deterministic order: priority descending, estimated
cost ascending, then stable `work_id` ascending. Greedy selection observes
maximum selected-work and total-cost bounds, records skipped and deferred work,
and produces an explicit selected, no-ready-work, budget-exhausted,
frontier-converged, or frontier-inconclusive outcome. The scheduler makes no
fairness or starvation guarantee.

### Runtime And Surface Cutover

Because Rey is pre-alpha, this decision makes a hard schema cutover:

- `rey.runtime-state.v2` inserts `scheduling` between `ready` and `orienting`;
  `begin_scheduling` fixes the transition identity,
  `scheduling_completed` records the decision before orientation, and
  `scheduling_stopped` enters commit with explicit unresolved or inconclusive
  state;
- `rey.reasoning-surface.v2` and the row projection version `2` bind the exact
  scheduling decision identity.

ADR 0013 remains the rationale for the reducer and surface design, but its v1
runtime-state and reasoning-surface schema selections are superseded by this
decision. Scheduling cannot establish convergence from a `ready` state;
convergence belongs to bootstrap or post-observation evaluation and commit.

### Boundary

This decision does not implement application-specific frontier derivation,
dependency invalidation, evidence retrieval, action choice, policy transport,
admission, execution, persistence, recurring loops, parallel allocation, or
multi-user scheduling.

## Consequences

- Every orientation has a content-identified scheduling predecessor.
- Capability or committed-input drift prevents selection rather than creating
  a stale reasoning surface.
- Empty, blocked, inconclusive, and cost-exhausted work remain distinct from
  convergence.
- Progress can direct later work without becoming proof status or a universal
  reward score.
- Provider and policy adapters can consume one bounded selection contract
  without owning Rey's lifecycle or frontier semantics.
