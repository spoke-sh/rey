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

Landscape is accepted as a geographic map, not merely as a correct mesh. Its
default composition is near north-up, mostly overhead, and canvas-filling. The
base visual hierarchy is relief, water, land cover, and contours; roads,
railways, boundaries, structures, labels, selection, and evidence are later
cartographic layers. A floating slab, visible source grid, collection of
outlined polygons, or abstract semantic diagram fails this outcome even when
its projection and validity math are correct.

At 1920x1080, a side-by-side review against a high-fidelity consumer terrain
map is a hard qualitative acceptance check. Rey need not copy imagery or match
pixels, but the review may identify no major perceptual gap in composition,
multi-scale relief, hillshade continuity, land-cover coherence, terrain-bound
hydrology, contour hierarchy, or vector density appropriate to the admitted
source. Source resolution or absent authority that prevents the target must be
reported as an open qualification gap, not hidden with renderer-generated
geography.

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

A selected regional Atlas member now retains one
`rey.atlas-landscape-transition.v1` binding from its exact synthetic sector
fragment to its exact regional terrain field. One reversible projector drives
the accelerated terrain model, reference/vector plane, terrain opacity,
elevation rise, and bounded camera pitch/yaw. Atlas and terrain overlap through
the transition instead of swapping at the semantic lens threshold. Click
traversal animates the same zoom path; wheel traversal uses the same
direction-independent curve. Terrain pan remains the camera target, focus is
solved analytically in camera axes, Shift-drag orbits within declared bounds,
and the native coordinate readout inverts the same target.

The render graph now binds every pass to a separate implementation revision,
exact input revision, authority, and dependency set. The accelerated material
executes base terrain, normal-driven hillshade, and ambient/valley occlusion as
independently gated stages. Contours, derived hydrology/weather, exact County
boundaries, admitted feature envelopes, points, and selection compile into
terrain-draped R3F inputs. Drape sampling visits every crossed terrain cell and
splits at no-data; it cannot bridge an unsupported interval. The same bounded
terrain transform carries surface and overlays through Atlas-to-Landscape.
Labels, interaction, evidence links, and accessible descriptions remain in the
deterministic reference overlay, which stays mounted across backend loss.

This proves admission, renderer parity, bounded tiled execution, transition
continuity, and executable geographic passes, not the final fidelity bar. The
current 41x41 source is a correctness and residency fixture presented through
a tilted object-view composition. It is too coarse to support the required
multi-scale terrain read, and its validity plane and outlined semantic
envelopes currently make the region resemble a floating slab. No imagery
provider or license authority is admitted, so the engine renders admitted
material and discloses the absence instead of fabricating familiar map imagery.

Fidelity qualification is now executable but has not yet been retained for the
complete matrix. `rey.explorer.landscape-fidelity@1` names steep relief, low
relief, coastline/water, dense vectors, explicit holes, stale data, and backend
loss at 1920×1080 and 3840×2160. Browser captures retain source terrain counts
and relief span, pass-set identity and kinds, maximum screen-space error, seam
mismatches, no-data triangle leakage, resident budgets, labels, exact scene
lineage, and omissions. Parity binds those values across reference, WebGL2, and
WebGPU; the performance aggregator enforces their ceilings. The harness never
injects a synthetic scene: each named workload must be separately admitted and
then selected with `--landscape-workload`.

The first source-controlled input is a 3×3 explicit-hole grid. It has been
admitted through `rey editor` and the qualified `scene-admission` workload,
then traversed through World → Atlas → Landscape → Objects → Evidence in a
1920×1080 fulfilled-transport reference voyage and its WebGL2 counterpart.
Those voyages retained eight valid vertices, one no-data vertex, zero leaked
triangles, zero seam mismatches, bounded residency, exact dataset/compiler
lineage, and all semantic-stage snapshots. The corresponding software-WebGPU
voyage remains a retained failure: renderer starvation can still skip the
required exit-dissolve samples. These runs close two
workload/backend/viewport rows only; fulfilled transport exercises the
disclosed main-thread fallback and cannot close direct networking or
dedicated-worker coverage.

The normal Rey County fixture now also carries a deterministic 41×41 authored
regional grid. Its exact County footprint and Unexplored Scrub become 523
explicit no-data vertices; 1,158 valid vertices retain five bounded materials
and 86.17–1,689.11 meters of authored semantic relief. The source is generated
reproducibly from checked-in boundary, feature, hydrology, and terrain-control
inputs, retained through the ordinary editor path, and admitted as a production
regional scene. This makes the implemented Landscape path visible in the
default project bearing without treating terrain controls as observed height or
closing the still-open named fidelity matrix.

## Geographic Synthesis Boundary

Rey County is fictional semantic geography that an agent may generate and
refine. Fictional does not mean implicit: the renderer cannot derive claimed
landforms from labels at draw time. The durable flow is:

```text
admitted editor packages
  -> evidence topology and authoring constraints
  -> revisioned agent geography compiler
  -> deterministic seam/conflict report
  -> explicit elevation, validity, water, land-cover, contour, and vector data
  -> editor review and scene admission
  -> renderer-neutral fields, tiles, and cartographic passes
```

This separates three responsibilities:

1. Evidence topology retains package identity, relationships, authority,
   omissions, and unknowns.
2. Geographic synthesis authors a coherent multi-resolution world and exposes
   every input, output, seam decision, conflict, limit, and algorithm revision.
3. Cartographic rendering controls projection, camera, materials, lighting,
   independent level of detail, labels, and transitions without minting
   geographic evidence.

The existing source-controlled generator is the first geography compiler. Its
next revision must demonstrate denser multi-scale relief and coherent base
materials while keeping exact no-data. Multi-package ingestion, explicit
cross-package seam resolution, raster-native field storage, and deeper vector
hierarchy remain subsequent slices of the same boundary.

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

- [x] Replace the overhead terrain camera with a bounded target/orbit camera
      that preserves north, scale, focus, pointer anchoring, and analytic
      native/County-local picking.
- [x] Define one reversible Atlas-to-Landscape projector so selected geography,
      footprint, terrain, vectors, and pick targets stay attached through every
      intermediate frame.
- [x] Add transition hysteresis and perceptual curves independently from
      semantic LOD; a camera threshold cannot become a scene swap.

### 5. Turn geographic layers into executable passes

- [x] Accelerate validity/background, base terrain, height/normals/hillshade,
      and ambient/valley occlusion as separately revisioned inputs and passes.
- [x] Add admitted material, hydrology/water, boundaries, roads,
      structures, labels, selection, and evidence overlays without upgrading their
      authority or flattening native geometry into cards. Keep imagery absent until
      a provider and license authority are admitted; keep labels/evidence in the
      accessible reference overlay while their anchors execute in 3D.
- [x] Make render-graph dependencies and invalidation executable, preserving a
      deterministic accessible fallback for every semantic layer.

### 6. Qualify fidelity and performance

- [x] Define versioned target viewports and named requirements for steep relief,
      low relief, coastline/water, dense vectors, explicit holes, stale data, and
      backend loss without injecting synthetic evidence into the browser.
- [ ] Add retained Landscape captures at target viewports with steep relief,
      low relief, coastline/water, dense vectors, explicit holes, stale data, and
      backend-loss fixtures.
- [x] Instrument and assert screen-space terrain error, no-data leakage, tile seams,
      selection/picking continuity, stable labels, bounded resident bytes, and
      interaction convergence in named voyage, parity, and performance manifests.
- [ ] Retain the complete named workload matrices across reference, WebGL2, and
      WebGPU and evaluate rendered parity and performance on one exact machine.
- [ ] Repeat World → Atlas → Landscape → Object → Evidence through direct
      browser transport and retain exact source, dataset, compiler, backend,
      omissions, limits, and performance lineage.

### 7. Rebaseline Landscape as a geographic map

- [ ] Make the Landscape entry camera near north-up and mostly overhead while
      preserving reversible Atlas attachment, analytic pan/pick behavior, and
      an explicit bounded orbit gesture.
- [ ] Remove the stage-like validity slab. Unsupported cells must remain holes,
      while canvas/background treatment communicates absence without drawing a
      rectangular world object.
- [ ] Establish a cartographic layer hierarchy in which relief, water, land
      cover, and contours form the base read and semantic envelopes, markers,
      and selection cannot dominate at Landscape LOD.
- [ ] Revise the Rey County geography compiler and admitted dataset to carry
      denser multi-scale landforms, drainage response, and coherent land-cover
      fields under exact deterministic lineage and validity.
- [ ] Add explicit water surfaces/areas and scale-aware contour styling before
      treating roads, railways, structures, or label density as fidelity
      completion.
- [ ] Retain 1920x1080 side-by-side captures and record perceptual gaps for
      composition, relief, hillshade, land cover, water, contours, and vector
      hierarchy. Do not mark this slice complete while a major gap remains.
- [ ] Extend the compiler from one source-controlled County into admitted
      multi-package constraints with explicit cross-package seam and conflict
      artifacts; do not let package ingestion silently become geography.

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
