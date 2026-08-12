# Rey Glossary

This glossary defines how Rey uses its key terms. The definitions are
project-specific: a familiar word such as *commit*, *index*, *run*, or *proof*
may have a narrower meaning here than it does elsewhere.

The glossary is a navigation aid, not a new source of authority or an
implementation-status claim. The [Constitution](../CONSTITUTION.md), owning
architecture documents, accepted ADRs, active plans, and repository evidence
remain authoritative in that order. A term may describe implemented behavior,
target architecture, or both; follow its linked owning document for the exact
current boundary.

[A](#a) · [B](#b) · [C](#c) · [D](#d) · [E](#e) · [F](#f) ·
[G](#g) · [H](#h) · [I](#i) · [J](#j) · [K](#k) · [L](#l) ·
[M](#m) · [N](#n) · [O](#o) · [P](#p) · [Q](#q) · [R](#r) ·
[S](#s) · [T](#t) · [U](#u) · [V](#v) · [W](#w) · [Z](#z)

## Essential Distinctions

| Do not conflate | Rey's distinction |
| --- | --- |
| Admission and authority | Admitting evidence, a document, or a proposal records a validated object. It does not grant permission to execute a tool, run a query, or mutate a target. |
| Attention and scheduling | Portfolio mining derives *why* something needs attention. The scheduler may select admitted ready attention under limits, but cannot invent or erase its reason. |
| Discovery and execution | Finding an executable or provider records availability. It does not admit invocation or assignment. |
| Process success and semantic success | A provider process can exit successfully while the expected and observed states still differ. Post-action observation and evaluation determine semantic outcome. |
| Completeness and coverage | Completeness asks whether one observation satisfied its lens contract. Coverage asks how much of a declared domain was observed. Neither implies the other. |
| Channel observation and runtime observation | A Channel observation is a compact collaboration statement over exact evidence. A runtime observation is provider/runtime evidence such as a frame or action result; citing it does not transfer ownership to the collaboration plane. |
| Channel observation and Journal entry | An observation is a low-latency standalone frontier unit. A Journal entry is deliberate notebook synthesis that may cite and compose exact observations. |
| Channel frontier and runtime frontier | The Channel frontier projects unresolved collaboration observations for catch-up and communication. The runtime frontier derives schedulable unresolved work from typed deltas and claims. |
| Delta and rendering | The typed or native directed delta is authoritative. A patch, table, graph, metric panel, or Tabular Diff document is a bounded projection of it. |
| Environment commit and Git commit | An environment commit retains an admitted capability snapshot in Rey's local linear history. It neither creates a Git object nor mutates the discovered environment. |
| Graph order and frontier scheduling | Graph edges order nodes inside one graph execution. Frontier scheduling selects unresolved evidence between transitions or graph revisions. |
| Journal admission and compute | A Journal entry is retained collaboration context. Its query and action blocks remain inert until separately admitted through runtime contracts. |
| Qualification and universal correctness | Qualification proves that all required scenarios freshly passed for one exact workload, graph, suite, evaluator, and bounded evidence scope. |
| Staleness and failure | Failure means a predicate was conclusively false. Staleness means previously evaluated evidence no longer binds current semantic inputs. |

## A

### ADR

An Architecture Decision Record in `docs/decisions/` that preserves the
context, decision, consequences, and status of a consequential architectural
choice. Accepted ADRs constrain active plans and code; later changes supersede
rather than silently rewrite their history.

### Action

An explicitly described operation that may produce an effect. An action binds
its exact target, effect class, typed arguments, preconditions, capability
snapshot, provider, and bounds. See [Runtime](RUNTIME.md) and
[Interfaces](INTERFACES.md).

### Action admission

The runtime check immediately before an action can begin. Admission revalidates
identity, frozen preconditions, capabilities, effects, authority, and remaining
budget; stale or unsupported proposals fail before side effects.

### Activation

A content-identified match between a trigger and an exact source delta. An
activation proposes a workload, scenario selection, or graph entry point; it
still passes normal runtime admission and is replay-safe rather than
exactly-once. See [Git](GIT.md).

### Admission

Validation and acceptance of an object into a declared boundary, such as an
environment history, workload catalog, Journal log, campaign, or action queue.
The admitted object gains only the authority stated by that boundary.

### Admission index

The HEAD-bound environment snapshot staged by `rey env add`. It is the
`INDEX` in `HEAD → INDEX → WORKING`; `rey env commit` retains exactly this
verified snapshot without re-observing ambient state. See
[Environment](ENVIRONMENT.md).

### Admissible action

A versioned action contract the current reasoning surface permits policy to
propose. Being listed as admissible does not mean the action has been selected,
admitted, or executed.

### Agent

One possible policy source that can propose a compute-graph revision or action
through the same untrusted interface as a deterministic rule or human. An
agent cannot redefine evidence, bypass limits, qualify its own graph, or
declare its own attention resolved.

### Agent runtime

An application capable of hosting agent work, such as `codex`, `claude`,
`agy`, `copilot`, `droid`, or `opencode`. Rey currently discovers their
executable presence without starting them; discovery is neither assignment nor
execution authority.

### Application inventory

The exact desired set of applications Rey intends to search for, including the
purpose of each declaration. It is separate from the search record describing
what was actually found. See [Environment](ENVIRONMENT.md).

### Arrow

Apache Arrow, Rey's preferred typed interchange family for genuinely
relational data. Arrow IPC preserves schema and metadata across process or
artifact boundaries; native text and binary artifacts are not forced into
Arrow merely for uniformity.

### Architectural planes

Responsibility boundaries separating workload, context-surface, mining,
reasoning, observation, delta, runtime, and policy concerns. They clarify
ownership and data flow and do not require separate processes. See
[Architecture](ARCHITECTURE.md).

### Artifact

An addressable output or input such as native bytes, ordered text, a frame,
tree, graph, metric, delta, visualization, capture, or manifest. Its identity,
source, media type, completeness, limits, and lineage determine how it may be
used.

### Assessment

A typed semantic conclusion about evidence. Examples include delta assessment
(`equal`, `different`, `incompatible`, `inconclusive`), frontier assessment
(`open`, `converged`, `inconclusive`), and proof status. Assessments from
different dimensions are not interchangeable.

### Attention

Evidence that identifies a workload or uncovered surface requiring review or
work. Attention retains action, reason, readiness, blockers or exclusions,
citations, priority, and estimated cost as distinct fields. See
[Portfolio attention](#portfolio-attention).

### Authoritative evidence

The typed or native evidence whose exact semantics determine an assessment.
Summaries and visualizations help humans navigate it but cannot replace it or
change its meaning.

### Auto profile

The provider-selection attitude that uses discovered capabilities satisfying
the declared requirements. It does not change claim meaning or silently
strengthen local evidence.

## B

### Baseline

An exact reference observation or committed record used for comparison. During
bootstrap, a baseline is committed without fabricating a transition from an
unobserved prior world.

### Bearing

The current concrete implementation direction carried by an active plan. A
bearing identifies the next bounded anchor and proof path; it is collaboration
language, not a retained runtime state or scheduler object.

### BLAKE3 semantic digest

Rey's content identity for many semantic documents. Inputs are canonically
encoded and domain-separated before hashing. A BLAKE3 digest is not a Git
commit SHA and must not be linked or presented as one.

### Blocker

An explicit fact preventing work from being ready or preventing an eligibility
decision. Current classes include dependency, capability, evidence, budget,
unsupported, and incomplete conditions.

### Bound

A declared maximum on rows, bytes, files, matches, nodes, edges, depth, time,
memory, actions, concurrency, or iterations. Reaching a bound is observable
incomplete evidence, never silently treated as equality, readiness, or
convergence.

### Bounded search

The environment's `02 / BOUNDED SEARCH` evidence plane. It places the exact
desired application inventory beside the bounded found, missing, and errored
search record without implying that a discovered application may be invoked.

### Bootstrap

The initial runtime phase that discovers capabilities, materializes declared
observations, commits a baseline, and derives an initial frontier or explicit
stop. Bootstrap has no imaginary predecessor and cannot claim directional
progress.

### Broadcast

Planned channel admission of one immutable Channel-observation identity to an
explicit bounded set of channels. Broadcast retains channel-local admission
edges rather than copying observation content, and it grants no action,
assignment, proof, relay, or remote-delivery authority.

### Budget

The remaining bounded allowance for work, usually expressed through time,
iterations, actions, evidence bytes, or abstract cost units. Budget exhaustion
is a stop fact and may make evaluation inconclusive; it is not convergence.

## C

### Cadence

The operator projection of partially ordered clocks such as Git reachability,
Rey environment admissions, and mounted browser scans. Cadence does not invent
one total event log and does not imply that a scan schedule is runtime
scheduling.

### Camera

Explorer presentation state consisting of a center, continuous scale, and
viewport. The camera determines what the semantic lens projects but is not part
of coordinate or source identity.

### Campaign

A bounded lineage of related attempts toward an exact goal. A workload test
campaign contains graph revisions, scenario executions, deltas, reasoning, and
qualification; the outer portfolio campaign derives and re-evaluates workload
attention.

### Candidate graph

The exact compute-graph revision currently under evaluation. It remains
separate from the last fresh qualified graph, so a failing candidate does not
make an older qualification appear to have failed.

### Capability

One typed operation or guarantee advertised by a provider, with explicit
identity, availability, trust, supported semantics, and enforceable limits.
An executable name or server version alone is not a capability contract.

### Capability delta

A directed comparison between two capability snapshots. It reports provider,
tool, operation, path, version, provenance, trust, availability, and limit
changes that may invalidate dependent evidence or actions.

### Capability snapshot

The frozen typed relation of providers, tools, operations, trust, availability,
and limits available to one transition. It is a first-class runtime input and
participates in action, mining-result, reasoning-surface, and proof identity.

### Capability requirement

An exact capability or guarantee a workload, lens, action, or claim requires.
If unavailable, the dependent operation is removed, blocked, or inconclusive;
Rey never silently substitutes a weaker contract.

### Capture

Provider-owned retained output from an execution attempt. Rey cites it as
evidence and validates its completeness and media type.

### Catalog

The provider boundary that resolves workload declarations, immutable graph and
scenario revisions, drafts, and mutable selectors to exact identities. The
default product catalog is workspace packages; the compiled catalog is an
explicit conformance surface.

### Catalog provider

The provider role that resolves workload declarations, drafts, immutable graph
and scenario assets, and mutable selectors to exact identities. It does not
own retained campaign or run results.

### Certificate

A canonical assessment artifact binding a claim to exact inputs, evidence,
limits, comparator, and evaluator identities. Verification recomputes those
bindings rather than trusting a stored status. A certificate alone is
retention-neutral.

### Channel

A stable addressable workspace collaboration boundary. The implemented
built-in graph defines one workspace-local channel; later admission logs will
associate observations with exact channel revisions. A channel owns admission
and subscription scope, not observation or Journal content, runtime action, or
a universal event clock. Feed streams may subscribe to channels but are not
channels themselves.

### Channel admission

The planned association of one exact Channel observation with one exact channel
at a channel-local sequence position. It preserves observation identity and
bounds; admission does not authenticate the author or grant compute/effect
authority.

### Channel frontier

The planned bounded deterministic projection of retained Channel observations
that have not been resolved or superseded by exact later observation relations.
It represents the collaboration catch-up surface and does not replace Rey's
runtime frontier or make an observation schedulable work.

### Channel graph

The bounded relation of channel definitions, subscriptions, Feed streams,
ordered layouts, admitted communications applications, relay declarations,
and polling beacons. `rey.channel-graph.v1`, the built-in default, typed
snapshots/deltas, and the local Channel revision loop are implemented.
The graph provides addresses for separate observation-admission logs but does
not contain their high-cadence history. Its local revision loop is separate
from environment, workload, runtime, and proof identity.

### Channel index

The staged channel-graph proposal between channel HEAD and WORKING. `rey
channels add` stages the complete verified WORKING graph in this index, and
`rey channels commit` retains exactly the verified index without rereading a
graph file. Partial staging remains planned.

### Channel observation

A planned compact immutable collaboration statement retained independently
from the Journal. It binds an exact subject locator, bounded kind and statement,
desired delta or frontier relation, evidence locators, source revision, author
label, completeness/limits, and exact resolution or supersession relations.
Findings, questions, progress, blockers, and handoffs are common kinds.

### Claim

A versioned predicate over a named scope with declared evidence, comparison
direction, coverage, completeness, limits, and pass/fail/inconclusive rules.
Claims that do not reduce honestly to one comparison remain typed claim facts.

### Claim fact

A typed predicate result used when relational, text, or structural comparison
would misrepresent the evidence. Claim facts can direct frontier work beside
deltas without being flattened into an artificial mega-delta.

### Coding harness

An external agentic or deterministic system that mines context and generates a
workload package's compute graph and frozen scenario suite. Rey writes and
validates the handoff, but the deterministic runtime does not embed an LLM or
fabricate the harness response.

### Communication plane

The fixed operator-UI surface opened from the footer. Its mailbox/history and
conversation controls are separate axes over one plane; neither implies a
durable message store or agent session that Rey has not admitted.

### Commit — environment

A local Rey history record binding an `ENV@n` sequence, exact parent, message,
retention time, and admitted capability snapshot. It records an observation;
it is not a Git object, environment mutation, or execution grant.

### Commit — Git

An immutable Git object binding a tree, ordered parents, metadata, and message
under an algorithm-qualified OID. Git commits are source and cadence inputs,
not environment-history records.

### Comparator

A versioned implementation of equality or difference semantics for compatible
evidence. Its identity, keys, ordering, tolerances, and limits participate in
delta and proof identity.

### Completeness

Whether one observation or mining result satisfied its declared contract.
Common states are complete, partial, truncated, unsupported, unavailable, and
failed. A narrow complete observation may still have little domain coverage.

### Compute graph

An immutable, content-identified typed graph describing how one workload
transforms admitted inputs into outputs. Nodes cite exact operation contracts;
ports carry types; edges establish dependencies; and total graph limits are
explicit. See [Workloads](WORKLOADS.md).

### Compute graph revision

One immutable content-identified version of a workload's compute graph. Every
revision binds node, port, edge, operation, capability, effect, limit, and
generator provenance, and changing it invalidates prior scenario freshness.

### Conformance catalog

The explicitly selected compiled catalog containing system and diagnostic
fixtures. It proves Rey's contracts and is not presented as the user's product
portfolio.

### Content identity

A semantic digest derived from canonical meaning-bearing fields rather than a
mutable name or presentation. Identical verified content has identical
identity; changing a semantic input creates a new identity.

### Context

The explicitly bounded environment, sources, artifacts, revisions, and
relationships available to Rey. Context is mined into evidence rather than
copied wholesale into policy input.

### Context surface

A bounded source of information or action, such as a workspace, repository,
file, executable, runtime, or service. A provider owns how a
surface is discovered, identified, read, and acted upon.

### Context topology

The bounded typed anchors plus classified relationships Rey can currently
explain. Explorer projects it, but the topology is admitted evidence rather
than a visualization-owned graph or durable source store.

### Context topography

Context topology plus scale, density, surveyed coverage, boundaries, frontier,
and explicit unexplored space. It is incrementally composed from admitted
topography patches. Explorer may derive contour isolines from admitted anchor
samples, projected atmospheric fronts from unresolved conditions, and
projected runoff and erosion over that scalar field. Exact survey edges remain
inspection evidence rather than terrain or paths. None of the interpolation,
weather, hydrology, visual distance, envelope area, or visually empty terrain
becomes observed semantic evidence.

### Context lifecycle

The four-phase sequence `DISCOVERY → REASONING OVER DISCOVERY → SURVEY →
PROCESS`. The phases separate process-owned seeds, agent-generated relevance,
exact locator resolution, and ongoing delta/attention processing.

### Conversation axis

The communication-plane axis intended for operator ↔ Rey ↔ agent dialogue.
The current UI shows the conventional transcript/composer grammar but disables
sending because no session, transport, admission, or retention contract is
implemented.

### Coordinate

A typed provider-qualified semantic address in Rey's coordinate model for
an object or bounded region. A coordinate retains its provider, space or
namespace, native locator payload, identity class, and exact source/version
binding where available. Local bindings carry explicit guarantees;
neither form grants authority. Camera, scale, lens, and selection are view
state rather than coordinate identity. See [Explorer](EXPLORER.md).

### Coordinate query dimension

A unique named query parameter within a semantic coordinate, currently
`revision` and agent-only `role`. Canonical coordinates serialize dimensions
in stable order and reject duplicates, unknown names, and non-canonical
encodings. Explorer camera scale is not a coordinate dimension.

### Corpus

An exact bounded set of source artifacts supplied to a mining operation. The
implemented source-search corpus binds a canonical root, reversible relative
paths, native content identities, encodings, and source limits.

### Coverage

How much of a declared domain was observed, with an explicit unit and
denominator. Coverage is distinct from completeness, confidence, progress, and
proof status.

### Cursor

A retained coordinate marking the last completely processed source snapshot.
A Git poll cursor advances only after deltas, activations, and required
transition evidence reach their claimed retention boundary.

### Current truth

Behavior, schemas, and guarantees proven by the present repository, tests, and
human interface. It remains distinct from target architecture and future plan
intent; documentation must label the difference.

## D

### DAG

A directed acyclic graph. Rey's initial compute-graph contract is a finite
typed DAG; feedback occurs through bounded campaigns proposing new immutable
graph revisions, not through an implicit cycle inside one execution.

### DataFrame

A Polars DataFrame used as Rey's canonical bounded in-process coordinate system
for genuinely typed collections such as capabilities, matches, attention, and
frontier rows. It is working state, not the sole durable copy of authored or
native content.

### Delta

An authoritative directed comparison from `SOURCE` to `TARGET`, with exact
input identities, compatibility rules, limits, completeness, and assessment.
Relational, text, structural, capability, and workload-specific deltas retain
their native semantics. See [Diffs](DIFFS.md).

### Delta-directed

The property that unresolved or changed evidence determines what Rey examines
or computes next. Rey does not mine an ambient workspace merely to create a
generic prompt; it begins with selected frontier citations and expands under
declared dependencies and bounds.

### Derivation

A versioned deterministic rule that produces evidence from exact inputs, such
as a portfolio snapshot producing attention or scenario deltas producing
frontier rows. The derivation contract and its dependencies participate in
identity and replay verification.

### Desired application inventory

The canonical declaration of applications Rey intends to locate and why. Its
identity is separate from the capability snapshot that records the bounded
search outcomes.

### Directed text

The environment's `01 / DIRECTED TEXT` evidence plane: an ordered source-to-
target text comparison of located environment-variable values or states under
the selected `HEAD`, `INDEX`, and `WORKING` direction.

### Diff

The semantic act or human/machine projection of comparing two exact states.
In Rey usage, always ask which direction, comparison family, inputs, and
authoritative delta the word refers to.

### Diff-directed compute runtime

Rey's core system description: a deterministic runtime that turns context into
evidence, computes directed deltas, derives a frontier, and spends the next
bounded unit of compute on unresolved change.

### Discovery

The first context-lifecycle phase. Rey captures only process-owned `HOME`,
`PWD`, and `PATH` seeds plus declared built-in adapter observations under hard
bounds; it does not load project configuration by convention.

### Draft workload

A request-only catalog entry containing `request.yaml` but no admitted
`workload.yaml`. It remains visible as `AWAITING CODING HARNESS` and cannot be
tested or run.

## E

### Effect class

The declared authority category of an operation. The runtime distinguishes
read-only retrieval, pure deterministic projection, probe, and mutation; a
query or mining result never silently acquires a mutation effect.

### Effective limits

The actual limits enforced after requested bounds are combined with provider,
runtime, workload, and policy constraints. Effective limits participate in
semantic identity because changing them can change completeness or results.

### Editor project

A Rey-owned `rey.editor-project.v1` declaration of bounded workspace-native
scene sources, explicit feature roles, and one coordinate-system contract. It
lives at `.rey/editor/project.json` by default and is mutable WORKING state,
not admitted evidence. The declared native artifacts remain workspace files
and are not replaced by the local state projection.

### Editor planes

`HEAD`, `INDEX`, and `WORKING` are the latest immutable `SCENE@n` commit, the
exact staged candidate snapshot plus frozen native objects, and fresh
observation of an initialized editor project. Before initialization, WORKING is
absent and read-only status remains clean. Scene HEAD is retained authoring
history, but its package is not admitted evidence. `commit` packages only INDEX
and creates an unadmitted workload request.

### Environment

The explicit boundary from which Rey discovers context surfaces and available
compute. It is represented by versioned capability evidence, not by sourcing
ambient shell startup or assuming conventional project variables.

### Environment map

An explicitly supplied `rey.env-map.v1` reasoning resource that declares
relevant variables, input files, desired applications, and directed
relationships. It is agent-generatable context, not bootstrap configuration,
provider implementation, proof, or execution authority.

### Environment planes

`HEAD`, `INDEX`, and `WORKING` are the committed environment snapshot, staged
admission snapshot, and fresh observation. `rey env diff` shows
`INDEX → WORKING`; `rey env diff --staged` shows `HEAD → INDEX`.

### Environment operator projection

The shared typed variable, application, input, and reference view derived from
the same authoritative capability snapshots and deltas for `rey env` and
`/environment`. The browser does not probe the host independently.

### Environment seed

One of the process-owned `HOME`, `PWD`, or `PATH` values from which bootstrap
discovery begins. Seeds are bounded observations and do not authorize recursive
scans or shell-profile loading.

### Evaluator

A versioned implementation that turns comparison and claim evidence into a
semantic result such as scenario evaluation, qualification, progress, or proof
status. An evaluator change makes dependent retained evidence stale.

### Evidence

Retained, addressable material used to orient work or evaluate a claim. Exact
source, capability, operation, completeness, limits, and lineage determine its
meaning and strength. See [Proofs](PROOFS.md).

### Evidence address

A credential-free locator for retained evidence and its provenance. It must
not expose secrets, signed URLs, private credentials, or mutable connection
details as identity.

### Evidence bundle

A bounded manifest plus content-addressed evidence objects published under an
explicit retention profile. The implemented local bundle claims filesystem-
only guarantees and explicitly denies remote durability and process lineage.

### Execution status

The provider-process dimension: queued, running, succeeded, failed, cancelled,
timed out, or lost as applicable. It remains independent of semantic outcome,
evidence status, and proof status.

### Explorer

The human operator's primary read-only high-fidelity spatial game engine at
`/explore`, specialized for evidence-bound projections of high-dimensional
context. Its continuous patch-backed lens ranges from World through Atlas,
Landscape, Neighborhood, and Object to exact Evidence over one persistent
relief scene.
Anchors remain stable map POIs while projected weather, streams, rivers,
probes, labels, relationships, objects, and evidence enter progressively,
preserving identity, scope, omissions, and authority. Source edges do not
appear as far-map transport, and no displayed natural feature is a path.

### Explorer view

A presentation envelope over one selected coordinate: camera center,
continuous scale, viewport, projection revision, and optional selection. The
implemented link carries a percent-encoded semantic coordinate and separate
numeric scale on `/explore`; the removed matrix path is not accepted.

### Exact binding

A reference that includes the strongest immutable identity and revision
available, rather than only a mutable name or display path. Exact bindings are
required for reproducible evidence, actions, and proofs.

### Exact retrieval

A read of already identified immutable evidence through the provider that owns
it. Exact retrieval can occur during orientation without becoming a new probe,
provided it does not observe mutable state or invoke a tool.

## F

### Feed

The high-cadence human inspection plane at `/feed`. Its default independently
scrolling, TweetDeck-like streams project rich Signals, inspect-only Admission,
and observed workload Flow. The Firehose can compose up to eight URL-addressed
stream lenses without creating another store. Feed owns no event store, read
cursor, attention rows, admission authority, live telemetry, or causal order.

### Feed stream

One independently scrolling, configured lens over the Feed Firehose. A stream
selects a source plane and filter, such as `signals.journal`, `admission.now`,
or `flow.failing`, plus an optional human name. Its title is editable inline and
currently autosaves into the deep-linkable URL coordinate. The implemented
Channel graph now gives the built-in Signals, Admission, and Flow streams stable
topology identities, but the browser is not yet bound to Channel WORKING.
Repeating, naming, ordering, tuning, or removing a stream changes the human
projection only; it does not copy, admit, schedule, or mutate source records.

### Firehose

The bounded union of records already projected into Feed from Cadence, Journal,
portfolio attention, repository posture, and admitted workloads. The Firehose
rail is the configuration surface for adding and tuning Feed streams. It is not
a durable global event log, an unbounded stream, or a new runtime owner.

### Fixture

A bounded reviewed input and expected behavior used to prove a contract. A
conformance fixture is diagnostic evidence; it is not automatically a product
workload or universal coverage claim.

### Focus

The selected semantic coordinate retained while the camera scale or visual
grammar changes. Focus guides navigation but cannot mutate topology, resolve a
locator, execute a workload, or resolve attention.

### Frame

A bounded typed observation with schema, keys, space, lens, sources,
capability snapshot, limits, completeness, content identity, and lineage. An
empty frame retains its declared schema and keys; it is not an untyped absence.

### Freshness

Whether retained evidence still binds the current exact semantic inputs.
Freshness is recomputed from identities and dependencies rather than trusted as
a manually stored label.

### Frontier

A bounded, content-identified relation of unresolved work derived from exact
deltas and claims. Stable `work_id` aligns logical work across revisions;
`row_id` changes when the row's current readiness, citations, ranking inputs,
or contents change. See [Frontier](FRONTIER.md).

## G

### Git OID

An opaque algorithm-qualified Git object identifier. Rey does not assume SHA-1
width, derive identity from a shortened display SHA, or manufacture an OID for
unhashed worktree content.

### Git HEAD

The per-worktree symbolic or detached selection of a Git commit. HEAD is a Git
source/cadence coordinate and is unrelated to the `HEAD` label used for Rey's
committed environment snapshot except by deliberate analogy.

### Git provider

The context provider that owns repository, object, commit, ref, HEAD, semantic
index, reachability, and bounded worktree observations. It is separate from the
environment inventory row describing the `git` executable.

### Graph edge

A directed dependency between compatible typed ports in a compute graph. Edges
determine node execution order and data flow, not portfolio or frontier
priority.

### Graph node

A stable compute-graph unit that cites one admitted versioned operation
contract, typed inputs and outputs, capability and effect requirements, and
limits.

### Guarantee

A provider claim Rey can bind and verify about identity, completeness,
durability, isolation, enforcement, or semantics. Missing guarantees remain
visible and may make an action unavailable or claim inconclusive.

## H

### Hifi

The pinned design-grammar library used by the operator UI. Rey uses Hifi's
Kinetic interaction grammar and Precision theme as presentation; typed Rey
documents remain the source of semantic truth.

### Hard cutover

A pre-alpha change that replaces an old contract without compatibility aliases
or silent decoding under new semantics. Rey uses hard cutovers unless a plan
explicitly defines migration behavior.

### High-fidelity human interface

A CLI or UI surface through which an operator can verify inputs, progress,
results, deltas, evidence, omissions, bounds, and revision lineage without
reading implementation code. Feature slices are incomplete until this path is
present.

### Human operator

The person steering Rey primarily through the browser UI, using the CLI for
exact diagnosis or explicit runtime operations. Humans and agents propose
through bounded contracts rather than gaining ambient authority.

### Human verification invariant

The project rule that the browser UI is the human's primary collaboration
surface, the CLI is the agent's primary runtime surface, and every feature must
remain inspectable through a high-fidelity human interface backed by the same
typed evidence.

## I

### Idempotency

The property that replaying the same exact request or activation does not
create a semantically different duplicate effect or object. Idempotency is
bound to an operation-specific identity; it is not a claim of exactly-once
delivery.

### Identity

The exact value used to distinguish a semantic object. Rey prefers canonical
content identities or provider-owned immutable revisions; mutable labels and
short display forms never replace full evidence identity.

### Inconclusive

An assessment meaning available evidence or limits do not permit a sound
decision. It is neither a pass nor necessarily a conclusive failure; causes
include missing, incompatible, unsupported, truncated, timed-out, or stale
inputs.

### Inspection queue

Feed's Admission-stream projection of current signals that deserve a closer look. It
derives from authoritative attention, qualification/request posture, and local
repository state; it is not scheduler output, assignment, admission, or a new
runtime frontier.

### Index — environment

The staged admission snapshot between environment HEAD and WORKING. See
[Admission index](#admission-index).

### Index — Git

Git's per-worktree proposed tree, including logical entries, stages, modes,
object OIDs, and selected semantic flags. Rey derives semantic index identity
from those entries rather than raw file bytes, timestamps, or stat-cache-only
changes.

### Invalidation

The derivation that maps changed sources, capabilities, operations, schemas, or
dependencies onto evidence that may no longer be current. Invalidation is
conservative; later recomputation determines the new semantic result.

## J

### Journal

The retained collaboration surface shared by humans, agents, and Rey's derived
system recommendations. Entries are exact Explorer-bound typed documents;
admission orders and retains them but executes no block. See
[Journal](JOURNAL.md).

### Journal entry

An ordered, content-identified typed collaboration document bound to one exact
Explorer coordinate. Its typed blocks have one deterministic 12-column
broadsheet reading order. It has a stable identity-bearing route, may supersede
an earlier entry without rewriting it, and grants no compute or proof
authority.

### Journal broadsheet

The bounded layout grammar for a Journal entry: exactly 12 columns and 1–32
ordered bands whose cells place every typed block exactly once in semantic
reading order. Layout can juxtapose evidence and direction but cannot mint a
relationship, assessment, omission, or authority.

### Journal block

One typed unit inside a Journal entry: prose, Explore reference, read-only
query declaration, frame preview, directed diff, or proposed action. Every
block has a stable fragment permalink and explicit bounds.

### Journal seed

A planned deterministic, unretained Journal-entry proposal projected from one
or more exact Channel observations for a human or agent catching up. It cites
the selected observation identities and source revisions and becomes a Journal
entry only through normal Journal admission.

### Journey

A human-readable projection over known workflow operation states, such as
`DISCOVER → REASON → SURVEY → PROCESS` or
`ORIENT → AUTHOR → TEST → REFINE → RUN`. Journeys are derived and are not a
parallel retained state store.

### Just

The task runner behind Rey's canonical root development commands such as
`just check`, `just test`, `just build`, and `just rey`. It is a reproducible
developer surface, not part of the runtime model.

## K

### Kinetic grammar

Hifi's interaction and material grammar used by Rey UI components. It governs
presentation behavior, not runtime transitions or evidence semantics.

## L

### Level of detail

A deterministic rule selecting field resolution, scene density, render passes,
contour intervals, labels, and object grammar for a camera scale under explicit
budgets. Explorer has both semantic LOD regimes and geometric/render LOD; a
transition may change representation but not coordinate identity or source
truth.

### Lens

A versioned deterministic observation or projection definition over exact
sources. Runtime lenses materialize frames or artifact references; the
Explorer's semantic lens changes visual object grammar while preserving the
same source truth.

### Lineage

The exact chain connecting sources, capabilities, requests, operations, runs,
attempts, captures, observations, deltas, decisions, and proofs. Lineage
explains how evidence was derived and what changes make it stale.

### Local retention profile

Evidence retained in Rey's bounded local filesystem state with explicitly
narrow guarantees. It does not claim remote durability, authenticated writers,
multi-process transactionality, fenced execution, or external-service semantics.

### Locator

An exact anchor naming an object or bounded region in context. A locator does
not retrieve the object, prove it exists, or grant read or execution authority;
resolution is a separate bounded provider operation that may return a
coordinate binding or a typed unresolved outcome. See
[Locators](LOCATORS.md).

## M

### Mailbox

The history axis of the UI communication plane, populated by typed attention
and revalidation-failure evidence. A quiet mailbox means no operator attention
is requested; it is not filled with synthetic heartbeat activity.

### Mining

The bounded transformation of explicit context into navigable, addressable
evidence. Mining binds exact sources, operation semantics, parameters,
capabilities, limits, completeness, artifacts, and lineage before policy acts.
See [Mining](MINING.md).

### Mining artifact

An output referenced by a mining result: native content, relation, tree,
graph, metric, delta, or visualization. The result manifest indexes artifacts
but does not become a replacement content store.

### Mining operation

A versioned relational or source transformation with typed input/output
contracts, parameters, capabilities, effects, limits, completeness, and
invalidation rules.

### Mining request

The exact invocation contract binding workload/frontier rationale, sources or
input artifacts, operation, canonical parameters, capability snapshot,
provider, requested/effective limits, and expected output shape.

### Mining result

A manifest binding the request to realized provider/tool lineage, produced
artifacts, schemas, completeness, omissions, resource consumption, and
invalidation dependencies.

### Mutation

An explicit effect that changes a declared target through an authorized
admitted provider action. Mutation is
not mining and cannot hide behind a query path.

## N

### Native artifact

Evidence retained in its natural representation, such as source bytes,
ordered text, a patch, tree, graph, document, or binary capture. Native
artifacts may expose typed index relations for navigation without being
flattened into DataFrames.

### Natural feature

A deterministic Explorer projection produced by admitted survey-field
conditions rather than a literal source relationship. Current natural features
are unresolved weather fronts and runoff-derived streams or rivers. They are
neither observed climate nor discovered or constructed context paths.

### Nix

The pinned development and package environment defining Rey's toolchains,
supported systems, reproducible builds, applications, checks, and shells. Nix
environment setup is distinct from Rey's observed environment model.

### Neighborhood

A bounded set of topology objects around one meaningful Explorer coordinate.
The patch-backed neighborhood regime compares exact anchors and classified
relationships; the fallback portfolio regime compares admitted workloads,
creation requests, and attention rows.

### No news is good news

The operator principle that successful quiet operations and an empty
communication channel should remain quiet. Rey does not fabricate activity;
attention, delayed revalidation, and other actionable ticks are reported when
they exist.

### Normalizer

A versioned deterministic transformation that removes declared
representational variance before comparison. It cannot erase meaningful
ownership, status, causality, security, or error differences merely to produce
equality.

## O

### Object database

Git's repository storage for commits, trees, blobs, and tags under an explicit
object-format algorithm. Linked worktrees may share this database while
retaining distinct HEAD, index, and worktree state.

### Observation

A read-only materialization of current state through a declared lens or
provider contract. Observation status is separate from provider execution,
semantic outcome, and evidence retention.

### Omission

Explicit evidence that known rows, bytes, relationships, actions, or artifacts
were excluded or folded by declared bounds, unsupported behavior, or provider
failure. An omission prevents a projection from pretending to be complete.

### Operation

A versioned semantic unit that can be composed in a graph or task. Operations
declare inputs, outputs, effects, capabilities, limits, and completeness;
free-form generated code is not an operation contract.

### Operator projection

A bounded human-facing view over authoritative typed evidence. CLI tables,
environment planes, workload relations, Explorer scenes, and UI instruments
are projections and cannot introduce independent assessment or authority.

### Orientation

The bounded runtime phase that turns scheduled frontier work into a reasoning
surface. It retrieves exact read-only evidence and applies deterministic
projections until the surface is ready, no eligible evidence remains, or a
bound is reached.

## P

### Passive revalidation

The mounted UI behavior that periodically refreshes typed read-only
projections without a manual Refresh control, route invalidation, remounting,
or scroll reset. Failure retains the last good document and raises visible
communication evidence.

### Partial order

An ordering in which some ticks have proven sequence while events on unrelated
clocks remain unordered. Rey's cadence preserves Git reachability,
environment-admission sequence, and scan schedules without manufacturing one
global chronology.

### Path

A future separately admitted context-space artifact that is explicitly
discovered or constructed and binds its coordinates, method or author,
revision, authority, cost, effects, omissions, and evidence. A survey edge,
natural feature, camera selection, or visually convenient line is not a path.

### Plan

A checkable implementation slice in `plans/` containing outcome, completion
criteria, current proof, next concrete anchor, and deferred work. Plans sequence
delivery but do not outrank architecture or accepted ADRs.

### Policy

An external decision source—agent, rule, or human—that receives a bounded
reasoning surface and proposes an action or graph revision. The deterministic
runtime owns validation, execution, comparison, limits, and proof.

### Polars

The Rust DataFrame library Rey uses for bounded in-process typed relations.
Its role is local relational representation; it does not become a durable
database, native artifact store, or universal data model.

### Portfolio

The bounded set of workspace workload packages, drafts, retained results,
environment/dependency inputs, ownership, and coverage considered together.

### Portfolio attention

The canonical typed relation derived from one exact portfolio snapshot. Its
actions are `REFINE`, `RETEST`, `CREATE`, `BLOCK`, and `POLICY_EXCLUDED`; its
rows are evidence upstream of generic scheduling. See [Mining](MINING.md).

### Portfolio snapshot

The frozen bounded input to portfolio mining, binding catalog, graph,
qualification, result, environment, dependency, capability, ownership,
coverage, policy, and limit facts.

### Precision theme

The Hifi Kinetic visual theme selected for the Rey operator UI. It affects
presentation, not evidence identity or semantic assessment.

### Profile

A declared provider-selection and required-guarantee policy, such as
standalone, auto, or local retention. A profile changes
available capabilities and guarantees, not the deterministic Rey runtime.

### Probe

A read-only operation that observes mutable state, invokes a tool, or creates a
new lens result and therefore crosses ordinary proposal, admission, execution,
observation, and budget boundaries. Exact immutable retrieval and pure
projection need not become probes.

### Probe trail

The former Explorer corridor from a source anchor to an unresolved frontier
point. ADR 0043 removes this projection because it made survey extraction
scaffolding look like world geometry. The current Explorer retains the
frontier POI and its prerequisite and may show a local weather front, but draws
no crossing or source connection.

### Process

The fourth context-lifecycle phase, in which Rey incrementally consumes survey
artifacts and independent cadence ticks, derives deltas and attention, and
continues from committed transition boundaries.

### Progress

A directional assessment between compatible frontier states, reporting
resolved, introduced, updated, and unchanged work. Progress is navigation
metadata, not a universal scalar score, proof status, or process exit code.

### Projection

A deterministic bounded representation of authoritative evidence for a
specific use, such as a table, patch, frame, summary, or scene. A projection
declares selection, grouping, ordering, elision, limits, and omissions.

### Projection engine

Explorer's evidence-bound mechanism for compiling admitted high-dimensional
context into immutable scenes, bounded scalar/vector fields, semantic LOD,
render passes, picking, and browser pixels. It borrows real-time game and map
engine architecture but is not an evidence store, resolver, scheduler, or
mutation plane.

### Projection packet

The bounded typed input to Explorer's scene compiler. It binds exact evidence,
coordinate or embedding basis, scalar/vector channel semantics, typed field
dimensions and byte allocation, validity masks, world bounds, layer inventory,
implementation revisions, limits, completeness, omissions, and lineage without
becoming a second source store.

### Pure projection

A deterministic transformation over already frozen evidence that performs no
new mutable read or external tool invocation. It still binds its operation
revision and limits but need not cross a new probe boundary.

### Proposal

Untrusted structured input from policy requesting a graph revision or action.
It cites exact reasoning, frontier, evidence, precondition, and budget
identities and gains no authority until validated and admitted.

### Provider

The owner of discovery and operations for one context-surface class. A
provider defines identity, revisions, reads, effects, trust, guarantees,
limits, completeness, and errors; Rey composes providers without taking over
their source semantics.

### Provider execution

The provider-owned run or attempt that performs admitted work. Its terminal
process state is evidence but does not decide Rey's semantic transition.

### Proof

A scoped reproducible assessment that a declared claim holds for exact
observations under explicit comparison, coverage, completeness, and limits.
Rey proofs are computational evidence, not theorem proofs or universal
correctness claims. See [Proofs](PROOFS.md).

### Proof status

One of `pending`, `passed`, `failed`, `inconclusive`, or `stale`. Status is
derived from exact claim and evidence inputs; similarity, coverage, confidence,
process exit, and portfolio attention do not substitute for it.

### Publication

The act of making retained evidence visible at a claimed boundary only after
its required objects and identities are complete. Publication guarantees are
profile-specific; a successful calculation alone does not imply durable
publication.

## Q

### Qualification

The scoped result that every required scenario has a fresh, complete passing
result for the same exact workload, graph, suite, fixtures, capabilities,
comparators, evaluators, and limits. Optional scenarios remain visible but do
not gate qualification unless declared.

### Qualified graph

The exact graph revision selected by a fresh qualification record. It remains
separate from the current candidate graph and is the default graph eligible
for `rey workloads run`.

### Query

A read-only request to a provider's declared query surface. Journal query
blocks are inert declarations; actual query execution requires a separate
admission handshake and exact provider/input binding.

## R

### Readiness

A frontier or attention eligibility fact. `ready` means no blocker prevents
generic selection; `blocked` retains explicit blockers; `inconclusive` means
evidence cannot decide eligibility.

### Ref

A mutable Git name pointing to an object, including local branches, tags,
remote-tracking refs, and symbolic refs. Because refs can move backward or
sideways, Rey compares frozen snapshots rather than treating refs as append-
only streams.

### Reference plane

The environment's `03 / REFERENCE PLANE`, which renders admitted input-file
identities and directed topology relationships. It retains exact paths,
revisions, and changes without storing mapped file bytes.

### Reasoning map

The agent-generated environment map produced during reasoning over discovery.
It records judged relevance and desired anchors but does not become bootstrap
configuration or action authority.

### Reasoning over discovery

The second context-lifecycle phase, in which an agent, rule, or human examines
the frozen discovery record and may emit an explicit bounded reasoning map.

### Reasoning surface

The bounded, content-identified policy input constructed from scheduled
frontier rows and exact mined evidence. It retains deltas, claims, sources,
capabilities, omissions, admissible actions, limits, and scheduling lineage;
it is not a source store or authority grant. See [Runtime](RUNTIME.md).

### Ref movement

The directed change of a Git reference, classified as created, deleted,
fast-forward, rewound, rewritten/diverged, or unknown. Rebase and force-push
must never be presented as append-only commit events.

### Regime

One object grammar on the Explorer's semantic lens continuum: World, Atlas,
Landscape, Neighborhoods, Objects, or Evidence. Zoom can change regime without
changing underlying identity or evidence, and hysteresis prevents boundary
flicker.

### Render graph

An explicit directed order of rendering passes and intermediate resources for
one immutable scene, such as base material, hillshade, occlusion, contours,
water, weather, boundaries, POIs, labels, and selection. Render-graph output is
presentation and cannot change semantic assessment.

### Relation

A bounded typed collection with declared schema, keys, ordering, completeness,
limits, and lineage. DataFrames are Rey's in-process representation for
relations, while authored and native artifacts remain in their natural forms.

### Relay

An explicit provider-backed channel-graph edge that forwards admitted messages
to another channel. A relay freezes source/target locators, application,
authority, and hop bounds; each effect retains exact environment, message,
attempt, and delivery lineage. Defining a channel or discovering an executable
never enables transport.

### Polling beacon

A versioned Channel-graph policy over one admitted communications application
and a finite relay set. The implemented beacon command performs one bounded
tick over admitted messages, suppresses already delivered pairs, and retains
attempt evidence. It is not a daemon, heartbeat, or remote-inbox cursor.

### Relational mining

Mining over typed collections using operations such as retrieve, select,
filter, join, group, aggregate, traverse, align, compare, summarize, and
visualize. Schemas, keys, ordering, scope, and contributing lineage remain
explicit.

### Relational delta

A typed directed comparison of compatible relations under exact schema, key,
ordering, normalizer, equality, completeness, and limit contracts. It retains
inserted, deleted, and modified typed values rather than relying on a text
rendering.

### Replay verification

Recomputation of identity and semantic shape from cited exact inputs to prove
that a retained decision, delta, result, surface, certificate, or bundle was
not tampered with or silently reinterpreted.

### Residual delta

A comparison from a declared expected or baseline observation to the current
observation, describing what remains unresolved. It is separate from a
transition delta describing what changed during the action.

### Retention profile

The declared boundary and guarantees under which evidence is stored. The
implemented profile is local and may claim only guarantees it actually used.

### Required and optional scenarios

Required scenarios gate qualification; every one must freshly pass with
complete evidence for the same graph revision. Optional scenarios remain
visible review evidence but do not enter the qualification denominator unless
the workload explicitly says otherwise.

### Result provider

The provider role that retains campaigns, graph proposals, scenario attempts,
outputs, deltas, qualification records, runs, and indexes used by workload
inspection. It remains separate from catalog resolution.

### Revision

An exact semantic version of an object, operation, provider, source, graph,
suite, or contract. A revision may be a provider-owned identifier, Git OID, or
content digest; the identity family must remain explicit.

### Rey

The diff-directed mining and compute runtime defined by this repository. Rey
owns workload composition, deterministic transition and evidence semantics,
delta/frontier rationale, and local operator projection while leaving source
and execution capabilities with their owning providers.

### `rg` awareness

Bounded discovery of the `rg` executable as a possible source-mining tool. The
current identity probe does not itself admit `rg` execution or prove regex,
encoding, ignore-rule, or search-parity semantics.

### Root task

One of Rey's canonical repository commands—`setup`, `rey`, `check`, `test`,
`build`, or `fmt`—defined through Just and the pinned Nix environment.

### Run

Execution of an exact fresh qualified graph against admitted real inputs and
providers. Test and run use the same graph contract, but test binds fixtures
and safe substitutions while run binds caller-admitted real inputs.

### Runtime state

The content-identified state of Rey's formal phase machine, including active
transition, frontier, execution/observation/evaluation facts, evidence state,
limits, and stop reason. It is an artifact, not a persistence service.

## S

### Scenario

A versioned test case binding fixtures, expected observations or claims,
comparison rules, capabilities, completeness, coverage, and limits for one
workload contract. Comparisons are directed `EXPECTED → OBSERVED`.

### Scenario suite

The exact versioned collection of required and optional scenarios admitted for
one workload. A workspace package freezes the generated suite before testing;
changing it makes prior results and qualification stale.

### Scenario oracle

The frozen set of reviewed expected scenario outcomes used to qualify a graph.
Admission freezes the oracle before execution so a failing graph cannot rewrite
its own expectations during the campaign.

### Scenario evaluation

The semantic result of evaluating one scenario: pending, passed, failed, or
inconclusive. It remains distinct from provider execution state, result
freshness, workload qualification, and proof status.

### Scheduler

The deterministic mechanism that selects verified ready frontier work by
declared priority, estimated cost, stable identity, and total bounds. It does
not derive domain attention, choose an action, schedule graph nodes, or run a
recurring daemon by itself.

### Scheduling decision

The content-identified result of deterministic frontier selection. It binds
the verified frontier, committed record, capability snapshot, scheduler
contract, effective limits, selected rows, deferred rows, and cost-skipped
rows.

### Scene snapshot

An immutable, stably ordered projection-engine scene compiled from one exact
projection packet. Its semantic identity binds source evidence, projection,
field, validity, compiler, and limits while excluding camera motion and
measured frame time.

### Scene candidate snapshot

A deterministic bounded `rey.scene-candidate-snapshot.v1` observation of one
editor project: exact native source identities, explicit roles, coordinate
system and bounds, feature/POI index, coverage, completeness, omissions, and
limits. It is an editor INDEX/package input, not the projection-engine scene
snapshot used by `/explore`.

### Scene commit

An immutable linear `rey.scene-commit.v1` editor revision identified to humans
as `SCENE@n`. It binds sequence, parent, timestamp, message, and one exact scene
package reference. It is authoring history, not an admission decision.

### Scene generation recipe

A `rey.scene-generation.v1` record embedded in a generator-owned native scene
source. It binds the generator revision, source identity, seed, coordinate
bounds, and every effective geometry/effect hyperparameter required for exact
replay. Generated values remain candidate hints until admitted by a qualified
workload.

### Scene package

An immutable `rey.scene-package.v1` editor candidate binding one exact scene
candidate snapshot, native object references, parent package, and directed
change set. Candidate-only authority is part of its identity. It cannot affect
Explorer until a separate qualified workload admits derived evidence.

### Scene admission request

A content-identified `rey.scene-admission-request.v1` handoff naming one exact
scene package and the operation required to validate it. The implemented
request says `requires_workload` and `admitted=false`; it is not itself a
workload result, action admission, or proof.

### Search record

The exact target capability snapshot produced by bounded application search.
It records found, missing, or errored observations separately from the desired
inventory that requested the search.

### Semantic atlas

A bounded content-identified `rey.semantic-atlas.v1` projection over admitted
regional evidence. It preserves stable region identity and exact source
revisions while deriving world clusters and synthetic spherical placement from
an explicit compiler, policy, limits, completeness, omissions, and lineage.
The current workload-list atlas is reproducible from retained admission state;
retained prior revisions and movement deltas remain planned.

### Semantic spherical coordinate

The namespaced `semantic_longitude` and `semantic_latitude` placement of an
admitted region on Rey's synthetic sphere, stored in integer microdegrees. It
wraps like a globe but has no Earth CRS, physical-distance meaning, geographic
area, or general semantic-similarity authority. Native semantic coordinate
identity and camera state remain separate.

### Semantic convergence

The evaluated state in which the applicable frontier is conclusively empty,
required claims are satisfied, observations are complete, and evidence is
retained or verified. Empty or truncated output, process success, or budget
exhaustion alone cannot establish convergence.

### Semantic identity

Identity derived only from fields that can change an object's meaning.
Presentation-only color or incidental wall time is excluded unless the
contract explicitly makes it semantic.

### Semantic outcome

The runtime dimension describing unresolved, progressing, unchanged,
regressing, converged, or inconclusive work after evaluation. It is orthogonal
to provider execution and evidence-retention state.

### Semantic index

A meaning-bearing index over source or repository structure. For Git, it is
the ordered logical staging entries rather than index-file metadata. For code
mining, it may mean symbols and references with exact parser/source lineage;
the owning contract must disambiguate the term.

### Source binding

An exact provider-owned identity and revision for input data or content. Local
paths become reproducible bindings only when combined with workspace,
worktree, Git, or content identity.

### Source mining

Mining over ordered text, code, configuration, logs, documents, and native
artifacts through retrieval, search, segmentation, parsing, indexing,
traversal, metrics, comparison, and visualization. Rich derived structure
retains links to exact native spans and source revisions.

### Source and target

The explicit direction of every comparison. Deletions exist only in source,
insertions only in target, and modifications align the same entity across
both. Scenarios usually label them `EXPECTED` and `OBSERVED`.

### Source-match delta

The implemented typed relation delta aligning reviewed and observed source
matches by reversible path identity and byte span. It preserves exact match,
context, request, result, and completeness lineage; incomplete mining makes
the comparison inconclusive.

### Space

A named versioned boundary over sources, lenses, actions, claims, policy,
environment requirements, mutations, and limits. Workloads compose spaces and
other runtime concepts so users need not orchestrate them independently.

### Stale

Previously evaluated evidence whose bound semantic inputs no longer match the
current exact sources, capabilities, operations, graph, suite, evaluator,
normalizer, policy, or effective limits. Staleness is derived, not manually
toggled.

### Standalone profile

Rey's local-only operating mode over explicitly selected built-in and local
providers. It remains useful while disclosing narrower identity, durability,
query, isolation, and execution guarantees.

### Stop reason

The explicit cause for ending or pausing a runtime transition, such as
convergence, budget, cancellation, timeout, evidence, eligibility, capability,
inconclusive state, or failure. It must agree with the evaluated semantic and
evidence state.

### Structured output

A stable bounded machine document or relation emitted without human progress
or diagnostic text. JSON preserves mixed envelopes, Arrow preserves relational
schemas, and raw/native artifacts retain their own media types.

### StyleX

The UI styling system used to compile authored Rey application rules into a
deterministic layered atomic stylesheet. StyleX owns presentation structure;
Kinetic values remain typed runtime material properties and Rey evidence
remains authoritative.

### Structural delta

A directed comparison of declared tree or graph entities under explicit
identity, parent/edge, ordering, parser/index, alignment, and completeness
rules. Moves, modifications, edge changes, and unresolved alignment remain
distinct.

### Subscription

A planned bounded selection over exact channels, observation kinds, filters,
and limits. The built-in graph now declares the first local subscription; its
future observation projection is not implemented. A Feed stream projects a
subscription through a human visual lens; the subscription does not copy
observations or establish order across channel sequences.

### Survey

The third context-lifecycle phase, in which canonical locators are resolved to
exact anchors under explicit provider, source revision, authority, limits,
completeness, and error evidence.

### Survey voyage

A bounded series of admitted survey-workload runs over a declared context
frontier. A voyage incrementally produces topography patches; it is a derived
operational description rather than a new scheduler, store, or implicit
recursive crawler.

## T

### Tabular Diff

The Frictionless Data Tabular Diff 0.8 portable rendering used for compatible
tables. It displays schema, insertion, deletion, modification, reorder, and
context markers, while Rey's structured typed delta remains authoritative.

### Task

A bounded current coordination envelope over intent, one named operation,
artifact references, desired delta, readiness, and assignment. Tasks organize
agent collaboration without becoming an unbounded parallel artifact store.

### Test campaign

The bounded workload campaign that freezes capabilities, evaluates exact graph
revisions against a scenario suite, retains `EXPECTED → OBSERVED` deltas,
derives unresolved work, and either qualifies the graph or stops with an
explicit non-qualification reason.

### Terrain field

A bounded multiresolution scalar/vector field with explicit channel semantics,
normalization, surveyed-validity mask, revision, limits, and omissions. Height,
rainfall, flow, erosion, normals, curvature, and material channels remain
distinct; smoothing and shading do not turn unknown cells into evidence.

### Topography patch

A content-identified survey-workload result containing exact coordinate
anchors, classified relationships, surveyed regions, coverage, frontier,
omissions, completeness, lineage, and a directed delta against a selected
prior map revision. Explorer deterministically projects admitted patches; it
does not author them.

### Text delta

A directed comparison of ordered text under exact source identities,
encoding, newline, segmentation, normalization, context, and bounds. The
native patch is a human projection; structured spans and artifact references
remain authoritative.

### Tick

One observed change or scheduled observation on a particular cadence clock,
such as a reachable Git commit, Rey admission, or mounted passive scan. Ticks
from independent clocks are only partially ordered unless evidence proves more.

### Trace

The bounded graph connecting workload, source, capability, proposal, action,
run, observation, delta, frontier, decision, evidence, and proof identities.
Trace lineage explains causality without becoming a global event store.

### Transition

One admitted traversal through scheduling, orientation, proposal, admission,
execution, observation, evaluation, and commit. Every event cites the same
active transition identity and preserves non-happy-path evidence.

### Transition delta

A comparison from pre-action to post-action observations describing what
changed during one transition. It is independent of the residual delta that
describes what remains relative to an expectation or baseline.

### Trigger

A versioned predicate mapping a typed source delta to an exact workload,
scenario selection, or graph entry point. A trigger match produces an
activation proposal and never bypasses runtime admission.

### Trust class

A provider's declared trust and enforcement posture, such as trusted local
process or stronger provider-backed isolation. Rey records the actual class;
it does not call a local executor a sandbox without proof.

### Typed delta

A delta that preserves logical schemas, keys, ordering, source/target labels,
and typed before/after values. Textual or tabular renderings may combine values
for review but cannot become the only representation.

### Typed empty relation

A relation containing zero rows while preserving its exact schema, keys,
lineage, limits, and completeness. It can mean “no result under these exact
inputs,” not universal absence or convergence.

## U

### Upstream publication state

The exact local Git relation between `HEAD` and a retained local upstream ref,
including ahead/behind reachability and per-commit `pushed`, `local`, or
`unknown` classification. It performs no network fetch and is not a claim
about current remote-host state.

## V

### Validity mask

The per-region or per-field-cell distinction between sampled, surveyed-empty,
unexplored, omitted, stale, unsupported, truncated, and frontier state. A
renderer may feather a mask boundary visually but cannot infer values across
it or hide the underlying completeness class.

### Visualization

A bounded mining projection such as a table, patch, tree, graph, timeline,
metric panel, or Explorer scene. It declares source evidence, selection,
grouping, ordering, aggregation, elision, limits, omissions, and deep links;
layout and color cannot change semantic assessment.

## W

### World geometry

Explorer's farthest read-only projection. The current World places admitted
regional topographies and their major POIs on the revision-bound synthetic
semantic sphere. Regional charted land and survey horizons remain available in
closer lenses. World is not an Earth map, physical-distance metric, global
survey-completeness claim, or statement about the unknown context universe.

### Workload

Rey's public unit of computation. A workload binds one versioned compute graph,
scenario suite, environment requirements, admitted operations, policy boundary,
claims, effects, qualification rules, budgets, and retention requirements so
users can create, list, test, run, and inspect one coherent object.

### Workload attention actions

The canonical portfolio actions are:

- `REFINE` — revise a workload whose current result conclusively differs;
- `RETEST` — reevaluate a workload whose bound inputs changed or evidence is
  stale;
- `CREATE` — create a workload for a relevant uncovered surface;
- `BLOCK` — retain an unavailable or inconclusive prerequisite; and
- `POLICY_EXCLUDED` — record explicit portfolio policy exclusion without
  presenting it as convergence.

### Workload creation request

The immutable `request.yaml` handoff created by `rey workloads create`. It
binds workload identity, purpose, bounded intent, target package, generation
requirements, limits, and exact next action without fabricating a graph,
scenario, or oracle.

### Workload package

An untrusted `rey.workload-package.v1` proposal at
`sys/*/workload.yaml` binding typed ports, compute graph, frozen scenario
suite, generator provenance, and source identity. It owns no admission
decision.

### Workload HEAD, INDEX, and WORKING

The workspace workload admission planes. WORKING is the current verified
agent-authored package catalog. INDEX is the exact content-addressed snapshot
frozen by `rey workloads add` and qualified by `test --staged`. HEAD is the
newest human-approved, parent-linked workload commit and is the only workspace
catalog `run` may execute.

### Workload ownership

An exact declaration that a workload is responsible for a mapped context
surface. Portfolio mining uses ownership plus coverage and revisions to avoid
duplicate creation and to derive `CREATE` or `RETEST`; general ownership syntax
remains active plan work.

### Workspace package catalog

The default product catalog rooted at the workspace-relative `sys/`
directory. Immediate child directories contain either a creation request draft
or a workload package proposal under strict path, symlink, count, and byte
bounds. Admission is retained separately in workload history.

### Working environment

The fresh bounded capability observation used as `WORKING` in the environment
admission loop. It is observed by status, diff, or add and is not itself
retained as HEAD until staged and committed.

### Workspace ignore file

The optional bounded `.reyignore` contract at the workspace root. Each
`kind: pattern` rule narrows a typed WORKING observation with case-sensitive
`*`/`?` matching. Relevant rules, source identity, and match counts are retained
as explicit omissions in the resulting workload or environment snapshot; the
file never mutates already-retained HEAD or INDEX state.

### Worktree

The filesystem checkout associated with a Git repository. Linked worktrees may
share object storage and refs while retaining distinct HEAD, index, and
worktree state.

## Z

### Local-only

The requirement that Rey's deterministic foundation remains useful from local
evidence. Local-only evidence explicitly carries local guarantees and never
claims remote durability, query, fencing, or lineage semantics.

## Related Documents

- [Architecture](ARCHITECTURE.md) — system planes, data model, and ownership.
- [Environment](ENVIRONMENT.md) — discovery, capabilities, maps, and
  environment history.
- [Mining](MINING.md) — operations, requests, results, artifacts, and nested
  mining loops.
- [Workloads](WORKLOADS.md) — packages, graphs, scenarios, campaigns, and
  qualification.
- [Runtime](RUNTIME.md) — lifecycle phases, transitions, and reasoning
  surfaces.
- [Frontier](FRONTIER.md) — unresolved work, progress, readiness, and
  scheduling.
- [Diffs](DIFFS.md) — comparison families, direction, and projections.
- [Proofs](PROOFS.md) — claims, evidence, verification, and staleness.
- [Git](GIT.md) — repository identity, polling, triggers, and activations.
- [Explorer](EXPLORER.md) — semantic lenses, coordinates, and UI projection.
- [Journal](JOURNAL.md) — retained collaboration documents and authority.
- [Locators](LOCATORS.md) — survey anchors and resolution.
- [CLI](CLI.md) — command philosophy, revision loops, output, and exit behavior.
- [Interfaces](INTERFACES.md) — policy, provider, persistence, HTTP, and operator surfaces.
