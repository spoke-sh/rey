# Plan 0003: Frontier Progress And Scheduling Contracts

## Outcome

Implement the smallest deterministic scheduling slice between committed
frontier and orientation. Freeze canonical frontier and progress relations,
reject stale scheduling inputs, select bounded work units under an explicit
versioned order, and record the decision in the runtime and reasoning-surface
lineage. Do not derive domain work, retrieve providers, choose an action,
execute effects, or add a recurring scheduler.

## Completion Checklist

- [x] ADR 0014 fixes the frontier, progress, scheduler, runtime v2, and surface
  v2 contracts.
- [x] `docs/FRONTIER.md` defines identity, coverage, convergence, direction,
  bounds, ordering, and non-goals.
- [x] `rey-frontier` implements canonical frontier, progress, and scheduling
  documents plus Polars/Arrow projections.
- [x] Frontier fixtures cover canonical identity, bounds, citations,
  blockers, convergence, Arrow metadata, and tampering.
- [x] Progress fixtures cover resolved, introduced, updated, unchanged,
  mixed, converged, incompatible, replay, and bounded behavior.
- [x] Scheduling fixtures cover deterministic order, unit/cost bounds,
  deferred work, stale record/frontier/capability inputs, terminal frontier
  outcomes, replay, and tampering.
- [x] `rey-runtime` records scheduling before orientation and exposes an
  explicit scheduling-stop path.
- [x] `rey-policy` binds the scheduling decision in reasoning-surface identity
  and Arrow metadata.
- [x] Focused crate tests, full workspace tests, Clippy, build, and Nix checks
  pass.
- [x] Repository truth and verification evidence are recorded below.

## Contract Boundaries

- `rey-frontier` depends only on shared identity and DataFrame contracts plus
  existing workspace dependencies.
- The scheduler consumes declared priority and estimated-cost inputs; it does
  not calculate domain priority or select an action.
- Runtime and policy crates retain opaque scheduling identities and do not
  depend on `rey-frontier`.
- No provider read, policy request, action admission, effect, persistence
  engine, async runtime, or CLI command is introduced.

## Acceptance

- Stable `work_id` aligns logical work while derived `row_id` exposes semantic
  change.
- Empty work converges only with complete delta/claim coverage and satisfied
  required claims.
- Progress preserves source-to-target direction and never guesses the meaning
  of updated work or emits a scalar proof score.
- Selection order is priority descending, cost ascending, then `work_id`
  ascending.
- Row, unit, cost, reference, blocker, change, and string-byte limits fail
  closed and participate in artifact identity.
- Exact expected record, frontier, and capability identities are checked before
  scheduling.
- Orientation is illegal until the active transition records a scheduling
  decision.
- The reasoning surface binds that decision, and all artifacts detect semantic
  tampering.
- No fairness, starvation, generic invalidation, provider, executor, or loop
  claim is made.

## Deferred

Application declarations, dependency invalidation, frontier derivation from
real deltas and claims, provider retrieval, iterative orientation strategy,
policy proposals, action admission/execution, retry, activation, trace
persistence, and recurring scheduling remain later slices.

## Verification Evidence

Frontier, progress, scheduling, runtime-v2, and surface-v2 proof captured on
2026-08-07:

```text
cargo clippy -p rey-frontier -p rey-runtime -p rey-policy --all-targets \
  -- -D warnings
cargo test -p rey-frontier -p rey-runtime -p rey-policy
# 12 frontier, 13 runtime, and 9 policy tests passed; doc tests passed

just check
just test
# 71/71 workspace tests and all ten crate doc-test suites passed
just build

nix flake check "path:$PWD"
nix flake check "path:$PWD" --all-systems --no-build
# packaged x86_64-linux checks built; x86_64-linux, aarch64-linux, and
# aarch64-darwin outputs evaluated

# all local Markdown links and repository truth checked
```
