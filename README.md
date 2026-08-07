# Rey

Rey is an environment-aware, diff-directed compute runtime for agents. It
probes the context surfaces available where it is running, turns observations
of a large state space into typed DataFrames, computes explicit deltas between
revisions, and uses those deltas to direct the next probe or mutation.

Rey works without [Spoke](../spoke). In standalone mode it can inspect an
explicit local workspace and use discovered tools with narrower durability,
query, and execution guarantees. When Spoke is available, it becomes Rey's
durable reasoning plane: versioned content, composed query, admitted compute,
and durable lineage amplify the same runtime rather than replace it.

Rey is also Spoke's first external runtime application. Rey exercises Spoke as
a real client, uses Spoke to explore and improve both projects, and turns the
resulting gaps into evidence that directs the next Spoke capability.

Rey is currently a pre-alpha foundation repository. It contains the initial
contracts and a pinned Rust development environment, but no runtime code. No
documented command, crate, file format, or Spoke integration should be treated
as implemented until the repository contains corresponding code and tests.

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
          ranked frontier                     scoped proof
               │                             and lineage
               └──────────── next action ────────────┘
```

The diff is not merely a report produced after work is finished. It is a
control signal. Changed rows invalidate dependent observations, unresolved
differences form a frontier, and the runtime spends its next unit of attention
or compute on that frontier.

This makes high-dimensional spaces tractable without pretending they fit in
one prompt. A codebase, for example, can be observed as related frames for
files, symbols, references, dependency edges, tests, diagnostics, ownership,
runtime behavior, and proposed changes. An agent reasons over the small set of
meaningful deltas while exact source content remains addressable through the
best source binding the environment can prove.

## The Model

Rey begins with twelve concepts:

- A **capability snapshot** is a bounded inventory of the context surfaces,
  tools, providers, trust classes, and limits Rey can currently use.
- An **application** is a versioned composition of spaces, lenses, triggers,
  components, actions, claims, policy, and total budgets.
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
- A **frontier** is the bounded set of unresolved deltas eligible to direct the
  next computation.
- An **activation** is an idempotent match between a declared source-delta
  trigger and one independently runnable application component.
- A **policy** proposes the next action from a frontier. A policy may be an
  agent, a deterministic rule, or a human; it does not bypass runtime
  admission.
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

Triggers can map those deltas to specific application components. A staged Rust
change might refresh only symbol and diagnostic lenses; a new commit on a
watched ref might activate a broader parity or deployment proof.

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
activate the relevant standalone or connected Rey conformance components.

## Project Boundaries

Rey is not:

- a model server or an opinionated model provider;
- a replacement for Spoke storage, query, documents, or compute;
- a hard dependency on Spoke for useful local exploration;
- an ambient shell that executes every binary found on `PATH`;
- a general workflow engine whose tasks have no diff semantics;
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
- [Environment and Capabilities](docs/ENVIRONMENT.md) — local/remote discovery,
  zero-Spoke behavior, and provider guarantees.
- [Git Context and Activation](docs/GIT.md) — commit/ref/index polling and
  delta-triggered application components.
- [Diffs and Frames](docs/DIFFS.md) — typed frame and delta semantics.
- [Proofs and Evidence](docs/PROOFS.md) — claim, evidence, certificate, and
  staleness rules.
- [Interfaces](docs/INTERFACES.md) — provisional CLI, provider, trigger, and
  Spoke contracts.
- [Development](docs/DEVELOPMENT.md) — current repository truth and target task
  surface.
- [Roadmap](docs/ROADMAP.md) — delivery sequence.
- [Plans](plans/README.md) — active, checkable implementation work.
- [Architecture Decisions](docs/decisions/README.md) — accepted choices that
  constrain implementation.

## Current Status

The repository currently contains foundational documents, a pinned Nix Rust
development shell, and an honest root task surface. It has no Cargo workspace
or runtime yet. The active foundation plan covers environment discovery, a
Git snapshot-to-activation slice, a zero-Spoke Frame-to-Delta-to-Proof slice,
and the same evidence flow amplified through routed Spoke integration. See
[Plan 0001](plans/0001-foundation.md).
