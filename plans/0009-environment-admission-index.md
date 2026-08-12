# Plan 0009: Environment Admission Index

- Status: Complete
- Completed: 2026-08-08
- Decision: [ADR 0021](../docs/decisions/0021-environment-admission-index.md)

## Outcome

Make `rey env status` the single Git-shaped environment view and place a
reviewable admission index between working observation and commit. Users can
stage the complete environment or select capability changes interactively, and
commit exactly what was staged.

## Completion Checklist

- [x] Accept ADR 0021 and update decision/plan indexes.
- [x] Add a bounded, verified, HEAD-bound admission index with safe local
  publication and stale-index rejection.
- [x] Derive `HEAD → INDEX` and `INDEX → WORKING` deltas in
  `rey.environment-status.v1`.
- [x] Implement `rey env add` and capability-selective `rey env add -p`.
- [x] Make `env commit` consume only the retained index and clear it after a
  successful history publication.
- [x] Make `env diff` show unstaged changes by default and staged changes with
  `--staged`.
- [x] Remove `env inspect` and surface inventory, mapping, index, and both
  change planes through status.
- [x] Cover unborn, clean, staged, unstaged, mixed, partial, stale, tampered,
  symlinked, no-op, JSON, stdout/stderr, and exit behavior.
- [x] Update foundational docs and human command examples.
- [x] Run focused tests, full workspace tests, Clippy, build, Nix checks, link
  review, and repository-truth audit.

## Proof

Captured on 2026-08-08:

```text
cargo test -p rey --lib env::tests
cargo test -p rey --test cli env_
just check
just test
# 123/123 tests passed; all workspace doc tests passed
just build
nix flake check "path:$PWD"
# package, workspace-test, and development-wrapper derivations passed
```

Human verification exercised `rey env --help` and `rey env status` against
the checked-in mapping. The CLI exposed only `status`, `add`, `commit`, `log`,
and `diff`; status rendered independent `HEAD → INDEX` and
`INDEX → WORKING` planes plus complete inventory and mapping evidence. CLI
fixtures additionally prove partial interactive selection, exact-index commit
despite later working drift, JSON cleanliness, categorized errors, stale and
tampered index rejection, and safe symlink handling.

## Deferred

Pathspecs, named index entries, non-interactive selectors, patch editing,
reset/restore, revision expressions, stronger transactionality, and
remote index retention remains a later slice.
