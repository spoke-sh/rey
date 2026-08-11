# Plan 0005: First Executable Workload Slice

## Outcome

Implement one bounded zero-agent workload path through `workloads list`,
`status`, `test`, and `run`. Preserve typed scenario mismatch deltas, retain
local cross-command progress with explicit narrow guarantees, qualify only an
exact all-passing graph, and cut the legacy application/component identity
schemas before exposing the commands.

## Completion Checklist

- [x] ADR 0016 fixes the built-in catalog, schemas, local result boundary,
  formats, exits, and identity cutover.
- [x] `rey-diff` implements verified typed UTF-8 scenario output deltas.
- [x] `rey-runtime` implements bounded workload/graph/scenario documents,
  validation, stable DAG execution, tests, qualification, and run results.
- [x] The fresh v1 frontier/progress/scheduling contracts bind workload, graph,
  scenario suite, and campaign identities.
- [x] The fresh v1 reasoning surface binds the same workload-centered identities.
- [x] The local result provider bounds and verifies reads, publishes with a
  same-directory rename, derives staleness, and states its non-guarantees.
- [x] `workloads list` shows exact graph identities and passing/evaluated
  progress without executing work.
- [x] `workloads status` exposes retained scenarios, deltas, qualification,
  freshness, stop reason, and latest run.
- [x] `workloads test` proves passing, failing, aggregate, deterministic replay,
  stdout/stderr, exit behavior, incremental human results, default failure
  diffs, `-v` matching evidence, and `-vv` exact bindings.
- [x] `workloads run` refuses absent/stale qualification and executes the exact
  qualified graph against caller input.
- [x] Repository truth, CLI help, examples, and plan indexes match behavior.
- [x] Focused tests, full workspace tests, Clippy, build, Nix checks, and local
  Markdown links pass.

## Boundaries

- The catalog contains only reviewed built-in fixture workloads.
- Graph operations are built-in UTF-8 `trim` and `uppercase`; no tool or shell
  execution is introduced.
- Every graph is a finite typed DAG and uses a deterministic serial baseline.
- The local state document is a result index, not a general database or a
  source of workload definitions.
- No agent, policy transport, graph optimization loop, Spoke provider, generic
  persistence engine, recurring scheduler, or service is introduced.

## Verification Evidence

Captured on 2026-08-07:

```text
just check
# git diff, Rustfmt, warnings-as-errors Clippy, and flake evaluation passed

just test
# 86/86 workspace tests and all documentation tests passed

just build
# every workspace crate and feature built

nix flake check "path:$PWD"
# package, workspace-test, and dev-wrapper derivations passed on x86_64-linux

# all local Markdown links resolved and git diff --check passed
```

CLI fixtures prove redirected JSON and forced table output, read-only list,
blocked run exit `3`, deterministic passing qualification and same-graph run,
typed failing delta exit `2`, aggregate precedence, malformed-state exit `1`,
and clean stdout/stderr separation.

Interface-fidelity verification reran on 2026-08-08: `just check`, all 89
workspace tests plus documentation tests, and `just build` passed. The added
fixtures cover incremental declaration-order observation, plain passing and
failing tables, failure-first diffs, `-v`, `-vv`, ANSI-free forced tables, help,
semantic exits, and unchanged redirected JSON.
