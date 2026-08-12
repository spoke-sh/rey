# Plan 0010: Portfolio Mining And Workload Attention

- Status: Active
- Decisions: [ADR 0022](../docs/decisions/0022-portfolio-mining-and-workload-attention.md), [ADR 0023](../docs/decisions/0023-workspace-workload-packages.md), [ADR 0024](../docs/decisions/0024-workload-creation-requests.md), [ADR 0049](../docs/decisions/0049-workload-admission-history.md)

## Outcome

Make ongoing mining operational at the workload-portfolio level. Rey should
derive a bounded evidence-linked attention relation from exact catalog,
qualification, environment, dependency, capability, and coverage inputs; show
that relation through the workload CLI; and use it to direct later workload
creation and refinement without putting domain policy into the scheduler.

Portfolio attention remains available as an explicit system/conformance
workload, while workspace packages are now the default product catalog:

```text
rey workloads create <workload-id> --intent <bounded-objective>
rey workloads status
rey workloads add
rey workloads test --staged -vv
rey workloads list
rey workloads --catalog conformance test rey.portfolio.attention -vv
rey workloads --catalog conformance run rey.portfolio.attention
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
- [x] Hard-cut the default product catalog from compiled fixtures to bounded
  `rey.workload-package.v1` workspace packages; keep system/fixture workloads
  behind explicit `--catalog conformance`.
- [x] Bind coding-harness producer/revision/inputs, generated graph and suite
  roles, frozen scenario oracle, package source digest, and source path into
  the WORKING proposal; bind admission separately through exact workload
  INDEX qualification and human commit.
- [x] Prove one harness-generated package through WORKING, INDEX,
  qualification, HEAD, and run, including requalification after a
  provenance-only restage.
- [x] Add `workloads create` as a bounded content-addressed request for an
  external coding harness; expose request-only packages as visible drafts and
  reject their test/run admission without fabricating graphs or scenarios.
- [ ] Define workload-owned surface declarations and resolve declared owners
  against exact workload and environment revisions.
- [ ] Derive changed-dependency and missing-capability inputs from retained
  environment/Git deltas rather than fixture or empty live fields.
- [ ] Project admitted ready attention rows into the generic frontier and one
  bounded reasoning surface without copying the whole portfolio.
- [ ] Bind one `CREATE` or `REFINE` attention row and its bounded reasoning
  surface into a creation request, let a coding harness materialize the package
  response, test its frozen suite, and re-mine whether the row resolved.
- [ ] Run full workspace tests, Clippy, build, Nix checks, link review, and
  repository-truth audit before closing the plan.

## Current Proof

Implemented tests exercise the typed derivation and workload CLI surface. In
particular, an admitted `rey.env-map` input file with no declared
owner appears as a ready `CREATE` row in `workloads list` and in a qualified
portfolio run. The portfolio workload's six required scenarios retain the
authoritative relation alongside their exact expected-to-observed text delta.

Captured on 2026-08-09 after the workspace-package cutover:

```text
just check
# rustfmt, Clippy -D warnings, git diff check, and Nix flake evaluation passed
just test
# 133/133 tests passed; all workspace doc tests passed
just build
# workspace build passed
```

A human CLI walkthrough used isolated workload state against this workspace.
The current admission proof leaves product packages outside HEAD until `add`,
exact `test --staged`, and human commit. After approval, `list` exposes the
coding-harness producer, exact package revision, and frozen oracle and `run`
executes that same graph. Explicit
`--catalog conformance list` separately rendered the four compiled system and
fixture workloads with unmistakable origin labels. The integration test also
changes only harness provenance and proves that retained qualification becomes
stale.

The creation-request slice additionally proves create-new/no-overwrite
behavior, a request-only catalog entry, high-fidelity `create`, `list`, and
`status` output, structured request/result schemas, draft test/run rejection,
and immutable compiled conformance catalogs. No fake graph, scenario, or oracle
is emitted.

## Next Concrete Anchor

Complete the response half of the bounded coding-harness handshake without
adding a new peer resource hierarchy. `workloads create` now owns the exact
request and visible `AWAITING CODING HARNESS` draft. Next, bind one selected
attention row, reasoning surface, current package/graph/suite identities,
failing deltas, permitted operations, and limits into that request. A harness
response must materialize an immutable WORKING package revision; Rey then
validates it, stages exact bytes, runs its already-frozen scenarios, and awaits
human approval before reporting whether admitted attention changed. The CLI
must visibly distinguish `AWAITING CODING HARNESS`, `WORKING`, `INDEX
UNQUALIFIED`, `INDEX QUALIFIED`, and `HEAD`.

Surface ownership remains the first meaningful proposal payload: one unowned
mapped source should become an admitted owned workload, then a changed retained
source revision should derive `RETEST` for that exact owner.

## Deferred

Recurring daemon scheduling, learned ranking, automatic retirement, package
formats beyond the bounded V1 slice, arbitrary tool execution, parser/index
breadth, and remote durability are not prerequisites for this plan's next
anchor.
