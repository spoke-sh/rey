# Git Context And Activation

This document defines Rey's target Git provider and delta-trigger contracts.
Git is both a source of exact code identities and a pollable change substrate
for software-development workloads. The first foundation slice implements a
read-only contained repository observation through bounded direct Git argv. It
records repository/worktree identity, object format, bare/shallow state, HEAD,
and a complete logical index-entry digest for the supported stage, mode, OID,
path, assume-unchanged, skip-worktree, and intent-to-add semantics. The
operator cadence slice also reads
the newest 24 commits currently reachable from `HEAD`, preserving exact OIDs,
ordered parents, committer time, subject, object format, shallow state,
truncation, and a semantic sequence identity. It now pairs that sequence with
bounded porcelain-v2 working-tree counts and exact local-upstream publication
state. `rey-git` now also derives one bounded HEAD/watched-ref/semantic-index
transition from a verified retained cursor. It classifies each changed ref as
created, deleted, fast-forward, rewound, rewritten, or unknown and matches
typed triggers into deterministic proposal-only activations. The `rey git`
CLI freezes an exact watched-ref scope at initialization, including refs that
are currently absent, and retains the baseline cursor, one pending transition
with its triggers and proposals, and acknowledged transition history under
`.rey/git`. Every changed HEAD or watched ref also carries bounded canonical
added/removed reachable-commit sets and a bounded tree-to-tree path delta with
reversible byte identities, direction, modes, and object OIDs. The bounded
`rey git watch` surface
repeats that observation, retains every cadence tick and normal terminal
receipt, and stops at the first changed transition without advancing the
cursor. Cross-poll debounce and remote synchronization remain future Git
work. Workload packages can already bind exact
HEAD or semantic-index revisions and derive attention from the acknowledged
cursor snapshot without treating an ambient observation or activation proposal
as authority. A separate workload command can admit a current acknowledged
proposal for scheduling without executing it.

Git is not part of the `rey env` admission snapshot. That loop discovers the
`git` executable as an application, while this provider owns repository HEAD,
refs, semantic index, reachability, and later activation evidence on a separate
clock. A Git transition can direct a workload without manufacturing an
environment-variable or application-identity change.

## Purpose

A software project already has a useful feedback structure:

- the commit graph records durable reviewed transitions;
- refs select moving graph positions;
- the index records the next proposed tree, including partial staging and
  conflicts; and
- the worktree contains unstaged and untracked context.

Rey observes those surfaces as typed frames. Deltas between frozen Git
snapshots can invalidate lenses, update a frontier, and activate only the
workload graph entry points or scenario selections that declared a dependency
on the change.

Git also supplies exact source bindings for mining. Blob/tree/commit identity,
semantic index state, and bounded worktree content let text search, parsing,
symbol/reference indexing, metrics, and visualization state precisely which
software revision they analyzed. A mined relation never substitutes a lossy
display path for Git's identity-bearing path bytes.

Git does not provide a monotonic event log by itself. Refs can move backward or
sideways, commits can be replaced by rebases, the index is mutable, and a
worktree may be shared with other processes. Rey therefore polls snapshots and
derives explicit directed deltas rather than treating `git log` output as an
append-only queue.

The implemented `/cadence` Git lane is correspondingly labeled reachable HEAD
history, not a commit event stream. It is newest-first, bounded, and incomplete
when shallow or truncated. It neither advances a cursor nor infers that the
visible commits were appended since a prior observation. The lane and its
working-tree/publication projection are enabled only when the committed
Environment HEAD admits an available `git` application. Every observation uses
that admission's exact resolved executable path; ambient `PATH` presence does
not enable Git inspection.

`rey.git-repository-status.v1` keeps two axes independent. Working-tree state
counts staged, unstaged, untracked, and conflicted porcelain-v2 records;
ignored files are outside its declared scope. Publication records the branch,
exact `HEAD` and local upstream OIDs, and ahead/behind reachability. Cadence
classifies each visible commit against the retained upstream OID as `pushed`,
`local`, or `unknown`. No status read performs fetch or push, so these are
local-ref facts rather than claims about current remote-host state. See
[Interfaces](INTERFACES.md) for the Cadence projection.

When an upstream names a configured remote, Cadence reads that remote's URL
through the exact admitted `git` executable without contacting it. Credential-
free HTTPS and SSH forms for GitHub, GitLab, and Bitbucket are normalized into
typed web bindings for branch, upstream, and commit links. Local remotes,
unsupported hosts, embedded credentials, and malformed URLs remain explicitly
unbound; the UI never guesses a hub from a ref name.

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
the proposed tree. That must not activate a workload entry point whose contract
depends only on staged content.

The implemented inspector asks Git for its logical split/sparse view and binds
assume-unchanged, skip-worktree, and intent-to-add flags into every entry
identity. Stat-cache and fsmonitor-valid state remain deliberately
non-semantic. An unknown persistent flag makes the snapshot partial with an
explicit omission; an index lock, corrupt index, or unsupported logical form
fails closed. Conflict stages remain complete typed evidence rather than being
silently flattened.

## Canonical Development Deltas

Git-backed spaces commonly compare:

- commit `A` to commit `B` — durable tree/graph change;
- watched ref snapshot `n` to `n+1` — ref movement and reachability change;
- `HEAD` tree to index — staged proposal;
- index to worktree — unstaged tracked change;
- `HEAD` tree to worktree — complete tracked local change; and
- one semantic index snapshot to another — staging activity.

Untracked files are a separate declared worktree surface. Ignored files remain
excluded unless a lens explicitly and safely includes them. A Git text patch
is useful evidence, but Rey also retains typed path, mode, and OID changes
needed for scheduling and proof. The current tree delta deliberately disables
rename/copy inference: a moved blob is one deletion and one addition unless a
later bounded operation explicitly supplies a stronger classification.
Conflict and richer rename/copy relations remain future work.

Source mining may add line/text, syntax-tree, symbol/reference, dependency, or
metric deltas over the same frozen Git inputs. Those artifacts cite their
operation/parser/index revisions and completeness. They do not replace Git's
authoritative object and index semantics or turn partial parsing into a
complete repository claim.

## Poll Cursor

A Git poll cursor identifies the last completely processed snapshot, not just a
timestamp. It can include:

- repository and worktree identity;
- watched ref names and target OIDs;
- HEAD symbolic/detached state;
- semantic index digest;
- declared worktree-status digest;
- shallow state and the disabled-replacement interpretation;
- provider implementation revision; and
- last committed Rey trace/transition identity.

The cursor advances only after the derived delta, matched activations, and
required transition evidence reach their claimed retention boundary. A crash
before cursor advancement can replay work. Consumers use activation identity
and action idempotency; Rey does not claim exactly-once Git triggering.

The implemented `rey.git-poll-cursor.v1` is a verified value contract retained
by the local `.rey/git` store. A baseline cursor can be constructed only with
the exact retained snapshot identity. Its `advance` operation requires the
exact derived `rey.git-poll-transition.v1` identity as retained evidence;
passing a target snapshot id or another transition fails closed. Repeating a
poll from the unchanged source cursor reproduces the same transition and
activation identities.

`rey git status` is read-only. `init` retains the baseline; `poll` retains one
changed transition without moving the cursor; `ack` requires that exact
transition identity, retains it in history, then advances. A repeated poll is
idempotent and a different observation cannot overwrite pending evidence.
Local cursors have local-file retention guarantees. Git remains the
authoritative source of repository state.

`rey git init --watch-ref refs/...` accepts repeatable canonical full ref
names, sorts them into one exact retained scope, and records each name with its
current OID or explicit absence. Later `status`, `poll`, and `watch` reuse that
scope; they do not discover additional refs. Each watched ref is classified
independently from `HEAD`. Trigger `ref_names` select `HEAD` or exact watched
ref names, and each proposal retains the exact names it matched.

`rey git watch` is an explicitly invoked bounded recurrence over the same
poll. Maximum iteration count, interval, and elapsed cadence time are positive
bounded inputs; retry count is a separate nonnegative bound. Before another
observation can be scheduled, the current `rey.git-cadence-tick.v2` is verified
and atomically retained. An unchanged tick cites the already retained cursor
snapshot instead of copying it; a changed tick atomically retains the full
pending poll record and then stops. A failed tick has no observed snapshot and
instead retains a bounded typed failure kind, retryability, detail/truncation,
and omission against the unchanged source cursor. Only retryable failures may
consume `--max-retries`; all attempts also consume `--max-iterations`.

Every terminal invocation retains `rey.git-watch-receipt.v2`, binding its exact
tick range, retry/iteration/time bounds, measured elapsed time, completeness,
omissions, stop reason, pending transition when present, and
`rey.git-watch-outcome.v2` identity. A recovered observation never erases an
earlier failed tick: the receipt remains partial and exits inconclusive.
Retry exhaustion or non-retryable evidence failure also exits inconclusive.
SIGINT or SIGTERM requests cooperative cancellation; Rey finishes the current
bounded Git command, retains the tick boundary and a `cancelled` receipt, then
exits 130. A hard process loss after a tick but before its receipt leaves an
explicit unreceipted gap visible in `git status`. Neither a tick nor a receipt
advances the poll cursor or admits or executes an activation, and no stop
reason claims convergence.

The workload portfolio consumes only this acknowledged cursor snapshot.
Declarations may select its exact HEAD (repository/worktree, optional symbolic
ref, and algorithm-qualified object id or `unborn`) or semantic-index entry
digest (or `absent`). A pending poll and the fresh observation shown by
`status` remain evidence awaiting admission and cannot invalidate a workload.
After `ack`, a revision mismatch derives an ordinary typed dependency-change
attention row citing the acknowledged snapshot; it does not execute an
activation proposal.

## Polling And Ref Movement

One bounded poll performs:

1. revalidate the repository and capability snapshot;
2. freeze current HEAD, watched refs, semantic index, and declared worktree
   state;
3. compare them with the prior cursor;
4. traverse only the bounded raw commit graph and exact endpoint trees needed
   to classify ref movement, derive added/removed reachability, and compare
   paths;
5. materialize typed Git deltas with explicit traversal limits and omissions;
6. match trigger predicates and create deterministic activation identities;
7. retain the transition and proposal evidence;
8. acknowledge that exact evidence and advance the cursor;
9. separately admit an eligible proposal into ordinary workload scheduling;
   and
10. schedule and execute the selected workload scenarios or graph entry point.

The current library and CLI implement steps 1–8 for HEAD, exact watched refs,
bounded reachable-commit sets, bounded tree path deltas, and the complete
supported logical-index semantics. `rey workloads admit-activation` implements
step 9, and
`execute-activation` implements the scenario-selection form of step 10. It
revalidates exact current Git, workload HEAD, graph, scenarios, capabilities,
and budgets before executing. `rey workloads verify-activation` can then
recompute the complete declared suite and retain an exact selected-versus-full
comparison without changing qualification. The CLI can now repeat steps 1–7
through bounded `git watch`, but it deliberately stops before step 8;
arbitrary graph-entry
activation and autonomous recurring execution remain absent.

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

The implemented poll derives canonical OID sets for commits reachable only
from the target (`added`) or only from the source (`removed`) for each changed
`HEAD` or watched ref. `--max-reachable-commits-per-direction` is a positive
hard bound; overflow retains the bounded set and an explicit omission.
Shallow or unavailable history also remains partial even when some known
commits can be retained. Git replacement objects are disabled for inspection,
so classification and reachability use the raw object graph. A rebase or
force-push emits a ref rewrite and reachable-set delta; it does not fabricate
append events as though history were monotonic.

For every changed `HEAD` or watched ref, the implemented poll compares the
exact source and target commit trees. Created and deleted refs use bounded tree
inventories. Each retained change carries an added, deleted, modified, or
type-changed direction; reversible base64url path bytes plus a lossy display;
and the applicable source/target mode and object OID. Changes are canonical by
raw path bytes and `--max-path-changes-per-ref` is a positive hard retention
bound. Overflow retains the canonical prefix and an explicit omission. Missing
trees make only that path delta partial. Rename and copy heuristics are
disabled, so their similarity thresholds cannot alter trigger identity.

## Triggers And Activations

A trigger maps a typed source delta predicate to one or more workload
revisions, scenario selections, or declared graph entry points. A declaration
includes:

- stable trigger id and revision;
- repository/worktree selector;
- source relation and event classes;
- path, ref, commit, stage, or change predicates;
- required provider capabilities and completeness;
- target workload/graph revision, scenarios, observations, claims, or actions;
- coalescing/debounce policy where timing is meaningful;
- per-activation budgets and concurrency policy; and
- replay and idempotency behavior.

The implemented event classes are:

```text
head.ref_changed
ref.created
ref.deleted
ref.fast_forward
ref.rewound
ref.rewritten
ref.unknown
commit.reachable_added
commit.reachable_removed
path.added
path.deleted
path.modified
path.type_changed
index.changed
index.conflicted
```

Later source/path work may add:

```text
worktree.changed
```

An activation is a deterministic proposal to start or resume a workload test
selection or graph entry point. Its identity covers the trigger revision,
source and target snapshot ids, matched delta subset, and exact
workload/graph/scenario selection. It still passes normal runtime admission; a
Git delta does not directly execute a tool or mutation.

The implemented admission accepts only an activation in acknowledged history
whose target snapshot and transition still define the current cursor. It binds
the exact workload HEAD commit and snapshot, matching workload/graph contracts,
resolved scenario ids, retained environment snapshot, automatic intrinsic
runtime snapshot, proposal completeness, and narrowed effective budget into
`rey.workload-activation-admission.v2`. Repeating the same admission is
identity-stable. A pending proposal, stale cursor, changed graph, unknown
scenario, or missing environment/runtime snapshot fails closed. The admission says only
`admitted_for_runtime_scheduling`; no graph has run and no progress is implied.

For a new execution, `rey workloads execute-activation <admission-id>` accepts
only a current unexecuted admission, revalidates its acknowledged Git cursor,
workload HEAD,
exact workload/graph/suite/scenario contracts, retained environment snapshot,
and current intrinsic runtime snapshot, then evaluates the selected scenarios. The typed
`rey.workload-activation-execution.v1` binds the admission and activation to
`rey.workload-scenario-execution-result.v1`, its directed deltas and mining or
topography evidence, and the measured serialized evidence bytes. Evidence
over the admitted cap is rejected before retention. Repeating the command
returns the exact retained result without rerunning the graph. This selected
evidence cannot issue or replace full-suite qualification, and no Git mutation
occurs.

`rey workloads verify-activation <execution-id>` independently revalidates the
same frozen Git, workload, contract, and capability inputs before executing
the complete declared scenario suite. Its bounded retained proof compares
each selected scenario result exactly with the full result counterpart. An
equivalent proof establishes deterministic parity for that frozen activation
fixture; a different proof remains conclusive evidence rather than being
collapsed into runtime failure. Neither assessment qualifies the workload or
mutates Git, and proof replay does not execute scenarios again.

Distinct proposals from the same retained transition may safely coalesce at
execution. Rey reuses only a directly evaluated result with identical Git
source/target, workload HEAD, contracts, scenario selection, and capability
snapshot, and only when its measured evidence fits the receiving admission's
budget. The receiving execution retains its own activation/admission identity
and the exact source execution id. Incompatible proposals and stricter budgets
cannot inherit the result. This is deterministic same-transition work reuse,
not cross-poll debounce or evidence loss.

`rey.git-activation-trigger.v1` currently selects repository/worktree, event
classes, optional exact `HEAD` or watched-ref names, optional reversible
raw-byte path prefixes for path events, completeness posture, exact
workload/graph/scenarios, and action, scenario, and evidence budgets.
Reachability and path events use the same exact ref selection. A path proposal
retains each exact matched ref, path identity, and change direction; only
omissions from selected evidence enter the proposal. Supported semantic-index
transitions are complete and can satisfy a completeness-requiring trigger. If
an unknown persistent flag, traversal or path bound, shallow history, or
missing object makes selected evidence incomplete, it matches only when a
trigger explicitly permits incomplete evidence and the proposal retains the
omission. HEAD and watched-ref ancestry, reachability, and path completeness
remain independent from the index axis.

Workloads can therefore activate narrowly. An index delta touching Rust
sources might select symbol and diagnostic scenarios, while a new commit on a
release ref might run a broader conformance workload. Unrelated workload graph
entry points remain idle.

## Cross-Project Activation

Git activation can give public-contract conformance a concrete clock without
creating a package dependency:

- an external commit can activate declared compatibility lenses;
- the resulting delta can identify a closed gap or a new missing capability;
- a Rey commit can activate standalone self-checks; and
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
- Local watch iterations, interval, elapsed cadence scheduling, retained tick
  count, retained receipt count, and state bytes are bounded; reaching one is
  a stop or explicit error, never convergence.
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
- retained quiet/failed cadence ticks, iteration/time/retry/cancellation stops,
  recovered partial failure, changed-stop atomicity, interrupted receipt gaps,
  and cadence-state tampering; and
- proof that polling executes no repository hooks or mutation commands.
