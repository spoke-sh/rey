# ADR 0019: Git-Shaped Environment History

- Status: Accepted
- Date: 2026-08-08
- Extended and CLI surface superseded by:
  [ADR 0020](0020-environment-mapping-graph.md) and
  [ADR 0021](0021-environment-admission-index.md)
- Default human log projection superseded by:
  [ADR 0029](0029-environment-history-projection.md)
- Commit timestamp and default human loop superseded by:
  [ADR 0033](0033-git-shaped-environment-loop-fidelity.md)

## Context

Rey's first environment CLI exposes independent `inspect`, `diff`, `prove`,
and `verify` utilities. Those commands prove the underlying snapshot and delta
contracts, but they leave the user to name files and manually reconstruct the
sequence of observations. That interaction does not embody a diff-directed
runtime: the current environment, its last accepted revision, and their delta
should form one tight revision surface.

The desired interaction is Git-shaped. A user should be able to commit an
observed environment revision, inspect whether the working observation differs
from it, and reopen a patch-bearing log. Rey must not imply that these commits
are Git objects, mutations of the host environment, or Spoke-durable records.

## Decision

The pre-alpha CLI makes a hard naming cut from `rey environment` to `rey env`.
No compatibility alias is retained. At the time of this decision, existing
snapshot, delta, certificate, and bundle commands remained beneath `env`; ADR
0020 later replaced that loose-file surface with `HEAD → WORKING` diff and
removed manual proof commands.

The first revision workflow adds:

```text
rey env [--workspace PATH] [--state-dir PATH] status
rey env [--workspace PATH] [--state-dir PATH] commit -m MESSAGE
rey env [--workspace PATH] [--state-dir PATH] log [-p] [-n COUNT]
```

`commit` performs bounded environment discovery and appends the verified
capability snapshot to `rey.local-environment-history.v1`. A
`rey.environment-commit.v1` binds its sequence, exact parent commit, message,
and capability-snapshot identity into a semantic commit id. Commits form one
linear chain in this slice. An unchanged observation is rejected rather than
creating an empty semantic revision.

The default store is `${workspace}/.rey/env/state.json`; an explicit
`--state-dir` selects another boundary. The state file retains complete commit
documents and snapshots, is bounded by commit count and bytes, verifies the
entire chain on every read, rejects symlinked or non-file state paths, and is
published through a same-directory temporary-file rename.

`status` is read-only with respect to Rey state. It performs a fresh bounded
observation and compares the HEAD snapshot, or an empty snapshot before the
first commit, to that working observation. There is no staging area in v1.

`log` reads retained state newest-first. `-p` recomputes and renders the exact
parent-to-commit capability delta for every selected commit, including an
empty-to-root delta. `-n` bounds selected history. Human output exposes commit,
parent, snapshot, completeness, capability counts, delta direction, assessment,
change counts, changed fields, and the local retention boundary. Explicit JSON
emits bounded typed status, commit, and log envelopes without diagnostics.

Commit ids intentionally omit wall-clock time, ambient author identity, and
host-specific display state. Sequence and parent order the local chain;
snapshot and message provide semantic content. The v1 store claims no `fsync`
crash durability, multi-process transactionality, locking, authenticated
writer, branching, merge semantics, remote retention, or Spoke durability.

## Consequences

- Environment deltas become a navigable user workflow instead of loose files.
- At the time of this decision, `status`, `commit`, and `log -p` provided the
  high-fidelity human verification surface for environment revision behavior.
- `inspect` remained a lower-level diagnostic in this decision. ADR 0020 made
  `diff` history-aware, and ADR 0021 later removed `inspect`, added the
  admission index, and made commits index-only.
- Later work may add revision selection, `show`, named refs, or Spoke-backed
  retention without changing what v1 commits claim.
- A Rey environment commit never changes the discovered environment and is not
  a Git commit.
