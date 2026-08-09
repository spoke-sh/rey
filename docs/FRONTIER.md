# Frontier, Progress, And Scheduling

This document defines the first executable frontier, progress, and scheduling
contracts. ADR 0014 fixed their first schemas; ADR 0016 made a pre-alpha
identity cutover to frontier/progress/scheduling v2 and reasoning-surface v3.
The implementation is a deterministic library slice; it
does not derive workload-specific work, retrieve evidence, select an action,
execute an effect, or run a recurring loop.

ADR 0015 places this contract inside workload test campaigns. The implemented
v2 envelope now binds workload, graph, scenario suite, and campaign identities
directly. The first CLI slice executes scenarios but does not yet derive a
frontier from their deltas.

ADR 0017 makes mining the evidence-acquisition layer after selection. The
frontier may be directed by relational, text, structural, or claim evidence;
the generic scheduler remains ignorant of domain-specific mining semantics.

## Frontier Envelope

`rey.frontier.v2` is a bounded, content-identified relation derived from exact
deltas and claims. Its inputs bind:

```text
workload · graph · scenario suite · campaign · space · trace
committed record · capability snapshot
frontier derivation contract · prioritization-input contract
```

The committed record may identify a bootstrap baseline or a retained
transition. The derivation contract owns the meaning of work membership. The
prioritization contract owns the declared priority and estimated-cost inputs;
the generic scheduler does not infer domain importance or cost.

Each frontier row contains:

```text
work_id · row_id · entity_kind · entity_id
transition_delta_ids · residual_delta_ids · claim_ids
dependent_lens_ids · admissible_action_ids
readiness · blockers · priority · estimated_cost_units
```

`work_id` is the stable key used to align the same logical work across frontier
revisions. `row_id` is a derived semantic digest of that work's current
contents. Changing readiness, blockers, citations, ranking inputs, or entity
identity changes `row_id` without changing `work_id`.

Rows are sorted by unique `work_id`; all reference lists and blockers are
sorted and unique. Every row cites at least one transition delta, residual
delta, or claim. A delta cannot hold both transition and residual roles within
one row. Estimated cost is a nonzero abstract unit, not elapsed time or money.

Readiness is one of:

- `ready` — no blockers and eligible for generic scheduling;
- `blocked` — one or more explicit dependency, capability, evidence, budget,
  unsupported, incomplete, or other blockers; or
- `inconclusive` — one or more explicit blockers prevent an eligibility
  decision.

The row, delta-reference, claim-reference, lens-reference, action-reference,
blocker, and semantic-string-byte limits are nonzero, effective inputs to
frontier identity. Construction fails closed when any limit is exceeded.

## Coverage And Convergence

Frontier coverage independently records whether applicable deltas and claims
were completely evaluated and whether required claims are satisfied, violated,
or unknown. The frontier assessment is derived rather than accepted from a
caller:

| Rows | Coverage | Assessment |
| --- | --- | --- |
| one or more | any valid coverage | `open` |
| empty | complete deltas and claims; required claims satisfied | `converged` |
| empty | incomplete or unknown | `inconclusive` |

An empty frontier cannot represent a known violated required claim. Missing,
truncated, or unknown inputs therefore never become convergence. A scheduler
does not reassess this result.

The canonical `rey.frontier-rows` version `2` Polars/Arrow relation uses
`work_id` as its unique key. Array and blocker columns use canonical compact
JSON strings in this first projection; the semantic document retains their
typed representation. Arrow metadata preserves the exact frontier and input
identities, assessment, coverage, and row count.

## Directional Progress

`rey.frontier-progress.v2` compares compatible source and target frontiers in
that direction under an exact comparator contract. Workload, scenario suite,
campaign, space, trace, derivation, and prioritization contracts must agree.
Source and target graph identities may differ because progress often compares
candidate revisions. Capability snapshot and committed-record revisions may
also differ because those are ordinary transition inputs.

Rows align by stable `work_id`:

- source only is `resolved`;
- target only is `introduced`;
- both with different `row_id` values is `updated`; and
- both with the same `row_id` increments the unchanged count.

The change relation preserves source and target row identities. This first
two-frontier comparator does not call target-only work `reopened`; proving a
reopen requires bounded prior history. It also does not guess whether an
updated row is improvement or regression. Its assessment is:

- `progressing` for resolved-only change;
- `regressing` for introduced-only change;
- `mixed` for updated work or simultaneous resolution and introduction;
- `unchanged` when row identities agree;
- `converged` when the target frontier is conclusively converged; or
- `inconclusive` when either frontier has incomplete coverage or an
  inconclusive assessment.

These facts are navigation metadata. They do not replace authoritative deltas,
proof status, coverage, confidence, or the runtime evaluator's semantic
outcome, and there is no generic scalar progress score.

The canonical `rey.frontier-progress-changes` version `2` relation is keyed by
`work_id` and retains the change kind plus nullable source and target row ids.
Source-row, target-row, change, and string-byte bounds fail closed. Full replay
verification recomputes the relation from both cited frontiers.

## Deterministic Scheduling

The v2 scheduler selects ready work units from one verified open frontier. It
does not select an admissible action. Before selection it checks exact expected
committed-record, frontier, and capability-snapshot identities. Any mismatch is
a stale-precondition error and produces no decision.

The scheduler contract identity and effective limits participate in
`rey.scheduling-decision.v2`. The fixed selection order is:

1. priority descending;
2. estimated cost ascending; and
3. stable `work_id` ascending.

Selection scans that order, admits no more than `max_work_units`, and never
exceeds `max_total_cost_units`. A candidate that does not fit the remaining
cost budget is skipped so a later cheaper unit may fit. Frontier size must not
exceed `max_rows_considered`; the scheduler rejects rather than silently
truncates. The decision records readiness counts, selected cost, deferred ready
rows, and cost-skipped rows.

Outcomes are `selected`, `no_ready_work`, `budget_exhausted`,
`frontier_converged`, or `frontier_inconclusive`. Only `selected` can enter
runtime orientation. A converged or inconclusive decision is useful as a
contract check, but a correctly committed runtime should have stopped before a
converged frontier reaches `ready`.

The first scheduler intentionally claims no fairness, starvation prevention,
parallel allocation, dependency invalidation, deadline optimization, learned
ranking, or multi-tenant behavior. Those require explicit later contracts.

The canonical `rey.scheduled-work` version `2` relation preserves selection
rank, work and row identities, priority, and cost. A decision can verify its
own shape and semantic digest; replay verification against the frontier proves
the actual selection.

## Runtime Placement

`rey.runtime-state.v2` inserts scheduling before orientation:

```text
ready
  -> begin_scheduling
scheduling
  -> scheduling_completed(decision)
orienting
  -> reasoning_surface_ready
...
```

Orientation is illegal until the active transition records a scheduling
decision identity. A nonselected scheduling result can enter `committing`
through `scheduling_stopped` with explicit unresolved or inconclusive semantics
and a non-converged stop reason. The ordinary commit guards still require the
stop reason and retained evidence state to agree.

`rey.reasoning-surface.v3` binds the scheduling decision identity and the same
workload, graph, scenario-suite, and campaign scope, so the projected rows
retain the cited deterministic selection lineage. The pure runtime and policy
reducers intentionally keep those identities opaque. A future composition
layer must replay-verify the
decision against its frontier and check projected-row membership before it
records `scheduling_completed` or `reasoning_surface_ready`.

Runtime state, scheduling decision, and reasoning surface are separate
content-identified artifacts; none is a persistence service.

## Workload Frontier Mapping

For a workload test campaign, frontier rows are derived from failing scenario
output deltas, unresolved scenario claims, missing evidence, and the declared
dependencies of the exact graph revision. A stable work identity names the
logical mismatch or claim; its row identity changes when the graph, observed
output, readiness, blockers, citations, or priority inputs change.

The cited evidence may originate from relational frames, source text, syntax
trees, semantic graphs, metrics, or their directed deltas. A derivation
contract maps those artifact-specific facts to stable work identities. The
frontier does not flatten native artifacts, infer parser meaning, or promote a
visualization to authoritative work membership.

The scheduler selects bounded unresolved graph-revision work. It does not
schedule graph nodes: typed graph edges establish node dependency order within
one scenario execution. The selected rows direct evidence retrieval and a
possible agent-, rule-, or human-proposed graph revision. Deterministic
scenario evaluation, not the proposal policy or generic scheduler, decides
whether the resulting graph qualifies.

## Deferred Behavior

Later slices still own workload-specific frontier derivation, dependency
invalidation, provider retrieval, orientation readiness strategy, policy
proposal parsing, action admission, execution, retry, transition persistence,
activation, and recurring scheduling. They must consume these contracts rather
than introducing a provider-specific queue or second lifecycle.

Plan 0006 adds the first provider-neutral mining request/result and a
delta-directed reasoning-surface fixture. It does not change the scheduling
order or make the frontier a query, parsing, indexing, or visualization engine.
