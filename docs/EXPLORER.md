# Operator Feed And Context Topology Explorer

The Rey UI is the human operator's primary collaboration surface. `/feed`
projects high-cadence change through rich Signals, Admission, and Flow streams,
while Explorer is a high-fidelity spatial game engine for evidence-bound
projections of high-dimensional context. It maps the bounded context Rey can
currently explain, lets the operator move between semantic scales, and
preserves exact runtime identities while the visual grammar and rendering
fidelity change. Agents continue to use the `rey` CLI as their primary
execution and diagnostic interface.

The Explorer itself is a read-only projection. The adjacent `/agents` Journal
may retain a typed entry that points to an exact semantic coordinate and
numeric camera scale, but that
does not mutate topology or make Explorer a runtime, scheduler, evidence store,
or assessment authority.

Explorer is also the read-first runtime side of a level-editor architecture.
The separate `rey editor` CLI assembles WORKING projects, stages exact native
objects in INDEX, and commits linear `SCENE@n` history with immutable candidate
scene packages. Deterministic generators may author tunable source features in
WORKING, but generation itself grants no evidence authority. Those packages
are not Explorer inputs. The file-backed `scene-admission` workload now
validates one exact current package and its frozen GeoJSON objects, retains a
typed accepted/rejected result, and emits a regional projection packet.
`/explore` consumes only the latest accepted production result from that
workload record. It rejects scenario fixtures, rejected results, candidates,
and mismatched bindings before projection; editor commits alone still leave the
browser unchanged. See
[CLI](CLI.md), [Mining](MINING.md), and [Plan
0003](../plans/0003-scene-to-explorer.md).

## Operator Model

The intended division of labor is:

| Persona                    | Primary surface                   | Normal use                                                                                                                    |
| -------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Human operator             | `rey ui`, `/feed`, and `/explore` | Triage bounded change, orient on context, traverse attention, inspect workload neighborhoods, and understand the next bearing |
| Agent or coding harness    | `rey` CLI and structured output   | Create, test, run, diagnose, and revise workloads through admitted contracts                                                  |
| Human diagnosing a problem | `rey` CLI                         | Drop beneath the visual projection to inspect exact command evidence, verbosity layers, stderr, and exit semantics            |

The UI and CLI are different projections over the same typed facts. The UI is
not required to mimic a terminal document, but it must retain enough identity,
scope, direction, completeness, limits, and lineage to reach the exact CLI
evidence when investigation requires it.

## Presentation Concepts

The Explorer separates semantic address, retained map evidence, and browser
presentation. Only canvas, lens, regime, camera, focus, and omission are
React/read-model concepts:

```text
Coordinate         = typed provider-qualified semantic address for an object or
                     bounded region; local bindings carry narrower guarantees
Locator            = candidate address; resolution is a separate bounded act
Context topology   = bounded typed anchors + classified relationships
Context topography = topology + scale + surveyed coverage + frontier +
                     explicit unexplored space
Projection packet  = exact evidence + projection basis + fields + validity +
                     revisions + limits + omissions
Scene snapshot     = immutable stably ordered engine scene
Terrain working set = bounded transient scalar/vector channels + validity
Canvas             = spatial view over one bounded topography projection
Camera             = center + continuous scale + viewport
Lens               = semantic projection(topography, focus, camera)
Regime             = one level-of-detail grammar on the lens continuum
Render graph        = ordered material, relief, feature, label, and UI passes
Editor project      = mutable workspace declaration of native candidate inputs
Scene package       = immutable candidate + native objects; never admission
World              = far projection of admitted charts, survey weather, and
                     unresolved survey horizons
Orientation globe  = pre-survey presentation sphere carrying exact workload
                     beacons; never an admitted atlas or semantic-distance claim
Workload beacon    = exact request, WORKING, INDEX, or admitted-but-unrun
                     workload revision asking for operator attention
Sector             = revision-bound synthetic atlas partition; never a source
                     boundary or an inferred relationship
County             = one admitted detailed regional scene with a stable
                     footprint and local tangent coordinate frame
Neighborhood       = bounded objects around one meaningful coordinate
Focus              = selected coordinate retained while changing scale
Omission           = evidence that the current projection folded or excluded
                     known objects because of declared limits
```

The current interface hard-cuts the former matrix route. A semantic
`rey+local://...` coordinate and continuous numeric scale are separate values;
the `/explore` query envelope combines them only for navigation. Old matrix
paths are unresolved and have no compatibility parser. [Locators](LOCATORS.md)
defines the address and resolution boundary.

Relationships are always labeled. Portfolio projections use `contains`,
`directs`, `produces`, `observes`, and `depends`; admitted survey patches add
exact `contains` and `references` edges. Line placement or proximity alone
does not assert causality, ownership, or authority.

## Projection Engine

Explorer is specialized like a high-fidelity map engine and structured as a
small real-time game engine. The engine boundary is:

```text
admitted evidence
  → evidence adapter / projector
  → versioned projection packet
  → immutable scene + field compiler
  → camera + semantic/geometric LOD + culling
  → explicit render graph
  → deterministic reference or Three.js WebGPURenderer/TSL adapter
  → WebGPU preferred or WebGL2 compatibility backend
  → accessible React overlays and exact evidence links
```

Evidence adapters decide what a coordinate, field channel, validity class, and
layer mean. Engine code decides how bounded scene objects and data-oriented
fields are transformed, culled, picked, and rendered. Renderer code decides how
materials and passes become pixels. React owns the route, controls,
accessibility, evidence panels, and lifecycle around that surface. None may
take over another layer's semantic authority.

Before admitted evidence exists, Explorer has a separate consent-first
orientation flow:

```text
file-backed request or WORKING package
  → exact workload beacon on an unmapped orientation globe
  → operator inspects source, digest, producer, graph, and scenario oracle
  → operator explicitly qualifies and admits the exact workload revision
  → agent may run the admitted survey only over explicitly chosen seeds
  → retained topography patch + projection packet
  → admitted semantic atlas and generated terrain
```

The orientation globe is the default `/explore` posture for a fresh `.rey`
state and remains the only Explorer grammar until an admitted topography patch
exists. Its stable beacon placement is presentation-only: it is neither
`rey.semantic-atlas.v1` nor evidence of similarity, distance, geography, or a
surveyed project boundary. A beacon may draw attention to exact file state but
cannot execute, qualify, or admit itself. The initial
`context-anchor-survey` beacon is the proposed first mapping step; selecting it
opens exact workload inspection and an explicit route to the Feed admission
control. Only consent followed by a bounded agent run can reveal terrain.
The orientation renderer uses dense deterministic spherical projection fabric,
layered atmosphere, limb lighting, and depth-aware beacon occlusion rather than
a coarse wireframe. Drag orbits that fabric and every label through one shared
camera transform. The fabric remains presentation-only until admitted survey
coordinates can emphasize it; it never stands in for surveyed land.

The upstream editor pipeline is deliberately outside this render flow:

```text
native survey files → editor WORKING → INDEX → candidate package
                                              │
                                              ▼ qualified admission workload
                                      admitted evidence → projection packet
```

The first adapter accepts only geographic RFC 7946 GeoJSON in OGC CRS84. It
indexes explicit features and marker POIs while preserving native bytes.
GeoJSON coordinates cannot stand in for an unbound high-dimensional semantic
chart, and line features do not become paths or source relationships. Detailed
raster terrain and provider-qualified semantic chart formats require separate
adapters and admission scenarios.

The renderer-independent result contracts and their file-backed admitting
workload now exist. `rey.admitted-regional-scene.v1` binds one exact editor
commit, package, snapshot, admission request, qualifying workload/graph/suite,
capability snapshot, native bounds and objects, coordinate transforms, typed
layers, validity/no-data records, limits, omissions, and lineage. Its embedded
`rey.regional-projection-packet.v1` keeps five coordinate planes explicit:
native OGC CRS84, synthetic semantic placement, semantic-Mercator chart,
County-local east/north/up, and camera view state. The last is always
view-only. Topography patch, retained atlas revision, projection packet, and
terrain program identities are separate bindings; qualification fixtures may
leave the atlas nullable, while an accepted production run records and
cross-verifies its exact retained atlas revision without recursively changing
`scene_id`. With no qualified
terrain adapter, the contract must retain an unsupported terrain-height
validity row, an absent terrain program, and the fact that candidate controls
were not copied into observed terrain truth. `rey workloads test --staged
scene-admission -vv` qualifies frozen acceptance and rejection cases;
`rey workloads run scene-admission --scene SCENE@n` revalidates a real current
editor revision and exposes the five coordinate planes, exact identities,
validity, omissions, limits, and lineage in human or JSON form.

`rey.explore-grammar.v1` freezes the six semantic levels independently from
World-globe, semantic-Mercator, County-isometric, and County-evidence postures.
It binds scale thresholds and hysteresis, bounded posture morphs, semantic and
geometric LOD budgets, inverse picking, polar disclosure, antimeridian draw
fragment identity, and renderer-independent camera constraints. The grammar
contains no camera instance, evidence, admission, or renderer authority. Rust
fixtures cover overlapping regional footprints, polar and antimeridian
envelopes, rejected coordinate metadata, typed County boundary objects, and
candidate-terrain authority rejection. Workload/CLI production of these
documents and the accepted-result Explorer adapter now exist. The browser shows
exact regional point placement, stable occupied fixed-grid sector polygons, and
native object envelopes. A sector expresses synthetic membership only; it is
not surveyed coverage or a native County footprint. Footprint geometry,
constructed layers, qualified regional terrain, and the complete posture
transitions frozen by the grammar remain open.

The engine is high-dimensional because its input basis may project many source
dimensions into a stable navigable scene. It is not allowed to invent that
basis. An admitted provider or operation must bind dimensions, exact inputs,
algorithm and implementation revision, parameters, normalization, random seed
when applicable, distance or neighborhood semantics, distortion, validity,
limits, and omissions. The current standalone anchor placement remains a
synthetic orientation layout rather than a language-space embedding.

The current implementation remains incomplete but now crosses the live renderer
and semantic-LOD boundaries. The admitted-survey adapter, camera transforms,
immutable scene wrapper, typed terrain-field modules, SVG/DOM reference
renderer, and pinned Three.js `0.185.1` WebGPU adapter are separated. The
projection packet now retains a deterministic terrain evaluator, seed,
macro/meso/micro bands, absolute-coordinate and validity rules, and a bounded
transient working set. The browser evaluates an exact camera window with
validity, elevation, rainfall, flow, erosion, normal, curvature, and
presentation-only material buffers. Its frequency-band selection may add
visual detail as sample spacing tightens but never adds semantic evidence or
fills invalid support. A TSL node material consumes the active buffers as one
continuous relief mesh in `/explore`; React retains the controls, accessible
overlays, exact evidence links, active band/backend status, and current/maximum
working-set allocation. `topology.ts` still combines portfolio adaptation,
scene assembly, contour extraction, and lens data. [Plan
0003](../plans/0003-scene-to-explorer.md) owns transient-patch transitions,
the remaining render-graph extraction, device-loss qualification, and retained
visual and performance proof.

### Terrain fidelity

The 2026-08-11 visual comparison establishes the target. Current Rey terrain
is primarily isolines and feature strokes over a uniform plane. Mature map
terrain reads as one continuous surface because elevation, multiscale detail,
slope, aspect, hillshade, ridge/valley occlusion, tint, contours, water, labels,
and overlays are composed together.

Google Maps-level fidelity means comparable perceptual terrain legibility, not
Google data or style replication. Rey's target render graph is:

```text
validity/background
  → base terrain material
  → height normals + multidirectional hillshade
  → ambient/valley occlusion + ridge/curvature enhancement
  → LOD-aware contours
  → water + weather + boundary state
  → POIs + labels + selection
  → evidence and accessibility overlays
```

The base surface must read before contours or POIs are added. A field uses a
bounded procedural program and camera-relative transient working set, explicit
channel revisions, and a per-cell validity mask. Unknown, surveyed-empty,
omitted, stale, unsupported, truncated, and frontier cells do not acquire
height through blur, interpolation, erosion, or shading. Visual feathering may
blend a known boundary into the application background while the exact mask
and disclosure remain available.

The continuous `rey.explore-grammar.v1` contract couples a lit 3D World sphere,
a flat semantic Mercator Atlas, and a stylized isometric County surface. This supersedes the
former single top-down 2.5D target while retaining bounded cameras rather than
unrestricted free orbit. Volumetric space, physics, and a general ECS remain
deferred. World and County camera posture is presentation state; neither can
change semantic coordinates, height authority, or admission.
The production path uses a narrow Three.js `WebGPURenderer` and TSL boundary.
WebGPU is preferred and Three.js's WebGL2 backend is the
compatibility path. The implemented adapter awaits asynchronous initialization,
can force WebGL2 for qualification, bounds viewport pixel work, records the
active backend, disposes resources, and fails closed to reference status. It is
the live base-terrain surface when initialization and the first render succeed.
The renderer-independent reference path remains visible until that point and
preserves scene semantics and visible degradation when acceleration is
unavailable. `?renderer=webgpu`, `?renderer=webgl2`, and
`?renderer=reference` are view-envelope qualification controls; they do not
change evidence or execute a probe.
The production build emits stable lazy chunk names for the globe, terrain,
WebGPU adapter, and pinned Three.js module, and the `rey ui` server embeds and
serves each chunk. Starting the documented CLI surface therefore reaches the
same accelerated path as a direct Vite build.

## Semantic Lens

Zoom is a semantic operation, not only a CSS transform. The target lens owns a
continuous camera scale and projects six deterministic levels of detail:

| Level                     | Projection posture | Operator posture                                                        | Target object grammar                                                                                                                             |
| ------------------------- | ------------------ | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| World / projection        | World globe        | Understand admitted regions and global topology                         | Semantic sphere, sectors, regional clusters, major admitted POIs, atlas revision, boundedness, and global omissions                              |
| Atlas / topographic       | Mercator chart     | Choose a sector or admitted county                                      | Sector polygons, county footprints, coarse scene terrain, major natural/constructed systems, survey boundaries, frontier, and unexplored regions |
| Landscape / telescope     | County isometric   | Survey one county and find concentrations or unresolved direction       | Continuous relief, boundary, watersheds, highways, districts, major roads, landmarks, workloads, and attention aggregates                       |
| Neighborhood / mesoscopic | County isometric   | Compare local structures around one coordinate                          | Relief plus exact anchors, local roads, lots, structures, utilities, admitted beacons/construction, requests, attention, and relationships       |
| Object / microscope       | County isometric   | Inspect the machinery within a selected coordinate                      | One parcel, feature, artifact, file, document, symbol, package/context binding, graph, scenario, dependency, or directed delta                    |
| Evidence / specimen       | County + overlay   | Inspect the exact basis of an object or relation                        | Native source/tile, span, row, graph node, diff hunk, admission result, validity, omission, bound, and lineage                                    |

Projection posture and semantic detail are orthogonal contracts. Geometry can
morph at a posture boundary while semantic LOD cross-fades its own admitted
layers. The current six named levels remain the public evidence ladder.

### Globe, Mercator, and county transforms

World presents `rey.semantic-atlas.v1` as a high-fidelity lit sphere. Drag
rotates that sphere and zoom preserves the semantic focus under the pointer.
As World approaches Atlas, the same sector and region vertices unwrap into a
horizontally wrapping spherical-Mercator chart. Rey calls it **semantic
Mercator** because its inputs are synthetic semantic longitude/latitude rather
than Earth coordinates. It is not Web Mercator, EPSG:3857, OGC CRS84, physical
distance, or geographic area.

Semantic Mercator clamps at approximately `±85.05112878°`; World retains the
polar caps and Atlas discloses clipped cap contents instead of silently
dropping them. Antimeridian-crossing polygons may split into draw fragments
while retaining one identity. Native GeoJSON/CRS84 coordinates remain native
geographic evidence and require a qualified native-to-semantic region
transform before admission.

The implemented `rey.semantic-mercator-projection@1` browser primitive makes
that mechanism reversible and bounded: it canonicalizes the `360000000µ°`
wrap, reports each chart-copy index, clamps at `±85051129µ°`, returns an
analytic inverse synthetic coordinate, and gives every antimeridian draw
fragment a distinct fragment ID beside one unchanged semantic identity. Its
orthographic World endpoint and interpolation retain the same focus and object
identity. Regional Atlas points and occupied sectors consume this primitive,
and immutable scene snapshots retain its compiler revision. Renderer event
picking across repeated chart copies and continuous presentation of the
World-to-Atlas interpolation remain Plan 0003 work.

Atlas partitions the abstract sphere into revision-bound **sectors** and shows
the footprints of admitted **county** scenes within them. Hover or keyboard
focus may raise one sector as transient presentation and expose the exact
admitted measure behind its interest. That lift cannot alter terrain, atlas
layout, or evidence. Entering one selected admitted county expands its local
east/north/up tangent frame and blends into a stylized isometric camera. If no
admitted county exists under focus, the lens stops at Atlas and says so.

Detailed county fabric comes from exact editor packages only after a qualified
scene-admission workload. Its procedural terrain, boundary, hydrology,
roads, highways, lots, structures, utilities, beacons, construction, labels,
and markers remain separate typed layers. Source graph edges never become
roads. Cross-county highways require compatible admitted connector identities;
a beacon does not imply a running poller or relay; construction does not imply
observed work. Candidate packages remain absent from `/explore`.

The projection grammar is part of this contract; [Plan
0003](../plans/0003-scene-to-explorer.md) owns its incomplete implementation
and proof.

The implementation covers all six levels over one persistent spatial scene.
World aggregates admitted regional topographies on the semantic sphere. Atlas
and closer lenses derive charted-land envelopes from displayed admitted
anchors and separate survey horizons from anchors plus retained frontier
points. Retained frontier conditions become local weather fronts without a
line back to their source coordinate. Exact `contains` and `references` edges and shared
coordinate identity remain available at deep inspection levels, but do not
appear as roads, rivers, passages, probe trails, or curation paths.
World now consumes `rey.semantic-atlas.v1`: admitted regional identity remains
separate from synthetic semantic longitude/latitude on a revision-bound sphere.
Those axes have no Earth CRS or physical-distance meaning. Atlas's target
semantic-Mercator transform gives them a familiar wrapping chart without an
EPSG:3857 or Earth claim. Zoom selects retained LOD and never reclusters; only
a changed admitted source set or source topography revision changes the atlas
layout. Atlas
extracts nested contour isolines from a scalar field whose
only height inputs are admitted anchor samples. A deterministic rainfall and
eight-neighbor descent pass accumulates runoff, classifies projected streams
and rivers, and erodes the displayed field before contour extraction.
Overlapping anchor influence produces peaks, ridges, saddles, watersheds, and
drainage basins; anchors remain visible as map points of interest. Landscape
adds more POI labels and survey-state zones, Neighborhood expands the local
station reading, Objects add inspection cards around the selected POI, and
Evidence adds bounded locator, relationship, and lineage
detail. Coordinates and POI positions do not jump when the level changes.
When no survey patch is admitted, Atlas labels the topography unexplored and
falls back to the narrower portfolio projection.

Relief is an evidence projection, not an embedding claim. Current local
topography places anchors deterministically and derives their prominence from
admitted seed and resolution sampling rather than graph degree. Contour
geometry communicates projected sample concentration and runoff erosion only.
Weather, rainfall, watercourses, and erosion are deterministic presentation
parameters rather than observed natural facts. They do not assert that visual
distance is language similarity, interpolate an unexplored semantic region,
manufacture an untyped relationship, or claim a discovered path. A future provider may bind high-dimensional semantic
coordinates, but must expose that coordinate revision and projection contract
before Rey may render semantic distance as observed terrain.

The canvas supports pointer-centered wheel zoom, discrete semantic zoom
controls, drag-to-pan, keyboard `+`, `-`, and `0`, selection-driven traversal,
Relief/Water/Weather/Probes visibility controls, and a native full-screen mode. A
control step cannot skip a semantic regime. Selecting a World POI advances to
Atlas, then through Landscape, Neighborhood, Object, and Evidence while
centering that same POI. Level boundaries retain
hysteresis so small wheel reversals do not flicker between grammars. The
terrain camera spans `0.05..=5.4`; zooming out can reveal additional admitted
survey scenes as the bounded world grows, and zooming in progressively admits
denser visual layers without replacing the map.

The map reading separates navigation from epistemic change. A selected anchor
reports its admitted local sample conditions without deriving a route from
source edges. A selected frontier names the required prerequisite: widen a
bound, revalidate, admit a resolver, obtain authority, curate a locator, or
verify absence. Its weather front indicates unresolved boundary pressure but
does not supply a crossing. Discovered and constructed paths require a
separate future typed artifact. Selection and layer toggles never reshape
relief; only a later admitted patch with changed anchors, sampling, coverage,
omissions, or frontier can change terrain.

In the implemented camera, wheel zoom keeps the semantic coordinate beneath the
pointer stationary and control zoom keeps the selected focus stationary.
Level-of-detail boundaries use hysteresis so small changes do not flicker. The
coordinate and source identity survive every visual grammar change.

`/explore`, its exact coordinate routes, and `/feed` own exactly the browser
space remaining below Rey's application chrome. Explorer is height-locked to
`100dvh`; its document cannot scroll and wheel input moves the semantic lens.
Feed is also viewport-bound, but explicitly divides its remaining space into
independently scrolling vertical streams and a narrow Firehose control rail.
Additional streams extend horizontally instead of creating document scroll.
`/cadence`, `/agents`, `/environment`, and `/workloads` remain ordinary
scrollable documents.

## Projection Invariants

- Source identities and assessments survive a lens transition. Representation
  and information density may change; source truth may not.
- Projection basis, scene compiler, field derivation, material, render-graph,
  LOD, and renderer revisions remain distinguishable. Camera motion and
  measured frame time do not enter semantic scene identity.
- Coordinates remain semantic addresses. Camera center, scale, viewport,
  selection, and lens regime may be shareable view state but never become part
  of resource identity.
- The map is composed only from admitted topography patches. Empty space must
  distinguish surveyed-empty from unexplored, omitted, stale, unsupported, and
  frontier regions; visual interpolation is not evidence.
- Before that map exists, the orientation globe may show exact file-backed
  workload beacons. It must label itself unmapped, disclose its
  presentation-only coordinate authority, and never render those beacons as
  terrain, regions, paths, inferred activity, or an admitted semantic atlas.
- Panning, zooming, selecting, or opening a deep link may retrieve and project
  retained evidence. None of those gestures may run a locator, execute a
  survey workload, admit a patch, or silently broaden authority.
- Every projection is bounded. Patch-backed terrain renders at most 64 anchor
  POIs, six frontier POIs, and 96 natural features per admitted
  patch, plus four detail cards around an inspected POI, and reports folded
  rows and admitted patch omissions in the canvas footer. Legacy portfolio neighborhoods remain bounded to eight
  workload/request and eight attention objects.
- Object views disclose folded evidence and dependency references rather than
  pretending one displayed reference is complete.
- The selected focus remains a typed coordinate. It cannot grant access,
  execute a workload, admit an action, or resolve its own attention row.
- Relationship labels carry meaning; geometry is a navigation aid.
- Source relationships and shared-coordinate identity remain inspection
  evidence and never appear as terrain transport. Weather fronts do not imply
  crossings; streams and rivers do not imply discovered or constructed paths.
- World envelopes bound displayed evidence and retained frontier, not the
  unknown context universe. No global area or coverage percentage is inferred.
- Color is redundant with family, label, state, and relationship text.
- Shading, antialiasing, occlusion, tint, smoothing, and simulated erosion may
  improve spatial legibility but cannot change height-channel semantics,
  validity, source assessment, or proof status.
- Renderer loss, context loss, or unavailable acceleration must preserve the
  last valid scene and expose fallback or degradation instead of returning a
  semantically different map.
- Full screen changes only viewport ownership. It does not change scope,
  authority, limits, or the underlying topology.
- Do not introduce a second scroll plane around the canvas. If application
  chrome or explanatory copy grows, the canvas must still fit the remaining
  viewport rather than causing document scroll and making the wheel ambiguous.
- Passive revalidation may replace the source snapshot, but it cannot silently
  mutate runtime state. Explorer remains read-only even when the UI admits a
  separate Journal entry.
- The fixed footer is a live communications channel. Its mailbox contains only
  typed attention or revalidation failure evidence; zero messages explicitly
  means no operator attention is requested. `MAILBOX` selects that history
  axis; the center chevrons select the separate operator/Rey/agent conversation
  axis. Selecting the active axis closes the plane, selecting the other axis
  switches it, and either Escape or a click on the background closes it.
- A coordinate whose `revision` no longer matches is stale. A coordinate
  whose identity is absent is missing. Neither may silently drift to a current
  object while retaining the old URI.

## Coordinate And View URIs

The implemented standalone semantic coordinate is:

```text
rey+local://{kind}/{identity}?revision={revision}[&role={agent-role}]
```

Current kinds are `portfolio`, `cluster`, `workload`, `attention`, `agent`,
`workspace`, `file`, `document`, `external_resource`, and `topography`.
Every coordinate is revision-bound. Agent coordinates alone require
`role=coding_harness|rule|human`. Query dimensions serialize in the exact order
`revision`, `role`; duplicates, unknown dimensions, missing values, invalid
roles, and non-canonical encodings are rejected.

The browser view envelope is:

```text
/explore?coordinate={percent-encoded-coordinate}&scale={canonical-number}
```

For example:

```text
/explore?coordinate=rey%2Blocal%3A%2F%2Fagent%2Fcodex%3Frevision%3Dgpt-5%26role%3Dcoding_harness&scale=2.05
```

`scale` is presentation state and never enters the coordinate identity. The
accepted range is `0.05..=5.4`, with deterministic World, Atlas, Landscape,
Neighborhood, Object, and Evidence stops inside that continuum. The selected
coordinate anchors the camera; free pan and viewport remain ephemeral. The
scene extent is derived from the bounded projection instead of a fixed world
rectangle. The matrix route is outside the current contract. Journal v2 stores
coordinate and numeric scale as separate fields and derives the browser
envelope. [Locators](LOCATORS.md) owns coordinate parsing and bounded
resolution; this document owns World geometry and read-only probe navigation.

## Implemented Routes

`GET /` redirects to `/explore`. The application routes are:

- `/feed`: a TweetDeck-like workspace whose default rich Git/environment/
  Journal Signals, current inspect-only Admission, and admitted workload Flow
  streams can be composed from the Firehose;
- `/explore`: the context-topology canvas and default human entry;
- `/explore?coordinate=...&scale=...`: an exact coordinate-bound camera view;
- `/cadence`: partially ordered Git, Rey-admission, and passive-scan clocks;
- `/agents`: the Explore-bound Journal index and observed-work ledger; derived
  system entries remain distinct from retained human/agent documents;
- `/journal/new`: the unauthenticated, validated human Journal composer;
- `/journal/{slug}`: one exact retained Journal document, with typed blocks
  addressed by `#block-{block-id}` fragments;
- `/environment`: three stacked Kinetic Precision evidence sections over the
  exact typed `HEAD → INDEX → WORKING` environment delta—directed text,
  bounded search, and the reference plane;
- `/workloads`: separate dense tables for admitted revisions and creation
  requests, preserving aligned conformance, graph, evidence, mining, attention,
  intent, admission, source, and target dimensions; and
- `/workloads/$workloadId`: dense runtime/request posture and exact binding
  relations, plus the admitted revision's bounded mining output.

The Refresh control has been removed. The root workload, mounted Feed,
environment, Cadence, and Journal projections passively revalidate every 5000
ms from their typed GET endpoints. Revalidation changes only the browser
projection; it does not invalidate the route, reset viewport or scroll state,
test, run, create, add, commit, or schedule work. Failed background reads retain
the last good projection and remain visible as delayed revalidation.

Journal entries point at Explorer; they do not enter its source topology by
being admitted. See [Collaboration Journal](JOURNAL.md) for the typed notebook,
author paths, and separate execution boundary.

`/feed` does not replace Cadence or portfolio attention. Its independently
scrollable streams are bounded lenses over one Firehose: Signals carries rich
posts and source bounds, Admission carries current proposals and rationale, and
Flow carries admitted workload progress. The default three lanes can be tuned,
reordered, removed, or repeated, and the Firehose rail can add up to eight
lanes. Signal lenses select all, Journal, Git, or environment records;
Admission lenses select all, NOW, WATCH, or BOUND posture; Flow lenses select
all, attention-bearing, failing, or qualified workloads. The exact composition
and each optional human stream name are encoded in the `streams` URL parameter
rather than retained as new runtime state. Clicking a stream title edits it
inline and autosaves on blur or Enter. Post evidence is collapsed by default
and expands in place.

Timestamped Signals use newest-first display order followed by source-ordered
records with no wall time. This is not causal order, unread state, or a durable
global event log. Admission is inspect-only and cannot move a post into Flow.
The first slice renders at most 64 recent Signals and reports older folded
records. [Interfaces](INTERFACES.md) owns the exact Feed and HTTP boundary.

`GET /api/v1/cadence` returns `rey.ui-cadence.v1`. Its leading repository-state
plane separates working-tree attention from the exact local-upstream push
relation. The remaining lanes keep newest-first Git reachability and
environment sequence separate, report truncation and shallow boundaries, and
describe existing browser scan contracts without claiming server-side or
runtime scheduling. Git tick publication is relative to a retained local
tracking-ref OID and never implies a network fetch. Environment commit v1 has
no wall time, so those ticks explicitly render as order-only.

The global footer displays a typed-attention history mailbox, chevrons that
open the traditional conversation axis of the same plane, and the shortened Rey implementation Git revision linked through
the complete revision to the canonical GitHub commit. This is separate from
the BLAKE3 portfolio-attention identity: semantic evidence digests must never
be presented as source commits. Mailbox history is currently only the mounted
projection. The conversation axis projects the bounded workspace-local
transcript and conditionally appends through the exact session-declared human
browser writer. An unavailable transcript or writer keeps the composer
disabled; every retained append remains delivery-not-attempted and carries no
agent invocation, Channel relay, action, or proof authority.

The implemented Explorer topology is derived from `rey.workload-list.v1`:
exact workload packages, HEAD/INDEX/WORKING revision state, drafts,
graph/scenario/mining counts, portfolio
attention, retained `rey.topography-patch.v1` artifacts, and their exact
`rey.projection-packet.v1` envelopes. It also consumes the deterministic
`rey.semantic-atlas.v1` projection of admitted survey patches and the latest
accepted production `rey.scene-admission-result.v1` for each regional workload.
Production survey runs and accepted production regional-scene runs expose one
retained bounded atlas history and directed typed delta. Survey and regional
members remain separate evidence families. Each regional member binds the exact
scene, admission, package revision, packet, and unchanged synthetic placement.
The immutable browser scene and bearing bind the latest delta only when its
target and the last retained revision equal the current atlas; qualification
fixtures and list/UI reads cannot advance it.
With a fresh
`.rey`, it projects incoming workload file state as an unmapped orientation
globe and directs the operator toward consensual survey admission. At World the
reference backend renders an accessible orthographic sphere and the Three.js
backend renders a lit WebGPU-first globe; both bind the same admitted evidence.
Regional results must match their retained atlas member, which preserves the
packet's exact synthetic point with no inferred radius. Atlas applies semantic
Mercator to that same point and its stable occupied synthetic sector, and closer
lenses show exact native object bounds inside the scene envelope. Native County
footprint geometry and regional terrain remain explicitly absent. Survey
terrain fails closed unless the
packet source patch and topography revision match. Packet objects, validity,
extent, limits, and omissions now direct the existing SVG reference scene; the
separate `/environment` route consumes `rey.environment-status.v1` and renders
its exact variable, application, input, and reference operator projection.
`/agents` consumes the workload-list document at a higher semantic level: it
ranks current requests and attention as recommendations, then summarizes work
supported by retained test, run, mining, delta, and revision evidence. Agent
runtime discovery remains on `/environment`. Generator provenance still
supplies agent neighborhoods in Explorer, but it is not
presented as runtime availability, live activity, or assignment. The Explorer
does not yet contain exact environment nodes, Git commit objects, source spans,
scenario deltas, or proof manifests. Aggregates are labeled as aggregates; the
Explorer must not imply that unavailable objects have been rendered. The
workload endpoint returns local admitted topography patches. Remote or
federated coordinates are not part of the current contract.

## Current React Boundaries And Engine Cut

`ExplorePage` owns route composition. `ContextCanvas` owns zoom, pan, focus,
keyboard, and full-screen state while using framework-independent camera math.
`ReferenceRenderer` renders accessible overlays and the deterministic fallback;
it refuses graph edges on terrain even if one is supplied accidentally.
`AcceleratedTerrainSurface` lazily mounts the Three.js adapter and TSL relief
mesh, reports its selected backend, active terrain bands, bounded
field/triangle counts, and transient working-set allocation, and retains the
reference terrain through initialization or failure. At World it materializes
the semantic globe rather than the local terrain mesh, while the reference
overlay preserves region labels and accessibility.
`buildTopologyScene` is a deterministic read-model projection over
`rey.workload-list.v1` and is tested separately from browser mechanics. It
requires an exact patch/packet pair before compiling admitted survey terrain,
and an accepted non-scenario scene result with exact workload, graph,
capability, package, snapshot, packet, terrain, coordinate-plane, and placement
bindings before compiling regional World/Atlas/County objects. Typed
field derivations live under `src/explore/terrain`; `topology.ts` still owns
their scene adaptation plus contours and natural-feature overlays.

This is current repository truth, not the target ownership shape. [Plan
0003](../plans/0003-scene-to-explorer.md) completes typed evidence adapters,
immutable scenes, camera/LOD/invalidation, the render graph, renderer backends,
picking, and React overlays. Future windows and lenses add typed engine inputs
rather than fetch or invent a second graph inside a visualization component.

## Next Boundaries

The seed-to-map voyage, World globe rotation, and one exact
editor-to-admission-to-regional-Explorer voyage are implemented and verified
through the human CLI, structured workload endpoint, deterministic Explorer
read model, and reference renderer. [Plan
0003](../plans/0003-scene-to-explorer.md) now owns the critical path: carry the
reversible semantic-Mercator geometry through repeated-chart renderer picking
and continuous focus-preserving World-to-Atlas presentation, expand the bounded
County into a local
isometric surface with exact links, finish the engine/render-graph boundary,
and qualify detailed terrain and typed constructed layers. Exact scenario/delta
routes retain the delivered CLI `-v`/`-vv` evidence ladder in the browser
without adding an independent assessment. Travel, trade, and economic layers
require their own
typed qualified evidence and are not inferred from survey edges or visual
proximity.

Browser mutation, workload campaign controls, authentication, multi-user
scope, remote deployment, and remote streams remain separate decisions.
