# Terrain Pipeline

The accelerated terrain path turns admitted, bounded field grids into a
continuous-relief R3F scene. It never converts missing support, control
geometry, or application cards into terrain.

## Pipeline

```text
TerrainFieldSetInput[]
  → camera-qualified tiles + bounded worker
  → buildTerrainMeshData
  → verifyTerrainMeshParity
  → enforce GPU byte budget
  → CompiledContinuousRelief + revisioned render-pass set
  → ContinuousReliefScene
  → orthographic camera + TSL stages + indexed relief + draped geography
```

## Field Contract

Each `TerrainFieldSetInput` supplies one bounded regular grid:

| Channel   | Representation                  | Use                                                 |
| --------- | ------------------------------- | --------------------------------------------------- |
| Grid      | Columns, rows, and local bounds | Maps cells into local scene coordinates.            |
| Validity  | `Uint8Array`                    | Determines whether a triangle has admitted support. |
| Elevation | `Float32Array` plus scale       | Becomes the mesh Y coordinate.                      |
| Normal    | `Float32Array` or `Int8Array`   | Drives lighting in Three.js coordinates.            |
| Curvature | `Float32Array`                  | Enhances ridges and valleys.                        |
| Tint      | RGB `Float32Array`              | Supplies base material color.                       |
| Occlusion | `Float32Array`                  | Darkens bounded valleys and ambient support.        |
| Roughness | `Float32Array`                  | Controls the TSL surface response.                  |

`@rey/agent` derives these fields from two admitted sources:

- survey terrain programs produce camera-relative, haloed procedural fields;
- regional terrain datasets produce exact bounded elevation fields with
  source-declared per-vertex validity/no-data.

Field generation, dataset interpretation, hydrology, validity classification,
LOD selection, halo evaluation, worker orchestration, and tile residency remain
outside this package.
`@rey/explorer` sees the same structural field contract in both cases and
cannot upgrade either source's authority.

The current Rey County working field refines each admitted source interval four
times, producing a 321×321 renderer-neutral field from the qualified 81×81
dataset before `@rey/explorer` receives it. Every refined point is evaluated
inside the same supported source triangle and carries an independent validity
value. Deterministic microrelief is constrained to zero at admitted source
vertices and remains explicitly presentation-only; diagnostics continue to
report the original admitted vertex counts and elevation range. The reference
renderer selects conservative root tiles instead of mounting hundreds of
thousands of fallback polygons.

That dense working field is then treated as a causal geography graph, not a
bag of independent visual channels. A priority-flood pass resolves local
depressions without crossing no-data, every valid cell receives one bounded
downstream receiver, and a height-ordered accumulation pass carries rainfall
through the complete admitted basin. Presentation-only channel incision is
zero at every admitted source vertex. Normals and curvature are recomputed
from that conditioned elevation; moisture, slope, height, exposure, and
accumulation then derive a coherent land-cover material. Metric contour
intervals tighten by semantic lens, while smoothed synthetic stream and river
linework remains distinctly qualified from exact admitted hydrology.

## Validity-Safe Mesh Compilation

Grid X/Y becomes mesh X/Z; admitted elevation becomes mesh Y. For each quad,
`terrainTriangleIndices` deterministically chooses the diagonal that retains
the most fully valid triangles; equal choices alternate to avoid a directional
bias. A triangle is emitted only when all three vertices have valid support.
This lets a supported half-cell survive next to no-data without bridging the
invalid vertex. The reference renderer uses the same exported index function,
so fallback cannot fill a hole that the GPU path omits.

Compilation creates separate upload arrays for positions, normals, tint,
occlusion, roughness, curvature, and indices. Upload storage is disposable and
cannot mutate the authoritative CPU fields.

`verifyTerrainMeshParity` checks every uploaded field sample after the
coordinate transform and rejects any index touching invalid support. The
current parity identity is `rey.terrain.cpu-mesh-upload-parity@1`.

## Material And Lighting

`createContinuousReliefMaterial` produces a `MeshStandardNodeMaterial` with
TSL. Its separately revisioned pass inputs gate:

- source tint;
- world-space multidirectional hillshade;
- explicit occlusion;
- curvature-based ridge brightening and valley darkening; and
- bounded roughness.

`ContinuousReliefScene` shares the material across compiled meshes and adds
warm and cool directional lights plus ambient fill. Its orthographic camera is
a bounded target/orbit view: the application supplies a near-north-up,
mostly-overhead cartographic entry, pitch and yaw are clamped, screen-axis pan
resolves to one ground target, and an optional model transform keeps the
terrain attached during projection changes. R3F owns the declarative camera and
terrain-group lifecycle; `@rey/agent` owns the explicit Shift-drag orbit
interaction, the semantic projection curve, and the reversible interior
composition scale used to make Landscape fill the viewport. Evidence and
validity boundaries remain mounted while their visual weight is independently
controlled by the application lens.

## Executable Geographic Passes

`@rey/agent` owns `rey.explorer.render-graph@3` and compiles its active terrain
subset into `rey.terrain-render-pass-set.v1`. The package accepts only typed,
already bounded inputs:

| Pass                      | Accelerated result                                                             | Authority retained                  |
| ------------------------- | ------------------------------------------------------------------------------ | ----------------------------------- |
| Validity/background       | Canvas treatment visible through missing triangles; never a terrain slab.      | Evidence support boundary.          |
| Base terrain              | Source/derived material tint on valid triangles.                               | Derived from admitted material.     |
| Height/normals/hillshade  | Multidirectional normal response.                                              | Derived presentation of height.     |
| Ambient/valley occlusion  | Curvature and occlusion response.                                              | Presentation only.                  |
| Contours                  | Conservative terrain-draped line segments.                                     | Derived contour revision.           |
| Water/weather/boundary    | Draped lines plus validity-clipped water-area triangles.                       | Per-feature authority and source.   |
| Features/labels/selection | Draped exact vectors, disclosed bounds fallbacks, and point/selection anchors. | Interface over retained identity.   |
| Evidence/accessibility    | Mounted application reference overlay; no accelerated replica.                 | Exact links and accessible meaning. |

Every pass binds an implementation revision, input revision, and dependency.
The scene identity includes the compiled pass-set identity, while the shared
continuous-relief material identity includes only shader-affecting base-stage
membership. Contours, water, and other vector-overlay revisions can therefore
change without recompiling an identical base-terrain shader. A missing
dependency prevents its children from executing. Presentation graph and
pass-set identifiers compact their sorted exact inputs into a bounded 64-bit
invalidation hash plus input count and character count. The compact identifier
is cache/invalidation mechanism, not source evidence; exact source revisions
and geometry remain on the scene and individual pass inputs.

Line draping is conservative. The compiler adds probes at every crossed field
grid boundary and within every crossed cell, evaluates only fully valid source
support, and splits a line when support is absent. A native vector therefore
cannot bridge a no-data hole merely because its endpoints are valid. The
application also projects an exact admitted hydrology Polygon into a
terrain-following water surface by selecting only supported terrain triangles
whose centroids fall inside its even-odd rings. Its exact vector outline stays
visible, while the filled edge is explicitly quantized to terrain resolution;
no triangle touching a no-data vertex can enter the surface. The surface,
areas, lines, and point anchors share one R3F terrain group and one
Atlas-to-Landscape model transform. Validity is represented by missing
triangles over the canvas background, not by a rectangular mesh that can read
as geographic support. Disconnected valid line intervals remain independent
endpoint pairs but batch once per source feature into an R3F `LineSegments`
object. Batching changes draw mechanism, never the validity cuts or source
identity.

Text labels, evidence links, descriptions, and pointer semantics deliberately
remain in `@rey/agent`'s deterministic reference overlay. It stays mounted
under the accelerated surface and becomes visible on backend failure. No
imagery source or license authority is currently admitted, so the package does
not synthesize an imagery layer from elevation or styling.

## Tiling, Workers, And Residency

`@rey/agent` projects admitted regional grids into
`rey.terrain-tile-pyramid.v1`. Tile identities bind the source field and
revision, level, row, and column. Every level shares exact edge samples and
validity borders. Coarse validity is conservative: a no-data source sample may
remove coarse support but cannot become a valid coarse vertex. Camera
selection chooses a uniform level from measured geometric error, preventing
mixed-level edge cracks while retaining screen-space control.

`rey.terrain.compilation-worker@1` runs tile projection, resampling, relief
derivation, procedural field evaluation, parity checking, and mesh preparation
in a cancellable dedicated worker. The deterministic reference field remains
visible while work is pending or after failure. A disclosed main-thread
fallback exists where `Worker` is unavailable. `rey.terrain.tile-residency@1`
retains compiled tiles under independent 48 MiB CPU and 64 MiB GPU budgets and
evicts the oldest unrequested identity first.

When an Atlas contains exactly one admitted regional terrain field, the
application may mount an invisible `rey.explorer.atlas-terrain-prewarm@1`
terrain canvas after the Atlas camera has remained stable for 600 milliseconds
and before selection. Any camera movement cancels and reschedules that idle
work. It compiles only that already admitted field; it does not select the
County, change the camera, render coverage, execute a locator, or widen
evidence. The same field identity and resident compilation remain available
when Atlas-to-Landscape traversal begins, avoiding an avoidable
first-visible-frame setup stall without racing an active projection morph. The
application exposes `scheduled`, `mounted`, and `submitted` states so
qualification can require actual renderer submission instead of inferring
warmth from elapsed time.

## Bounds And Accounting

The compiler measures exact typed-array byte length before rendering and
rejects output above `MAX_ACCELERATED_TERRAIN_GPU_BYTES`, currently 64 MiB.
Statistics retain field-set, tile, level, vertex, triangle, source-byte,
resident CPU/GPU byte, budget, hit, miss, eviction, parity-sample, update,
projection, evaluation, geometry-compilation, draw, and submission counts or
timings. Browser diagnostics additionally retain source valid/no-data vertex
counts and relief span, maximum selected screen-space error, shared validity
seam mismatches, and no-data triangle leaks. GPU execution time remains
explicitly unavailable without a capable GPU timer.

## Current Boundary

The package owns the accelerated continuous-relief material and typed
geographic area/line/point presentation. The application still owns native geometry
interpretation, pass compilation, label layout, picking, evidence links, and
the accessible reference renderer. The application projects exact retained
Point, LineString, and Polygon coordinates; wider GeoJSON families use a
disclosed bounds fallback. Structure footprints remain two-dimensional; a
later qualified mesh adapter may add volumetric geometry without changing
their authority.

Cartographic text is a separate presentation pass. The application consumes
only exact scene-admitted label metadata, applies its admitted zoom interval,
and uses deterministic selected-first collision culling. A hidden label leaves
its native object, pick target, evidence route, and validity unchanged.

Regional packets without a terrain program do not enter this pipeline. Exact
isolated regional samples remain source points because they authorize no
surface interpolation. A qualified `rey.regional-terrain-grid.v1` enters only
after the application has built one unique native-object identity index,
verified every row-major source binding and explicit valid/no-data cell against
that index, and compiled it into this field contract. Grid validation is linear
in objects plus cells; increasing terrain density must not reintroduce a
per-cell scan of every native object. Terrain-control geometry never becomes
observed elevation.

The source regional field is currently one bounded in-memory grid rendered by
a bounded 3D orthographic terrain camera through a tiled accelerated working
set. The application binds one selected synthetic Atlas sector to the exact
regional field and drives the same reversible model transform through surface,
passes, and reference paths. Native raster/imagery streaming and retained
Landscape fidelity voyages remain work tracked by [Plan
0005](../../../plans/0005-landscape-terrain.md).
