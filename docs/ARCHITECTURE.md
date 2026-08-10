# Rey Architecture

This document defines Rey's target ownership boundaries and data flow. It is an
implementation baseline, not a claim that the components already exist.

## Purpose

Rey is an environment-aware, deterministic diff-directed mining and compute
runtime. It first inventories the bounded context surfaces and useful tools
available where it is running. It then mines those surfaces through versioned
relational and source operations, retains typed collections as DataFrames and
native evidence in its natural form, computes directed deltas, and uses
unresolved deltas to schedule subsequent probes or mutations. It records enough
lineage to explain how evidence was derived, why each action ran, and what a
scoped proof actually covers.

Rey has a useful standalone profile over explicit local context. Spoke is an
optional capability amplifier that supplies durable content, exact resource
revisions, composed query, admitted execution, and durable lineage. A model may
guide the feedback loop, but model inference and provider integration are
policy concerns rather than runtime correctness dependencies.

## Architectural Separation

Rey separates eight responsibilities:

1. **Workload plane** — versioned workloads compose generated compute graphs,
   scenarios, claims, policy, qualification, effects, and total limits.
2. **Context-surface plane** — environment providers expose explicit local and
   remote sources, tools, runtimes, and guarantees as a capability snapshot.
3. **Mining plane** — provider-neutral relational and source operations
   retrieve, extract, organize, compare, and visualize bounded evidence while
   concrete providers retain source and execution ownership.
4. **Reasoning plane** — selected frontier work and mined evidence become a
   bounded reasoning surface with exact omissions and admissible operations;
   optional Spoke amplifies retrieval and retention, not surface semantics.
5. **Observation plane** — lenses bind exact inputs and materialize bounded
   typed frames or native artifact references.
6. **Delta plane** — relational, text, and structural comparison preserves
   directed changes and derives invalidation.
7. **Runtime plane** — transitions validate proposals, execute bounded probes or
   effects, update the frontier, and stop on convergence or an explicit bound.
8. **Policy plane** — an agent, deterministic rule, or human proposes a compute
   graph revision or another admissible action.

These are responsibility boundaries, not requirements for separate processes.
The first topology is a local Rey process. `rey ui` attaches an operator
projection to that process; its only browser write is bounded unauthenticated
Journal admission on any explicitly configured listener. It is not a separate
runtime or scheduler, and Journal admission grants no compute authority. A
Spoke provider, when configured or discovered, uses Spoke's routed HTTP
interface.

## System Graph

```text
                  workload declaration
          graph · scenarios · claims · policy · limits
                    │                         │ environment requirements
                    │                         ▼
                    │            explicit environment boundary
                    │                         │
                    │        ┌────────────────┼────────────────┐
                    │        ▼                ▼                ▼
                    │ local workspace  discovered tools  optional Spoke
                    │        └────────────────┬────────────────┘
                    │                         ▼
                    │              capability snapshot frame
                    └───────────────────┬─────┘
                                        │
                    ┌───────────────────▼───────────────────┐
                    │ mining capabilities                   │
                    │ relational          source            │
                    │ query · group       search · parse     │
                    │ traverse · compare  index · measure    │
                    └───────────────────┬───────────────────┘
                                        │
          Git/source activation · policy proposal
                              │
                              ▼
                    ┌───────────────────┐
                    │ Rey runtime       │
                    │ admit · budget    │
                    │ transition · stop │
                    └───┬───────────┬───┘
                        │           │
          mine/observe │           │ act through provider
                        ▼           ▼
              ┌──────────────┐  ┌──────────────┐
              │ frame/native │  │ local action │
              │ projections  │  │ or Spoke run │
              └──────┬───────┘  └──────┬───────┘
                     │                 │
                     └────────┬────────┘
                              ▼
                    ┌───────────────────┐
                    │ typed/native delta│
                    │ invalidation      │
                    │ frontier          │
                    └───────┬───────────┘
                            │
                   ┌────────┴───────────────┐
                   ▼                        ▼
          mine · visualize           proof evaluator
                   │                        │
                   ▼                        ▼
          reasoning surface    local or Spoke-backed evidence
                   │
                   ▼
             next proposal
```

See [Environment and Capabilities](ENVIRONMENT.md) for detailed provider,
snapshot, profile, admission, and degradation contracts.
See [Mining Context Into Evidence](MINING.md) for relational/source operation,
artifact, result, visualization, and provider boundaries.
See [Workloads, Compute Graphs, and Scenarios](WORKLOADS.md) for the public
composition, test-campaign, qualification, progress, catalog, and command
contracts.
See [Git Context and Activation](GIT.md) for software-repository snapshots,
poll cursors, and delta-triggered workloads.

## Core Data Model

| Concept | Meaning | Owner or retention boundary |
| --- | --- | --- |
| Environment | Explicit boundary from which providers may discover context | Host/deployment configuration; observed by Rey |
| Capability snapshot | Frozen inventory of providers, tools, operations, trust, and limits | Rey evidence; local or Spoke-backed |
| Workload | Public versioned composition of graph contract, scenarios, environment, claims, policy, qualification, effects, and budgets | Rey declaration and catalog provider |
| Compute graph | Immutable content-identified typed nodes, ports, and dependency edges proposed for one workload | Catalog/result provider with explicit retention profile |
| Scenario | Exact fixtures, expected observations or claims, comparator, and limits used to test one graph revision | Rey declaration and retained evidence |
| Test campaign | Bounded lineage of graph proposals, scenario attempts, typed deltas, and qualification decision | Local or Spoke-backed result provider |
| Space | Named boundary over sources, lenses, actions, claims, and limits | Rey declaration; local or stored through Spoke |
| Source binding | Exact Spoke or local immutable input identity | Source system; referenced by Rey |
| Lens | Versioned deterministic observation definition | Rey declaration; local or stored through Spoke |
| Frame | Bounded typed observation plus schema and lineage | Working state in Rey; local or Spoke evidence when retained |
| Mining operation | Versioned relational or source transformation with typed inputs, outputs, effects, limits, and completeness | Rey contract; implemented by built-in or discovered provider adapters |
| Mining request | Exact source/artifact bindings, operation, parameters, capability snapshot, limits, and frontier rationale | Rey transition or graph-node evidence |
| Mining result | Manifest of produced native, relational, tree, graph, delta, metric, or visual artifacts plus lineage and omissions | Rey evidence index; artifacts remain provider-owned or explicitly retained |
| Portfolio snapshot | Exact bounded catalog, qualification, environment, dependency, capability, ownership, and coverage inputs for one portfolio observation | Rey runtime evidence; derived from catalog/result/environment providers |
| Workload attention | Canonical typed relation of refine, retest, create, block, or policy-excluded subjects with reasons, readiness, evidence, priority, and cost | Rey runtime working evidence; local or Spoke-backed when retained |
| Journal entry | Ordered typed collaboration document bound to an exact Explorer coordinate; admission grants no execution authority | Local Rey journal; Spoke retention remains separate |
| Action proposal | Policy request naming frozen inputs, effect class, and bounds | Rey trace |
| Run/attempt | Provider-owned execution and capture lineage | Local executor or Spoke compute, explicitly distinguished |
| Delta | Directed typed comparison between compatible frames | Rey evidence; local or Spoke-backed |
| Frontier | Bounded prioritized unresolved work | Rey working state; checkpointed when needed |
| Trigger | Versioned predicate mapping a source delta to workload test selection or graph entry points | Rey declaration |
| Activation | Idempotent trigger match against exact source/target snapshots | Rey transition evidence |
| Claim | Predicate and required evidence over a named scope | Rey declaration; local or stored through Spoke |
| Proof | Claim assessment bound to exact evidence and evaluator inputs | Rey artifact with explicit provider guarantees |
| Trace | Graph connecting the concepts above | Local artifacts or Spoke events/artifacts |

Working DataFrames and queues are never the only durable copy of authored
content. A frame may be reproducible from exact sources and a lens, or retained
as an Arrow evidence artifact when replay cost, external volatility, or proof
requirements demand it.

## Operator Projection

`rey ui` embeds a TanStack Router single-page application and serves the live
bounded workload-list document used by the CLI. The human operator lands on
`/explore`; the CLI remains the agent's primary interface and the human's
deeper diagnostic plane. The Explorer projects one context topology through a
semantic lens: landscape aggregates become workload/attention neighborhoods,
then exact graph/scenario/evidence/delta objects as zoom and focus change.
Identity, relationship classification, bounds, and omissions survive those
visual transitions. Matrix-style coordinate routes bind kind, identity,
revision, lens, and agent role so other operator views can deep-link without
moving topology authority into the URL. `/environment` projects the same typed
`HEAD → INDEX → WORKING` environment delta as `rey env status`; `/workloads`
retains the exact catalog/detail routes. `/agents` begins with the Journal:
current requests and non-excluded attention produce derived system entries;
retained human and agent entries use one bounded typed contract and point to
exact `/explore` coordinates. `/journal/new` admits human entries and routes
to their exact `/journal/{slug}` document; entry blocks expose stable fragment
permalinks. Agents admit through `rey journal add`, and neither path executes
notebook blocks. It then projects an
observed-work ledger from exact workload revisions, tests, runs, mining
outputs, deltas, and attention. Journal entries communicate direction without
becoming assignments or execution authority. Tasks still organize intent,
operation, artifact references, desired delta, readiness, and assignment;
journeys remain derived. Agent
application discovery stays on `/environment`, and generator tuples remain
provenance rather than activity or assignment. `/cadence`
keeps bounded Git reachability, environment sequence, and mounted browser scan
schedules on separate clocks instead of fabricating a total event order. Its
repository-state plane separately shows staged, unstaged, untracked, and
conflicted working-tree attention plus exact `HEAD`-to-local-upstream
publication. That relation is revision-bound and performs no remote transport.
The fixed footer is a live communications channel over typed attention and
passive-revalidation health. A quiet mailbox means no operator attention is
requested; it is not filled with synthetic heartbeat activity. The mailbox
control selects the history axis. The center chevrons select a separate
operator/Rey/agent conversation axis with a conventional transcript and
composer. The current read-only server has no admitted chat transport, so the
composer is disabled and no UI-owned transcript is invented.

Hifi's
Kinetic grammar with the Precision theme defines the interaction and material
language. StyleX owns compiled structural and stateful presentation while
typed Kinetic material values remain runtime data; Rey's typed documents
remain authoritative.

The listener defaults to loopback and carries no authentication, multi-user,
or remote-service guarantee. Its only write is bounded unauthenticated local
Journal admission. An explicit non-loopback bind exposes that write to every
client that can reach the listener and therefore emits a warning; no bind
turns the UI into an execution control plane. See [Context Topology
Explorer](EXPLORER.md), [ADR 0025](decisions/0025-local-operator-ui.md), and
[ADR 0026](decisions/0026-context-topology-explorer.md), and [ADR
0030](decisions/0030-operator-cadence-agents-and-explorer-coordinates.md).
Cadence repository state is specified by [ADR
0036](decisions/0036-cadence-repository-state-and-publication.md).
The shared collaboration Journal is specified by [ADR
0037](decisions/0037-explore-bound-collaboration-journal.md), [ADR
0038](decisions/0038-unauthenticated-hyperlinkable-journal.md), and
[Collaboration Journal](JOURNAL.md).

## Workloads, Graphs, And Scenarios

A Rey workload is the public unit users list, test, run, and inspect. It
declares providers, typed inputs and outputs, a compute-graph contract,
scenarios, triggers, admissible operations, claims, policy, qualification, and
total budgets under one versioned identity.

One immutable graph revision contains stable nodes, typed ports, dependency
edges, exact operation contracts, capability/effect requirements, and limits.
The initial graph is acyclic. An agent, rule, or human may propose a graph, but
the runtime validates it and deterministic scenarios decide qualification.

The first product catalog resolves accepted
`workloads/*/workload.yaml` packages. A package binds the generated graph and
scenario suite, proposal producer/revision/inputs, and a frozen-oracle
admission decision. Exact source bytes and path participate in the workload
proposal identity. Compiled workloads are explicitly selected conformance and
system diagnostics, not default portfolio entries.

`workloads create` precedes package admission with a content-addressed
`workloads/*/request.yaml` contract. That request is an explicit handoff to an
external coding harness, not an LLM embedded in the runtime. Request-only
entries remain visible drafts and cannot be tested or run. Rey imports the
materialized package only after its graph, suite, provenance, frozen oracle,
limits, and request/package identity match validate. Automatic harness
invocation remains a later campaign boundary. See [ADR
0023](decisions/0023-workspace-workload-packages.md) and [ADR
0024](decisions/0024-workload-creation-requests.md).

A scenario executes that exact graph against fixture bindings and compares
`EXPECTED` to `OBSERVED`. Conclusive mismatches retain typed deltas; missing or
incompatible evidence is inconclusive. All required scenarios must freshly
pass for the same graph revision before `workloads run` selects it by default.

Manual, policy-selected, Git, or future stream activations select a workload
test campaign, scenario subset, or declared graph entry point through normal
admission. Workload, graph, scenario, campaign, and run revisions participate
in transition and proof identity. See [Workloads, Compute Graphs, and
Scenarios](WORKLOADS.md).

## Environment And Capability Discovery

Environment awareness is provided through narrow providers rather than an
unbounded host scan. Initial provider classes may include:

- process-owned `HOME`, `PWD`, and `PATH` discovery seeds;
- built-in Rey functions that require no external executable;
- an explicitly supplied, agent-authored environment mapping resource for
  relevant variables, input files, desired executable inventory, and reference
  edges;
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

Bootstrap discovery loads no project configuration and assumes no Spoke
variable names. The frozen discovery record becomes input to agent reasoning;
an emitted mapping resource enters observation only through explicit `--map`.
The mapping graph is a context declaration, not a provider adapter or policy
grant. Each desired executable declares why it belongs in the inventory. An
exact semantic identity over those declarations is the inventory record; the
exact target capability snapshot is a separate bounded search record.
Executable nodes remain potential capabilities
until an admitted adapter freezes their exact operation contract. The mapping
provider projects bounded graph, node, edge, variable-presence/digest/value,
file-identity, and executable-identity evidence into the snapshot used by
environment history. Exact variable values are retained only for explicit
non-sensitive `capture: value` declarations under a byte bound. Sensitive
variables remain presence-only, and mapped file bytes are never retained.

The capability snapshot is a typed relation. A first schema should be able to
represent provider id/revision, capability id, kind, resolved location,
version, content or provenance digest when available, availability, trust,
supported operations, enforcement claims, observation time, and errors.

Capability discovery is repeatable during a trace. A delta between snapshots
can invalidate actions, lenses, and proofs. An executable path, version, digest,
provider health, or Spoke capability change is part of runtime state rather
than ambient trivia.

The human environment revision loop adds a separate admission index between a
fresh capability snapshot and committed history. Status exposes
`HEAD → INDEX → WORKING`; add updates that index; commit records only the index
and never re-runs discovery. This provides review stability without turning an
accepted executable observation into action authority.

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

## Mining Plane

Mining is the shared capability layer between environment inventory and
runtime orientation. It does not introduce one universal query language or
force every provider into one execution mechanism. Instead, a common operation
contract binds exact input/output kinds, parameters, semantics, effects,
capabilities, effective limits, completeness, invalidation, and implementation
identity.

The two primary families are:

- **relational mining**, which retrieves and organizes typed records through
  select, filter, join, group, aggregate, align, traverse, compare, summarize,
  and visualization operations; and
- **source mining**, which locates, searches, segments, tokenizes, parses,
  indexes, traverses, measures, compares, and visualizes text, code,
  configuration, logs, documents, and native artifacts.

The families interoperate through exact projections. Search matches, syntax
nodes, symbols, references, dependencies, diagnostics, and metrics may become
typed frames. Their rows retain links to exact native source spans and the
operation/parser/index revision that derived them. Source bytes, ordered text,
syntax trees, patches, and binary artifacts remain native when tabularization
would lose meaning.

An exact read of already identified immutable evidence may participate in
bounded orientation. Pure projection over frozen evidence is deterministic
compute. Reading mutable state or invoking `rg`, a parser, compiler service,
language server, or other external miner is an explicit probe with normal
admission and execution lineage. Discovery never grants that authority.

Visualization is a versioned mining projection over authoritative artifacts.
It records grouping, ordering, context, layout, aggregation, elision, sampling,
limits, omissions, and deep links. Tables, patches, trees, graphs, timelines,
and metric panels cannot change the underlying delta assessment, proof status,
coverage, confidence, or progress.

Mining operates in two nested loops. The inner workload campaign mines a
declared domain, evaluates scenarios, and uses output deltas to refine its
graph. The outer portfolio campaign mines exact catalog, retained result,
environment/dependency, capability, ownership, and coverage inputs to derive
which workload or uncovered surface needs attention. Workloads are therefore
both mining instruments and mineable runtime context.

`rey.workload-attention.v1` is evidence between derivation and scheduling. Its
ready rows may feed the generic frontier; blocked and policy-excluded rows stay
visible but ineligible. The scheduler does not invent attention reasons, and a
policy cannot resolve its own row. See [ADR 0022](decisions/0022-portfolio-mining-and-workload-attention.md).

See [Mining Context Into Evidence](MINING.md) and
[ADR 0017](decisions/0017-mining-capability-model.md).

## Git Provider And Activation

Git is a specialized context and activation provider for software spaces. It
observes the object database, commit graph, refs, per-worktree HEAD and index,
and optionally bounded worktree status. It produces typed repository, ref,
commit, parent, path-change, index-entry, status, and activation relations.
Repository state is not folded into the `rey env` admission snapshot: that
surface retains the `git` application identity, while cadence and workload
activation retain exact Git observations on their own clock.

A poll compares the current repository snapshot with its last completely
processed cursor. Fast-forward refs can expose newly reachable commits; rewinds
and divergence emit explicit ref/reachability deltas; semantic index changes
expose staged proposals before a commit exists. Raw index changes caused only
by stat-cache refresh do not activate staged-content workload entries.

Triggers select delta subsets and name an affected workload revision, scenario
selection, or declared graph entry point. An activation has deterministic
identity over the trigger, workload/graph/scenario selection, source/target
snapshots, and matched delta. It enters ordinary action admission and can be
replayed after a crash. The poll cursor advances only after required transition
evidence reaches its claimed retention boundary.

## Delta And Frontier

The delta engine selects a comparison contract that matches the mined artifact
shape. Relational comparison aligns compatible frames under explicit direction,
keys, ordering, and normalizers. Text comparison preserves ordered content,
encoding, segmentation, and context. Structural comparison aligns declared
tree or graph identities without guessing through incompatible or incomplete
evidence. Human tables, patches, trees, graphs, summaries, Tabular Diff CSV,
JSON, and Arrow are projections of authoritative results.

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
  -> schedule -> mine -> project
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
reasoning surface. Rey identifies mining needs from frontier rows, retrieves
exact read-only evidence through the provider that owns it, and applies
versioned deterministic relational or source projections. Mining does not
grant new execution authority or duplicate Spoke query, index, and storage
ownership. Any read that observes mutable state, invokes a tool, or creates a
new lens result is an explicit probe transition.

Mining and projection may repeat inside one orientation phase as exact evidence
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

Within a workload test campaign, one pass freezes a graph revision, executes
selected scenarios, compares expected to observed outputs, and derives a
frontier from failing deltas and unresolved claims. The recurring transition
machine then retrieves and projects that bounded evidence before policy may
propose the next graph revision. Graph dependency order is distinct from
frontier scheduling: edges determine which nodes can run, while the frontier
determines which unresolved scenario evidence should receive the next bounded
unit of reasoning or compute.

## Actions And Transitions

An action has one of two effect classes:

- **probe** — read-only computation that may produce new observations or
  derived artifacts; or
- **mutation** — an explicit change to a declared target through a Spoke
  resource method, admitted Spoke compute run, or explicitly authorized local
  provider action.

One transition follows this protocol after a committed frontier is available:

1. freeze the current workload, graph, scenario selection,
   activation/frontier, source revisions, and relevant frame ids;
2. select bounded ready frontier work under exact record, frontier, capability,
   scheduler, and budget inputs;
3. mine declared exact read-only evidence and project the bounded reasoning
   surface, recording operation lineage, omissions, and effective limits;
4. receive a policy proposal citing that surface and frontier evidence;
5. validate action identity, capability snapshot, allowed effect,
   preconditions, and remaining budget;
6. submit or perform the action through its owning contract;
7. retain action, run, attempt, output, and failure lineage;
8. materialize affected post-action lenses;
9. compute transition and applicable residual deltas;
10. evaluate claims, progress, and the next frontier; and
11. commit the transition record before selecting another action.

An action can complete successfully while its transition fails semantically.
For example, a compiler process may exit zero while the dependency graph or
test evidence still differs from the claim. Post-action observations, not exit
status, determine convergence.

## Policy Boundary

A policy receives a bounded view of the current workload, graph/scenario
evidence, space, frontier, admissible graph operations or actions, and budgets.
It returns a structured proposal. The runtime treats that proposal as
untrusted input and validates it identically whether it came from a model, a
rule, or a human.

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
conformance workloads without changing that ownership boundary.

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

Those relations may be produced by different mining rungs: bounded text search
can populate matches, a parser can populate syntax nodes, a semantic index can
populate symbols and references, and grouped transforms can derive metrics.
Every rung retains exact source, operation, completeness, and dependency
lineage so a richer but partial index cannot silently replace source truth.

## Target Crate Ownership

The first design proposes these Rust ownership boundaries:

| Crate | Ownership |
| --- | --- |
| `rey` | Workload CLI, catalog/configuration composition, and user-facing orchestration |
| `rey-core` | identities, revisions, limits, statuses, and shared value contracts |
| `rey-mining` | provider-neutral mining operation/request/result, artifact, completeness, dependency, and visualization contracts; no query engine, parser bundle, or storage |
| `rey-dataframe` | frame metadata, Polars schemas, Arrow codecs, and bounded rendering |
| `rey-environment` | capability discovery, snapshots, provider contracts, and local context adapters |
| `rey-git` | repository identity, bounded current reachable-commit sequence, commit/ref/index frames, polling cursors, triggers, and activations |
| `rey-diff` | relational, text, and structural comparison contracts, typed changes, summaries, and diff projections |
| `rey-runtime` | workload/graph/scenario lifecycle, spaces, lenses, actions, transitions, budgets, cancellation, and trace assembly |
| `rey-frontier` | canonical frontier/progress relations, prioritization inputs, convergence evaluation, and bounded deterministic selection |
| `rey-proof` | claims, evidence manifests, certificates, verification, and staleness |
| `rey-policy` | bounded reasoning surfaces plus provider-neutral proposal and admissible-action contracts |
| `rey-spoke` | Optional Spoke provider, exact source bindings, compute runs, and artifact persistence |

This table is an ownership proposal, not a requirement for one process per
crate. Plan 0006 has created the narrow `rey-mining` contract crate; provider
execution remains in the adapters that own its source and tool semantics.

## Failure And Limits

Rey treats capability drift, Git ref rewrites, incomplete history, index
conflicts, cursor replay, source drift, stale proposals, unsupported mining,
partial parsing/indexing, duplicate keys, incompatible schemas, probe failure,
action rejection, process loss, capture/visualization truncation,
cancellation, budget exhaustion, and unavailable optional or required
capabilities as ordinary explicit outcomes. None imply equality or
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
guarantees. The `env` CLI now adds a verified bounded linear history of
capability snapshots: `status` derives HEAD-to-working state, `commit` accepts
one non-empty semantic revision, and `log -p` reopens exact parent-directed
environment patches. Status separates staged and unstaged working-tree rows,
interactive add confirms environment-native hunks, and new commit identities
bind explicit retention time. These environment commits are local Rey observations, not
Git objects or Spoke-durable revisions. `rey-runtime` implements the pure
formal state reducer through an explicit scheduling phase; `rey-frontier`
implements canonical frontier, progress, and bounded selection contracts; and
`rey-policy` implements the bounded reasoning-surface document and DataFrame
projection.
The workload slice implements a bounded workspace package catalog, typed DAG
execution, scenario deltas, exact qualification, verified local result state,
and the `list`, `status`, `test`, and `run` commands. The prior compiled
fixture catalog remains behind explicit conformance selection.
Frontier/progress/scheduling v2 and reasoning-surface v3 bind workload, graph,
scenario-suite, and campaign identities; runtime state remains v2.
The source-search conformance workload now supplies one narrow workload-specific
frontier derivation and provider execution path. Generic dependency
invalidation, recurring scheduling, policy proposals, Git activation, and the
Spoke provider remain target architecture.

The `rey-mining` crate now implements the provider-neutral operation, request,
result, artifact, completeness, lineage, dependency, and bound contracts
accepted by ADR 0017. Canonical semantic identities include evidence-changing
parameters and effective limits; replay verification rejects tampering and
request, provider, capability, or implementation drift. `rey-environment` now
implements an exact explicit local corpus binding, deterministic case-sensitive
UTF-8 literal search, native context retention, and the typed
`rey.source-matches` relation through those manifests. `rey-diff` implements
`rey.text-delta.v1` and `rey.source-match-delta.v1`; `rey-runtime` composes the
source provider and renderer in one qualified graph, derives one frontier row
from its complete failing evidence, selects it with the generic scheduler, and
projects one verified reasoning surface. The CLI exposes complete, different,
and truncated mining evidence through all four workload commands.

The first portfolio-mining slice adds `rey.portfolio-snapshot.v1`, the
Polars-backed `rey.workload-attention.v1` relation, and the qualified
`rey.portfolio.attention` system workload. The workload CLI now exposes exact
attention actions, reasons, readiness, coverage, evidence, priority, cost, and
exclusions. Read-only list/status consume retained environment state; run
evaluates the same retained portfolio inputs under fresh qualification.

No workload ownership declaration, live dependency invalidation, attention-to-
frontier adapter, coding-harness request/response, recurring scheduler,
admitted `rg` search, parser/index provider, general structural delta, or agent
loop is implemented. Those require the same human-verifiable end-to-end
boundary before they count as delivered.
