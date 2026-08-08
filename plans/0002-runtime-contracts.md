# Plan 0002: Runtime Transition And Reasoning Surface Contracts

## Outcome

Freeze and prove Rey's formal runtime transition machine and bounded,
delta-directed reasoning surface before implementing generic frontier
scheduling. Deliver pure deterministic Rust contracts and typed fixtures only;
do not retrieve from providers, select work, invoke policy, admit actions, or
execute effects in this plan.

## Completion Checklist

- [x] ADR 0013 fixes the v1 state and reasoning-surface contracts.
- [x] `docs/RUNTIME.md` defines phases, events, guards, identities, bounds, and
  provider ownership.
- [x] `rey-runtime` implements a pure deterministic event reducer with no
  executor or scheduler.
- [x] Runtime fixtures reject illegal phase jumps, malformed or mismatched
  transition ids, fabricated bootstrap progress, incomplete-observation
  progress, premature convergence, and continuation without retained evidence.
- [x] `rey-policy` implements the canonical bounded reasoning-surface document
  and DataFrame projection with no policy adapter.
- [x] Surface fixtures prove canonical identity, Arrow metadata round-trip,
  bounds, citations, omissions, and tamper detection.
- [x] Focused crate tests, full workspace tests, Clippy, build, and Nix checks
  pass.
- [x] Repository truth and verification evidence are recorded below.

## Contract Boundaries

- `rey-runtime` depends only on shared identity/serialization/error contracts.
- `rey-policy` depends on shared identities and the existing DataFrame wrapper.
- No dependency on `rey-environment`, `rey-git`, `rey-diff`, `rey-proof`, or a
  future `rey-frontier` scheduler is introduced.
- Runtime state records opaque exact frontier, surface, proposal, observation,
  delta, and evidence identities; owning crates retain their semantics.
- The reasoning surface cites provider-owned exact evidence and admissible
  action contracts but performs no retrieval or execution.

## Runtime Acceptance

- Bootstrap commits a real baseline without fabricating a predecessor delta.
- Every active event matches the transition identity fixed when orientation
  begins.
- A successful execution can advance only to observation.
- Cancellation during execution still requires terminal provider outcome,
  observation, evaluation, and commit.
- Transition and residual delta identities remain separate.
- Convergence requires no next frontier and a matching committed stop reason.
- Nonterminal continuation requires a frontier and retained or verified
  evidence under the selected profile.
- Event, observation-reference, transition-delta-reference, and
  residual-delta-reference bounds fail closed and participate in state
  identity.
- State order, phase shape, schema, and semantic identity verify after replay.

## Surface Acceptance

- Surface identity binds exact application, component, space, trace,
  transition, frontier, capability, projection, evidence, action, omission, and
  effective-limit semantics.
- Rows are canonical and uniquely keyed by frontier row identity.
- Every row cites at least one transition/residual delta or claim.
- Transition and residual delta roles cannot be conflated within a row.
- Every evidence/action citation resolves within the bounded envelope.
- Complete surfaces contain no omissions; partial/truncated surfaces expose
  them.
- Row, delta, evidence, action, omission, evidence-byte, string-byte, and
  retrieval-iteration bounds fail closed.
- Native evidence remains referenced content rather than synthetic DataFrame
  cells.

## Deferred

Generic frontier schemas and progress comparison, dependency invalidation,
priority policy, orientation strategy, provider retrieval, policy proposals,
action admission/execution, transition persistence, and multi-step scheduling
remain later slices.

## Verification Evidence

Runtime-state and reasoning-surface contract proof captured on 2026-08-07:

```text
cargo clippy -p rey-runtime -p rey-policy --all-targets -- -D warnings
cargo test -p rey-runtime -p rey-policy
# 11 runtime and 8 policy tests passed; both doc-test suites passed
just check
just test
# 56/56 workspace tests and all nine crate doc-test suites passed
just build
nix flake check "path:$PWD"
nix flake check "path:$PWD" --all-systems --no-build
# packaged x86_64-linux checks built; x86_64-linux, aarch64-linux, and
# aarch64-darwin outputs evaluated
```
