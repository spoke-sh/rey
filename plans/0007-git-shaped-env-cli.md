# Plan 0007: Git-Shaped Env CLI

- Status: Complete
- Completed: 2026-08-08
- Decision: [ADR 0019](../docs/decisions/0019-git-shaped-environment-history.md)
- Extended by: [Plan 0008](0008-environment-mapping-graph.md),
  [Plan 0009](0009-environment-admission-index.md), and
  [ADR 0021](../docs/decisions/0021-environment-admission-index.md);
  human log projection superseded by
  [ADR 0029](../docs/decisions/0029-environment-history-projection.md)

## Outcome

Replace the file-utility feel of the environment CLI with the first bounded
revision workflow. Users can inspect HEAD versus a fresh environment, commit a
verified capability revision, and reopen parent-directed patches from a local
log without confusing local Rey state with Git or remote durability.

## Completion Checklist

- [x] Make a hard CLI cut from `rey environment` to `rey env` everywhere.
- [x] Define and verify bounded environment commit, history, status, and log
  envelopes with exact snapshot and parent lineage.
- [x] Persist a single linear local history beneath the workspace using safe,
  bounded, atomic replacement and explicit narrow retention claims.
- [x] Implement `rey env status` as HEAD-to-working capability comparison with
  no hidden state mutation.
- [x] Implement `rey env commit -m` with fresh discovery, no-op rejection, and
  human plus JSON output.
- [x] Implement newest-first `rey env log`, bounded `-n`, and evidence-linked
  `-p` capability patches.
- [x] Retain the existing inspect/diff/prove/verify/bundle behavior beneath the
  renamed command.
- [x] Cover stdout, stderr, JSON, exit codes, tampering, symlinks, bounds,
  empty history, no-op commits, and deterministic replay.
- [x] Exercise status, two commits, and `log -p` through the high-fidelity human
  CLI and update foundational documentation.
- [x] Run focused tests, full workspace tests, Clippy, build, Nix checks, link
  review, and repository-truth audit.

## Deferred

Plan 0008 delivered the history-aware default `diff` and removed the retained
manual proof commands. Plan 0009 later delivered the first admission index;
its deferred staging breadth replaces this plan's original staging deferral.
Revision expressions, `show`, branches, merges, rewriting, garbage collection,
concurrent writers, crash-durable flushing, signatures, and remote
retention remain later slices.
