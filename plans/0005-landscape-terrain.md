# Plan 0005: Deliver Geographic Landscape Terrain

- Status: Active
- Owns: admitted regional elevation datasets, validity-safe Landscape fields,
  terrain tiling and residency, bounded 3D County camera, Atlas-to-Landscape
  continuity, geographic render passes, and fidelity/performance voyages

## Outcome

Make Landscape a high-fidelity geographic terrain posture over exact admitted
regional evidence. It must equal the Globe and Mercator work in projection
coherence while remaining stricter about validity: missing elevation stays a
hole through admission, field compilation, level of detail, meshing, fallback,
and accelerated rendering.

```text
native regional elevation + explicit no-data
  → editor INDEX + immutable SCENE@n
  → qualified terrain dataset manifest
  → renderer-neutral fields and tiles
  → reference + WebGPU/WebGL2 terrain
  → Atlas ↔ Landscape continuity
  → vectors, water, materials, labels, and exact evidence
```

## Current Boundary

The first vertical slice is implemented. A `terrain` GeoJSON source may bind a
complete rectilinear grid through exact row/column, dimension, dataset, and
`valid`/`no_data` properties. Editor staging freezes those values and native
bytes. Scene admission independently re-inspects the bytes, rejects mixed or
incomplete grids, and emits `rey.regional-terrain-grid.v1` only when every
row-major vertex, source object, artifact revision, coordinate, and validity
class agrees. The grid is carried by `rey.regional-terrain-program.v2`; v1
retains its original point-only, no-interpolation meaning.

The grid authorizes piecewise-linear interpolation only inside triangles whose
three source vertices are valid. `no_data` carries neither height nor material.
The CLI reports dimensions and valid/no-data counts. `@rey/agent` compiles the
same admitted grid into one renderer-neutral field; the reference SVG and
`@rey/explorer` accelerated mesh share validity-safe triangle selection. The
dataset and compiler revisions enter immutable scene lineage. Landscape and
Neighborhood suppress individual grid-vertex markers as level-of-detail
presentation, while Object and Evidence can still expose their exact source
identities.

The accelerated path now projects that reference field into a conservative
quadtree with stable source-bound tile identities, shared samples and validity
borders, exact parent/child links, measured geometric error, and explicit CPU
and GPU sizes. Camera selection uses one uniform error-qualified level so
adjacent active tiles cannot form mixed-level cracks. Coarse validity may
remove support but cannot gain it. A cancellable dedicated worker owns tile
projection, resampling, relief derivation, procedural field evaluation, parity
checking, and mesh preparation; environments without workers disclose a
main-thread fallback. Deterministic residency retains compiled tiles under
separate 48 MiB CPU and 64 MiB GPU budgets.

This proves admission, renderer parity, and bounded tiled execution, not the
final fidelity bar. The current source dataset remains one bounded in-memory
field, its accelerated camera is overhead orthographic, and geographic
imagery, water, vectors, labels, and the Atlas-to-Landscape transition are not
yet accelerated passes.

## Delivery Sequence

### 1. Admit one bounded regional elevation dataset

- [x] Extend terrain-source indexing with exact grid identity, row/column,
      dimensions, and explicit `valid`/`no_data` vertices.
- [x] Re-inspect frozen native bytes during scene admission and reject index,
      coordinate, value, material, dimension, or validity tampering.
- [x] Retain a content-identified dataset manifest whose cells bind exact
      native objects, artifacts, revisions, positions, values, and validity.
- [x] Expose grid dimensions, valid/no-data counts, interpolation scope,
      authority, and exact JSON through the human and structured CLI paths.

### 2. Establish one renderer-neutral reference surface

- [x] Compile admitted height, validity, material, normals, curvature, and
      bounded presentation channels without treating unobserved hydrology or
      erosion as source facts.
- [x] Use one adaptive, deterministic triangle rule in the reference and
      accelerated paths; never emit a triangle that touches no-data.
- [x] Bind dataset and compiler revisions into the immutable scene and keep
      exact terrain vertices below Landscape/Neighborhood object LOD.
- [x] Cover editor parsing, admission tampering, no-data propagation,
      triangulation, topology integration, reference output, and GPU parity with
      Rust tests and Vitest.

### 3. Add tiled evaluation, residency, and bounded work

- [x] Define an admitted dataset-to-tile projection with stable tile identity,
      geometric error, validity borders, parent/child relationships, and explicit
      CPU/GPU byte budgets.
- [x] Move available terrain decoding, resampling, normal/curvature derivation,
      procedural evaluation, and mesh preparation off the React render path into
      a cancellable bounded worker. The current typed grid requires no native-byte
      decode; that cost remains explicitly zero rather than fabricated.
- [x] Retain a camera-driven resident tile set with deterministic eviction,
      shared borders, crack prevention, and no validity expansion across levels.
- [x] Measure update, decode, submission, residency, draw, and interaction costs on
      named workloads; keep GPU execution claims absent without a capable timer.

### 4. Deliver a bounded 3D Landscape camera and transition

- [ ] Replace the overhead terrain camera with a bounded target/orbit camera
      that preserves north, scale, focus, pointer anchoring, and analytic
      native/County-local picking.
- [ ] Define one reversible Atlas-to-Landscape projector so selected geography,
      footprint, terrain, vectors, and pick targets stay attached through every
      intermediate frame.
- [ ] Add transition hysteresis and perceptual curves independently from
      semantic LOD; a camera threshold cannot become a scene swap.

### 5. Turn geographic layers into executable passes

- [ ] Accelerate validity/background, base terrain, height/normals/hillshade,
      and ambient/valley occlusion as separately revisioned inputs and passes.
- [ ] Add admitted imagery/material, hydrology/water, boundaries, roads,
      structures, labels, selection, and evidence overlays without upgrading their
      authority or flattening native geometry into cards.
- [ ] Make render-graph dependencies and invalidation executable, preserving a
      deterministic accessible fallback for every semantic layer.

### 6. Qualify fidelity and performance

- [ ] Add retained Landscape captures at target viewports with steep relief,
      low relief, coastline/water, dense vectors, explicit holes, stale data, and
      backend-loss fixtures.
- [ ] Assert screen-space terrain error, no-data leakage, tile seams,
      selection/picking continuity, stable labels, bounded resident bytes, and
      interaction convergence across reference, WebGL2, and WebGPU.
- [ ] Repeat World → Atlas → Landscape → Object → Evidence through direct
      browser transport and retain exact source, dataset, compiler, backend,
      omissions, limits, and performance lineage.

## Open Choices

- A qualified GeoTIFF/COG or other native raster adapter remains future work;
  the current GeoJSON grid is the smallest CLI-verifiable admission slice, not
  the long-term bulk-elevation format.
- Tile dimensions, geometric-error metric, worker topology, and camera bounds
  must be selected against named workloads rather than by drive-by dependency.
- Imagery and material inputs require their own provider and license authority;
  the renderer must not infer them from elevation or familiar map styling.

## Non-Goals

This plan does not authorize automatic locator execution, ambient downloads,
unbounded caches, invented terrain outside validity, a general ECS, a plugin
framework, physics, first-person navigation, or a new persistence engine.
