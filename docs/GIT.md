# Git Context And Activation

This document defines Rey's target Git provider and delta-trigger contracts.
Git is both a source of exact code identities and a pollable change substrate
for software-development applications. Concrete schemas and implementation
library choices remain Plan 0001 work.

## Purpose

A software project already has a useful feedback structure:

- the commit graph records durable reviewed transitions;
- refs select moving graph positions;
- the index records the next proposed tree, including partial staging and
  conflicts; and
- the worktree contains unstaged and untracked context.

Rey observes those surfaces as typed frames. Deltas between frozen Git
snapshots can invalidate lenses, update a frontier, and activate only the
application components that declared a dependency on the change.

Git does not provide a monotonic event log by itself. Refs can move backward or
sideways, commits can be replaced by rebases, the index is mutable, and a
worktree may be shared with other processes. Rey therefore polls snapshots and
derives explicit directed deltas rather than treating `git log` output as an
append-only queue.

## Git Surfaces

The provider distinguishes:

- **object database** — commits, trees, blobs, tags, and their hash algorithm;
- **commit graph** — parent edges and reachable history;
- **refs** — branches, tags, remotes, and symbolic refs;
- **HEAD** — per-worktree symbolic or detached selection;
- **index** — per-worktree proposed tree and conflict stages;
- **worktree** — tracked, modified, deleted, and untracked filesystem state;
- **repository configuration** — bounded metadata, never ambient execution
  permission; and
- **shallow/replacement state** — facts that can make history incomplete or
  change graph interpretation.

A bare repository has no index or worktree. Linked worktrees share an object
database and most refs but have distinct HEAD, index, and worktree state.
Submodules are separate repositories and providers; their recorded gitlink OID
is not the same thing as recursively trusting or executing the submodule.

## Repository Identity

A repository observation records:

- provider id and implementation revision;
- canonical allowed workspace root;
- repository common-directory identity;
- worktree identity and worktree root when present;
- object-format algorithm;
- bare, shallow, sparse, split-index, and linked-worktree facts;
- selected HEAD symbolic ref and commit OID;
- watched ref names and OIDs;
- semantic index digest when present;
- optional bounded worktree-status digest;
- capability snapshot id; and
- observation completeness, limits, and errors.

Git OIDs are opaque, algorithm-qualified values. Rey does not assume SHA-1
width or manufacture an OID for unhashed worktree content.

The repository path alone is not durable identity. A local binding combines
repository/worktree identity with exact OIDs and semantic digests available at
the observation boundary.

## Typed Relations

The initial Git provider should be able to produce these logical relations:

```text
git_repositories(repository_id, worktree_id, object_format, bare, shallow, ...)
git_refs(repository_id, ref_name, ref_kind, symbolic_target, target_oid)
git_commits(repository_id, commit_oid, tree_oid, author_time, committer_time, ...)
git_commit_parents(repository_id, commit_oid, parent_index, parent_oid)
git_commit_changes(repository_id, commit_oid, parent_oid, path, change_kind, ...)
git_index_entries(repository_id, worktree_id, path, stage, mode, blob_oid, flags)
git_worktree_status(repository_id, worktree_id, path, index_state, worktree_state, ...)
git_activations(activation_id, trigger_id, source_snapshot, target_snapshot, ...)
```

Commit metadata and all display forms are untrusted bounded inputs. Blob
content remains content rather than being copied into every relation.

Git paths are identity-bearing byte sequences, not assumed UTF-8 strings. A
frame preserves raw path identity or a reversible encoding and carries a
separate bounded display form. Lossy decoding never participates in keys,
digests, trigger matching, or proof identity. Commit headers and messages are
likewise decoded only under explicit rules while retained bytes remain
addressable when required.

## Index Semantics

The Git index is a semantic staging relation, not merely a file timestamp or
raw checksum. A trigger-relevant index identity is derived from ordered logical
entries including:

- path bytes under Git's path rules;
- stage `0`, or conflict stages `1`, `2`, and `3`;
- object mode and blob/gitlink OID;
- intent-to-add, skip-worktree, assume-unchanged, and other selected flags whose
  meaning is part of the declared contract; and
- repository/worktree identity and index format semantics.

Raw index bytes and checksum may be retained as provenance, but they are not the
default semantic trigger. Git can refresh stat-cache metadata without changing
the proposed tree. That must not activate an application component whose
contract depends only on staged content.

Split and sparse indexes must be expanded or interpreted through supported Git
semantics before Rey claims a complete logical index relation. An index lock,
corrupt index, unsupported extension, or unresolved conflict is an explicit
observation state.

## Canonical Development Deltas

Git-backed spaces commonly compare:

- commit `A` to commit `B` — durable tree/graph change;
- watched ref snapshot `n` to `n+1` — ref movement and reachability change;
- `HEAD` tree to index — staged proposal;
- index to worktree — unstaged tracked change;
- `HEAD` tree to worktree — complete tracked local change; and
- one semantic index snapshot to another — staging activity.

Untracked files are a separate declared worktree surface. Ignored files remain
excluded unless a lens explicitly and safely includes them. A Git text patch is
useful evidence, but Rey should also retain typed path, mode, OID, rename/copy
classification, and conflict relations needed for scheduling and proof.

## Poll Cursor

A Git poll cursor identifies the last completely processed snapshot, not just a
timestamp. It can include:

- repository and worktree identity;
- watched ref names and target OIDs;
- HEAD symbolic/detached state;
- semantic index digest;
- declared worktree-status digest;
- shallow/replacement interpretation identity;
- provider implementation revision; and
- last committed Rey trace/transition identity.

The cursor advances only after the derived delta, matched activations, and
required transition evidence reach their claimed retention boundary. A crash
before cursor advancement can replay work. Consumers use activation identity
and action idempotency; Rey does not claim exactly-once Git triggering.

Local cursors have local-file retention guarantees. A connected profile may
store poll events and cursors through Spoke streams/tables, but Git remains the
authoritative source of repository state.

## Polling And Ref Movement

One bounded poll performs:

1. revalidate the repository and capability snapshot;
2. freeze current HEAD, watched refs, semantic index, and declared worktree
   state;
3. compare them with the prior cursor;
4. traverse only the bounded commit graph needed to classify ref movement;
5. materialize typed Git deltas;
6. match trigger predicates and create deterministic activation identities;
7. admit and run selected application components; and
8. advance the cursor after transition evidence commits.

Polling observes snapshots, not every intermediate mutation. Commits can often
be recovered from the object graph within retention and traversal bounds, but
transient index/worktree states between polls may never be observed. Poll
interval, coalescing, skipped intermediate-state count when knowable, and
completeness are evidence. A proof cannot claim that every staging action was
seen merely because the final semantic index was processed.

Ref movement is classified explicitly:

- **created** or **deleted**;
- **fast-forward** — the old target is an ancestor of the new target;
- **rewound** — the new target is an ancestor of the old target;
- **rewritten/diverged** — neither target is an ancestor of the other within
  complete observed history; or
- **unknown** — shallow, missing, corrupt, or bounded history prevents a sound
  classification.

A fast-forward can yield an ordered set of newly reachable commits under a
declared traversal order. A rebase or force-push emits a ref rewrite and
reachable-set delta; it does not fabricate append events as though history were
monotonic. Merge commits retain all ordered parent edges.

## Triggers And Activations

A trigger maps a typed source delta predicate to one or more application
components. A declaration includes:

- stable trigger id and revision;
- repository/worktree selector;
- source relation and event classes;
- path, ref, commit, stage, or change predicates;
- required provider capabilities and completeness;
- target lenses, claims, or actions;
- coalescing/debounce policy where timing is meaningful;
- per-activation budgets and concurrency policy; and
- replay and idempotency behavior.

Initial Git event classes may include:

```text
ref.created
ref.deleted
ref.fast_forward
ref.rewound
ref.rewritten
head.changed
commit.reachable_added
commit.reachable_removed
index.changed
index.conflicted
worktree.changed
```

An activation is a deterministic proposal to start or resume a component. Its
identity covers the trigger revision, source and target snapshot ids, matched
delta subset, and component revision. It still passes normal runtime admission;
a Git delta does not directly execute a tool or mutation.

Applications can therefore activate narrowly. An index delta touching Rust
sources might refresh symbol and diagnostic lenses, while a new commit on a
release ref might run a broader conformance proof. Unrelated components remain
idle.

## Rey–Spoke Development Loop

Git activation gives the Rey–Spoke feedback loop a concrete clock without
creating a package dependency:

- a new Spoke commit can activate Rey's external-client capability and
  conformance lenses;
- the resulting delta can identify a closed gap or a new missing capability;
- a Rey commit can activate standalone self-checks and connected Spoke-backed
  exploration; and
- index changes in either checkout can trigger cheap local diagnostics before
  a commit exists.

Each repository remains independently buildable. Polling observes exact Git
state and invokes only public or local provider contracts already admitted by
Rey.

## Safety And Bounds

- Repository discovery remains beneath explicitly selected roots.
- Read-only polling does not acquire optional index locks or refresh the index.
- The provider does not run Git aliases, hooks, credential helpers, filters,
  fsmonitor hooks, submodule commands, or project scripts during discovery.
- A Git CLI implementation must use direct argv, controlled configuration and
  environment, no optional locks, bounded captures, and deadlines. A library
  implementation must prove equivalent repository semantics.
- Commit traversal, changed paths, untracked paths, object reads, rename
  detection, and output bytes are bounded.
- Network fetch, pull, push, checkout, reset, clean, add, commit, and index
  mutation are separate explicit actions and never part of polling.
- Commit content and repository configuration are untrusted input.

## Required Fixtures

The first Git provider must cover:

- non-repository and bare-repository discovery;
- SHA-1 and supported SHA-256 repository identity;
- symbolic and detached HEAD;
- initial/unborn branch;
- fast-forward, rewind, divergence, force-push/rebase, merge, ref creation, and
  ref deletion;
- shallow or missing history with unknown classification;
- clean, staged, unstaged, untracked, intent-to-add, sparse, split-index, and
  conflicted index/worktree states where supported;
- raw index metadata refresh with no semantic activation;
- linked worktrees and submodule gitlinks;
- corrupt/locked/unsupported index behavior;
- bounded history and path overflow;
- crash before and after cursor advancement with idempotent activation replay;
  and
- proof that polling executes no repository hooks or mutation commands.
