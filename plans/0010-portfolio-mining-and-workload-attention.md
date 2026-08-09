# Plan 0010: Portfolio Mining And Workload Attention

- Status: Active
- Decision: [ADR 0022](../docs/decisions/0022-portfolio-mining-and-workload-attention.md)

## Outcome

Make ongoing mining operational at the workload-portfolio level. Rey should
derive a bounded evidence-linked attention relation from exact catalog,
qualification, environment, dependency, capability, and coverage inputs; show
that relation through the workload CLI; and use it to direct later workload
creation and refinement without putting domain policy into the scheduler.

The concrete anchor is the system workload `rey.portfolio.attention`:

```text
rey workloads test rey.portfolio.attention -vv
rey workloads run rey.portfolio.attention
rey workloads list
rey workloads status rey.portfolio.attention
```

## Completion Checklist

- [x] Accept ADR 0022 and define the nested workload/portfolio mining loops.
- [x] Implement bounded `rey.portfolio-snapshot.v1` and canonical Polars-backed
  `rey.workload-attention.v1` contracts with replay verification.
- [x] Derive distinct `REFINE`, `RETEST`, `CREATE`, `BLOCK`, and
  `POLICY_EXCLUDED` rows while preserving readiness, evidence, dependencies,
  priority, cost, coverage, and typed empty results.
- [x] Add the scenario-qualified `rey.portfolio.attention` graph and reviewed
  fixtures for failing, stale/changed/untested, unowned, unavailable,
  inconclusive, excluded, and clean portfolio states.
- [x] Expose current attention and mapped-surface coverage through
  `workloads list`, typed evidence through `test -v/-vv`, retained-input
  evaluation through `run`, and the current frontier through `status`.
- [x] Keep `list` and `status` read-only by consuming only retained workload
  state and retained environment HEAD/admission-index snapshots.
- [x] Prove stdout, stderr, structured output, semantic exits, deterministic
  replay, qualification-gated run, policy exclusion, and unowned admitted
  environment surfaces through CLI tests.
- [ ] Define workload-owned surface declarations and resolve declared owners
  against exact workload and environment revisions.
- [ ] Derive changed-dependency and missing-capability inputs from retained
  environment/Git deltas rather than fixture or empty live fields.
- [ ] Project admitted ready attention rows into the generic frontier and one
  bounded reasoning surface without copying the whole portfolio.
- [ ] Admit an agent/rule/human proposal for one `CREATE` or `REFINE` row,
  materialize a candidate workload/graph revision, test it, and re-mine the
  portfolio to measure whether the row resolved.
- [ ] Run full workspace tests, Clippy, build, Nix checks, link review, and
  repository-truth audit before closing the plan.

## Current Proof

Implemented tests exercise the typed derivation and the four-command CLI
surface. In particular, an admitted `rey.env-map` input file with no declared
owner appears as a ready `CREATE` row in `workloads list` and in a qualified
portfolio run. The portfolio workload's six required scenarios retain the
authoritative relation alongside their exact expected-to-observed text delta.

Captured on 2026-08-08:

```text
just check
# rustfmt, Clippy -D warnings, git diff check, and Nix flake evaluation passed
just test
# 128/128 tests passed; all workspace doc tests passed
just build
# workspace build passed
```

A human CLI walkthrough used isolated workload state against this workspace.
`list` rendered attention and coverage plus the canonical frontier; `test -vv`
rendered all six scenario classes and exact relation/source/derivation
bindings; `run` passed with retained inputs and emitted three current rows; and
`status` reopened qualification, retained evidence, and the portfolio frontier.
The integration fixture additionally commits one mapped input and proves its
unowned `CREATE` row through list and run.

## Next Concrete Anchor

Add a bounded workload surface-ownership declaration and resolve it against
the retained environment mapping. The first end-to-end change should turn one
currently unowned mapped source from `CREATE` into owned coverage, then use a
changed retained source revision to produce `RETEST` for that exact owner.
That gives dependency invalidation a real evidence path before attention rows
are handed to generic scheduling or agent policy.

## Deferred

Recurring daemon scheduling, learned ranking, automatic retirement, external
workload manifest syntax beyond the ownership slice, arbitrary tool execution,
parser/index breadth, and Spoke durability are not prerequisites for this
plan's next anchor.
