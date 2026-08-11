# Plan 0015: Git-Shaped Environment Loop Fidelity

- Status: Completed
- Decision: [ADR 0033](../docs/decisions/0033-git-shaped-environment-loop-fidelity.md)

## Outcome

Make the environment admission loop readable and verifiable through the Rey
CLI with Git-shaped status, patch selection, and dated bounded history while
preserving authoritative typed deltas.

## Completion Checklist

- [x] Replace the default `env status` wall with staged and unstaged
  working-tree groups over environment-native objects.
- [x] Keep observation, application-search, and reasoning-map evidence in
  structured status and diff while the human view remains change-directed.
- [x] Reduce clean human status to its environment coordinate and result.
- [x] Render `env add -p` as confirmable environment hunks without changing
  canonical capability-key staging.
- [x] Keep the Git application identity in environment discovery while moving
  repository snapshots and semantic-index movement to cadence/activation.
- [x] Prevent generic patch fallback from printing raw structured provenance.
- [x] Make `env log -n <count>` prove its selection bound in the CLI fixtures.
- [x] Put `ENV@n`, parent, date, and message into a Git-shaped log header.
- [x] Bind dates into complete v1 commit identities; ADR 0048 later removed the
  earlier undated reader during the fresh-state reset.
- [x] Make successful human commits silent while preserving explicit JSON
  receipts and stderr failure diagnostics.
- [x] Advance affected structured schemas and update foundational contracts.
- [x] Cover stdout, stderr, JSON, exit behavior, partial staging, and timestamp
  tampering in focused tests.

## Concrete Anchor

```text
rey env status
rey env diff
rey env add -p
rey env commit -m 'accept mapped toolchain'
rey env log -n 3
rey env log -p -n 1
```

The default status is the scan surface, diff is the exact review surface,
interactive add is the admission surface, and log is the retained chronology.
No command grants execution authority to a discovered application.

## Current Proof

The CLI integration fixture exercises unborn, clean, changed, staged, and mixed
states; full and partial admission; commit without re-observation; `-n 1` and
`-n 2` history selection; plain and patch history; structured output; and
silent table-mode commit success plus stdout/stderr failure contracts. Unit
fixtures reject timestamp tampering and
verify a synthetic undated v1 commit under its original digest contract. A
Git-index fixture proves that staging a file changes Git while preserving the
environment snapshot identity, and a CLI fixture proves generic interactive
hunks omit structured provenance.

Captured on 2026-08-10:

```text
nix develop path:$PWD#ci --command just check
# Prettier, TypeScript, 23/23 UI tests, Vite, Rustfmt, Clippy -D warnings,
# flake evaluation, and repository diff validation passed

nix develop path:$PWD#ci --command just test
# 142/142 Rust tests, 23/23 UI tests, and every documentation test passed
```
