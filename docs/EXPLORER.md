# Operator Feed And Context Topology Explorer

The Rey UI is the human operator's primary collaboration surface. `/feed`
projects high-cadence change through rich Signals, Admission, and Flow streams,
while the Rey
Explorer maps the bounded context Rey can currently explain and lets the operator move
between semantic scales, and preserves exact runtime identities while the
visual grammar changes. Agents continue to use the `rey` CLI as their primary
execution and diagnostic interface.

The Explorer itself is a read-only projection. The adjacent `/agents` Journal
may retain a typed entry that points to an exact semantic coordinate and
numeric camera scale, but that
does not mutate topology or make Explorer a runtime, scheduler, evidence store,
or assessment authority.

## Operator Model

The intended division of labor is:

| Persona | Primary surface | Normal use |
| --- | --- | --- |
| Human operator | `rey ui`, `/feed`, and `/explore` | Triage bounded change, orient on context, traverse attention, inspect workload neighborhoods, and understand the next bearing |
| Agent or coding harness | `rey` CLI and structured output | Create, test, run, diagnose, and revise workloads through admitted contracts |
| Human diagnosing a problem | `rey` CLI | Drop beneath the visual projection to inspect exact command evidence, verbosity layers, stderr, and exit semantics |

The UI and CLI are different projections over the same typed facts. The UI is
not required to mimic a terminal document, but it must retain enough identity,
scope, direction, completeness, limits, and lineage to reach the exact CLI
evidence when investigation requires it.

## Presentation Concepts

The Explorer separates semantic address, retained map evidence, and browser
presentation. Only canvas, lens, regime, camera, focus, and omission are
React/read-model concepts:

```text
Coordinate         = typed provider-qualified Spoke address for an object or
                     bounded region; local bindings carry narrower guarantees
Locator            = candidate address; resolution is a separate bounded act
Context topology   = bounded typed anchors + classified relationships
Context topography = topology + scale + surveyed coverage + frontier +
                     explicit unexplored space
Canvas             = spatial view over one bounded topography projection
Camera             = center + continuous scale + viewport
Lens               = semantic projection(topography, focus, camera)
Regime             = one level-of-detail grammar on the lens continuum
World              = far projection of admitted charts, survey weather, and
                     unresolved survey horizons
Neighborhood       = bounded objects around one meaningful coordinate
Focus              = selected coordinate retained while changing scale
Omission           = evidence that the current projection folded or excluded
                     known objects because of declared limits
```

The current interface hard-cuts the former matrix route. A semantic
`rey+local://...` coordinate and continuous numeric scale are separate values;
the `/explore` query envelope combines them only for navigation. Old matrix
paths are unresolved and have no compatibility parser. This distinction is
defined by [ADR 0041](decisions/0041-continuous-coordinate-topography.md).

Relationships are always labeled. Portfolio projections use `contains`,
`directs`, `produces`, `observes`, and `depends`; admitted survey patches add
exact `contains` and `references` edges. Line placement or proximity alone
does not assert causality, ownership, or authority.

## Semantic Lens

Zoom is a semantic operation, not only a CSS transform. The target lens owns a
continuous camera scale and projects six deterministic levels of detail:

| Level | Operator posture | Target object grammar |
| --- | --- | --- |
| World / projection | Understand the admitted field and where its known boundary ends | Charted-land envelopes, probe horizons, unresolved weather fronts, projected river systems, major POIs, and prerequisite-marked frontier stations |
| Atlas / topographic | Read anchor-shaped relief and see more admitted scenes | Anchor-derived contour isolines, major points of interest, survey boundaries, frontier, and unexplored regions |
| Landscape / telescope | Survey a bounded region and find concentrations or unresolved direction | Persistent relief, anchor POIs, corpora, workloads, evidence, requests, portfolio, and attention aggregates |
| Neighborhood / mesoscopic | Compare local structures around one coordinate | Persistent relief and POIs plus exact anchors, admitted workloads, creation requests, surface-attention rows, and classified relationships |
| Object / microscope | Inspect the machinery within a selected coordinate | Files, documents, symbols, package/context bindings, graphs, scenarios, artifacts, dependencies, and directed deltas |
| Evidence / specimen | Inspect the exact basis of an object or relation | Source spans, rows, graph nodes, diff hunks, omissions, bounds, and lineage |

The implementation covers all six levels over one persistent spatial scene.
World derives a charted-land envelope from displayed admitted anchors and a
separate survey horizon from anchors plus retained frontier points. Retained
frontier conditions become local weather fronts without a line back to their
source coordinate. Exact `contains` and `references` edges and shared
coordinate identity remain available at deep inspection levels, but do not
appear as roads, rivers, passages, probe trails, or curation paths.
The graticule is a semantic orientation aid, not latitude, longitude, or a Web
Mercator claim. Atlas extracts nested contour isolines from a scalar field whose
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
- Coordinates remain semantic addresses. Camera center, scale, viewport,
  selection, and lens regime may be shareable view state but never become part
  of resource identity.
- The map is composed only from admitted topography patches. Empty space must
  distinguish surveyed-empty from unexplored, omitted, stale, unsupported, and
  frontier regions; visual interpolation is not evidence.
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
rectangle. The old matrix route is rejected, not
redirected or migrated. Journal v2 stores coordinate and numeric scale as
separate fields and derives the browser envelope. See [ADR
0041](decisions/0041-continuous-coordinate-topography.md). World geometry and
probe navigation are fixed by [ADR
0042](decisions/0042-world-geometry-and-probe-navigation.md).

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
records. See
[ADR 0039](decisions/0039-bounded-operator-feed.md).

`GET /api/v1/cadence` returns `rey.ui-cadence.v2`. Its leading repository-state
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
projection. The conversation transcript is empty and its composer disabled
because no transport, agent session, message admission, or retention contract
exists yet.

The implemented Explorer topology is derived from `rey.workload-list.v6`:
exact workload packages, drafts, graph/scenario/mining counts, portfolio
attention, and retained `rey.topography-patch.v1` artifacts. The separate
`/environment` route now consumes `rey.environment-status.v5` and renders its
exact variable, application, input, and reference operator projection.
`/agents` consumes the workload-list document at a higher semantic level: it
ranks current requests and attention as recommendations, then summarizes work
supported by retained test, run, mining, delta, and revision evidence. Agent
runtime discovery remains on `/environment`. Generator provenance still
supplies agent neighborhoods in Explorer, but it is not
presented as runtime availability, live activity, or assignment. The Explorer
does not yet contain exact environment nodes, Git commit objects, source spans,
scenario deltas, or proof manifests. Aggregates are labeled as aggregates; the
Explorer must not imply that unavailable objects have been rendered. The
workload endpoint returns local admitted topography patches; Spoke coordinates
remain an explicit opaque-carrier conformance gap and are not presented as
connected semantics.

## React Boundaries

`ExplorePage` owns route composition. `ContextCanvas` owns zoom, pan, focus,
fit, keyboard, and full-screen state. `SemanticLens` renders a pure
`TopologyScene`; regions, objects, and the SVG edge layer are independent
components. `buildTopologyScene` is a deterministic read-model projection over
`rey.workload-list.v6` and is tested separately from browser mechanics.

Future windows and lenses should add typed topology inputs rather than fetch or
invent a second graph inside a visualization component.

## Next Boundaries

Plan 0017's seed-to-map voyage is implemented and verified through the CLI,
structured workload endpoint, and deterministic Explorer read-model tests.
Exact scenario/delta routes should next carry the CLI `-v`/`-vv` evidence
ladder into the browser without adding an independent assessment.

Browser mutation, workload campaign controls, authentication, multi-user
scope, remote deployment, and Spoke-backed streams remain separate decisions.
