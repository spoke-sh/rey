# Explorer

Explorer is Rey's evidence-bound spatial interface. It turns admitted context
into a world that a human can navigate from a global bearing to exact source
evidence without changing the identity, scope, or authority of what is shown.

Explorer should feel like a high-fidelity 3D geospatial engine, but it is not
an Earth viewer and it is not a general game engine. Its globe, maps, terrain,
lighting, atmosphere, simulation, and level of detail are instruments for
understanding bounded evidence. They cannot create evidence, infer coverage,
or grant permission to act.

This document describes the product concept and the desired rendering
capabilities. [`@rey/explorer`](../packages/explorer/README.md) documents the
technical package, public API, renderer lifecycle, current implementation, and
extension points.

## The Product Idea

Explorer gives the operator one continuous spatial journey:

```text
unmapped orientation
        │ admit bounded survey or scene evidence
        ▼
World globe → Atlas map → County terrain → Object → Evidence
   bearing      region       local scene     thing     exact basis
```

The geometry may change posture as the operator moves closer, but the selected
semantic identity must remain stable. Zooming from a sphere to a map and then
into local terrain changes how evidence is perceived; it does not change the
underlying source truth.

Explorer has two starting postures:

- An **orientation globe** shows exact workload beacons when no admitted map
  exists. Its coordinates are deterministic presentation scaffolding, not an
  inferred semantic atlas.
- An **admitted world** places retained survey regions and qualified regional
  scenes on a revision-bound semantic globe. Only admitted evidence can shape
  this world.

Panning, orbiting, zooming, selecting, and opening a deep link are read-only.
None of those gestures may run a locator, execute a workload, admit a scene, or
silently widen source authority.

## The Engine Model

Explorer is easiest to understand as a sequence of cooperating components:

| Component | Responsibility |
| --- | --- |
| Evidence boundary | Accept only qualified survey, atlas, and regional-scene inputs with exact revisions, limits, omissions, and lineage. |
| Coordinate model | Keep native geographic, synthetic semantic, local scene, and camera coordinates distinct. |
| Projection | Transform one stable identity between globe, wrapping map, and bounded local-scene postures. |
| Scene compiler | Produce an immutable, stably ordered scene snapshot from admitted inputs. |
| Field system | Materialize bounded elevation, normal, curvature, material, hydrology, weather, and validity channels. |
| Camera and lens | Manage center, scale, viewport, orbit, pan, focus, and semantic level of detail without changing resource identity. |
| Render graph | Order validity, terrain, lighting, contours, water/weather, features, labels, selection, evidence, and accessibility passes. |
| Renderer | Reconcile declarative 3D scenes through WebGPU or WebGL2 while retaining a deterministic reference fallback. |
| Interaction | Pick one semantic object, preserve focus across posture changes, and link back to exact evidence. |

The intended data flow is:

```text
admitted evidence
  → evidence adapter
  → projection packet
  → immutable scene + bounded fields
  → camera + semantic/geometric LOD
  → ordered render graph
  → 3D renderer + accessible reference overlay
  → exact evidence links
```

Each boundary has one kind of authority. Evidence adapters decide what values
mean. Projection and scene compilation decide where admitted values appear.
The renderer decides how those values become pixels. The UI owns controls,
labels, accessibility, and evidence navigation. No downstream component may
reinterpret an upstream semantic claim.

## The Spatial Journey

Zoom is a semantic operation as well as a camera operation. Explorer exposes
six stable levels on one continuous scale:

| Level | Spatial posture | Operator question | Typical content |
| --- | --- | --- | --- |
| World | 3D globe | What admitted regions and global frontiers exist? | Regions, sectors, clusters, major POIs, atlas revision, global omissions |
| Atlas | Wrapping map | Which admitted region should I enter? | Sector polygons, regional footprints, boundaries, frontier, unexplored space |
| Landscape | 3D local terrain | What shapes this region? | Relief, watersheds, highways, districts, major roads, landmarks, workload aggregates |
| Neighborhood | 3D local terrain | What is near this coordinate? | Anchors, roads, lots, structures, utilities, requests, attention, relationships |
| Object | Local scene | What is this exact thing? | Feature, parcel, artifact, file, graph, scenario, dependency, or delta |
| Evidence | Scene plus evidence overlay | What is the exact basis for this claim? | Native source, span, row, graph node, diff hunk, validity, limits, omissions, lineage |

Projection posture and semantic detail are separate. A sphere may be
unwrapping while labels and feature classes cross their own LOD thresholds.
Hysteresis prevents small wheel reversals from flickering between grammars.
Selection, coordinates, and source identity survive every transition.

## Coordinate Spaces

Explorer deliberately uses several coordinate systems. Similar-looking
numbers are not interchangeable.

| Space | Meaning | Authority |
| --- | --- | --- |
| Native OGC CRS84 | Longitude, latitude, and optional altitude from qualified geographic source data | Source/provider evidence |
| Synthetic semantic sphere | Revision-bound longitude and latitude used to arrange admitted context globally | Admitted atlas projection; not Earth geography |
| Semantic Mercator | Reversible wrapping chart of the synthetic sphere | Presentation projection; not EPSG:3857 |
| County-local east/north/up | Bounded local frame derived from one admitted regional scene | Qualified regional transform |
| Camera/view | Pan, orbit, scale, viewport, and selection | Ephemeral presentation state |

The World projection preserves the poles. The Atlas projection discloses its
Mercator latitude cutoff, splits antimeridian geometry into draw fragments
without splitting semantic identity, and inverse-picks wrapped map copies back
to one canonical coordinate.

A semantic coordinate identifies an object or bounded region. A browser view
combines that coordinate with a numeric scale, but camera state never becomes
part of the resource identity. Canonical coordinate and view URI syntax lives
in [Locators](LOCATORS.md).

## Desired 3D Geospatial Capabilities

The engine is being built toward a continuous global-to-local geospatial
experience. The table separates today's foundation from the capability we
want, so target language is not mistaken for repository truth.

| Capability | Current foundation | Direction |
| --- | --- | --- |
| Global globe | Declarative lit sphere, deterministic stipple fabric, atmosphere, polar caps, orbit, occluded regions, and workload beacons | Rich global layers and identity-stable transitions without turning unsurveyed space into world geometry |
| Globe-to-map projection | Reversible semantic Mercator, polar disclosure, antimeridian fragments, bounded wrapping copies, and one declarative accelerated surface shared by the globe fabric, occupied sectors, and markers | Extend the same transition contract to accelerated vector layers, renderer-neutral picking, and accessibility |
| Regional coordinate frames | Qualified CRS84 GeoJSON, County-local transforms, exact footprints with holes, and typed native objects | Multiple qualified geospatial adapters while preserving each provider's CRS, resolution, and source identity |
| 3D terrain | Accelerated continuous relief for admitted survey fields with elevation, normals, curvature, material channels, validity masks, and hillshade | Detailed provider-qualified regional elevation and material surfaces, multiresolution working sets, crack-free seams, and bounded terrain streaming |
| Vector geography | Exact County footprints plus typed point and native-bounds marks for boundaries, hydrology, highways, roads, districts, lots, structures, utilities, labels, beacons, construction, and connectors in the reference scene | Retain exact admitted coordinate sequences and add batched accelerated line, polygon, point, extrusion, and annotation primitives with semantic picking parity |
| Raster and imagery | No general raster or imagery adapter is admitted | Qualified raster elevation, imagery, and material adapters that retain tile/source revision, sampling, no-data, and attribution contracts |
| Field effects | Deterministic bounded hydrology, weather, erosion, occlusion, and material derivation over admitted support | Declarative effects and simulation passes that remain inside exact validity and never masquerade as observations |
| Semantic and geometric LOD | Six semantic levels, bounded field bands, label budgets, and camera-relative terrain patches | Independent content, geometry, material, and label LOD with stable identity and explicit omissions |
| Picking and labels | Analytic inverse map picking, immutable picking index, stable focus, deterministic collision/culling, and accessible overlays | One renderer-neutral interaction contract across globe, map, terrain, vector features, and repeated chart copies |
| Renderer resilience | WebGPU-first React Three Fiber, WebGL2 compatibility, deterministic reference renderer, context/device-loss fallback, and last-good scenes | Backend feature parity with explicit degradation, bounded resource residency, and retained visual qualification |

These capabilities are constrained by evidence semantics. For example, a
beautiful road-like line cannot become a dependency, a simulated river cannot
become a discovered path, and smoothed terrain cannot fill an unexplored
region.

## Terrain And Material Language

Terrain should read as a continuous 3D surface before contours, labels, or POIs
are added. The intended pass order is:

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

The field model carries elevation, normal, curvature, tint, roughness,
occlusion, atmosphere, hydrology, and validity as separate revisioned channels.
Their separation matters: presentation lighting may improve legibility, but it
cannot alter authoritative height or support.

Every field cell has a validity class. Surveyed, surveyed-empty, unexplored,
omitted, stale, unsupported, truncated, and frontier regions remain distinct
through generation, simulation, meshing, LOD, and rendering. Interpolation,
erosion, shading, and feathering operate only where the admitted validity
contract permits them.

## Evidence And Presentation

The render graph classifies its work by authority:

- **Evidence** passes expose admitted support, boundaries, objects, and exact
  source identity.
- **Derived** passes compute bounded terrain, contours, hydrology, grouping,
  or other deterministic projections from admitted inputs.
- **Presentation** passes add lighting, atmosphere, tint, occlusion, smoothing,
  animation, and other perceptual aids.
- **Interface** passes provide labels, selection, diagnostics, evidence links,
  and accessibility.

Only the first category contains source observations. Derived and
presentation passes must retain their algorithm and input revisions and cannot
upgrade coverage, confidence, progress, or proof status.

## Interaction Principles

- Wheel zoom stays anchored to the semantic point beneath the pointer and
  cannot turn a clamped zoom interval into pan-only motion.
- A World drag beginning on the atmosphere or globe orbits the sphere. A drag
  beginning outside it pans the full projection.
- Map pan wraps and recenters horizontally without changing the canonical
  semantic coordinate.
- Selecting an object preserves focus as the lens moves closer. A missing
  admitted region stops traversal instead of choosing an arbitrary scene.
- Full screen changes viewport ownership only.
- Camera motion is quiet. Explorer asks for operator attention only when
  retained map state, focus, source revisions, revalidation, or renderer
  degradation changes materially.

## Authoring And Admission

Explorer is the read side of a separate level-editor architecture:

```text
native geospatial files
  → editor WORKING
  → reviewed INDEX
  → immutable SCENE@n candidate
  → qualified scene-admission workload
  → admitted regional scene
  → projection packet
  → Explorer
```

Editor projects and scene packages are candidates, never evidence. The
admission workload is the only bridge into Explorer. It freezes exact native
objects, coordinate transforms, validity, limits, omissions, and lineage
before a scene can affect the map. See [CLI](CLI.md), [Mining](MINING.md), and
[Workloads](WORKLOADS.md) for those boundaries.

## Product Invariants

- Source identity and assessment survive every camera, LOD, and posture
  change.
- Unknown space remains unknown. A renderer cannot interpolate evidence into
  it.
- Coordinate basis, scene, field, material, render graph, and renderer
  revisions remain distinguishable.
- Geometry and proximity aid navigation; typed relationships carry meaning.
- Every scene, field, label set, picking index, and GPU allocation is bounded.
- Rendering failure preserves the last valid scene and exposes degradation.
- Color, lighting, depth, and motion are redundant aids, not the only carriers
  of meaning.
- Exact evidence, omissions, limits, and lineage remain reachable from the
  visual surface.
- Explorer is read-only even while the surrounding application revalidates or
  admits separate Journal, Observation, or conversation records.

## Current Ownership

The implementation deliberately spans two TypeScript packages:

| Owner | Responsibility |
| --- | --- |
| `@rey/agent` | Evidence adaptation, semantic projection, immutable scene snapshots, field evaluation, terrain working sets, render graph, picking, camera controls, accessible reference renderer, labels, routes, and evidence UI |
| `@rey/explorer` | Reusable R3F canvas, globe and terrain GPU compilation, declarative 3D scene components, bounded renderer lifecycle, WebGPU/WebGL2 selection, resource accounting, and renderer reports |

The one-way dependency is `@rey/agent → @rey/explorer`; the rendering package
cannot import application evidence or policy. The reference renderer remains
mounted until acceleration has produced a valid frame and becomes visible
again after renderer loss.

The current package-level design is documented in
[`packages/explorer/README.md`](../packages/explorer/README.md). The active
delivery and qualification boundary remains [Plan
0003](../plans/0003-scene-to-explorer.md).

## Related Documents

- [Architecture](ARCHITECTURE.md) — system planes and ownership.
- [Glossary](GLOSSARY.md) — canonical Explorer and evidence terminology.
- [Locators](LOCATORS.md) — semantic coordinates and bounded resolution.
- [Mining](MINING.md) — evidence projection and visualization authority.
- [Interfaces](INTERFACES.md) — browser and structured-data contracts.
- [`@rey/explorer` technical guide](../packages/explorer/README.md) — package
  architecture, APIs, rendering pipelines, limits, tests, and extension path.
- [Plan 0003](../plans/0003-scene-to-explorer.md) — current implementation
  proof and remaining work.
