# Rey

Rey is a diff-directed mining and compute runtime for agents.

It inventories the context available where it is running, mines bounded
structure from data and source surfaces, computes directed deltas, and spends
the next unit of attention or compute on unresolved change. Users organize that
loop as workloads: versioned compute graphs qualified by exact scenarios whose
failing evidence directs the next graph revision.

Rey is built around a practical view of programming: a programmer makes
progress by mining their environment. They locate evidence, retrieve the exact
parts that matter, expose useful structure, compare it with an expectation or
prior revision, visualize the result, and act on the smallest meaningful
frontier. Code generation is one possible action near the end of that loop; it
is not the loop itself.

## Mining As Applied Programming

Rey treats **mining** as the bounded transformation of context into navigable,
addressable evidence. It brings two primary capability families together:

| Capability family | What it mines | Representative operations |
| --- | --- | --- |
| **Relational mining** | Typed records, tables, events, measurements, and graph relations | retrieve, select, filter, join, group, aggregate, align, traverse, diff, summarize, visualize |
| **Source mining** | Text, code, configuration, logs, documents, and native artifacts | search, slice, tokenize, parse, index, walk ASTs/CSTs, resolve symbols and references, calculate metrics, diff text or structure, visualize |

The families meet through exact projections. Search matches can become a typed
relation. Symbols and references can become node and edge frames. A relational
delta can lead back to a precise source span. A text or syntax delta can
invalidate grouped metrics or dependency relations. Neither family is reduced
to the other: source bytes remain source bytes, while DataFrames remain the
canonical local coordinate system for genuinely typed collections.

Visualization is part of mining, not decoration added afterward. A table,
patch, tree, graph, metric panel, or workload progress view is a bounded
projection of authoritative evidence. It preserves direction, scope,
completeness, and deep links to exact sources; color never carries unique
meaning.

## Explorer As A High-Dimensional Game Engine

`/explore` is Rey's high-fidelity spatial game engine for evidence-bound
projections of high-dimensional context. Its immutable scene, coordinate
transforms, camera, semantic level of detail, bounded field simulation,
materials, ordered render passes, labels, picking, and incremental invalidation
turn admitted context into a world a human can navigate.

The engine is dimension-agnostic but cannot invent a dimensional meaning. A
high-dimensional provider must bind its exact input, projection or embedding
basis, implementation revision, parameters, normalization, distortion,
validity, limits, and omissions before Rey may present distance or terrain as
semantic evidence. The current standalone anchor layout remains a synthetic
orientation projection with explicitly narrower claims.

The fidelity target is a continuous Google-class 2.5D terrain surface: sampled
height and validity fields, multiscale detail, normals, multidirectional
hillshade, ridge and valley shading, restrained terrain tint, contours,
hydrology, weather, POIs, labels, and evidence overlays composed coherently
across zoom. “Google-class” describes perceptual terrain legibility, not use of
Google data, styling, or proprietary algorithms.

Rendering remains subordinate to evidence. Shading, erosion, weather, water,
visual proximity, and animation are projections; they cannot create a source
relationship, path, assessment, read authority, or surveyed claim. See [ADR
0044](docs/decisions/0044-explorer-projection-engine.md) and [Plan
0020](plans/0020-high-fidelity-projection-engine.md).

Explorer is the read-first projection side of a level-editor architecture. The
agent-facing `rey editor` CLI assembles bounded native survey sources into
WORKING, freezes exact objects in INDEX, and emits immutable scene packages.
Those packages remain candidate-only and cannot affect `/explore` until a
separate qualified workload admits them. The first implemented adapter accepts
RFC 7946 GeoJSON features and marker POIs in OGC CRS84; detailed raster terrain,
GeoPackage, GeoTIFF/COG, Arrow, and provider-qualified semantic charts remain
planned adapters. See [ADR
0046](docs/decisions/0046-read-first-scene-editor.md) and [Plan
0021](plans/0021-read-first-scene-editor.md).

## Context Lifecycle

Rey keeps four phases separate:

1. **Discovery** starts inside the compiled process from only `HOME`, `PWD`,
   and `PATH`. Declared built-in adapters may perform bounded identity
   discovery; no project configuration file or Spoke variable is assumed.
2. **Reasoning over discovery** gives that frozen record to an agent, rule, or
   human. A coding harness may emit an explicit `rey.env-map.v3` reasoning
   resource describing useful variables, input files, desired applications,
   and relationships.
3. **Survey** uses canonical locators to anchor exact environment, worktree,
   Git, workload, and provider objects without conflating identity with
   retrieval or authority.
4. **Process** incrementally consumes surveyed artifacts and independent
   cadence ticks, derives deltas, and raises typed attention.

See [Environment and Capabilities](docs/ENVIRONMENT.md),
[Locators](docs/LOCATORS.md), and [ADR 0032](docs/decisions/0032-seed-discovery-survey-and-live-communications.md).

## Agent Collaboration

Rey separates the runtime that may collaborate from the work it may eventually
perform. Process discovery searches a fixed major agent-runtime inventory:
`agy`, `claude`, `codex`, `copilot`, `droid`, and `opencode`. Found means only
that bounded PATH resolution found an executable; discovery does not start the
agent CLI and does not admit assignment or execution.

A **task** is the bounded current coordination envelope over intent, one named
operation, artifact references, a desired delta, readiness, and agent
assignment. Known workflow operations organize tasks without creating another
artifact store:

```text
CONTEXT   DISCOVER → REASON → SURVEY → PROCESS
WORKLOAD  ORIENT → AUTHOR → TEST → REFINE → RUN
```

Journeys are human projections over those operation states. They are not
retained alongside workloads, attention, evidence, and cadence. See [ADR
0034](docs/decisions/0034-agent-runtime-inventory-and-derived-task-plane.md).

The Environment surface owns runtime discovery. `/agents` operates one level
up: it ranks evidence-backed recommendations over current requests and
attention, then summarizes the work Rey can prove from retained tests, runs,
mining outputs, and deltas. It does not repeat executable inventory or claim
live agent activity. See [ADR
0035](docs/decisions/0035-agent-recommendations-and-observed-work.md).

## The Runtime Loop

```text
                    explicit environment boundary
          workspace · Git · tools · runtimes · optional Spoke
                                  │
                       inventory capabilities
                                  │
                    ┌─────────────┴─────────────┐
                    ▼                           ▼
           relational mining              source mining
       query · group · traverse       search · parse · index
                    └─────────────┬─────────────┘
                                  ▼
                bounded projections and native evidence
                                  │
                     compare SOURCE → TARGET
                                  │
                     typed and native deltas
                                  │
               ┌──────────────────┴──────────────────┐
               ▼                                     ▼
       unresolved frontier                      scoped proof
               │                               and lineage
               ▼
       bounded work selection
               │
               ▼
      mine exact relevant evidence
               │
       delta-directed reasoning surface
               │
               ▼
     propose → admit → probe or mutate
               │
               ▼
        observe and compare again
```

The delta is not merely a report produced after work finishes. It is a runtime
control signal. Changed rows, spans, symbols, edges, metrics, claims, or
capabilities invalidate dependent observations. Unresolved differences become
a frontier. The runtime mines only the evidence needed to orient on selected
frontier work, then measures whether the next admitted action actually reduced
the residual delta.

This creates a bounded inner loop:

```text
delta → frontier → schedule → mine → reason → propose → act → observe → delta
```

That loop is nested inside an ongoing portfolio loop. Workloads mine their
domains, while Rey mines the workload catalog, retained scenario results,
environment/dependency revisions, capabilities, and coverage to discover which
workloads need attention or which relevant surfaces have no workload yet:

```text
catalog + results + environment + coverage
  → portfolio snapshot → workload attention
  → RETEST | REFINE | CREATE | BLOCK | POLICY_EXCLUDED
  → admitted work → test → observe portfolio again
```

The resulting attention relation is evidence, not a scheduler guess. It keeps
ready, blocked, and explicitly excluded rows distinct. The generic scheduler
may later select bounded ready rows; an agent may propose a change for a
selected row; neither may declare the row resolved without re-evaluation.

Process completion alone is not progress. A command may exit successfully
while the semantic frontier is unchanged or worse. Conversely, reaching a
limit, losing evidence, or encountering an unsupported parser is not
convergence; it is an explicit incomplete or inconclusive result.

## Workloads Are The Public Unit

A workload composes the mining and compute contracts needed to pursue one
purpose. It binds:

- an immutable typed compute-graph revision;
- admitted relational and source mining operations;
- exact context surfaces and capability requirements;
- required and optional scenarios;
- expected observations, comparison rules, and claims;
- policy, effect, qualification, and staleness rules; and
- graph, traversal, evidence, time, byte, and iteration limits.

An agent, deterministic rule, or human may propose a graph revision through
the same untrusted contract. Rey validates its operations, inputs, types,
capabilities, effects, and bounds. The runtime executes the graph and computes
scenario deltas; the proposer cannot declare its own graph qualified.

The default catalog is the workspace's `workloads/` directory. Each accepted
`rey.workload-package.v1` binds a generated graph, a generated but frozen
scenario oracle, exact harness provenance, and its package source revision.
`rey workloads create` adds the missing agentic entry point: it writes a
content-addressed `request.yaml` for an external coding harness and exposes the
workload as a draft until that harness materializes an admitted package. Rey
does not fabricate a graph or scenario oracle in the request step.
Compiled fixtures are an explicit diagnostic surface selected with
`--catalog conformance`; they are not presented as the user's portfolio.

The product surface stays intentionally small:

```text
rey channels list
rey channels status
rey channels diff
rey channels apply <channel-graph.yaml>
rey env status
rey env diff
rey env add [-p]
rey env commit -m <message>
rey env log [-p] [-n <count>]
rey editor init --id <project>
rey editor import <source.geojson> --id <source> --role <role>
rey editor status
rey editor diff [--staged]
rey editor add
rey editor validate
rey editor package
rey editor inspect <package-id>
rey workloads create <workload-id> [--title <title>] [--intent <intent>]
rey workloads list
rey workloads test [<workload-id>] [-v|-vv]
rey workloads run <workload-id> --input <utf8>
rey workloads status [<workload-id>]
rey workloads --catalog conformance list|test|run|status ...
rey journal add <proposal.yaml>
rey journal list
rey ui [--host <ip>] [--port <port>]
```

Mining operations are capabilities used inside workload graphs and reasoning
surfaces. They are not a second hierarchy of top-level resources that users
must manually orchestrate.

## Try The Executable Slice

Rey is pre-alpha, but the standalone foundation and first workload slice are
executable:

```sh
nix develop
just setup
just rey channels list --format table
just rey channels status --format table
just rey channels diff --format table
# After authoring a workspace-contained rey.channel-graph.v1 resource:
just rey channels apply path/to/channel-graph.yaml --format table
just rey env status
just rey env diff
just rey env add -p
just rey env add
just rey env commit -m 'accept local toolchain'
just rey env log
just rey env log -n 3
just rey env log -p
just rey editor init --id semantic-atlas --format table
# After creating an RFC 7946 survey.geojson with stable feature ids:
just rey editor import survey.geojson --id anchor-pois --role markers --format table
just rey editor status --format table
just rey editor diff --format table
just rey editor validate --format table
just rey editor add --format table
just rey editor package --format table
just rey workloads create api-drift \
  --title 'API drift mining' \
  --intent 'Mine authoritative API behavior and formalize its graph and scenarios' \
  --format table
just rey workloads list --format table
just rey workloads test rey.portfolio.label-normalization --format table -vv
just rey workloads run rey.portfolio.label-normalization --input ' refine ' --format table
just rey workloads --catalog conformance test rey.fixture.text-mismatch --format table -vv
just rey workloads --catalog conformance test rey.fixture.source-search --format table -vv
just rey workloads --catalog conformance test rey.portfolio.attention --format table -vv
just rey workloads --catalog conformance run rey.portfolio.attention --format table
just rey workloads --catalog conformance run rey.fixture.source-search \
  --input evidence \
  --source crates/rey-environment/tests/fixtures/source-corpus/alpha.txt \
  --format table
just rey journal add path/to/agent-entry.yaml
just rey journal list
just rey ui
# Explicit network exposure; Rey warns that reachable clients can write unauthenticated Journal documents.
just rey ui --host 0.0.0.0 --port 5714
```

`rey ui` opens the human operator's primary collaboration surface. `/`
redirects to `/explore`, a full-screen-capable context-topology canvas whose
semantic lens moves from bounded world geometry and anchor relief, through workload and attention
neighborhoods, into exact graph, scenario, evidence, dependency, and directed-
delta objects. Exact locations separate a provider-qualified semantic
coordinate such as
`rey+local://agent/codex?revision=gpt-5&role=coding_harness` from continuous
numeric scale in the `/explore?coordinate=...&scale=...` browser envelope;
stale revision bindings remain visible instead of drifting silently. The old
matrix path is rejected with no compatibility parser.
With that address boundary in place, Explorer now projects an incremental
context-topography map. The admitted `context-anchor-survey` workload begins
from bounded project seeds such
as `AGENTS.md` and README variants, locate URI/reference candidates, resolve
them under explicit authority and limits, and emit typed topography patches
with coverage, frontier, omissions, lineage, and directed deltas. A continuous
camera projects the same admitted evidence as terrain-style isolines shaped by
admitted anchor samples, with anchors retained as stable points of interest.
World scale adds charted-land and probe-horizon geometry. Unresolved conditions
form projected weather fronts; deterministic rainfall and downslope
accumulation over the anchor-only field may carve streams, rivers, and visible
erosion. These natural features are projections rather than source edges,
observed climate, or discovered paths. Zooming in progressively adds labels,
relationships, objects, and exact evidence to that same scene. Current local
relief is not language similarity. Unexplored space stays unknown, and moving
the canvas never launches a hidden crawl or builds a path. See
[ADR 0041](docs/decisions/0041-continuous-coordinate-topography.md), [ADR
0042](docs/decisions/0042-world-geometry-and-probe-navigation.md), and [Plan
0018](plans/0018-world-context-navigation.md), as corrected by [ADR
0043](docs/decisions/0043-emergent-natural-features-and-separate-paths.md) and
[Plan 0019](plans/0019-emergent-context-features.md). `/feed` comes first in
primary navigation as the high-cadence inspection plane. Its default
composition is three TweetDeck-like vertical streams: rich Signals,
inspect-only Admission, and observed workload Flow. A quiet Firehose rail adds
new streams or tunes, reorders, and removes existing ones. Each stream is a
bounded lens—Journal-only Signals, NOW-only Admission, and failing-workload
Flow can coexist without copying their source records. The composition is
encoded in the `streams` URL parameter for deep linking and is capped at eight
lanes. Clicking a stream title edits its optional name inline; blur or Enter
autosaves the name into that same URL coordinate. Evidence bodies stay
collapsed until requested so the feed remains scannable. Git,
environment-admission, and Journal posts retain exact evidence without
presenting their display order as causality or a durable global event log.
Admission does not move work into Flow until a later explicit validated browser
contract exists. `/agents` is the shared Journal index:
retained human and agent notebook entries sit beside
current derived system recommendations without being confused for assignments
or execution. New entries use `/journal/new`; selecting or admitting an entry
enters its exact `/journal/{slug}` document, and each typed block has a stable
fragment permalink. Its second section is an observed-work ledger over retained
portfolio evidence. Agent applications remain on `/environment`.
`/cadence` presents Git reachability, Rey environment admissions, and
mounted passive scans as separate partially ordered tick lanes rather than a
fictional global event log. Every displayed Git commit SHA is itself an exact
GitHub commit link when the source repository is bound; Rey exposes an unbound
repository boundary instead of rendering an inert or guessed SHA. Semantic
digests and non-Git revisions are never linked as commits. `/environment`
projects the same bounded
`HEAD → INDEX → WORKING` environment document as the CLI through exactly three
stacked evidence sections: `01 / DIRECTED TEXT` for variables,
`02 / BOUNDED SEARCH` for applications, and `03` `REFERENCE PLANE` for inputs
and topology. `/workloads` projects admitted revisions and creation requests
as two Hifi dense evidence tables. Journey, qualification, freshness, scenario
conformance, graph/test identity, mining output, attention, request intent,
admission, source, and target remain aligned instead of being folded into
cards; each row links to its exact workload or handoff detail. Exact admitted
details continue the grammar through runtime posture, binding, and mining
relations. Exact creation-request details use request posture and binding
relations so the coding-harness boundary remains explicit. The manual Refresh
control is gone: the read-only workload, Feed, Cadence, Journal, and environment
documents passively revalidate every five seconds in mounted application state.
Revalidation never navigates, remounts the active route, or changes the
operator's scroll position.

The fixed footer addresses one communication plane along two axes. `MAILBOX`
opens runtime and attention history; it is currently bounded to the latest
mounted projection because Rey has no durable mailbox store. The center
chevrons open a traditional operator ↔ Rey ↔ agent transcript and composer.
No conversation transport or admitted agent session exists yet, so that axis
states the boundary and disables sending instead of fabricating a chat.

The server embeds a TanStack Router application expressed with
[Hifi](https://github.com/rupurt/hifi)'s Kinetic grammar and Precision theme.
All authored application styles are StyleX modules compiled by the official
StyleX Vite integration into one deterministic atomic stylesheet; runtime
Kinetic material values remain typed custom properties.
The server defaults to `127.0.0.1:5714`, accepts explicit `--host` and `--port`
values, and reports its exact exposure before serving. Its data projections
are read-only. Its one write admits validated human Journal entries without
authentication on every explicit bind; a non-loopback bind therefore warns
that every reachable client can write. Journal admission remains separate from
compute and proof authority. See
[Context Topology Explorer](docs/EXPLORER.md) for lens and coordinate semantics
and [Collaboration Journal](docs/JOURNAL.md) for the notebook block and
admission contracts.

The default catalog contains the checked-in, coding-harness-generated
`rey.portfolio.label-normalization` package. Its exact YAML graph and frozen
scenario suite are visible at
[`workloads/portfolio-label-normalization/workload.yaml`](workloads/portfolio-label-normalization/workload.yaml).
Rey imports this accepted package. `workloads create` now produces the strict
request side of the external harness handoff; harness response admission and
automatic campaign continuation remain the next boundary.

The explicit conformance catalog contains four executable diagnostic workloads. Two graphs cover
deterministic UTF-8 `trim` and `uppercase`. The source-search graph composes
`rey.source-search.literal-utf8` with a deterministic match renderer over an
explicit bounded corpus. Its required empty and exact scenarios qualify the
graph; optional mismatch and truncation scenarios retain a typed match
relation delta, ordered line delta, omissions, one scheduled frontier row, and
a bounded reasoning surface. `run --source` executes that same qualified graph
against caller-selected files below the workspace.

The `rey.portfolio.attention` system workload mines a frozen portfolio snapshot
into the canonical `rey.workload-attention.v1` relation. Its required scenarios
cover refinement, retesting, workload creation, blockers, explicit policy
exclusion, and a clean typed-empty result. `workloads list` and `status` derive
the current attention view without probing ambient state; after qualification,
`workloads --catalog conformance run rey.portfolio.attention` evaluates the compiled catalog,
retained workload evidence, and retained environment HEAD/admission index.
Mapped input files without a declared workload owner appear visibly as
`CREATE` candidates.

`env status` is the compact working-tree view. It observes the working
environment, identifies the current `ENV@n` revision, then separates
environment-native objects into changes staged in the admission index and
changes not yet staged. A clean working environment renders only that revision
coordinate and the clean result. Workspace, observation, application-search,
and reasoning-map summaries do not pad the human status surface; complete
evidence remains in structured status, while exact values, search records, and
topology remain in `env diff`. Authoritative capability
changes that do not map to an operator object remain visible as individually
named typed entries with their exact capability ids. `env diff` selects the three environment planes for
`INDEX → WORKING`; `env diff --staged` selects them for `HEAD → INDEX`.
Its human output is directed variable text, bounded application search, then
input/reference topology—not a generic capability patch. The header still
reports the authoritative capability-delta assessment and change count;
`--format json` retains the complete `rey.environment-diff.v4` evidence.
`env add` stages the complete working snapshot, while `env add -p` presents
each canonical capability change as a confirmable environment-variable,
application, input, or reference hunk. Generic capability hunks never print raw
structured provenance; exact machine evidence remains in `env diff --format
json`.
`env commit` appends exactly the retained admission index beneath `.rey/env`
without re-observing ambient state. New commits bind their commit time into a
v2 identity. A successful default commit is silent: use `--format json` when a
machine receipt is required, or `env log -n 1` for human readback. Errors remain
diagnostic and nonzero. `env log -n <count>` bounds a newest-first chronology whose header
shows `ENV@n`, semantic commit and parent ids, date, and message; legacy v1
commits explicitly have an unknown date. Evidence, environment scope, changed
dimensions, and mapping follow. `env log -p` expands each selected parent-to-commit
transition through the same directed text, bounded search, and reference
planes. This is bounded single-process local state, not a Git object store or
a Spoke-durable log.

There is no implicit environment configuration file. Discovery always records
the process-owned `HOME`, `PWD`, and `PATH` seed set and the current compiled
application-adapter inventory.
The `git` application identity belongs to this inventory, but repository HEAD,
semantic index entries, and commit reachability do not. Those are Git cadence
and workload-activation evidence, so Git movement never dirties environment
admission by itself.
A coding harness may later generate a bounded reasoning map, and the caller
supplies that resource explicitly:

```yaml
schema: rey.env-map.v3
nodes:
  - id: workspace-manifest
    kind: file
    path: Cargo.toml
    required: true
  - id: parser
    kind: executable
    name: tree-sitter
    purpose: Parse source anchors discovered during survey
    required: false
    potential_capabilities: [source.parse.syntax-tree]
```

```sh
rey env status --map agent.environment.yaml
```

Rey parses the explicit resource as a closed, bounded graph and projects it into the committed
capability snapshot. Non-sensitive variables may opt into bounded UTF-8 value,
digest, or presence capture; sensitive variables are always presence-only.
Exact values selected with `capture: value`, bounded file identities, desired
application purposes, resolved executable identities, and declared edges
therefore remain reproducible across the human and structured revision
surfaces. An exact semantic identity over desired executable declarations is
the inventory record; the exact target capability snapshot is the search
record. Executables are resolved and hashed
but not invoked, and their declared potential capabilities remain visibly
unadmitted. Cargo remains a development tool without appearing in the desired
application inventory.

Environment commands default to Git-like human documents and accept explicit
JSON where no interactive patch selection is required. `status --format json`
contains the complete working capability snapshot, optional admission index,
both authoritative capability deltas, and the shared typed variable,
application, input, and reference operator projection used by `/environment`.
Workload commands support human tables and structured JSON; redirected `auto`
selects JSON. `workloads test` keeps passing scenarios compact by default and
always opens failing diffs. `-v` adds matching evidence, while `-vv` binds
evidence to exact workload, graph, suite, evaluator, scenario, execution,
result, and delta identities. List, test, status, and run also expose the
selected catalog, package path/revision, generator, and frozen-oracle state.
Mining scenarios additionally expose operation,
provider, capability snapshot, corpus, request, result, relation, native
source, match, context, frontier, scheduling-decision, and reasoning-surface
identities.

## The Mining Ladder

Source mining grows in capability without changing its evidence rules:

1. **Locate and retrieve** — bounded file, object, document, table, and source
   identity; exact bytes or rows remain addressable.
2. **Search and segment** — literal or regular-expression matches, line ranges,
   chunks, tokens, and declared context windows.
3. **Parse structure** — syntax trees, CSTs, ASTs, configuration paths, and
   language-aware spans under an exact parser contract.
4. **Index semantics** — symbols, references, definitions, types, call edges,
   ownership, and dependency relations with completeness metadata.
5. **Derive measures** — grouped diagnostics, complexity and quality metrics,
   test relationships, change impact, and other versioned analyses.
6. **Compare and visualize** — line, token, syntax, relation, tree, and graph
   deltas projected into reviewable human and machine forms.

Higher rungs never erase the lower evidence. A metric links to the relation
from which it was calculated; a relation links to exact source spans; a span
links to a source revision. Missing language support or bounded traversal is
reported explicitly rather than replaced by guessed structure.

Relational mining follows the same discipline. Queries and transforms bind
input schemas, keys, ordering, operation revisions, parameters, source
checkpoints, and limits. Grouping or visualization cannot become proof merely
because it looks complete. Typed before/after values remain authoritative even
when a human view renders them as text.

See [Mining Context Into Evidence](docs/MINING.md) for the capability contracts,
[ADR 0017](docs/decisions/0017-mining-capability-model.md) for the architectural
boundary, [ADR 0018](docs/decisions/0018-first-mining-workload.md) for the first
source slice, and [ADR 0022](docs/decisions/0022-portfolio-mining-and-workload-attention.md)
for ongoing portfolio mining.

## Exact Evidence, Diffs, And Proof

Every mining result records the strongest source identity its provider can
establish, the operation and implementation revision, parameters,
capabilities, effective limits, completeness, artifacts, and lineage. Pure
projection over frozen evidence is deterministic. Reading mutable state or
invoking a tool is an explicit probe; finding `rg`, a compiler, parser, or
language server does not grant permission to run it.

Rey preserves several authoritative comparison families:

- typed relational deltas for keyed frames;
- native text deltas for ordered text;
- structural deltas for declared trees and graphs; and
- typed claim facts when a claim does not reduce honestly to one comparison.

Each comparison is directed and names both sides. Human patches, tables,
trees, graphs, summaries, and Tabular Diff 0.8 are projections; none may erase
source identities, types, keys, schemas, comparison rules, omissions, or
limits from authoritative evidence.

A zero delta is not universal proof. It establishes agreement only for the
declared sources, operations, fixtures, keys, normalizers, parser/evaluator
revisions, completeness, and coverage. Changed source, mining implementation,
capability, or effective limit makes dependent evidence stale.

## Environment, Git, And Spoke

Rey treats its environment as an explicit runtime input. A bounded discovery
pass can inventory a selected workspace, Git repository, `rg`, language
toolchains, analyzers, test runners, and a reachable Spoke deployment. Each
provider advertises exact operations, trust, provenance, and enforceable
limits. Discovery remains read-only and separate from action admission.

The `env` CLI makes those observations revisionable. `status` computes
`HEAD → INDEX → WORKING`, `add` explicitly accepts observations into a bounded
admission index, `commit` records exactly that index in a local linear history,
and `log -p` reopens those directed deltas through the environment-native
three-plane projection. Environment commits record evidence; they neither
mutate the discovered environment nor create Git commits. Admission to history
does not grant execution authority to a mapped tool.

Git is both a source-binding provider and a natural activation surface. Commit,
ref, semantic index, and declared worktree deltas can select affected workload
scenarios or graph entry points. Ref rewrites, incomplete history, linked
worktrees, and conflicts remain explicit; Rey never fabricates an append-only
event stream from a mutable repository.

Rey remains useful with zero Spoke. In standalone mode it mines explicitly
selected local surfaces with narrower disclosed guarantees. When present,
Spoke is Rey's durable reasoning and compute plane: versioned content,
composed query, admitted compute, captures, and durable lineage amplify the
same mining contracts.

Rey does not duplicate Spoke storage, document, stream, table, query, tool,
run, or capture ownership. It uses public Spoke contracts and exact revisions.
Standalone adapters never mint Spoke identities or pretend to provide Spoke
durability.

The projects improve one another without a package or startup cycle:

```text
Rey mines Spoke ──► finds a public-contract delta ──► improves Spoke
      ▲                                                   │
      └────── discovers the new mining/compute capability ◄┘
```

## Boundaries

Rey is not:

- a model server or model-provider framework;
- a general-purpose database, search engine, code index, or visualization
  store;
- a replacement for Spoke storage, query, documents, or compute;
- an ambient shell that executes every discovered tool;
- a workflow engine whose tasks have no delta or scenario semantics;
- a claim that all text, code, graphs, or binary artifacts are naturally
  tabular;
- an authority that lets an agent mutate a target without explicit admission;
- a universal code-quality, coverage, or correctness system; or
- a proof system that hides missing, unsupported, ignored, stale, or truncated
  evidence.

## Repository Guide

- [Constitution](CONSTITUTION.md) — durable values and invariants.
- [Contributor Instructions](INSTRUCTIONS.md) — working loop and verification.
- [Architecture](docs/ARCHITECTURE.md) — ownership and end-to-end data flow.
- [Glossary](docs/GLOSSARY.md) — canonical project vocabulary and important
  semantic distinctions.
- [Explorer](docs/EXPLORER.md) — read-first world projection, semantic zoom,
  projection engine, and editor admission boundary.
- [Mining Context Into Evidence](docs/MINING.md) — relational/source mining,
  operation, result, visualization, and runtime contracts.
- [Workloads](docs/WORKLOADS.md) — public composition, compute graphs,
  scenarios, qualification, progress, and commands.
- [Runtime](docs/RUNTIME.md) — transition machine and reasoning surfaces.
- [Frontier](docs/FRONTIER.md) — canonical work, progress, and scheduling.
- [Environment](docs/ENVIRONMENT.md) — providers, capabilities, and profiles.
- [Locators](docs/LOCATORS.md) — exact survey anchors and resolution contracts.
- [Collaboration Journal](docs/JOURNAL.md) — shared typed notebook blocks,
  exact Explorer binding, admission, retention, and authority.
- [Git](docs/GIT.md) — source identity, polling, and activation.
- [Diffs](docs/DIFFS.md) — typed, textual, and structural comparison.
- [Proofs](docs/PROOFS.md) — claims, evidence, certificates, and staleness.
- [Interfaces](docs/INTERFACES.md) — CLI, policy, provider, and Spoke contracts.
- [Development](docs/DEVELOPMENT.md) — toolchain and repository truth.
- [Roadmap](docs/ROADMAP.md) — delivery sequence.
- [Plans](plans/README.md) — active implementation bearings.
- [Decisions](docs/decisions/README.md) — accepted architectural choices.

## Current Status And Next Bearing

The repository contains a twelve-crate Rust workspace. Implemented behavior
includes bounded standalone capability discovery, process-declared `git`,
`rg` identity probes, and major agent-runtime presence scans, a partial read-only Git
observation, typed capability
snapshot deltas, Arrow and Tabular Diff projections, scoped capability
certificates, bounded local proof bundles, verified local environment commits,
process-owned discovery seeds, explicit agent-generatable environment mapping
resources, compact `status`, exact
staged/unstaged `diff`, full and partial `add`, index-only commits, and
patch-bearing `log`, a formal runtime reducer, canonical
frontier/progress/scheduling contracts, bounded reasoning-surface contracts, the
deterministic workload CLI, and provider-neutral mining operation/request/result
manifests with canonical replay verification. The standalone
environment advertises a built-in literal source-search provider over exact
local corpora; the source-search workload executes it through the same test and
run graph, compares native ordered text and typed match relations, and projects
complete, failing, and truncated evidence through `list`, `test`, `status`, and
`run`. The dependency-light `rey-locator` crate supplies canonical local
coordinates, opaque Spoke coordinate carriage, canonical workspace/HTTP
locators, and typed resolution outcomes. The admitted
`context-anchor-survey` package exercises those contracts over bounded
`AGENTS.md` and README fixtures, retains `rey.topography-patch.v1` results and
directed deltas, and exposes the same patch through workload JSON and the
six-level Explorer.

The first Channel topology slice is also executable. It defines a canonical
built-in workspace channel, bounded subscription, stable three-stream Feed
layout, typed semantic deltas, and a symlink-safe atomic WORKING proposal.
`rey channels list`, `status`, `diff`, and `apply` provide human and structured
verification without moving observations into the topology index. Immutable
HEAD/INDEX admission and browser persistence remain the next topology slices.

The first read-first scene editor slice is executable. `rey editor init`,
`import`, `status`, `diff`, `validate`, `add`, `package`, and `inspect` preserve
workspace-authored GeoJSON, build a bounded feature/POI index, freeze exact
native objects, and retain directed immutable candidate packages plus explicit
admission requests. This is incomplete enabling work: no scene-admission
workload exists, candidate requests report `admitted=false`, and `/explore`
continues to consume only retained survey-workload topography.

Plan 0010 has now started the outer loop. Workspace packages are the default
product catalog and compiled workloads are explicitly diagnostic.
`rey.portfolio-snapshot.v1` and the
Polars-backed `rey.workload-attention.v1` relation are executable through the
scenario-qualified `rey.portfolio.attention` workload. The five workload
commands expose current attention, coverage, exact relation evidence, and
qualification-gated retained-input evaluation. Ready work, capability/evidence
blockers, and policy exclusions remain separate facts.
`rey.workload-creation-request.v1` makes draft creation visible in that same
portfolio plane: `create` records an immutable harness request, `list` and
`status` render `HYDRATE`/`AWAITING CODING HARNESS`, and `test`/`run` reject the
draft until an admitted package exists.

[Plan 0009](plans/0009-environment-admission-index.md) makes `status` the one
environment interface and places a reviewable admission index between working
observation and commit. Variables, input files, executable candidates, and
their declared relationships are visible across both staged and unstaged
planes. Revision selectors, reset/restore, branches, and Spoke retention remain
later work.

[Plan 0010](plans/0010-portfolio-mining-and-workload-attention.md) carries the
workload-centered mining strategy. Its next concrete anchor is the bounded coding-
harness response handshake. `workloads create` now supplies the request; the
next slice binds one ready attention row and exact evidence to it, has a harness
materialize the package, validates the frozen response, and re-mines whether
the row resolved. The first real payload will admit ownership for one mapped
input, then change that retained source revision and derive `RETEST` for the
exact owner. Parser/symbol mining remains a later tool for investigating
attention, not the portfolio strategy itself.

[Plan 0011](plans/0011-local-operator-ui.md) carries the high-fidelity operator
surface. `/feed` is the bounded high-cadence inspection plane and `/explore`
remains the default context-topology map with semantic zoom,
pan, selection-driven focus, full screen, visible bounds, passive revalidation,
coordinate/scale deep links, a shared typed Journal, an observed-work ledger, and a
partially ordered cadence view. Humans can author retained Explore-bound prose
and read-only query cells at `/journal/new` without authentication; exact entry
routes and block fragments make the retained notebook deeply hyperlinkable.
Agents admit the same bounded format through `rey journal add`, including
frame, diff, and action cells. Explorer now derives World, Atlas, Landscape,
Neighborhood, Object, and Evidence projections from retained survey patches;
exact patch-anchor routes remain read-only and execute no locator. The next
concrete anchor is a separate query-execution handshake that can turn one
retained declaration into an exact frame/diff result without granting document
admission implicit compute authority. Exact scenario delta routes remain a
subsequent evidence projection.

[Plan 0014](plans/0014-seed-discovery-and-locator-survey.md) carries the new
context lifecycle. Process-owned seed discovery, explicit reasoning-map input,
the live two-axis communication plane, canonical locator contracts, and the
first high-fidelity seed-to-map CLI/UI voyage are implemented. Feeding retained
survey change into independent cadence processing without fabricating a global
event log remains active work.

[Plan 0016](plans/0016-channel-graph-and-operator-index.md) carries the Channel
graph and agentic-networking bearing. Channels become stable collaboration
boundaries; compact standalone Channel observations retain the collaboration
frontier; subscriptions project them into Feed streams; and Journal entries
deliberately synthesize and cite exact observations. A Journal seed is an
unretained catch-up projection, not an automatically admitted document.
Broadcasts associate one observation identity with a bounded local channel set,
while relays remain explicit provider-backed edges. The first CLI-only anchor
is delivered: `rey channels list/status/diff/apply` exposes the canonical
built-in graph and a symlink-safe, tamper-detecting `CHANNEL WORKING` proposal
without writing state merely to inspect the default. The next slice adds
`rey observations add/list/show` without dirtying that topology index. Staged
Channel admission and draggable persistent Feed headers remain subsequent
slices over the same high-fidelity agent surface.

The longer-running [Plan 0001](plans/0001-foundation.md) still owns complete Git
activation and the first routed Spoke proof.
