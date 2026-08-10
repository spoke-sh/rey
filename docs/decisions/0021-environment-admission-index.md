# ADR 0021: Environment Admission Index

- Status: Accepted
- Date: 2026-08-08
- Supersedes CLI portions of: [ADR 0020](0020-environment-mapping-graph.md)
- Default human diff projection superseded by:
  [ADR 0028](0028-environment-three-plane-diff.md)

## Context

The first Git-shaped environment history still compares committed HEAD directly
with a fresh observation and exposes a separate `inspect` command. That leaves
no review boundary between observing environment drift and accepting it into a
commit. A commit re-observes ambient state, so the user cannot know that the
snapshot committed is exactly the snapshot they reviewed.

The environment interface needs Git's third plane: an admission index. The
index records the exact capability snapshot selected for the next environment
commit. It is acceptance into Rey's environment history, not authority to run
a discovered executable or admit one of its potential capabilities.

## Decision

The accepted environment surface is:

```text
rey env status
rey env add [-p]
rey env diff [--staged]
rey env commit -m MESSAGE
rey env log [-p] [-n COUNT]
```

`inspect` is removed. `status` is the single inventory and revision view. It
performs one fresh bounded observation, exposes the working mapping dimensions
and exact snapshot identity, then compares two directed planes:

```text
HEAD → INDEX       changes admitted for the next commit
INDEX → WORKING    observed changes not yet admitted
```

Without a retained index, the effective index equals HEAD, or the typed empty
snapshot before the first commit. Human status renders separate “changes to be
committed” and “changes not staged for admission” sections. Explicit JSON uses
`rey.environment-status.v2` and retains HEAD, the optional index, the complete
working snapshot, and both authoritative deltas.

`env add` observes the working environment and replaces the admission index
with that exact snapshot. `env add -p` computes `INDEX → WORKING`, prompts once
per canonical capability change, and applies only selected insertions,
deletions, or modifications to a new verified index snapshot. Selection never
operates on raw variable values or file bytes. An index is bound to the exact
HEAD commit id and becomes stale if HEAD changes independently.

`env diff` follows Git direction: by default it renders `INDEX → WORKING`;
`--staged` renders `HEAD → INDEX`. Before a first commit, HEAD is `EMPTY`.

`env commit` never observes the environment. It verifies and commits only the
retained admission index, then clears that index after the history update is
published. No index means nothing to commit. Working drift after `add` remains
unstaged and cannot leak into the commit.

The index is a separate bounded `rey.environment-admission-index.v1` document
at `${state-dir}/index.json`. Its identity binds schema, base commit id, and
snapshot id. Reads verify the document, exact base, snapshot identity, byte
bound, and symlink boundary. Publication uses the same local same-directory
temporary rename contract as history. History publication and index removal
are not one crash-atomic transaction; if removal fails after a successful
commit, the now-stale index is rejected against the new HEAD.

## Consequences

- The snapshot reviewed and staged is the snapshot committed.
- Status becomes the one high-fidelity human environment interface.
- Partial admission is capability-row based and deterministic.
- The index adds no executable, mutation, or proof authority.
- Pathspecs, named index entries, non-interactive selectors, patch editing,
  reset/restore, revision expressions, and crash-atomic multi-file transactions
  remain later work.
