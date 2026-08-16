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
interaction and the semantic projection curve.

## Executable Geographic Passes

`@rey/agent` owns `rey.explorer.render-graph@2` and compiles its active terrain
subset into `rey.terrain-render-pass-set.v1`. The package accepts only typed,
already bounded inputs:

| Pass                      | Accelerated result                                                        | Authority retained                  |
| ------------------------- | ------------------------------------------------------------------------- | ----------------------------------- |
| Validity/background       | Canvas treatment visible through missing triangles; never a terrain slab. | Evidence support boundary.          |
| Base terrain              | Source/derived material tint on valid triangles.                          | Derived from admitted material.     |
| Height/normals/hillshade  | Multidirectional normal response.                                         | Derived presentation of height.     |
| Ambient/valley occlusion  | Curvature and occlusion response.                                         | Presentation only.                  |
| Contours                  | Conservative terrain-draped line segments.                                | Derived contour revision.           |
| Water/weather/boundary    | Draped native or derived line segments.                                   | Per-feature authority and source.   |
| Features/labels/selection | Draped envelopes and point/selection anchors.                             | Interface over retained identity.   |
| Evidence/accessibility    | Mounted application reference overlay; no accelerated replica.            | Exact links and accessible meaning. |

Every pass binds an implementation revision, input revision, and dependency.
The material and scene identity include the compiled pass-set identity. A
missing dependency prevents its children from executing.

Line draping is conservative. The compiler adds probes at every crossed field
grid boundary and within every crossed cell, evaluates only fully valid source
support, and splits a line when support is absent. A native vector therefore
cannot bridge a no-data hole merely because its endpoints are valid. The
surface, lines, and point anchors share one R3F terrain group and one
Atlas-to-Landscape model transform. Validity is represented by missing
triangles over the canvas background, not by a rectangular mesh that can read
as geographic support.

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
geographic line/point presentation. The application still owns native geometry
interpretation, pass compilation, label layout, picking, evidence links, and
the accessible reference renderer. Structure polygons currently enter as
their admitted envelopes; a later qualified mesh adapter may add volumetric
geometry without changing their authority.

Regional packets without a terrain program do not enter this pipeline. Exact
isolated regional samples remain source points because they authorize no
surface interpolation. A qualified `rey.regional-terrain-grid.v1` enters only
after the application has verified its row-major source bindings and explicit
valid/no-data cells, then compiled it into this field contract. Terrain-control
geometry never becomes observed elevation.

The source regional field is currently one bounded in-memory grid rendered by
a bounded 3D orthographic terrain camera through a tiled accelerated working
set. The application binds one selected synthetic Atlas sector to the exact
regional field and drives the same reversible model transform through surface,
passes, and reference paths. Native raster/imagery streaming and retained
Landscape fidelity voyages remain work tracked by [Plan
0005](../../../plans/0005-landscape-terrain.md).
