# ADR 0007: Git Polling And Delta Activation

- Status: Accepted; public activation target terminology narrowed by ADR 0015
- Date: 2026-08-07
- Extends: [ADR 0001](0001-diff-directed-runtime.md),
  [ADR 0005](0005-environment-awareness-and-optional-spoke.md), and
  [ADR 0006](0006-rey-spoke-recursive-improvement.md)
- Narrowed by: [ADR 0015](0015-workload-centered-product.md)

## Context

Software projects already expose meaningful version and proposal state through
Git. Commits and refs describe durable graph positions, while the index
describes the next proposed tree before a commit. Polling those surfaces can
direct incremental codebase work without repeatedly exploring an entire
workspace.

Git cannot safely be modeled as one append-only event log. Refs may be rebased,
reset, deleted, or force-pushed; history can be shallow; HEAD and index are
per-worktree; and the index can change raw bytes merely because Git refreshed
stat metadata. A crash between observing a change and recording its effects can
also replay the poll.

Rey needs Git-aware snapshot, delta, cursor, and trigger semantics that preserve
those facts and do not execute repository-controlled hooks during discovery.

## Decision

Git is a first-class environment provider for software spaces. It exposes typed
relations for repository/worktree identity, refs, commits, parent edges, path
changes, semantic index entries, declared worktree status, and activations.

Repository identity records the common object database, worktree identity,
object hash algorithm, bare/shallow/sparse/split-index facts, HEAD, watched
refs, semantic index digest, completeness, and provider revision. OIDs are
opaque and algorithm-qualified.

The index trigger identity derives from logical entries: path, stage, mode,
blob/gitlink OID, and selected semantic flags. Raw index bytes may be retained
as provenance but do not trigger staged-content applications when only
stat-cache metadata changed.

A poll compares a frozen Git snapshot with the last completely processed
cursor. Ref movement is classified as created, deleted, fast-forward, rewound,
rewritten/diverged, or unknown. Only a sound fast-forward over complete bounded
history yields newly reachable commit activations. Other movements retain
explicit reachability deltas rather than fabricated append events.

A versioned trigger maps typed Git delta predicates to application components.
Its deterministic activation identity covers trigger revision, component
revision, source/target snapshots, and matched delta. The activation enters
normal runtime admission and cannot directly execute an effect.

The cursor advances only after required delta, activation, transition, and
proof evidence reaches its claimed retention boundary. Crashes may replay an
activation. Rey uses deterministic activation identity and action idempotency
and makes no exactly-once claim.

Polling observes snapshot endpoints. It does not claim to observe every
transient index or worktree state between intervals. Interval, coalescing, and
completeness metadata remain evidence for claims that depend on observation
coverage.

Polling is read-only. It does not run Git aliases, hooks, credential helpers,
filters, fsmonitor hooks, submodule commands, or project scripts and does not
refresh or lock the index. Fetch, checkout, add, commit, reset, clean, push, and
other mutations are separate admitted actions.

The Git implementation may use a Rust library, bounded Git plumbing commands,
or both. The implementation choice must prove the same repository semantics and
safety contract before it becomes broad dependency policy.

## Consequences

- Git becomes more than a discovered executable; `rey-git` owns repository and
  activation semantics behind the environment provider boundary.
- Commit, ref, index, and selected worktree deltas can activate only affected
  application components.
- Linked worktrees, bare repositories, shallow history, conflicts, merges,
  submodule gitlinks, and supported sparse/split indexes need focused fixtures.
- Local cursors have local retention guarantees; connected mode may retain poll
  events and cursors through Spoke without making Spoke the Git source of truth.
- Git commit and index deltas provide a concrete activation mechanism for the
  recursive Rey–Spoke conformance loop.
- Trigger and cursor schema changes can make pending or retained activations
  stale independently from source content.
