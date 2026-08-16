# Explorer

Explorer is Rey's evidence-bound spatial engine. It gives a human one
continuous journey from global orientation to exact source evidence without
changing the identity, scope, or authority of what is shown.

It should have the visual and interaction fidelity of a specialized 3D
geospatial game engine. That fidelity serves understanding: globe, map,
terrain, atmosphere, lighting, simulation, and level of detail are
presentation instruments. They cannot create evidence, infer coverage, or
grant permission to act.

This document defines the product principles and fidelity bar. The
[`@rey/explorer` technical guide](../packages/explorer/README.md) describes the
package boundary, current implementation, APIs, limits, and tests.

## First Principles

### 1. One identity, many postures

A globe, a Mercator map, and a local terrain scene are views of one admitted
world. Changing geometric posture must not replace the selected region, move
an attached feature to a new semantic coordinate, or break its evidence link.

### 2. One projection owns placement

The surface and everything attached to it use the same coordinate projector.
Stipple, polar fabric, admitted sectors, markers, pick targets, and later
vector geography must unfurl with the world. A screen-space rectangle sliding
over a changing globe is not a projection.

### 3. Keep four kinds of state separate

```text
source truth       what is admitted and what it means
projection posture how one semantic coordinate becomes world geometry
presentation       atmosphere, fabric, light, tint, and transition treatment
camera state       center, scale, viewport, orbit, pan, and focus
```

These layers cooperate, but none substitutes for another. Camera movement is
not a new resource. A visual effect is not an observation. A projection is not
a coordinate authority.

### 4. Perceptual continuity is engineered

Sharing one morph value is not enough. Spherical atmosphere, a globe
scaffold, and surface stipple have different jobs and therefore different exit
curves. Materials may need different contrast at globe and map postures to
look continuous to a human.

### 5. Every bounded gesture makes visible progress

Wheel and trackpad input accumulate into a smooth camera target. A valid input
must not disappear inside a scale plateau where internal state changes but the
render does not. Zoom remains anchored under the pointer; a drag on the world
orbits, while a drag outside it pans.

### 6. Unknown remains unknown

High visual fidelity never licenses invented world geometry. Validity survives
field generation, simulation, meshing, level of detail, and rendering.
Unexplored, surveyed-empty, omitted, stale, unsupported, truncated, and
frontier space remain distinguishable.

### 7. Closer means more spatial truth, not more dashboard

Zooming past the atlas must reveal admitted continuous terrain and exact
features in their spatial frame. Cards may explain a selection, but they
cannot stand in for a terrain lens or float as the world itself.

### 8. Qualify the journey, not only its endpoints

A correct globe and a correct flat map can still have a broken transition.
Qualification samples intermediate geometry, attachment, opacity, contrast,
camera anchoring, and backend fallback. The operator should never have to
mentally repair a detached sector, dead wheel notch, residual halo, or sudden
identity swap.

## The Fidelity Standard: Globe To Mercator

The World-to-Atlas transition is the reference example for the whole engine.
It establishes five simultaneous forms of continuity:

| Continuity  | Required result                                                                                             |
| ----------- | ----------------------------------------------------------------------------------------------------------- |
| Semantic    | One region, marker, or selection retains one identity and evidence basis.                                   |
| Geometric   | One reversible projector moves the surface and attached geometry through every intermediate posture.        |
| Perceptual  | Atmosphere, scaffold, fabric, light, and contrast enter and leave when their visual purpose begins or ends. |
| Interaction | Zoom is smooth, anchored, responsive to every bounded input, and stable across lens thresholds.             |
| Evidentiary | Validity, omissions, limits, coordinate authority, and source lineage survive the transition.               |

The projection path is one surface, not a scene swap:

```text
admitted semantic coordinate + stable identity
                       │
                       ▼
        shared reversible projector(progress, orientation)
          ├─ surface mesh
          ├─ stipple and polar fabric
          ├─ sectors, regions, and markers
          └─ picking and accessible anchors

presentation progress ─ atmosphere · scaffold · material response
camera progress       ─ scale · center · viewport · focus
```

At the World endpoint, the globe is unmistakably spherical. Its atmosphere is
clearly visible, its stipple reveals curvature, and its poles are legible in
the same subtle fabric rather than through labels.

As the lens moves toward Atlas:

- the globe unfurls and expands into the available canvas;
- every sector and marker stays fixed to its semantic coordinate and scales
  with the surface;
- the atmosphere shares the projected surface, while its outward shell
  thickness contracts and fades faster than the surface morph so it never
  becomes a halo inside the map;
- its shell extent and light are functions of projection posture, not traversal
  direction, so reversing the lens cannot switch into a brighter halo mode;
- while repeated Atlas charts remain, wrapped copies of the exterior warm
  shell echo on the x-axis and ease away with the bounded dissolve;
- the non-geographic circular scaffold fades without shrinking, avoiding a
  collapsing gray disc;
- the stipple darkens as the background and surface flatten so the same fabric
  remains perceptually legible; and
- semantic level of detail may change independently of projection progress,
  with hysteresis around grammar boundaries.

At the Atlas endpoint, the map fills its intended canvas, attached geography
is stable, texture remains readable, and no spherical atmosphere or globe body
is left behind. The transition must not pass through a white globe with a
floating sector or end in a collection of cards.

These are not ornamental polish requirements. Each one tells the operator
whether the engine is preserving a coherent world.

## The Fidelity Standard: Landscape

Landscape is a map-first geographic posture, not a model viewer. Its entry
camera is near north-up and mostly overhead, the admitted region fills the
available canvas, and relief remains legible without exposing a floating slab,
mesh grid, or stage beneath the world. Orbit is an intentional inspection
gesture after that stable cartographic entry; it is not the default
composition.

A high-fidelity consumer terrain map is the minimum qualitative reference for
the posture. This is a hard visual acceptance target rather than permission to
copy imagery or fabricate detail. At 1920x1080, a side-by-side review must find
no major perceptual gap in:

| Dimension   | Landscape acceptance                                                                                     |
| ----------- | -------------------------------------------------------------------------------------------------------- |
| Composition | Near north-up, mostly overhead, continuous geography fills the canvas.                                   |
| Relief      | Fine multi-scale ridges, valleys, drainage, and broad landforms read at once through smooth hillshade.   |
| Surface     | Land-cover materials form coherent fields; individual source cells and polygon outlines do not dominate. |
| Water       | Water bodies and channels follow the terrain and remain visually distinct from routes or boundaries.     |
| Contours    | Scale-aware contours reinforce relief without becoming the primary texture.                              |
| Vectors     | Roads, railways, boundaries, structures, and labels enter as a later cartographic hierarchy.             |
| Validity    | Unsupported space remains an unmistakable absence without turning the valid region into a floating tile. |

Fictional geography still has source discipline. Rey County is authored and
refined by an agent, but the renderer does not invent it while drawing. The
authoring path is explicit:

```text
editor packages
  -> admitted semantic and geographic constraints
  -> agent geography compiler
  -> explicit seam and conflict resolution
  -> admitted multi-resolution field pyramid
  -> relief + water + land cover + contours
  -> roads + railways + structures + labels
  -> evidence and accessible interaction
```

The engine therefore keeps three planes separate:

- **Evidence topology** says which packages, objects, relationships, bounds,
  unknowns, and authorities exist.
- **Geographic synthesis** turns admitted authoring constraints into explicit,
  reviewable fictional elevation, hydrology, land cover, and vector artifacts.
- **Cartographic rendering** projects those artifacts with camera, lighting,
  materials, level of detail, labels, and transitions.

Agent synthesis may stitch multiple admitted packages into one coherent world,
but its inputs, algorithm revision, seams, conflicts, omissions, output
identity, and validity must be inspectable before the result can enter
Explorer. Presentation-only microdetail may improve perception inside valid
support when it is identified as presentation; it cannot be reported as
authored elevation or used to extend coverage.

## One Continuous Spatial Journey

```text
World globe → Atlas map → Landscape → Neighborhood → Object → Evidence
   bearing      region      terrain        vicinity       thing     exact basis
```

- **World** answers which admitted regions, sectors, clusters, and frontiers
  exist.
- **Atlas** answers which admitted region the operator should enter.
- **Landscape** reveals the continuous relief, watersheds, districts, routes,
  and landmarks shaping one admitted region.
- **Neighborhood** resolves nearby anchors, lots, structures, utilities,
  requests, attention, and relationships.
- **Object** focuses one exact feature, artifact, file, graph, scenario,
  dependency, or delta.
- **Evidence** exposes the native source, span, row, node, diff hunk, validity,
  limits, omissions, and lineage behind the visible claim.

Projection posture and semantic detail are orthogonal. A surface may still be
unfurling while features and labels cross their own level-of-detail thresholds.
Selection and source identity survive both changes.

Explorer has two valid starting states. An **orientation globe** may place
exact workload beacons in deterministic presentation coordinates when no map
is admitted; those coordinates are not a semantic atlas. An **admitted world**
may place retained survey regions and qualified regional scenes on a
revision-bound semantic globe. Only admitted evidence shapes that world.

## Engine Responsibilities

```text
admitted evidence
  → evidence adapter          meaning, authority, validity, lineage
  → shared projection         stable coordinates and reversible placement
  → scene + bounded fields    immutable geometry and material inputs
  → camera + independent LOD  view state and semantic detail
  → ordered render graph      explicit evidence/derived/presentation passes
  → accelerated + reference   pixels, accessibility, and visible degradation
  → exact evidence links
```

Evidence adapters decide what values mean. Projection decides where admitted
values appear. Scene and field compilers produce bounded immutable inputs. The
renderer turns those inputs into pixels. The application owns interaction,
labels, accessibility, and evidence navigation. No downstream layer may
reinterpret an upstream semantic claim.

The render graph distinguishes four authorities:

- **Evidence** exposes admitted support, boundaries, objects, and source
  identity.
- **Derived** computes bounded terrain, contours, hydrology, grouping, or other
  deterministic projections.
- **Presentation** adds light, atmosphere, tint, occlusion, smoothing, and
  animation.
- **Interface** supplies labels, selection, diagnostics, links, and accessible
  alternatives.

Only the first category contains source observations. Derived and presentation
passes retain their input and algorithm revisions and cannot upgrade coverage,
confidence, progress, or proof.

## Coordinate Discipline

Similar-looking coordinates are not interchangeable:

| Space                      | Meaning                                                                                         |
| -------------------------- | ----------------------------------------------------------------------------------------------- |
| Native OGC CRS84           | Provider-qualified longitude, latitude, and optional altitude.                                  |
| Synthetic semantic sphere  | Revision-bound longitude and latitude arranging admitted context globally; not Earth geography. |
| Semantic Mercator          | Reversible wrapping chart of that semantic sphere; not EPSG:3857.                               |
| County-local east/north/up | Bounded local frame derived from one admitted regional scene.                                   |
| Camera/view                | Ephemeral pan, orbit, scale, viewport, and selection.                                           |

World preserves the poles. Atlas discloses its Mercator latitude cutoff,
splits antimeridian geometry into draw fragments without splitting semantic
identity, and inverse-picks wrapped copies back to one canonical coordinate.
Canonical resource and view syntax lives in [Locators](LOCATORS.md).

## Terrain Fidelity

Terrain should read as a continuous 3D world before contours, labels, or
points of interest are added. The product bar is relief and material response
that surpass conventional consumer maps whenever admitted source resolution
supports it—not colored polygons, abstract cards, or fabricated detail.

The base geographic read is mandatory and precedes semantic annotation:
relief, water, land cover, and contours must already make the region feel like
one place. Roads, railways, boundaries, structures, labels, selections, and
evidence then clarify that place. An outlined feature envelope may retain an
exact source boundary, but it cannot become the dominant Landscape visual
grammar.

```text
validity / background
  → base terrain material
  → normals + multidirectional hillshade
  → ambient and valley occlusion + ridge enhancement
  → LOD-aware contours
  → water + weather + boundaries
  → vector features + labels + selection
  → evidence and accessibility overlays
```

Elevation, normal, curvature, tint, roughness, occlusion, atmosphere,
hydrology, and validity remain separate revisioned channels. Lighting may make
height easier to read; it cannot become authoritative height. Interpolation,
erosion, shading, and feathering operate only where the admitted validity
contract permits them.

The foundational terrain unit is an admitted dataset, not a mesh. A dataset
binds exact source identity, native coordinates, elevation/material values,
validity/no-data, interpolation scope, limits, and lineage before either
renderer sees it. The first delivered unit is a bounded rectilinear regional
grid. Its no-data vertices carry no height or material, and a triangle may
exist only when all three of its source vertices are valid. Reference,
WebGL2, and WebGPU paths consume that same rule.

Acceleration is a bounded projection of that reference dataset. Stable tiles
retain source revision, parentage, shared edge samples, conservative validity,
geometric error, and byte cost. A camera may select and retain tiles under
explicit budgets, but coarse detail can only remove valid support; it cannot
bridge a hole. Tile evaluation and mesh preparation belong in cancellable
workers so interaction does not turn React reconciliation into a terrain
compute loop. Worker loss or budget failure reveals the deterministic
reference surface.

Atlas-to-Landscape continuity is one reversible projection, not a route-time
scene replacement. The selected Atlas sector, exact regional field, footprint,
vectors, and pick identities share one source-to-target mapping. Perceptual
curves control overlap, elevation rise, and camera tilt without becoming
semantic LOD. The bounded camera retains an analytic ground target, north/yaw,
scale, focus, and native-coordinate inverse through both traversal directions.

Terrain fidelity then grows in this order:

```text
admitted dataset + explicit validity
  → renderer-neutral reference field
  → bounded tiles + worker evaluation + resident LOD
  → reversible Atlas ↔ Landscape camera/projection
  → material + water + vectors + label/selection anchors as typed passes
  → provider-qualified imagery when source and license authority exist
  → retained fidelity, continuity, and performance voyages
```

This order is architectural. A richer material cannot compensate for a
point-only dataset; a denser mesh cannot compensate for missing validity; and
a faster renderer cannot compensate for a scene swap at the Atlas boundary.
Each stage must preserve the exact dataset and semantic identity established by
the previous stage.

Geographic passes are executable contracts, not a painter's-order comment.
Each pass binds its implementation, exact input revision, authority, and
dependencies. Material stages can be invalidated independently; vectors and
selection anchors live under the same terrain transform as relief. Every
terrain cell crossed by a draped vector is checked, and the vector splits at
no-data instead of spanning the unknown. Text, evidence links, and accessible
interaction remain in the deterministic reference overlay. In the absence of
an admitted imagery provider and license authority, an empty imagery pass is
more truthful than familiar-looking synthetic map texture.

## Interaction And Admission

- Panning, orbiting, zooming, selecting, and opening a deep link are read-only.
  They never run a locator, schedule a workload, admit a scene, or widen read
  authority.
- Map pan wraps and recenters horizontally without changing the canonical
  semantic coordinate.
- A missing admitted region stops traversal instead of choosing an arbitrary
  scene.
- Camera motion stays quiet. The footer asks for attention only when retained
  map state, focus, revisions, revalidation, or renderer degradation changes
  materially.
- Full screen changes viewport ownership only.

Explorer is also the read side of a separate authoring boundary:

```text
native geospatial files
  → editor WORKING → reviewed INDEX → immutable SCENE@n candidate
  → qualified scene-admission workload
  → admitted regional scene → projection packet → Explorer
```

Candidates are not evidence. Admission freezes exact native objects,
coordinate transforms, validity, limits, omissions, and lineage before a
scene may affect `/explore`.

## Qualification Standard

A rendering change is incomplete until its human journey is qualified.

- Pure tests prove reversible projection math, bounds, deterministic scene
  identity, attachment, validity, and resource accounting.
- Scene tests prove declarative structure and stable object identity.
- Browser voyages exercise real wheel, trackpad, drag, selection, resize,
  fallback, and loss behavior.
- Transition checks sample intermediate frames, not only globe and map
  endpoints, and retain exact backend, revision, limits, and omissions.
- Named Landscape captures separately qualify steep relief, low relief,
  coastline/water, dense vectors, explicit holes, stale data, and backend loss
  at both target viewports. A capture binds a real admitted fixture; the harness
  never fabricates one.
- Landscape manifests retain source validity and relief, screen-space error,
  tile seams, no-data leakage, pass identity, labels, resident budgets, picking
  continuity, interaction convergence, backend, and exact scene lineage.
- WebGPU, WebGL2 compatibility, and the deterministic reference renderer keep
  semantic parity; unsupported visual fidelity is disclosed rather than
  hidden.

## Product Invariants

- Source identity and assessment survive every camera, LOD, and posture
  change.
- Geometry, material response, input response, and evidence links remain
  continuous through a lens transition.
- Unknown space remains unknown.
- Geometry and proximity aid navigation; typed relationships carry meaning.
- Every scene, field, label set, picking index, and GPU allocation is bounded.
- Rendering failure preserves the last valid scene and exposes degradation.
- Color, lighting, depth, and motion are redundant aids, not sole carriers of
  meaning.
- Exact evidence, omissions, limits, and lineage remain reachable.

## Ownership And Further Reading

`@rey/agent` owns evidence adaptation, semantic projection, immutable scene
snapshots, field evaluation, terrain working sets, the render graph, camera
controls, picking, labels, routes, accessible reference output, and evidence
UI. `@rey/explorer` owns the reusable React Three Fiber canvas, globe and
terrain GPU compilation, declarative 3D scenes, bounded WebGPU/WebGL2
lifecycle, resource accounting, and renderer reports. The dependency remains
one-way: `@rey/agent → @rey/explorer`.

- [`@rey/explorer` technical guide](../packages/explorer/README.md) — package
  architecture, projection mechanics, APIs, limits, tests, and extension path.
- [Architecture](ARCHITECTURE.md) — system planes and ownership.
- [Locators](LOCATORS.md) — semantic coordinates and bounded resolution.
- [Mining](MINING.md) — evidence projection and visualization authority.
- [Interfaces](INTERFACES.md) — browser and structured-data contracts.
- [Plan 0003](../plans/0003-scene-to-explorer.md) — current implementation
  proof and remaining work.
