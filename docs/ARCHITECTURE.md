# Rey Architecture

This document defines Rey's target ownership boundaries and data flow. It is an
implementation baseline, not a claim that the components already exist.

## Purpose

Rey is an environment-aware, deterministic diff-directed compute runtime. It
first inventories the bounded context surfaces and useful tools available where
it is running. It then observes a space through versioned lenses, represents
typed collections as DataFrames, computes directed deltas, and uses unresolved
deltas to schedule subsequent probes or mutations. It records enough lineage
to explain why each action ran and to evaluate scoped proof claims over the
resulting observations.

Rey has a useful standalone profile over explicit local context. Spoke is an
optional capability amplifier that supplies durable content, exact resource
revisions, composed query, admitted execution, and durable lineage. A model may
guide the feedback loop, but model inference and provider integration are
policy concerns rather than runtime correctness dependencies.

## Architectural Separation

Rey separates six responsibilities:

1. **Context-surface plane** — environment providers expose explicit local and
   remote sources, tools, runtimes, and guarantees as a capability snapshot.
2. **Reasoning plane** — built-in/local providers supply a minimum standalone
   surface; optional Spoke supplies durable sources, composed query, compute
   coordination, captures, and durable lineage.
3. **Observation plane** — lenses bind exact inputs and materialize bounded
   typed frames.
4. **Delta plane** — comparison aligns frames, preserves typed changes, and
   derives invalidation.
5. **Runtime plane** — transitions validate proposals, execute bounded probes or
   effects, update the frontier, and stop on convergence or an explicit bound.
6. **Policy plane** — an agent, deterministic rule, or human proposes which
   admissible action should happen next.

These are responsibility boundaries, not requirements for separate processes.
The first topology is a local Rey process. A Spoke provider, when configured or
discovered, uses Spoke's routed HTTP interface.

## System Graph

```text
               explicit environment boundary
                              │
           ┌──────────────────┼───────────────────┐
           ▼                  ▼                   ▼
    local workspace     discovered tools    optional Spoke
           └──────────────────┬───────────────────┘
                              ▼
                   capability snapshot frame
                              │
              Git/source activation · agent · rule · human policy
                              │ proposal
                              ▼
                    ┌───────────────────┐
                    │ Rey runtime       │
                    │ admit · budget    │
                    │ transition · stop │
                    └───┬───────────┬───┘
                        │           │
               observe │           │ act through provider
                        ▼           ▼
              ┌──────────────┐  ┌──────────────┐
              │ frame/lens   │  │ local action │
              │ materializer │  │ or Spoke run │
              └──────┬───────┘  └──────┬───────┘
                     │                 │
                     └────────┬────────┘
                              ▼
                    ┌───────────────────┐
                    │ typed delta       │
                    │ invalidation      │
                    │ frontier          │
                    └───────┬───────────┘
                            │
                   ┌────────┴───────────────┐
                   ▼                        ▼
          retrieve · project          proof evaluator
                   │                        │
                   ▼                        ▼
          reasoning surface    local or Spoke-backed evidence
                   │
                   ▼
             next proposal
```

See [Environment and Capabilities](ENVIRONMENT.md) for detailed provider,
snapshot, profile, admission, and degradation contracts.
See [Git Context and Activation](GIT.md) for software-repository snapshots,
poll cursors, and delta-triggered components.

## Core Data Model

| Concept | Meaning | Owner or retention boundary |
| --- | --- | --- |
| Environment | Explicit boundary from which providers may discover context | Host/deployment configuration; observed by Rey |
| Capability snapshot | Frozen inventory of providers, tools, operations, trust, and limits | Rey evidence; local or Spoke-backed |
| Application | Versioned composition of spaces, components, triggers, actions, claims, policy, and budgets | Rey declaration |
| Space | Named boundary over sources, lenses, actions, claims, and limits | Rey declaration; local or stored through Spoke |
| Source binding | Exact Spoke or local immutable input identity | Source system; referenced by Rey |
| Lens | Versioned deterministic observation definition | Rey declaration; local or stored through Spoke |
| Frame | Bounded typed observation plus schema and lineage | Working state in Rey; local or Spoke evidence when retained |
| Action proposal | Policy request naming frozen inputs, effect class, and bounds | Rey trace |
| Run/attempt | Provider-owned execution and capture lineage | Local executor or Spoke compute, explicitly distinguished |
| Delta | Directed typed comparison between compatible frames | Rey evidence; local or Spoke-backed |
| Frontier | Bounded prioritized unresolved work | Rey working state; checkpointed when needed |
| Trigger | Versioned predicate mapping a source delta to application components | Rey declaration |
| Activation | Idempotent trigger match against exact source/target snapshots | Rey transition evidence |
| Claim | Predicate and required evidence over a named scope | Rey declaration; local or stored through Spoke |
| Proof | Claim assessment bound to exact evidence and evaluator inputs | Rey artifact with explicit provider guarantees |
| Trace | Graph connecting the concepts above | Local artifacts or Spoke events/artifacts |

Working DataFrames and queues are never the only durable copy of authored
content. A frame may be reproducible from exact sources and a lens, or retained
as an Arrow evidence artifact when replay cost, external volatility, or proof
requirements demand it.

## Applications And Components

A Rey application declares providers, spaces, lenses, independently activatable
components, triggers, admissible actions, claims, policy, dependency edges, and
total budgets under one versioned identity.

A component is the smallest unit a trigger can start or resume. It names exact
input frames/lenses, required capabilities, produced observations, evaluated
claims, permitted actions, component-local budgets, and concurrency behavior.
Components do not imply separate processes. Manual, policy-selected, Git, or
future stream activations all enter the same component admission contract.

Application and component revisions participate in activation, transition, and
proof identity. Changing a component does not silently reinterpret a retained
activation created for an earlier revision.

## Environment And Capability Discovery

Environment awareness is provided through narrow providers rather than an
unbounded host scan. Initial provider classes may include:

- built-in Rey functions that require no external executable;
- an explicit local workspace with bounded filesystem access;
- a Git provider with commit/ref/index/worktree frames and polling;
- known developer tools resolved from configured paths or `PATH`;
- language-specific toolchains, analyzers, build systems, and test runners; and
- a Spoke endpoint with discovered public capabilities.

Each provider has stable identity, version, detection rules, trust class,
source/effect capabilities, supported enforcement, and probe limits. Discovery
may use narrowly defined read-only operations such as executable resolution,
metadata inspection, or a bounded `--version` invocation. It never executes an
unknown file merely because it exists.

The capability snapshot is a typed relation. A first schema should be able to
represent provider id/revision, capability id, kind, resolved location,
version, content or provenance digest when available, availability, trust,
supported operations, enforcement claims, observation time, and errors.

Capability discovery is repeatable during a trace. A delta between snapshots
can invalidate actions, lenses, and proofs. An executable path, version, digest,
provider health, or Spoke capability change is part of runtime state rather
than ambient trivia.

### Operating Profiles

- **Standalone** requires only built-in capabilities and an explicitly selected
  local context. Evidence may be retained in a caller-selected local artifact
  directory, but Rey makes no restart, transactional, multi-process, or remote
  durability claim beyond the underlying filesystem and content digests.
- **Connected** adds Spoke capabilities to the same snapshot. Exact Spoke
  versions, query, compute, and persistence can satisfy stronger claims.
- **Required-capability** is a per-space or per-claim constraint, not a separate
  runtime. Admission fails early when the snapshot lacks a named capability or
  guarantee.

Automatic mode may discover Spoke, but it cannot silently change a claim's
required guarantees. A proof states its profile and provider set.

## Observation

A lens contains:

- stable identity and revision;
- expected logical schema and key semantics;
- exact or resolvable source requirements;
- a pure query, projection, or probe definition;
- normalizer identities;
- row, byte, time, memory, and traversal bounds; and
- completeness rules that decide whether truncated or unavailable input is
  usable, inconclusive, or an error.

Materialization resolves every mutable source name to the strongest exact
identity its provider can establish before evaluation. The frame records those
bindings, the capability snapshot, and relevant local tool or Spoke revisions,
query checkpoints, request ids, run ids, capture digests, and other lineage.

A lens may consume another frame. Those dependencies form the invalidation
graph. A source or upstream-frame delta marks only affected dependent lenses
eligible for re-evaluation; it does not imply that every eligible lens must run
immediately.

## Git Provider And Activation

Git is a specialized environment provider for software spaces. It observes the
object database, commit graph, refs, per-worktree HEAD and index, and optionally
bounded worktree status. It produces typed repository, ref, commit, parent,
path-change, index-entry, status, and activation relations.

A poll compares the current repository snapshot with its last completely
processed cursor. Fast-forward refs can expose newly reachable commits; rewinds
and divergence emit explicit ref/reachability deltas; semantic index changes
expose staged proposals before a commit exists. Raw index changes caused only
by stat-cache refresh do not activate staged-content components.

Triggers select delta subsets and name affected application components. An
activation has deterministic identity over trigger revision, component
revision, source/target snapshots, and matched delta. It enters ordinary action
admission and can be replayed after a crash. The poll cursor advances only after
required transition evidence reaches its claimed retention boundary.

## Delta And Frontier

The delta engine compares compatible frames under explicit direction, keys,
ordering, and normalizers. The result contains structured schema, row, and cell
changes with typed before/after values. Human tables, Tabular Diff CSV, JSON,
and Arrow are representations of this semantic result.

The frontier is a relation derived from deltas and claims. A frontier row can
name the affected entity, delta, violated claim, dependent lenses, admissible
actions, priority inputs, and estimated cost. Prioritization is versioned policy
and must not alter the underlying delta.

The initial runtime may recompute complete bounded frames. Incremental physical
execution is accepted only when it produces the same semantic frame and delta
as full recomputation for the declared contract.

## Runtime Lifecycle And Orientation

Rey separates initial observation from recurring transitions. Bootstrap
discovers capabilities, materializes declared initial observations, and
establishes baselines or evaluates claims without inventing a prior observation
or artificial transition delta. The steady-state loop begins only from a
committed transition and its derived frontier:

```text
committed delta/frontier
  -> retrieve -> project
  -> propose -> admit -> probe|mutate
  -> observe -> compare -> evaluate
  -> commit transition
  -> next delta/frontier
```

A transition delta states what changed from the relevant pre-action frame to
the post-action frame. A residual delta states what remains between a declared
expected or baseline frame and the current observation. Claims that do not
reduce naturally to one frame comparison remain typed claim facts; the
frontier combines them with applicable residual deltas, invalidation, and
dependencies rather than flattening them into an artificial mega-delta.

Orientation is the bounded inner loop that turns a committed frontier into a
reasoning surface. Rey identifies evidence needs from frontier rows, retrieves
exact read-only evidence through the provider that owns it, and applies a
versioned deterministic projection. Retrieval does not grant new execution
authority or duplicate Spoke query and storage ownership. Any read that
observes mutable state, invokes a tool, or creates a new lens result is an
explicit probe transition.

Retrieve and project may repeat inside one orientation phase as exact evidence
changes the surface. The runtime owns iteration, time, byte, and provider
bounds plus lineage; a versioned orientation strategy owns evidence order and
readiness. The phase stops as ready, without eligible evidence, or at an
explicit bound. Expected information value is navigation metadata, while
actual progress remains a post-action assessment.

The reasoning surface binds its frontier and delta inputs, exact source and
capability revisions, retrieved evidence identities, projection contract,
omissions, truncation, and effective limits. It may contain bounded typed
relations and handles to native artifacts. It is policy input and trace
evidence, not a replacement source or a durable content store.

After the action, Rey compares the next residual/frontier state with the prior
one. Progress is a typed assessment of resolved, introduced, reopened,
unchanged, or incomparable work; information and completeness gained; changed
guarantees; and cost consumed. A policy may use an explicit versioned ranking
objective, but no scalar progress score replaces the authoritative deltas or
proof status.

Provider execution, semantic transition, and proof/evidence state remain
orthogonal. A process can terminate successfully while semantic work is
unchanged or regresses, and retained evidence can later become stale. Budget
exhaustion, missing evidence, or incompatible residuals stop explicitly rather
than producing convergence. See
[ADR 0012](decisions/0012-delta-directed-orientation.md).

## Actions And Transitions

An action has one of two effect classes:

- **probe** — read-only computation that may produce new observations or
  derived artifacts; or
- **mutation** — an explicit change to a declared target through a Spoke
  resource method, admitted Spoke compute run, or explicitly authorized local
  provider action.

One transition follows this protocol after a committed frontier is available:

1. freeze the current activation/frontier, source revisions, and relevant frame
   ids;
2. retrieve declared exact read-only evidence and project the bounded reasoning
   surface, recording omissions and effective limits;
3. receive a policy proposal citing that surface and frontier evidence;
4. validate action identity, capability snapshot, allowed effect,
   preconditions, and remaining budget;
5. submit or perform the action through its owning contract;
6. retain action, run, attempt, output, and failure lineage;
7. materialize affected post-action lenses;
8. compute transition and applicable residual deltas;
9. evaluate claims, progress, and the next frontier; and
10. commit the transition record before selecting another action.

An action can complete successfully while its transition fails semantically.
For example, a compiler process may exit zero while the dependency graph or
test evidence still differs from the claim. Post-action observations, not exit
status, determine convergence.

## Policy Boundary

A policy receives a bounded view of the current space, frontier, admissible
actions, and budgets. It returns a structured proposal. The runtime treats that
proposal as untrusted input and validates it identically whether it came from a
model, a rule, or a human.

Provider credentials, prompt construction, inference retries, and model context
management do not belong in core diff or proof crates. A provider adapter may
exist later behind the policy contract.

## Spoke Boundary

The optional Spoke provider integrates through versioned service contracts:

- Files and Objects provide exact named content versions.
- Documents provide exact source bindings and derived text/chunk observations.
- Streams provide ordered trace events and projection inputs where useful.
- Tables and `QUERY /db` provide typed relational observations.
- Graph, lexical, and vector operations remain part of Spoke's composed query
  model as they become available.
- Compute resolves registered tools, admits bounded runs, owns attempts and
  fences, and exposes immutable captures.

Rey never opens Spoke's data directory, imports a capability implementation to
bypass HTTP, interprets a host path as a Spoke path, or advances a Spoke-owned
checkpoint directly. A future optimized internal contract must preserve the
same identity, authorization, consistency, and failure semantics as the routed
surface.

In connected mode, Rey's durable artifacts should use ordinary Spoke resources
and explicit media types. The initial mapping of spaces, lens declarations,
traces, frames, deltas, and proofs onto files, objects, streams, or tables
remains an open implementation decision in Plan 0001.

Spoke absence is not an error unless a requested action or claim requires it.
Standalone local providers are not permitted to mint Spoke resource ids,
simulate Spoke revision/checkpoint metadata, or claim Spoke durability.

## Rey–Spoke Recursive Improvement Loop

Rey is the first external application expected to exercise Spoke's runtime as a
real client. That creates a deliberate feedback loop:

1. standalone Rey can inspect the Spoke repository, run available development
   tools, and describe gaps even when Spoke cannot start;
2. connected Rey exercises public Spoke query, compute, persistence, and
   lineage contracts against real exploration workloads;
3. Rey records missing capabilities, incompatible schemas, excessive friction,
   failures, and parity gaps as typed evidence;
4. that evidence directs a change in Spoke or its public contract;
5. Rey's next capability probe discovers the new Spoke behavior; and
6. the same fixture proves whether the gap closed and what new frontier emerged.

The loop is recursive at the product and evidence level, not the package graph.
Rey never imports Spoke capability internals, and Spoke does not require Rey for
core startup. Shared fixtures may describe a public contract, but each
repository owns and can run its side of the conformance test independently.
Git commit/ref/index deltas provide a natural activation source for these
conformance components without changing that ownership boundary.

## Codebase Space Example

A codebase explorer might define these frames:

```text
files(path, digest, language, bytes, generated)
symbols(symbol_id, path, kind, name, visibility, span)
references(source_symbol_id, target_symbol_id, kind)
dependencies(source_unit, target_unit, kind, scope)
diagnostics(tool, path, span, severity, code, message)
tests(test_id, target, status, duration_ms, output_digest)
changes(path, before_digest, after_digest, change_kind)
claims(claim_id, entity_id, status, evidence_id)
```

Source text remains versioned content rather than a giant cell. Relations carry
stable identities and spans back to that content. A mutation to one file can
invalidate symbol, dependency, diagnostic, and test lenses; the resulting
deltas identify the smaller frontier that should direct the next action.

## Target Crate Ownership

The first design proposes these Rust ownership boundaries:

| Crate | Ownership |
| --- | --- |
| `rey` | CLI, configuration, local composition, and user-facing orchestration |
| `rey-core` | identities, revisions, limits, statuses, and shared value contracts |
| `rey-dataframe` | frame metadata, Polars schemas, Arrow codecs, and bounded rendering |
| `rey-environment` | capability discovery, snapshots, provider contracts, and local context adapters |
| `rey-git` | repository identity, commit/ref/index frames, polling cursors, triggers, and activations |
| `rey-diff` | compatibility, alignment, typed changes, summaries, and Tabular Diff projection |
| `rey-runtime` | spaces, lenses, actions, transitions, budgets, cancellation, and trace assembly |
| `rey-frontier` | dependency invalidation, prioritization inputs, and convergence evaluation |
| `rey-proof` | claims, evidence manifests, certificates, verification, and staleness |
| `rey-policy` | provider-neutral proposal and admissible-action contracts |
| `rey-spoke` | Optional Spoke provider, exact source bindings, compute runs, and artifact persistence |

This table is an ownership proposal, not a requirement for one process per
crate. Plan 0001 may combine crates for the first vertical slice if semantic
ownership remains legible.

## Failure And Limits

Rey treats capability drift, Git ref rewrites, incomplete history, index
conflicts, cursor replay, source drift, stale proposals, duplicate keys,
incompatible schemas, probe failure, action rejection, process loss, capture
truncation, cancellation, budget exhaustion, and unavailable optional or
required capabilities as ordinary explicit outcomes. None imply equality or
convergence.

Every loop has total time, iteration, action, and evidence-byte limits. Every
frame and delta has row, column, cell-change, and encoded-byte limits. A policy
has a response deadline and proposal-size limit. Proofs retain which limit
stopped evaluation.

## Security Boundary

Rey is not an execution sandbox. It records the actual trust and enforcement
claims of a local executor or delegates them to Spoke compute. Policy proposals
carry no ambient authority. Rey configuration and proof artifacts contain
references to secret handles, never secret values.

Local adapters must distinguish trusted developer input from remote or
adversarial content, remain within explicitly selected roots, and never
silently widen host filesystem access. Tool discovery is not execution
authority. Connected mode must never translate a local host path into a Spoke
path or silently widen Spoke access.

## Current Status

The standalone capability path is implemented across `rey-core`,
`rey-dataframe`, `rey-environment`, `rey-git`, `rey-diff`, `rey-proof`, and the
`rey` composition/CLI crate. It includes bounded environment observation, a
partial read-only Git observation, verified capability snapshot loading, an
exact capability comparator, typed structured and Arrow deltas, Tabular Diff
projection, required-capability certificate evaluation and verification, and
bounded content-addressed local proof bundles with explicit filesystem-only
guarantees. The generic runtime/frontier/policy layers, Git activation, and
Spoke provider remain target architecture.
