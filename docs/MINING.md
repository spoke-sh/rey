# Mining Context Into Evidence

This document defines Rey's mining model: the capability layer joining
environment surfaces to workload graphs, deltas, frontiers, reasoning surfaces,
and evidence projections. Provider-neutral operation, request, result,
artifact, completeness, lineage, dependency, and limit contracts are
implemented. The first deterministic provider, ordered text delta, typed
source-match delta, workload graph, terminal projections, and delta-directed
reasoning fixture are also implemented. External tool providers, structural
indexes, and general visualization specifications are not.

The executable survey family resolves canonical locator candidates from exact
`AGENTS.md` and README seeds under a frozen local provider and capability
snapshot. `rey.topography-patch.v1` retains typed anchors, classified
`contains`/`references` edges, surveyed and unknown region states, coverage,
frontier, omissions, lineage, hard limits, and a directed delta. Explorer
consumes that retained patch and performs no independent source scan or
assessment. Exact source edges and shared-coordinate equality remain inspector
evidence rather than transport; anchor samples shape relief, unresolved
frontier conditions become projected weather, and deterministic runoff may
carve displayed streams, rivers, and erosion. Those features neither recommend
mining nor claim a discovered or constructed path.

The implemented outer loop derives a canonical portfolio snapshot and
workload-attention relation through a scenario-qualified conformance workload
and workload CLI projections. Ownership declarations, live invalidation, the
generic scheduler handoff, and exact Git activation admission/execution are
implemented. Product proposal and admission use workload HEAD/INDEX/WORKING;
autonomous activation scheduling remains future work.

## Purpose

Mining is the bounded process of turning context into navigable, addressable
evidence. It answers four questions before policy proposes work:

1. What exact sources and operations are available?
2. What bounded structure can be extracted from them?
3. What changed or remains unresolved?
4. Which representation makes that evidence useful without overstating it?

Mining is how Rey makes high-dimensional environments tractable. It does not
mean scraping everything, building an undeclared durable index, or
giving an agent arbitrary query or execution authority. Every mining operation
has explicit sources, semantics, limits, completeness, and lineage.

## Capability Families

### Relational Mining

Relational mining operates on typed collections such as tables, events,
measurements, diagnostics, symbols, references, dependency edges, tests, and
claims. Its operation vocabulary can include:

```text
retrieve · select · filter · join · group · aggregate
align · order · traverse · compare · summarize · visualize
```

Polars DataFrames are Rey's canonical bounded in-process representation for
these collections, and Arrow is the preferred typed interchange family.
Providers still own source query semantics: a database provider owns its
snapshot and query contract; Rey owns the
versioned mining request, bounded projection, delta use, and lineage that joins
the result to a workload.

Relational operations bind logical schemas, key and ordering rules, operation
revision, parameters, input identities, provider checkpoints, and effective
row/column/cell/byte/time limits. Grouping and aggregation retain enough
lineage to identify their contributing scope. A summary or visualization is
not allowed to become the only authoritative copy of typed values.

### Source Mining

Source mining operates on ordered text, code, configuration, logs, documents,
and native artifacts. Its capability ladder includes:

```text
locate and retrieve
  -> search and segment
  -> tokenize and parse
  -> index symbols and relationships
  -> traverse syntax and semantic graphs
  -> derive metrics and grouped views
  -> compare and visualize
```

The ladder is cumulative. A syntax node links to its exact parser revision and
source span. A symbol or reference links to the syntax/source evidence from
which it was derived. A metric links to the contributing relation and declared
formula. Unsupported language features, parse recovery, ambiguous resolution,
partial traversal, generated code, binary input, and invalid encoding remain
visible completeness facts.

`rg` awareness is a useful low-rung capability: an admitted adapter can return
bounded exact match records and context spans. It does not imply AST support,
semantic resolution, or permission to run arbitrary commands. Language
parsers, compiler services, and semantic indexes are richer providers that
must advertise their own contracts and limitations.

## Ongoing Portfolio Mining

Mining is not finished when one workload operation returns. Rey runs two
nested conceptual campaigns:

```text
workload:  execute graph → mine observations → diff scenarios → refine graph
portfolio: mine catalog/results/environment/coverage → derive attention
           → admit work → test → observe portfolio again
```

Workloads are instruments for mining their declared domain and objects mined
for attention. The outer input is one exact `rey.portfolio-snapshot.v1`, not an
ambient repository sweep. It binds workload/graph revisions, qualification and
result evidence, retained environment revision, changed dependencies, missing
capabilities, mapped surfaces, declared owners, policy, and effective limits.

The output `rey.workload-attention.v1` is a canonical typed relation with
`REFINE`, `RETEST`, `CREATE`, `BLOCK`, and `POLICY_EXCLUDED` actions. Reasons,
readiness, blockers/exclusions, citations, priority, and estimated cost remain
separate. A typed empty relation means no attention was derived under those
inputs and bounds; it is not universal convergence or proof.

The generic scheduler consumes admitted ready attention; it does not derive
domain facts. A bounded reasoning surface may expand selected attention with
more relational or source mining. Parser, symbol, metric, and visualization
capabilities are tools used to investigate that attention, not substitutes for
the portfolio loop.

## Common Mining Lifecycle

```text
inventory capability
  -> bind exact source or snapshot
  -> admit operation and effective limits
  -> retrieve or probe
  -> extract/project/organize
  -> compare and assess completeness
  -> retain artifact references and lineage
  -> render bounded machine and human projections
```

The lifecycle distinguishes three execution cases:

- **exact retrieval** reads already identified immutable evidence through the
  provider that owns it and can occur during bounded orientation;
- **pure projection** transforms frozen evidence deterministically in process
  or as an admitted graph node; and
- **probe mining** observes mutable state or invokes a tool and therefore
  passes ordinary proposal, admission, execution, observation, and budget
  boundaries.

Mutation is not mining. A mining result may justify a later mutation proposal,
but no read, parse, index, metric, diff, or visualization grants that effect.

## Operation Contract

A versioned mining operation contract needs at least:

| Field               | Meaning                                                                                    |
| ------------------- | ------------------------------------------------------------------------------------------ |
| identity            | Stable operation id, revision, implementation digest, and semantic version                 |
| family              | Relational or source mining                                                                |
| kind                | Retrieve, search, transform, parse, index, traverse, measure, compare, or visualize        |
| input contract      | Accepted native artifact, relation, tree, graph, or prior mining-result types              |
| output contract     | Produced artifact kinds, schemas, media types, and identity rules                          |
| source requirements | Provider operations, snapshot guarantees, encodings, languages, and trust                  |
| determinism         | Pure/frozen semantics or explicitly variable/tool-observed semantics                       |
| effects             | Read-only retrieval, pure projection, or probe; never an implicit mutation                 |
| parameters          | Typed, canonical, bounded arguments and defaults                                           |
| limits              | Rows, bytes, matches, files, depth, nodes, edges, time, memory, and output bounds          |
| completeness        | Conditions for complete, partial, truncated, unsupported, unavailable, or failed results   |
| invalidation        | Source, provider, parser, operation, parameter, and limit changes that make evidence stale |

Operation discovery and operation admission are distinct. The capability
snapshot freezes provider identity, path or endpoint, version,
digest/provenance, trust, supported operation revision, enforceable limits, and
availability before a graph can select it.

## Request And Result

A mining request binds:

- workload, graph, scenario/campaign, space, and active transition when
  applicable;
- exact source bindings or exact input mining artifacts;
- the selected operation contract and canonical parameters;
- capability snapshot and provider selection;
- requested and effective limits;
- expected output kinds, schemas, keys, and completeness; and
- the frontier/delta/claim that justified the work.

A mining result manifest binds:

- request and result identities;
- realized provider, tool, parser, query, run, or capture lineage;
- exact source identities and any post-read drift check;
- native artifact, frame, tree, graph, metric, delta, and visualization
  references produced;
- schemas, media types, logical lengths, keys, and ordering where applicable;
- completeness, omissions, unsupported semantics, warnings, and errors;
- effective resource consumption and limits; and
- dependency edges needed for invalidation and staleness.

The manifest is an evidence index, not a content store. Native source remains
owned by its source provider. Working frames are bounded state. Retained
artifacts use the selected local evidence boundary and claim
only its actual guarantees.

## Artifact Shapes

Mining may produce several peer evidence shapes:

- **native artifacts** — exact or content-addressed bytes, ordered text,
  documents, captures, patches, or provider resource references;
- **relations** — bounded typed frames for matches, symbols, references,
  diagnostics, metrics, nodes, edges, or grouped results;
- **trees and graphs** — native structured artifacts plus typed node/edge/span
  relations when tabular navigation is useful;
- **deltas** — authoritative relational, text, tree, graph, or claim-specific
  comparisons with explicit direction; and
- **visual projections** — tables, patches, trees, graphs, timelines, metric
  panels, or summaries linked to authoritative evidence.

An artifact is not forced into a DataFrame when doing so loses native meaning.
Conversely, typed relational values are not stringified merely to reuse a text
patch. Cross-artifact references use exact credential-free evidence addresses.

## Diff Families

Mining results participate in comparison according to their declared shape:

1. **Relational delta** aligns typed keyed or explicitly ordered relations and
   preserves schema, row, and cell changes.
2. **Text delta** compares ordered text under exact encoding, segmentation,
   normalization, context, and byte/line limits while retaining source
   identities and native content addresses.
3. **Structural delta** aligns declared tree or graph entities under versioned
   identity rules and preserves insertions, deletions, moves, modifications,
   and unresolved alignment.
4. **Claim fact** records a typed predicate result when forcing the evidence
   into one of those comparisons would be dishonest.

Every family uses explicit `SOURCE` to `TARGET` direction. Workload scenarios
normally label that direction `EXPECTED` to `OBSERVED`. A human patch or graph
view is a projection of its authoritative delta, not a substitute for it.

## Visualization Contract

Visualization exists to improve orientation for humans and policies while
remaining evidence-honest. A visualization projection declares:

- source artifact and delta identities;
- projection contract and implementation revision;
- selected fields, grouping, ordering, layout, context, and elision;
- requested and effective display bounds;
- omissions, aggregation, sampling, and truncation;
- semantic labels and non-color encodings; and
- deep links back to exact evidence.

Tables are appropriate for repeated fields; patches for ordered local change;
trees for nesting; graphs for dependencies; timelines for transitions; and
metric panels for several distinct measurements. The choice is semantic, not
cosmetic. A visualization never changes comparison assessment, proof status,
coverage, confidence, or progress.

Machine projections expose stable typed documents or relations. Terminal
renderings may add ANSI styling only when interactive, and meaning remains
legible without it.

### Spatial projection engines

A high-fidelity Explorer scene is a visualization result with more mechanism,
not more semantic authority. Its projection contract additionally binds:

- coordinate or embedding basis, implementation revision, parameters,
  normalization, distortion, and stable-coordinate rules;
- immutable scene compiler and semantic/geometric LOD revisions;
- named scalar/vector field channels, units or normalization, derivations, and
  surveyed-validity masks;
- natural-feature simulation, material, render-graph, label, picking, and
  renderer revisions;
- scene-object, field-cell, tile, byte, graphics-resource, draw-call, label,
  compile-time, and frame-time limits; and
- backend, viewport, device-pixel-ratio, fallback, degradation, omissions, and
  exact evidence links used by human visual proof.

Renderer output is not the only copy of the scene or field. Height, normals,
hydrology, erosion, validity, object identity, and pass ordering need
backend-independent fixtures before GPU screenshot evidence is considered.
Pixel identity may vary across qualified graphics implementations while the
semantic scene manifest remains deterministic. A performance claim cites a
named workload, hardware/browser profile, viewport, scene and renderer
revisions, warm/cold posture, and retained result.

Smoothing, interpolation, antialiasing, hillshade, ambient occlusion,
hypsometric tint, curvature enhancement, and simulated natural features remain
visual or derived channels. None may infer semantic values across unknown
validity or change assessment, coverage, confidence, progress, or proof.

### Scene editor candidates

A committed scene editor package is an input candidate for mining and
admission, not a visualization result and not admitted evidence. `rey editor`
preserves exact native survey or generated artifacts and derives bounded
source, feature, geometry, bounds, marker, coverage, limit, and change indexes
so an operator can review what a future admission workload would inspect.
INDEX freezes exact bytes; `commit` advances `SCENE@n`, packages only that
verified index, and emits a separate unadmitted request. Generator recipes,
seeds, and hyperparameters are source lineage, not evidence claims.

An admission operation must verify every frozen object identity, qualify its
format adapter, preserve native artifacts, and bind coordinate semantics before
emitting topography, feature, or projection evidence. GeoJSON's fixed
geographic CRS cannot be repurposed as an arbitrary semantic coordinate
system. GeoPackage, GeoTIFF/COG, Arrow, and Rey-native terrain manifests remain
unsupported until their source, validity, no-data, unit, CRS/chart, tiling,
limit, and replay contracts are qualified.

`rey-mining` defines and verifies the admission output, while `rey-runtime`
implements its deterministic bounded admission operation.
`rey.admitted-regional-scene.v1` binds exact editor and
workload lineage, native objects, five distinct coordinate planes, transforms,
typed layers, validity/no-data, and the embedded
`rey.regional-projection-packet.v1`. Topography, atlas, projection, and terrain
identities remain separate; an absent qualified terrain adapter forces an
absent terrain program and explicit unsupported height validity. The separate
`rey.explore-grammar.v1` binds projection posture, hysteresis, morphing,
semantic/geometric LOD, inverse picking, polar/antimeridian behavior, and
camera bounds without containing a camera instance. Bounded multi-region tests
cover overlap, polar and antimeridian envelopes, typed County objects, rejected
coordinate metadata, identity tampering, and candidate-control authority.
The file-backed `scene-admission` workload freezes accepted and typed rejection
oracles for tampering, stale parents, formats, coordinates, identities, missing
objects, and bounds. Its CLI run path independently revalidates a committed
editor transfer envelope and retains the result and embedded projection packet.
The workload list exposes only the last production result, and Explorer accepts
only an accepted non-scenario result with exact workload, graph, capability,
package, snapshot, packet, terrain, coordinate-plane, and placement bindings.
The retained atlas change path remains incomplete.

## Workload And Runtime Placement

Workloads declare which mining operations a graph may compose, the context
surfaces they may read, and the scenarios that qualify their behavior. A graph
node cites an exact operation contract; generated shell, query, regex, parser
configuration, or source text does not become executable merely because a
policy proposed it.

Within the runtime:

```text
frontier work
  -> schedule
  -> mine exact evidence
  -> project reasoning surface
  -> policy proposes graph revision or action
  -> runtime admits and executes
  -> mine post-action observations
  -> compute transition and residual deltas
  -> derive next frontier and proof facts
```

Mining during orientation is delta-directed: it begins with selected frontier
citations and expands only through declared dependencies and remaining bounds.
It does not sweep the ambient workspace to create a generic prompt. The
reasoning surface cites mining-result artifacts and omissions rather than
copying an unbounded repository into policy input.

## Provider Boundary

Rey composes mining but does not seize provider ownership:

- filesystem and Git providers own local source identity and safe reads;
- tool adapters own executable invocation, parsing, capture, and limitation
  semantics for tools such as `rg`;
- language adapters own parser, syntax, semantic, and index interpretation;
- `rey-dataframe` owns bounded local relational representation and Arrow
  interchange;
- `rey-diff` owns authoritative comparison contracts and projections; and
- the Rey runtime owns workload admission, delta/frontier rationale, limits,
  mining composition, invalidation, and policy-surface projection.

An adapter exposes only the identities, operations, and guarantees its public
contract proves. Rey never upgrades those claims on the adapter's behalf.

## Current Truth And First Slice

Current Rey discovers process-declared `rg` and `git` through bounded identity
probes and major agent runtimes through non-executing PATH presence scans,
observes part of one Git repository, operates on typed
capability frames, implements ordered UTF-8 line deltas, and exposes
canonical `rey.mining-operation.v1`, `rey.mining-request.v1`, and
`rey.mining-result.v1` manifests. The v1 result makes observed wall time optional
so a deterministic pure projection does not acquire a timing-dependent
identity while a tool-backed probe may retain measured time. Constructors and
replay verification bind exact workload/frontier rationale, provider and
capability identity,
typed parameters, native or structured artifact references, effective limits,
completeness, omissions, consumption, lineage, and invalidation dependencies.

`rey-environment` now implements `rey.source-corpus.v1`, the
`rey.source-search.literal-utf8` operation, and the `rey.source-matches`
version `1` relation. One explicit file set beneath a canonical local root is
bound by reversible path identity and exact native content digests. The
built-in read-only probe revalidates the mutable local source before and after
deterministically searching frozen bytes for non-empty case-sensitive UTF-8
literals. It retains exact native context slices and emits
one-based line plus zero-based byte spans and deep links. File, byte, line,
path, match, row, context, string, output, and time boundaries remain explicit;
binary and invalid UTF-8 inputs, truncation, malformed parameters, symlinks,
path escapes, and source drift fail closed or produce typed incomplete results.

`rey.fixture.source-search` is the first end-to-end standalone mining slice.
Its typed graph executes `rey.source-search.literal-utf8@1` and
`rey.builtin.source-matches.render-lines@1` in deterministic dependency order.
Required empty and exact scenarios qualify the graph. Optional mismatch and
truncation scenarios preserve, respectively, a complete `DIFFERENT` relation
and a typed `INCONCLUSIVE` relation with a `match_limit` omission.

`rey.source-match-delta.v1` aligns the relation by reversible path identity and
byte span, preserving insertions, deletions, modifications, typed before/after
rows, native source/match/context identities, completeness, and replay.
`rey.text-delta.v1` preserves expected-to-observed direction, ordered UTF-8
lines, final-newline state, bounded LCS alignment, change counts, and replay;
`rey.scenario-output-delta.v1` embeds it for workload output evaluation.

The failing complete relation deterministically derives one
`rey.frontier.v1` row, selects it with one `rey.scheduling-decision.v1`, and
projects one `rey.reasoning-surface.v1` citing the source result, match/context
artifacts, relational and text deltas, and the admissible graph-revision
action. This is a workload-specific conformance fixture, not a recurring
scheduler or policy loop.

The human verification path is the workload surface: `workloads list` reports
portfolio and per-workload mining dimensions; `workloads test -v/-vv` renders
matches, native context, relation and line diffs, omissions, limits, deep
bindings, and reasoning selection; `workloads status` reopens retained
evidence; and `workloads run --source` executes the qualified graph against
explicit caller-selected paths. JSON retains the same verified semantic
artifacts without progress text.

`rey.portfolio.attention` adds the outer-loop verification path. `list` and
`status` expose current attention and mapped-surface coverage from retained
inputs; `test -vv` opens reviewed refine/retest/create/block/exclusion/clean
scenarios with exact relation identities; and an input-free qualified `run`
re-evaluates the retained catalog, workload results, and environment snapshot.

The first survey-mining family emits content-identified topography patches from
admitted survey workloads. A patch retains locator candidates,
typed resolution outcomes, provider-qualified coordinate anchors, classified
relationships, surveyed regions, coverage, frontier, omissions, completeness,
lineage, and a directed delta against a prior map revision. Explorer is a
deterministic visual projection of those artifacts. It does not mine, resolve,
or interpolate semantic terrain on its own. Broader locator families and
recurring voyage scheduling remain later work. Source-edge and camera-path
geometry is excluded from relief; only a subsequent admitted patch can change
topography. `rey.projection-packet.v1` binds each displayed patch to its
synthetic basis, bounded objects, validity, field/layer descriptors,
deterministic terrain program, macro/meso/micro bands, maximum transient
working-set allocation, limits, degradation, omissions, and lineage through
the CLI and `rey.workload-list.v1`. The browser compiles a bounded
camera-relative working set and selects only frequency bands supported by its
sample spacing without moving shared coordinates or creating evidence.
The workload list also derives `rey.semantic-atlas.v1` over the latest admitted
regional patches. Its bounded survey-structure clustering and synthetic
spherical placement are pure visualization mining with an exact compiler and
revision; visual proximity is not a mined relationship or similarity claim.
Zoom never enters that derivation. Production survey runs retain a bounded
linear atlas history and content-identified `rey.semantic-atlas-delta.v1`
documents. Region changes remain typed as inserted, removed, moved, or
interest-changed; cluster topology changes remain merged or split. Each delta
binds exact source and target atlas revisions, and replay/tamper verification
recomputes it from both documents. Qualification fixtures and read-only
projection do not advance this history. Accepted regional-scene membership in
that atlas remains Plan 0003 work.

The remaining render-graph extraction, transient-patch transitions, retained
visual proof, and named performance qualification remain
[Plan 0003](../plans/0003-scene-to-explorer.md) work.

Rey still does not execute `rg` as a mining provider, support regex/case-folded
search, compare arbitrary caller-selected source artifacts outside this graph,
parse ASTs/CSTs, build a semantic index, or render general tree/graph
visualizations.

AST/CST adapters, broad semantic resolution, broad code-quality metrics, durable
indexes, generic graph visualization, learned ranking, and recurring
scheduling follow only after that slice proves the common invariants.

## Required Fixtures

Mining implementation work needs fixtures for:

- exact source identity and source drift during retrieval;
- empty, single, multiple, overlapping, and Unicode text matches;
- binary or invalidly encoded content under explicit policies;
- long lines, deep paths, symlink escapes, ignored/generated files, and bounded
  file/match/context overflow;
- missing, changed, timed-out, malformed, and non-zero tool providers;
- deterministic built-in/tool parity where both claim the same semantics;
- typed empty match relations and unique match identity;
- insertion, deletion, modification, reorder, parse recovery, unsupported
  structure, and incomplete traversal;
- aggregation/grouping provenance and limit behavior;
- text, relation, tree, graph, and visualization direction without reliance on
  color;
- complete, partial, truncated, unsupported, unavailable, failed, and stale
  results; and
- identical semantic artifacts from identical local source bindings.

## Non-Goals

The mining model does not select a universal query language, parser framework,
language server protocol, index database, metric catalog, visualization
library, persistence engine, or new top-level CLI group. Those choices require
an end-to-end workload need, bounded fixtures, and an ownership decision.
