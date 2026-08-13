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

Rey has a useful standalone profile over explicit local context. A model may
guide the feedback loop, but model inference and provider integration are
policy concerns rather than runtime correctness dependencies.

## Architectural Separation

Rey separates ten responsibilities:

1. **Workload plane** — versioned workloads compose generated compute graphs,
   scenarios, claims, policy, qualification, effects, and total limits.
2. **Context-surface plane** — environment providers expose explicit local and
   remote sources, tools, runtimes, and guarantees as a capability snapshot.
3. **Mining plane** — provider-neutral relational and source operations
   retrieve, extract, organize, compare, and visualize bounded evidence while
   concrete providers retain source and execution ownership.
   The implemented survey slice uses `rey-locator` bindings and emits retained
   topography patches; browser projections never become a resolver.
4. **Scene-editor candidate plane** — agents, surveys, and eventually humans
   assemble bounded native terrain, feature, marker, hydrology, and boundary
   artifacts into reviewable INDEX state and linear `SCENE@n` commits with
   immutable candidate packages; deterministic generators retain their exact
   recipes, packages grant no admission authority, and they reach Explorer only
   through a qualified workload.
5. **Projection-engine plane** — exact admitted evidence and a versioned
   coordinate/projection basis compile into immutable scenes, bounded fields,
   semantic LOD, render passes, picking, and high-fidelity browser pixels;
   rendering never becomes semantic authority.
6. **Reasoning plane** — selected frontier work and mined evidence become a
   bounded reasoning surface with exact omissions and admissible operations.
7. **Observation plane** — lenses bind exact inputs and materialize bounded
   typed frames or native artifact references.
8. **Delta plane** — relational, text, and structural comparison preserves
   directed changes and derives invalidation.
9. **Runtime plane** — transitions validate proposals, execute bounded probes or
   effects, update the frontier, and stop on convergence or an explicit bound.
10. **Policy plane** — an agent, deterministic rule, or human proposes a compute
   graph revision or another admissible action.

These are responsibility boundaries, not requirements for separate processes.
The first topology is a local Rey process. `rey ui` attaches an operator
projection to that process. Its explicit browser writes are bounded
unauthenticated Journal admission, conditional Channel WORKING replacement,
and exact workload file qualification/admission on any explicitly configured
listener. It is not a separate runtime or scheduler; none of those writes
grants general compute or proof authority.

## System Graph

```text
                  workload declaration
          graph · scenarios · claims · policy · limits
                    │                         │ environment requirements
                    │                         ▼
                    │            explicit environment boundary
                    │                         │
                    │        ┌────────────────┴────────────────┐
                    │        ▼                                 ▼
                    │ local workspace                   discovered tools
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
              │ projections  │  │              │
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
          reasoning surface           local evidence
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
See [Rey Command-Line Interface](CLI.md) for the agent-facing command model,
explicit mutation posture, and `HEAD → INDEX → WORKING` admission loops.
See [Git Context and Activation](GIT.md) for software-repository snapshots,
poll cursors, and delta-triggered workloads.

## Core Data Model

| Concept             | Meaning                                                                                                                                                                                                | Owner or retention boundary                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| Environment         | Explicit boundary from which providers may discover context                                                                                                                                            | Host/deployment configuration; observed by Rey                                                                  |
| Capability snapshot | Frozen inventory of providers, tools, operations, trust, and limits                                                                                                                                    | Local Rey evidence                                                                                              |
| Workload            | Public versioned composition of graph contract, scenarios, environment, claims, policy, qualification, effects, and budgets                                                                            | Rey declaration and catalog provider                                                                            |
| Compute graph       | Immutable content-identified typed nodes, ports, and dependency edges proposed for one workload                                                                                                        | Catalog/result provider with explicit retention profile                                                         |
| Scenario            | Exact fixtures, expected observations or claims, comparator, and limits used to test one graph revision                                                                                                | Rey declaration and retained evidence                                                                           |
| Test campaign       | Bounded lineage of graph proposals, scenario attempts, typed deltas, and qualification decision                                                                                                        | Local result provider                                                                                           |
| Space               | Named boundary over sources, lenses, actions, claims, and limits                                                                                                                                       | Rey declaration                                                                                                 |
| Source binding      | Strongest exact input identity available                                                                                                                                                               | Source system; referenced by Rey                                                                                |
| Coordinate binding  | Provider-qualified semantic address for one object or bounded region; camera and lens are not identity                                                                                                 | Owning provider; local bindings disclose local-only guarantees                                                  |
| Lens                | Versioned deterministic observation definition                                                                                                                                                         | Rey declaration                                                                                                 |
| Frame               | Bounded typed observation plus schema and lineage                                                                                                                                                      | Working state or explicit local evidence                                                                        |
| Mining operation    | Versioned relational or source transformation with typed inputs, outputs, effects, limits, and completeness                                                                                            | Rey contract; implemented by built-in or discovered provider adapters                                           |
| Mining request      | Exact source/artifact bindings, operation, parameters, capability snapshot, limits, and frontier rationale                                                                                             | Rey transition or graph-node evidence                                                                           |
| Mining result       | Manifest of produced native, relational, tree, graph, delta, metric, or visual artifacts plus lineage and omissions                                                                                    | Rey evidence index; artifacts remain provider-owned or explicitly retained                                      |
| Topography patch    | Admitted survey result containing coordinate anchors, classified relationships, coverage, frontier, omissions, lineage, and a directed map delta                                                       | Rey evidence index; source artifacts and coordinates remain provider-owned                                      |
| Editor project      | Rey-owned mutable declaration of bounded workspace-native scene sources, explicit roles, and one coordinate-system contract                                                                            | Selected local editor state (`.rey/editor/project.json` by default); native authored sources remain workspace files |
| Scene commit        | Linear immutable editor revision binding sequence, parent, timestamp, message, and one exact candidate package                                                                                          | Local editor history; authoring HEAD but explicitly not admitted evidence                                       |
| Scene generation recipe | Generator revision, source identity, seed, bounds, and complete effective geometry/effect hyperparameters embedded in its native output                                                            | Workspace source lineage; deterministic authoring, never evidence authority                                     |
| Scene package       | Immutable candidate containing an exact scene snapshot, native-object references, POI/feature index, limits, omissions, and directed prior-package delta                                                | Local content-addressed editor candidate store; explicitly not admitted evidence                                |
| Scene admission request | Content-identified handoff naming one exact scene package and the workload operation required to validate it                                                                                       | Editor candidate store until an explicit workload accepts or rejects it                                         |
| Semantic atlas      | Content-identified bounded layout over admitted regional evidence with stable sector/region identity, synthetic spherical coordinates, clusters, compiler, limits, omissions, and lineage                    | Deterministic workload-list projection today; target retained admission revision, sector polygons, and directed movement delta |
| Admitted regional scene | Qualified result binding one exact editor package to native-to-semantic and county-local transforms, normalized terrain/feature layers, validity/no-data, limits, omissions, and lineage                  | Workload result and Explorer input; candidate packages remain outside this boundary                              |
| Projection packet   | Bounded target envelope binding admitted evidence, coordinate/projection basis, scalar/vector channels, surveyed-validity masks, scene layers, revisions, limits, completeness, omissions, and lineage | Pure Rey projection input; reproducible from exact evidence or retained only under an explicit evidence profile |
| Terrain program     | Deterministic evaluator, seed, absolute-coordinate and validity rules, multiscale bands, and bounded camera-working-set policy compiled from one exact projection packet                         | Pure projection input; authored controls and admitted sources remain authoritative, while evaluated buffers are disposable |
| Terrain working set | Camera-relative scalar/vector/mask buffers and geometry sampled from one exact terrain program under declared cell/byte limits                                                                   | Browser/GPU working state and optional bounded proof capture; never an admitted tile or sole source copy          |
| Scene snapshot      | Immutable, stably ordered engine scene compiled from one projection packet; semantic identity excludes camera motion and measured frame time                                                           | Browser working state and optional bounded proof artifact; never authoritative source evidence                  |
| Portfolio snapshot  | Exact bounded catalog, qualification, environment, dependency, capability, ownership, and coverage inputs for one portfolio observation                                                                | Rey runtime evidence; derived from catalog/result/environment providers                                         |
| Workload attention  | Canonical typed relation of refine, retest, create, block, or policy-excluded subjects with reasons, readiness, evidence, priority, and cost                                                           | Rey runtime working evidence                                                                                   |
| Journal entry       | Ordered typed collaboration document bound to an exact semantic coordinate, numeric camera scale, and source revision; admission grants no execution authority                                         | Local Rey journal                                                                                               |
| Action proposal     | Policy request naming frozen inputs, effect class, and bounds                                                                                                                                          | Rey trace                                                                                                       |
| Run/attempt         | Provider-owned execution and capture lineage                                                                                                                                                           | Local executor                                                                                                  |
| Delta               | Directed typed comparison between compatible frames                                                                                                                                                    | Local Rey evidence                                                                                              |
| Frontier            | Bounded prioritized unresolved work                                                                                                                                                                    | Rey working state; checkpointed when needed                                                                     |
| Trigger             | Versioned predicate mapping a source delta to workload test selection or graph entry points                                                                                                            | Rey declaration                                                                                                 |
| Activation          | Idempotent trigger match against exact source/target snapshots                                                                                                                                         | Rey transition evidence                                                                                         |
| Claim               | Predicate and required evidence over a named scope                                                                                                                                                     | Rey declaration                                                                                                 |
| Proof               | Claim assessment bound to exact evidence and evaluator inputs                                                                                                                                          | Rey artifact with explicit provider guarantees                                                                  |
| Trace               | Graph connecting the concepts above                                                                                                                                                                    | Local artifacts                                                                                                 |

Working DataFrames and queues are never the only durable copy of authored
content. A frame may be reproducible from exact sources and a lens, or retained
as an Arrow evidence artifact when replay cost, external volatility, or proof
requirements demand it.

## Operator Projection

`rey ui` embeds a TanStack Router single-page application and serves the live
bounded workload-list document used by the CLI. The human operator lands on
`/explore`; the CLI remains the agent's primary interface and the human's
deeper diagnostic plane. Before any workload HEAD or admitted topography
exists, Explorer projects exact request/WORKING/INDEX workload file state as
attention beacons on a presentation-only orientation globe. That globe is an
unmapped consent surface, not `rey.semantic-atlas.v1`; it claims no semantic
distance, terrain, project boundary, or agent activity. Inspection and consent
descend into the exact workload record and Feed approval control. Only an
explicitly admitted survey followed by a bounded run may replace it with
mapped evidence. The Explorer projects admitted topography patches
and, once the qualifying workload exists, admitted regional scenes through one
persistent semantic scene. World places admitted regions on the synthetic,
revision-bound sphere from `rey.semantic-atlas.v1`; its longitude/latitude axes
have no Earth CRS or physical-distance claim. The World globe shows sectors,
regional POIs, and clusters while local charted envelopes, unresolved probe
horizons, and boundary weather remain available in closer lenses.

As the lens enters Atlas, that same synthetic sphere unwraps through a
horizontally wrapping spherical-Mercator transform with explicit pole,
antimeridian, and distortion behavior. This **semantic Mercator** chart is not
EPSG:3857 and does not relabel native CRS84 scene sources. Sector polygons and
admitted county footprints are separate identities; focus may raise a sector
as transient presentation without changing its evidence or height.

Atlas derives terrain-style contour isolines from bounded anchor-sample
influence and will use admitted scene packages as the primary detailed map
fabric once the scene-admission workload exists. Entering one admitted county
expands its revision-bound local tangent frame under a stylized isometric
camera. Landscape, Neighborhood, Object, and Evidence progressively add
terrain, watersheds, admitted highways/roads/lots/structures/artifacts, labels,
relationships, inspection objects, and exact basis without replacing the map.
Exact survey edges remain deep inspection evidence rather than relief, roads,
or path geometry. Identity, relationship classification, bounds, and omissions
survive every visual transition. Semantic coordinates are provider-qualified addresses;
camera center, continuous scale, viewport, and lens remain separate view state.
The current hard-cut interface uses `rey+local://...` semantic coordinates and
the `/explore?coordinate=...&scale=...` browser envelope. Matrix paths are not
part of the v1 Explorer contract. Journal v2 stores coordinate and scale
separately and places its typed blocks in a bounded 12-column broadsheet.
`/environment` projects the same typed
`HEAD → INDEX → WORKING` environment delta as `rey env status`; `/workloads`
retains the exact catalog/detail routes and aligns admitted revisions plus
creation requests as separate Hifi dense evidence relations. Exact workload
routes continue that relation grammar across runtime or request posture, exact
bindings, and retained mining output. `/agents` begins with the Journal: current
requests and non-excluded attention produce derived system entries; retained
human and agent entries use one bounded typed contract and point to exact
`/explore` coordinates. `/journal/new` and exact `/journal/{slug}` routes share
one live editing surface; a retained edit appends an exact superseding entry
instead of rewriting history. Entry blocks expose stable fragment permalinks.
Agents admit through `rey journal add`, and neither path executes notebook
blocks. It then projects an
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
composer. A bounded workspace-local transcript provider admits exact sessions
and messages without delivery or execution effects. The operator server now
projects that provider and conditionally appends only through an exact
session-declared human browser writer; unavailable transport leaves the
composer disabled and no UI-owned transcript is invented.

The collaboration substrate preserves those boundaries. A workspace-local
Channel graph addresses channels, subscriptions, Feed
streams, ordered layouts, and explicit relay declarations through a separate
Git-shaped operator index. Standalone immutable Channel observations and their
channel-local admissions remain outside that topology index and form a bounded
collaboration-frontier projection. Journal remains the deliberate rich
synthesis surface and may cite exact observations or begin from an unretained
Journal seed. Feed, mailbox, conversation, observations, and Journal remain
different interfaces; channel admission grants no action or transport
authority. The local revision slice implements the canonical built-in graph,
bounded snapshots/deltas, symlink-safe `CHANNEL HEAD → INDEX → WORKING` store,
and `channels list/status/diff/apply/add/commit/log`. Immutable file-backed
messages, explicit relay attempts, and bounded one-shot polling-beacon ticks
are also implemented. `/channels` reads the same bounded status and can replace
WORKING only through the same validator/store under exact expected HEAD and
WORKING snapshot preconditions. Feed selects detached URL preview, WORKING,
HEAD, then built-in layout state; deliberate adoption and stable
pointer/keyboard movement use that same conditional WORKING write and retain
typed deltas or rollback failures. A separate tamper-detecting observation log
now owns immutable statements, exact source/evidence bindings, Channel
admission edges, retained partial broadcast receipts, single supersession or
resolution closure, and the bounded collaboration frontier. Its CLI exposes
add/list/show/resolve and exact partial broadcast receipts. Browser Feed and
mailbox projection plus deterministic unretained Journal seeding are
implemented. The separate local conversation log now retains immutable
sessions, declared participants/writers, per-session message order, exact
sources, availability, authority, limits, and failure posture; no append
invokes an agent or uses Channel relay. Browser conversation projection and
conditional append are delivered. Resident beacon scheduling and remote
inbound cursors remain planned behavior.

Hifi's
Kinetic grammar with the Precision theme defines the interaction and material
language. StyleX owns compiled structural and stateful presentation while
typed Kinetic material values remain runtime data; Rey's typed documents
remain authoritative.

The listener defaults to loopback and carries no authentication, multi-user,
or remote-service guarantee. Its explicit writes are bounded Journal and
conversation admission, conditional Channel WORKING replacement, and qualified
exact workload-INDEX approval. An explicit non-loopback bind exposes all four writes
to every client that can reach the listener and therefore emits a warning; no
bind grants Channel INDEX/HEAD, relay, workload execution, or proof authority. See
[Context Topology Explorer](EXPLORER.md), [Collaboration Journal](JOURNAL.md),
[Conversation Transcripts](CONVERSATIONS.md), and
[Git Context and Activation](GIT.md).

### Explorer projection-engine boundary

Explorer is a high-fidelity spatial game engine specialized for evidence-bound
projections of high-dimensional context. Its browser placement does not reduce
it to a React visualization component. The target boundary has five layers:

1. **Evidence adapters** translate exact workload, topography, portfolio, and
   future high-dimensional provider artifacts into one versioned bounded
   projection packet. They own semantic interpretation and validity.
2. **Scene and field compilation** creates stable scene identities,
   data-oriented scalar/vector working sets, procedural frequency bands, validity masks,
   natural-feature derivations, omissions, and invalidation dependencies.
3. **Engine mechanism** owns camera transforms, semantic and geometric LOD,
   culling, picking, label budgets, scene retention, and dirty-set scheduling.
4. **Render graph and backends** own ordered materials, hillshade, occlusion,
   contours, water, weather, boundaries, POIs, labels, selection, antialiasing,
   accelerated resources, and visible fallback.
5. **React shell** owns routes, browser controls, accessibility, evidence
   panels, exact links, and lifecycle integration around the engine surface.

The flow is one-way:

```text
request/WORKING/INDEX → orientation beacon → human consent
                                              │
                                              ▼ admitted survey + explicit run
editor WORKING → INDEX → immutable candidate package
                                  │
                                  ▼ explicit qualified admission workload
admitted evidence
  → versioned projection packet
  → immutable scene + fields + validity
  → camera/LOD/culling
  → ordered render passes
  → pixels
```

The editor arrow cannot bypass the admission workload. `rey.scene-package.v1`
is not a topography patch, projection packet, browser scene, or proof. The
editor freezes bounded native GeoJSON and a feature/POI index. The file-backed
`scene-admission` workload now validates exact current packages and native
objects, qualifies deterministic acceptance/rejection scenarios, and retains
an admitted regional scene plus projection packet through the CLI. This is
still enabling work because creating or admitting a package leaves `/explore`
unchanged. [Explorer](EXPLORER.md), [Mining](MINING.md), and [Plan
0003](../plans/0003-scene-to-explorer.md) own this boundary.

Picking reverses only screen position to a stable scene identity and exact
coordinate. It does not reverse pixels into evidence. The CLI inspects the
packet, compiler revisions, field semantics, validity, limits, omissions, and
lineage through the existing workload surface; the browser is responsible for
high-fidelity spatial verification.

Terrain fidelity begins with a bounded deterministic terrain program and a
camera-relative transient working set. Height, normal, slope, aspect,
curvature, runoff, erosion, material, and shading are separate channels or
passes with explicit derivation and revision. Unknown and unsupported validity
never become sampled height merely because a material feathers their visual
boundary. Semantic scene identity is backend-independent; generated buffers,
GPU pixels, and measured frame time are not authoritative evidence.

The target uses an immutable scene graph plus data-oriented field buffers and
an explicit render graph. The narrow Three.js `WebGPURenderer` and TSL adapter
uses WebGPU as the preferred backend and Three.js's WebGL2 backend as the
compatibility path; Rey's reference renderer owns deterministic semantic
proof. A generic ECS, physics runtime, or free-orbit 3D requires a later
qualified need. [Plan 0003](../plans/0003-scene-to-explorer.md) owns the
remaining code extraction and terrain-fidelity proof.

## Workloads, Graphs, And Scenarios

A Rey workload is the public unit users list, test, run, and inspect. It
declares providers, typed inputs and outputs, a compute-graph contract,
scenarios, triggers, admissible operations, claims, policy, qualification, and
total budgets under one versioned identity.

One immutable graph revision contains stable nodes, typed ports, dependency
edges, exact operation contracts, capability/effect requirements, and limits.
The initial graph is acyclic. An agent, rule, or human may propose a graph, but
the runtime validates it and deterministic scenarios decide qualification.

The product catalog observes `sys/*/workload.yaml` as WORKING proposals.
A package binds the generated graph and frozen scenario suite plus proposal
producer, revision, and inputs; it owns no admission decision. Exact source
bytes and path participate in the proposal identity. `workloads add` freezes
the complete catalog in INDEX, `test --staged` binds passing qualification to
that exact snapshot, and a human workload commit advances HEAD. Compiled
workloads are explicitly selected conformance and system diagnostics, not
default portfolio entries.

`workloads create` precedes package admission with a content-addressed
`sys/*/request.yaml` contract. That request is an explicit handoff to an
external coding harness, not an LLM embedded in the runtime. Request-only
entries remain visible drafts and cannot be tested or run. Rey imports the
materialized package into WORKING only after its graph, suite, provenance,
frozen oracle, limits, and request/package identity match validate. Automatic harness
invocation remains a later campaign boundary. See [Workloads](WORKLOADS.md),
[CLI](CLI.md), and [Runtime](RUNTIME.md).

A scenario executes that exact graph against fixture bindings and compares
`EXPECTED` to `ACTUAL`; the retained structured artifact remains the observed
output. Conclusive mismatches retain typed deltas; missing or incompatible
evidence is inconclusive. All required scenarios must freshly
pass for every package in the exact staged INDEX before a human can admit that
snapshot; `workloads run` resolves only the resulting HEAD.

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
- known developer tools resolved from configured paths or `PATH`; and
- language-specific toolchains, analyzers, build systems, and test runners.

Each provider has stable identity, version, detection rules, trust class,
source/effect capabilities, supported enforcement, and probe limits. Discovery
may use narrowly defined read-only operations such as executable resolution,
metadata inspection, or a bounded `--version` invocation. It never executes an
unknown file merely because it exists.

Bootstrap discovery loads no project configuration and assumes no remote
service variable names. The frozen discovery record becomes input to agent reasoning;
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
provider health, or capability change is part of runtime state rather
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
- **Required-capability** is a per-space or per-claim constraint, not a separate
  runtime. Admission fails early when the snapshot lacks a named capability or
  guarantee.

A proof states its profile and provider set. No provider may silently change a
claim's required guarantees.

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
bindings, the capability snapshot, and relevant local tool revisions,
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
ready rows feed the generic frontier; blocked and policy-excluded rows stay
visible but ineligible. The scheduler does not invent attention reasons, and a
policy cannot resolve its own row. See [Frontier, Progress, and
Scheduling](FRONTIER.md) and [Runtime](RUNTIME.md).

See [Mining Context Into Evidence](MINING.md).

## Git Provider And Activation

Git is a specialized context and activation provider for software spaces. It
observes the object database, commit graph, refs, per-worktree HEAD and index,
and optionally bounded worktree status. It produces typed repository, ref,
commit, parent, path-change, index-entry, status, and activation relations.
Repository state is not folded into the `rey env` admission snapshot: that
surface retains the `git` application identity, while cadence and workload
activation retain exact Git observations on their own clock.

Workloads may declare exact repository/worktree HEAD or semantic-index
revisions. Portfolio invalidation compares those declarations only with the
acknowledged Git cursor snapshot; an ambient observation or pending transition
does not become workload evidence before its exact acknowledgement.

A poll compares the current repository snapshot with its last completely
processed cursor. Initialization freezes exact watched-ref names and retains
their targets or absence; each later movement is classified independently
from HEAD. Every changed ref retains bounded canonical added/removed reachable
commit sets from the raw object graph and a bounded canonical tree-to-tree path
delta. Path evidence preserves reversible byte identity, direction, source and
target modes/OIDs, omissions, and exact ref scope; rename inference is
disabled. Semantic index changes expose staged proposals before a commit
exists, while raw index changes caused only by stat-cache refresh do not
activate staged-content workload entries.

The explicit Git watch recurrence retains every successful or failed attempt
before continuing. Iteration, elapsed time, retry, and cadence are independent
bounds; recovered failures remain partial, and cooperative signal cancellation
retains a terminal receipt without claiming convergence. This read-only loop
does not generalize retry safety to actions or execute an activation.

Triggers select delta subsets and name an affected workload revision, scenario
selection, or declared graph entry point. An activation has deterministic
identity over the trigger, workload/graph/scenario selection, source/target
snapshots, and matched delta. It is a proposal that must enter ordinary action
admission and can be replayed after a crash. The poll cursor advances only
after required transition evidence reaches its claimed retention boundary.

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
grant new execution authority or duplicate provider ownership. Any read that
observes mutable state, invokes a tool, or creates a
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
than producing convergence. See [Runtime Transitions and Reasoning
Surfaces](RUNTIME.md).

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
- **mutation** — an explicit change to a declared target through an admitted
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

## Provider Boundary

Providers implement narrow Rey-owned contracts and retain ownership of their
source, query, execution, capture, and retention semantics. Rey binds exact
provider identities and advertised guarantees into evidence; it never reaches
through a public interface into private storage or upgrades local evidence into
a stronger claim.

The implemented profile is local. Any future adapter requires a concrete Rey
workload need, an accepted decision, bounded public-contract fixtures, and a
human-verifiable CLI path. No provider becomes a privileged architectural
plane by anticipation.

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

| Crate             | Ownership                                                                                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rey`             | Workload CLI, catalog/configuration composition, and user-facing orchestration                                                                                |
| `rey-core`        | identities, revisions, limits, statuses, and shared value contracts                                                                                           |
| `rey-mining`      | provider-neutral mining operation/request/result, artifact, completeness, dependency, and visualization contracts; no query engine, parser bundle, or storage |
| `rey-locator`     | canonical coordinate bindings, locator syntax, resolution outcomes, and exact resolver limits; no retrieval authority                                 |
| `rey-dataframe`   | frame metadata, Polars schemas, Arrow codecs, and bounded rendering                                                                                           |
| `rey-environment` | capability discovery, snapshots, provider contracts, and local context adapters                                                                               |
| `rey-git`         | repository identity, bounded current reachable-commit sequence, commit/ref/index frames, polling cursors, triggers, and activations                           |
| `rey-diff`        | relational, text, and structural comparison contracts, typed changes, summaries, and diff projections                                                         |
| `rey-runtime`     | workload/graph/scenario lifecycle, spaces, lenses, actions, transitions, budgets, cancellation, and trace assembly                                            |
| `rey-frontier`    | canonical frontier/progress relations, prioritization inputs, convergence evaluation, and bounded deterministic selection                                     |
| `rey-proof`       | claims, evidence manifests, certificates, verification, and staleness                                                                                         |
| `rey-policy`      | bounded reasoning surfaces plus provider-neutral proposal and admissible-action contracts                                                                     |

This table is an ownership proposal, not a requirement for one process per
crate. The narrow `rey-mining` contract crate is implemented; provider
execution remains in the adapters that own its source and tool semantics.

The browser application has a parallel internal ownership boundary. Evidence
adapters own projection-packet semantics; the Explorer engine owns immutable
scenes, fields, camera, LOD, invalidation, render graph, and picking; terrain
modules own versioned field derivations; renderer backends own graphics
resources and pixels; and React owns routing, controls, accessibility, and
evidence panels. The Three.js adapter cannot absorb Rey's scene or evidence
ownership. No new Rust crate is implied until a shared CLI/browser contract or
server-side compiler requires one.

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
claims of a local executor. Policy proposals
carry no ambient authority. Rey configuration and proof artifacts contain
references to secret handles, never secret values.

Local adapters must distinguish trusted developer input from remote or
adversarial content, remain within explicitly selected roots, and never
silently widen host filesystem access. Tool discovery is not execution
authority. An adapter must never reinterpret a local host path as a provider
resource or silently widen access.

## Current Status

The standalone capability path is implemented across `rey-core`,
`rey-dataframe`, `rey-environment`, `rey-git`, `rey-diff`, `rey-proof`, and the
`rey` composition/CLI crate. It includes bounded environment observation, a
partial read-only Git observation plus verified local cursor, pending
transition/history, and proposal-only activation contracts, verified
capability snapshot loading, an
exact capability comparator, typed structured and Arrow deltas, Tabular Diff
projection, required-capability certificate evaluation and verification, and
bounded content-addressed local proof bundles with explicit filesystem-only
guarantees. The `env` CLI now adds a verified bounded linear history of
capability snapshots: `status` derives HEAD-to-working state, `commit` accepts
one non-empty semantic revision, and `log -p` reopens exact parent-directed
environment patches. Status separates staged and unstaged working-tree rows,
interactive add confirms environment-native hunks, and new commit identities
bind explicit retention time. These environment commits are local Rey observations, not
Git objects or remotely durable revisions. `rey-runtime` implements the pure
formal state reducer through an explicit scheduling phase; `rey-frontier`
implements canonical frontier, progress, and bounded selection contracts; and
`rey-policy` implements the bounded reasoning-surface document and DataFrame
projection.
The workload slice implements a bounded workspace package catalog, typed DAG
execution, scenario deltas, exact qualification, verified local result state,
and the `list`, `status`, `test`, and `run` commands. The prior compiled
fixture catalog remains behind explicit conformance selection.
The v1 frontier, progress, scheduling, reasoning-surface, and runtime-state
contracts bind workload, graph, scenario-suite, and campaign identities.
The source-search conformance workload supplies one narrow workload-specific
frontier derivation and provider execution path. The admitted
`context-anchor-survey` workspace package adds a bounded local survey provider,
typed topography patches, directed patch deltas, and CLI/UI projections over
the retained result. The browser additionally derives bounded World geometry,
anchor-only relief, unresolved atmospheric fronts, projected hydrology and
erosion, and probe prerequisites; the CLI exposes their admitted inputs,
excluded edge provenance, projection limits, and boundary actions. Discovered
or constructed paths require a separate future evidence contract. The current
browser implementation still assembles some of those concerns through large
React/TypeScript topology and overlay modules. `rey-mining` now defines and
validates `rey.projection-packet.v1`; `rey.workload-list.v1` carries it beside
the exact patch and also carries the deterministic `rey.semantic-atlas.v1`
portfolio projection. The CLI exposes the atlas revision, region/cluster
counts, synthetic coordinate authority, reclustering rule, and the packet's
terrain evaluator, macro/meso/micro bands, absolute-coordinate validity rules,
and maximum transient working-set allocation. Explorer requires those
identities to match. The browser compiles camera-relative typed
validity/elevation/hydrology/normal/curvature/material buffers and renders
continuous relief through the Three.js WebGPU/TSL adapter with WebGL2 and
deterministic reference paths. Render-graph extraction, transient-patch reuse,
retained voyages, and a qualified terrain-fidelity result remain incomplete
[Plan 0003](../plans/0003-scene-to-explorer.md) work. Generic graph-entry
activation, persistent cross-poll activation processing, and policy proposals
remain target architecture. Compatible admissions within one
retained Git transition already reuse a directly evaluated scenario result
under exact input equality and the receiving evidence budget.

`rey-mining` also defines and verifies `rey.explore-grammar.v1`,
`rey.admitted-regional-scene.v1`, and
`rey.regional-projection-packet.v1`. Those contracts bind projection posture,
LOD, picking, camera bounds, exact editor/workload lineage, distinct native,
synthetic, Mercator, County-local, and camera coordinate planes, typed native
objects/layers, validity, limits, omissions, and explicit optional
topography/atlas/terrain relationships. Multi-region fixtures prove their
bounded structural invariants. `rey-runtime` now produces these contracts
through the qualified file-backed workload, and the CLI exposes exact human and
JSON evidence for a real `SCENE@n`. This remains incomplete enabling work: the
browser consumes none of these regional documents and no atlas history or
movement delta is retained yet.

The `rey-mining` crate now implements the provider-neutral operation, request,
result, artifact, completeness, lineage, dependency, and bound contracts.
Canonical semantic identities include evidence-changing
parameters and effective limits; replay verification rejects tampering and
request, provider, capability, or implementation drift. `rey-environment` now
implements an exact explicit local corpus binding, deterministic case-sensitive
UTF-8 literal search, native context retention, and the typed
`rey.source-matches` relation through those manifests. `rey-diff` implements
`rey.text-delta.v1` and `rey.source-match-delta.v1`; `rey-runtime` composes the
source provider and renderer in one qualified graph, derives one frontier row
from its complete failing evidence, selects it with the generic scheduler, and
projects one verified reasoning surface. The CLI exposes complete, different,
and truncated mining evidence through the workload conformance projections and
retains product execution behind workload HEAD.

The first portfolio-mining slice adds `rey.portfolio-snapshot.v1`, the
Polars-backed `rey.workload-attention.v1` relation, and the qualified
`rey.portfolio.attention` system workload. The workload CLI now exposes exact
attention actions, reasons, readiness, coverage, evidence, priority, cost, and
exclusions. Read-only list/status consume retained environment state; the
explicit conformance run evaluates the same retained portfolio inputs under
fresh qualification. Workspace packages declare bounded mapped-surface and
Git HEAD/index dependencies. List/status derive live invalidation from retained
environment evidence and the acknowledged Git cursor, then hand ready rows to
the generic frontier and one bounded reasoning surface. One selected `CREATE`
row can cross an immutable harness request/response and exact human workload
admission cycle without Rey invoking the harness. An acknowledged Git
activation can separately cross exact workload/runtime preconditions into a
content-identified admission and replay-stable selected-scenario execution.

Recurring scheduling, cross-poll activation coalescing, admitted `rg` search,
parser/index breadth, general structural delta, and a provider-specific agent
loop remain unimplemented. Those require the same human-verifiable end-to-end
boundary before they count as delivered.
