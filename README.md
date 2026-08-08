# Rey

Rey is an environment-aware, diff-directed compute runtime for agents. It
probes the context surfaces available where it is running, turns observations
of a large state space into typed DataFrames, computes explicit deltas between
revisions, and uses those deltas to direct the next probe or mutation.

Users organize that compute as workloads: versioned generated compute graphs
qualified by exact scenarios whose failing expected-to-observed deltas direct
the next bounded graph revision.

Rey works without [Spoke](../spoke). In standalone mode it can inspect an
explicit local workspace and use discovered tools with narrower durability,
query, and execution guarantees. When Spoke is available, it becomes Rey's
durable reasoning plane: versioned content, composed query, admitted compute,
and durable lineage amplify the same runtime rather than replace it.

Rey is also Spoke's first external runtime application. Rey exercises Spoke as
a real client, uses Spoke to explore and improve both projects, and turns the
resulting gaps into evidence that directs the next Spoke capability.

Rey is currently pre-alpha. Its executable standalone slice can inspect an
explicit local workspace, probe the allowlisted `git` and `rg` tools under hard
process bounds, and project a contained Git repository observation into a
typed capability frame. It can compare two verified snapshots, emit structured
JSON, typed Arrow, summary, or Tabular Diff 0.8 output, and evaluate and verify
a required-capability certificate. The same proof can be published as a
bounded content-addressed local bundle with explicit filesystem-only retention
guarantees. Pure library contracts now formalize the runtime transition state
machine, canonical frontier and progress relations, deterministic bounded work
selection, and the delta-directed reasoning surface. They run with zero Spoke.
The accepted next product contract makes a workload the public composition of
a generated compute graph, scenarios, policy, claims, and bounds, with
`workloads list`, `test`, `run`, and `status` as its command surface. The first
executable slice now ships two built-in deterministic text workloads, typed
scenario deltas, exact qualification, and bounded local result state.
Workload-specific frontier derivation, generated graph revision, recurring
scheduling, provider retrieval/execution, activation, mutation, Spoke-backed
proof bundles, and connected Spoke behavior remain design contracts.

## Try The First Slice

```sh
nix develop
just setup
just rey environment inspect --format table
just rey workloads list --format table
just rey workloads test rey.fixture.text-normalize --format table
just rey workloads run rey.fixture.text-normalize --input ' rey ' --format table
```

Environment frame commands support bounded JSON documents and Arrow IPC;
environment `auto` selects a table on a terminal and Arrow when redirected.
Workload commands support table or JSON, with redirected `auto` selecting
JSON. Both command groups use `.` as their workspace unless `--workspace` is
supplied.

The next loop can be exercised entirely through files selected by the caller:

```sh
just rey environment inspect --format json > baseline.json
just rey environment inspect --format json > candidate.json
just rey environment diff baseline.json candidate.json
just rey environment diff baseline.json candidate.json \
  --diff-format tabular-diff
just rey environment prove baseline.json candidate.json \
  --require-capability frame.arrow-stream > certificate.json
just rey environment verify certificate.json baseline.json candidate.json
just rey environment prove baseline.json candidate.json \
  --require-capability frame.arrow-stream \
  --bundle proof.bundle > bundled-certificate.json
just rey environment verify-bundle proof.bundle
```

Snapshot inputs are bounded and their schema, canonical order, composite-key
uniqueness, completeness, and semantic digest are recomputed before use. Proof
commands return `0` for passed/verified, `2` for failed, `3` for inconclusive,
and `4` for stale; invalid input or runtime failure returns `1`.

Workload state defaults to `.rey/workloads/state.json` below the selected
workspace. `workloads list` and `status` are read-only. `test` returns `2` for
the deliberately failing `rey.fixture.text-mismatch` workload and preserves
its exact expected-to-observed delta; `run` returns `3` until the selected
graph has a fresh passing qualification. Redirected workload output is JSON.

## The Idea

Most agent runtimes repeatedly present a model with a large snapshot and ask it
what to do next. Rey makes the change between snapshots the central runtime
value:

```text
                  environment context surfaces
             files · VCS · tools · runtimes · optional Spoke
                                  │
                    bounded capability discovery
                                  │
                     lenses and explicit probes
                                  │
                           typed Frame set n
                                  │
                  activation or policy proposes action
                                  │
                       probe or admitted mutation
                                  │
                         typed Frame set n + 1
                                  │
                     typed Delta(Frame n, Frame n+1)
                                  │
               ┌──────────────────┴──────────────────┐
               ▼                                     ▼
          ranked frontier                     scoped proof
               │                             and lineage
               ▼
      bounded work selection
               │
               ▼
       retrieve exact evidence
               │
       bounded reasoning surface
               │
               ▼
      next probe or admitted mutation
```

The diff is not merely a report produced after work is finished. It is a
control signal. Changed rows invalidate dependent observations, unresolved
differences form a frontier, and the runtime spends its next unit of attention
or compute on that frontier.

Before policy proposes that next unit, Rey retrieves exact relevant evidence
through the available providers and projects a bounded, typed reasoning
surface. Retrieval is directed by unresolved deltas rather than ambient
workspace size. Post-action observation then measures whether the residual
work actually decreased; process completion alone is not progress.

This makes high-dimensional spaces tractable without pretending they fit in
one prompt. A codebase, for example, can be observed as related frames for
files, symbols, references, dependency edges, tests, diagnostics, ownership,
runtime behavior, and proposed changes. An agent reasons over the small set of
meaningful deltas while exact source content remains addressable through the
best source binding the environment can prove.

## The Model

Rey's public composition is a workload; its runtime uses these concepts:

- A **capability snapshot** is a bounded inventory of the context surfaces,
  tools, providers, trust classes, and limits Rey can currently use.
- A **workload** is a versioned composition of a compute-graph contract,
  scenarios, environment requirements, actions, claims, policy, qualification,
  and total budgets.
- A **compute graph** is one immutable proposal of typed nodes, ports, and
  dependency edges that implements a workload. An agent, rule, or human may
  propose it; the runtime validates it.
- A **scenario** binds exact fixtures and expected outputs or claims to one
  workload. Failing scenarios retain directed typed deltas from `EXPECTED` to
  `OBSERVED`.
- A **space** names the domain being explored and the exact local or Spoke
  sources, revisions, and capability requirements that bound it.
- A **lens** is a versioned, deterministic observation definition. It projects
  part of a space into a typed frame.
- A **frame** is a bounded Polars DataFrame plus schema, identity, source
  bindings, provenance, and evaluation limits.
- An **action** is either a read-only probe or an explicitly admitted mutation.
  It always names frozen inputs and an effect class.
- A **delta** is the typed, directed difference between compatible frames. It
  retains keys, types, comparison rules, and both source revisions.
- A **frontier** is the bounded relation of unresolved delta- or claim-directed
  work eligible to direct the next computation.
- An **activation** is an idempotent match between a declared source delta and
  a workload graph entry point or test selection.
- A **policy** proposes the next graph revision or action from a frontier. A
  policy may be an agent, a deterministic rule, or a human; it does not bypass
  runtime admission.
- A **trace** is the replayable graph of capabilities, observations, actions,
  local or Spoke runs, deltas, and decisions.
- A **proof** evaluates a declared claim over a named scope and binds the
  result to exact inputs, lenses, tools, limits, coverage, and evidence.

DataFrames are the canonical local shape for typed collections, not a new
durable storage layer and not a reason to flatten every value into a table.
Source bytes remain source bytes. Ordered text, structured documents, and
binary artifacts may retain native representations while frames carry their
identities, metadata, and relationships.

## Diff And Proof

Rey keeps a typed delta representation as the authoritative comparison result.
For compatible tabular frames it can project that result into the
[Frictionless Data Tabular Diff Format 0.8](https://specs.frictionlessdata.io/tabular-diff/),
where the difference between two tables is itself a table. Tabular Diff is a
portable human and interchange view; it is not allowed to erase typed values,
source labels, key definitions, or comparison semantics from the underlying
delta.

A zero diff is not an unqualified proof. It proves agreement only for the
declared sources, lenses, fixtures, keys, normalizers, limits, and coverage.
Every proof must expose that scope, distinguish `failed` from `inconclusive`,
and become `stale` when a bound input or evaluator changes.

See [Diffs and Frames](docs/DIFFS.md) and
[Proofs and Evidence](docs/PROOFS.md) for the target contracts.
See [Workloads, Compute Graphs, and Scenarios](docs/WORKLOADS.md) for the
accepted public model, qualification loop, progress semantics, and four-command
CLI.
See [Runtime Transitions and Reasoning Surfaces](docs/RUNTIME.md) for the
implemented pure lifecycle and policy-surface contracts and their explicit
non-goals.

## Environment Awareness

Rey treats its environment as an explicit, changing input. A bounded discovery
pass can identify useful context surfaces such as a workspace filesystem,
version control, `rg`, language toolchains, compilers, test runners, language
servers, and a reachable Spoke deployment.

Discovery does not grant ambient authority or run arbitrary binaries. Each
provider declares how it is detected, which read-only probes are allowed, how
identity and version are established, which actions it supports, its trust
class, and its resource limits. The resulting capability snapshot is a typed
frame and participates in trace and proof identity.

Rey has three deployment attitudes rather than two different runtimes:

- **standalone** uses built-in and explicitly discovered local capabilities;
- **connected** adds the capabilities advertised by a reachable Spoke; and
- **required-capability** fails before work if a space or claim needs a
  capability that the current snapshot cannot provide.

Missing Spoke never silently turns a Spoke-backed durability or proof claim
into a local one. The action becomes unavailable or the claim becomes
inconclusive with the missing capability named.

See [Environment and Capabilities](docs/ENVIRONMENT.md) for the provider,
snapshot, profile, admission, and degradation contracts.

## Git-Native Activation

For software projects, Git is both an exact source-binding provider and a
natural polling surface. Rey observes the commit graph, refs, HEAD, semantic
index entries, and optionally bounded worktree status as typed frames.

The useful development deltas are already present:

- commit to commit;
- previous ref target to current ref target;
- `HEAD` tree to index for staged changes;
- index to worktree for unstaged changes; and
- previous semantic index snapshot to current index snapshot.

Triggers can map those deltas to specific workload graph entry points or
scenario selections. A staged Rust change might refresh only symbol and
diagnostic observations; a new commit on a watched ref might activate a
broader parity workload.

Git is not treated as a monotonic event queue. Rebase, reset, force-push,
branch switching, conflicts, shallow history, and multiple worktrees produce
explicit states and deltas. Poll cursors advance only after transition evidence
commits, and activation replay is idempotent rather than claimed exactly once.

See [Git Context and Activation](docs/GIT.md) for repository identity, semantic
index, polling, ref movement, trigger, and safety contracts.

## Spoke Integration And Feedback

Rey does not duplicate Spoke's storage, query, or execution responsibilities:

- Rey binds frames to exact Spoke file, object, document, stream, table, query,
  tool, and run revisions where those capabilities apply.
- Read-only observation uses safe, bounded Spoke reads and `QUERY` operations.
- Effects use explicit Spoke resource mutations or admitted compute runs;
  observation never hides a mutation.
- In connected mode Rey records durable traces, frame artifacts, deltas, and
  proofs through public Spoke contracts rather than reaching into Spoke storage
  internals.
- Spoke compute owns process admission, attempts, fencing, limits, captures,
  and process lineage. Rey owns why a computation was selected and how its
  observations change the frontier.

Standalone execution is not a private Spoke bypass: it is an explicit provider
with weaker, disclosed guarantees. When Rey connects to Spoke, it uses Spoke's
public HTTP surface. In-process shortcuts are not the reference integration
even when both projects run on one machine.

The relationship is deliberately recursive without becoming a dependency
cycle:

```text
Rey probes Spoke ──► finds gaps and emits proof ──► improves Spoke
     ▲                                                │
     └──── discovers new query/compute capabilities ◄─┘
```

Rey must be able to inspect and help repair Spoke while Spoke is absent, broken,
or under development. Spoke must be able to build and start without Rey. Their
shared progress is driven by public-contract conformance evidence, not by one
repository importing the other's internals.

Git makes that loop concrete: commits and index deltas in either checkout can
activate the relevant standalone or connected Rey conformance workloads.

## Project Boundaries

Rey is not:

- a model server or an opinionated model provider;
- a replacement for Spoke storage, query, documents, or compute;
- a hard dependency on Spoke for useful local exploration;
- an ambient shell that executes every binary found on `PATH`;
- a general workflow engine whose tasks have no diff or scenario semantics;
- a claim that every artifact is naturally tabular;
- an authority that lets an agent mutate a target without explicit admission;
- a source-code coverage system merely because a scenario set passes; or
- a proof system that hides omitted observations, unsupported controls, or
  exhausted budgets.

## Repository Guide

- [Constitution](CONSTITUTION.md) — durable values and invariants.
- [Contributor Instructions](INSTRUCTIONS.md) — working loop and verification
  rules.
- [Architecture](docs/ARCHITECTURE.md) — component ownership and data flow.
- [Workloads, Compute Graphs, and Scenarios](docs/WORKLOADS.md) — public unit,
  graph/scenario contracts, qualification, progress, and command semantics.
- [Runtime Transitions and Reasoning Surfaces](docs/RUNTIME.md) — lifecycle,
  delta roles, surface bounds, and current contract truth.
- [Frontier, Progress, and Scheduling](docs/FRONTIER.md) — canonical work,
  directional progress, stale guards, and deterministic bounded selection.
- [Environment and Capabilities](docs/ENVIRONMENT.md) — local/remote discovery,
  zero-Spoke behavior, and provider guarantees.
- [Git Context and Activation](docs/GIT.md) — commit/ref/index polling and
  delta-triggered workload entry points.
- [Diffs and Frames](docs/DIFFS.md) — typed frame and delta semantics.
- [Proofs and Evidence](docs/PROOFS.md) — claim, evidence, certificate, and
  staleness rules.
- [Interfaces](docs/INTERFACES.md) — provisional CLI, provider, trigger, and
  Spoke contracts plus the accepted workload command surface.
- [Development](docs/DEVELOPMENT.md) — current repository truth and target task
  surface.
- [Roadmap](docs/ROADMAP.md) — delivery sequence.
- [Plans](plans/README.md) — active, checkable implementation work.
- [Architecture Decisions](docs/decisions/README.md) — accepted choices that
  constrain implementation.

## Current Status

The repository now contains a ten-crate Rust workspace and a zero-Spoke
capability Snapshot-to-Delta-to-Certificate loop. `rey environment inspect`
emits capability JSON/table/Arrow; `diff` emits typed structured JSON or Arrow,
summary JSON, and Tabular Diff 0.8 CSV; `prove` and `verify` bind required
capabilities to exact snapshots, comparator/evaluator contracts, and limits.
`prove --bundle` retains the snapshots, structured and Arrow delta, Tabular
Diff, certificate, and an explicit local retention manifest;
`verify-bundle` recomputes and bounds that evidence without trusting stored
status.
Bounded process supervision and a read-only partial Git repository/index
snapshot are also implemented. The Git index digest deliberately reports
incomplete flag semantics and is not yet an activation cursor. Pure contract
crates now reject illegal runtime transitions, derive convergence only from
complete frontier evidence, compare frontier progress, select bounded ready
work deterministically, and bind that decision into the reasoning surface.
They do not derive domain work, retrieve providers, or execute actions. The
first workload slice implements bounded built-in compute graphs, typed
scenario deltas, exact qualification, a verified local result index, and all
four workload commands. Frontier/progress/scheduling v2 and reasoning-surface
v3 now bind workload, graph, scenario-suite, and campaign identities. The
active foundation plan covers complete Git polling and routed Spoke proof work.
See [Plan 0001](plans/0001-foundation.md),
[Plan 0002](plans/0002-runtime-contracts.md), and
[Plan 0003](plans/0003-frontier-scheduling.md). The completed design bearing is
[Plan 0004](plans/0004-workload-contracts.md); the active executable slice is
[Plan 0005](plans/0005-first-workload-slice.md).
