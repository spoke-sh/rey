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
presentation. Object and Evidence keep those vertices in the qualified dataset
instead of mounting every source row as a browser object; an explicitly
selected or deep-linked terrain vertex still exposes its exact source identity.

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

An Atlas with exactly one admitted regional terrain field now prewarms that
field through `rey.explorer.atlas-terrain-prewarm@1` after 600 milliseconds of
camera stability and before selection. Camera movement cancels the idle start.
The canvas stays invisible and cannot change focus, coverage, or evidence; its
resident compilation is reused when the operator begins Landscape traversal.
The UI and voyage harness retain its `scheduled`, `mounted`, and `submitted`
states, so a warm-entry measurement must prove a real hidden render submission.

The render graph now binds every pass to a separate implementation revision,
exact input revision, authority, and dependency set. The accelerated material
executes base terrain, normal-driven hillshade, and ambient/valley occlusion as
independently gated stages. Its material identity depends only on those
shader-affecting stages, so a changed contour, water, or feature pass does not
discard an otherwise identical prewarmed shader. Contours, derived
hydrology/weather, exact County boundaries, admitted native vectors, disclosed
bounds fallbacks, points, and selection compile into terrain-draped R3F inputs.
Drape sampling visits every crossed terrain cell and splits at no-data; it
cannot bridge an unsupported interval. The same bounded terrain transform
carries surface and overlays through Atlas-to-Landscape.
Labels, interaction, evidence links, and accessible descriptions remain in the
deterministic reference overlay, which stays mounted across backend loss.

This proves admission, renderer parity, bounded tiled execution, transition
continuity, and executable geographic passes, not the final fidelity bar. The
original 41x41 source was a correctness and residency fixture presented
through a tilted object-view composition. The current 81x81 geography-compiler
output and map-first composition materially improve the base read, but roughly
one-kilometer source spacing remains too coarse to close the required
multi-scale fidelity gap. No imagery provider or license authority is
admitted, so the engine renders admitted material and discloses the absence
instead of fabricating familiar map imagery.

Fidelity qualification is now executable but has not yet been retained for the
complete matrix. `rey.explorer.landscape-fidelity@2` names steep relief, low
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

The normal Rey County fixture now carries a deterministic 81×81 authored
regional grid. Its exact County footprint and Unexplored Scrub become 1,938
explicit no-data vertices; 4,623 valid vertices retain five bounded materials
and 88.77–1,721.67 meters of authored semantic relief. Revision
`rey.agent-geography.rey-county@2` records domain-warped macro, meso, ridge,
and micro relief, authored-hydrology carving, coherent land-cover inputs, and
the explicit absence of cross-package stitching. The source is generated
reproducibly from checked-in boundary, feature, hydrology, and terrain-control
inputs. It has traveled through editor admission as `SCENE@4` and passed the
`scene-admission` workload before `/explore` consumption. This improves the
default project bearing without treating terrain controls as observed height
or closing the still-open named fidelity matrix.

That admission also exposes a transport boundary. The current workload-list
JSON is about 10.6 MB because it repeats thousands of terrain projections and
grid cells; one cold local projection takes about eight seconds on the
development machine. The UI now caches this heavyweight projection against
its exact workload, catalog, ignore, environment, and Git dependencies, so an
unrelated Channel or conversation revalidation tick cannot trigger a rebuild
loop. Raster-native field transport, non-repeated summaries, and lazy exact
terrain evidence remain required before this source density is a scalable
browser contract. A local debug measurement over the exact admitted `SCENE@4`
workload list retained 10,605,502 decoded JSON bytes, 1,712,485 gzip wire bytes,
an 8.13-second cold projection, and a 36-millisecond warm projection over
`127.0.0.1`. This closes the repeated-send regression, not the cold-compute or
semantic-duplication gap.

Client-side source validation now builds one unique native-object identity map
before checking terrain cells. The 81×81 field therefore validates in linear
time rather than performing roughly 43 million object comparisons. Repeating
the 1920×1080 fulfilled-transport WebGL2 voyage reduced measured scene
compilation from 490.2 ms to 6.6 ms and passed the complete World → rotated
Atlas → Landscape → Objects → Evidence traversal in local manifest
`sha256:fa5e0d3245f32c572da4bcb273ddfd2236ac34ebde929b6dc9df7310575a0df8`.
That voyage is a traversal baseline, not a named Landscape-fidelity row. Its
Landscape capture still shows dominant outlined semantic envelopes, blurred
coarse relief, no explicit water surfaces, and no scale-aware contours, so the
side-by-side acceptance item remains open.

Scene-admission implementation revision 2 now retains exact native coordinates
for admitted non-terrain Point, LineString, and Polygon objects. The browser
projects and drapes those paths directly, eliminating the rectangular bounds
substitution that caused the dominant hydrology envelopes in that capture.
Other RFC 7946 geometry families remain admissible with an explicit bounds
fallback, and terrain-grid Points remain canonical in the qualified row-major
grid rather than duplicating 6,561 coordinate payloads. This repairs vector
shape fidelity but does not by itself close water-surface or contour fidelity.

Regional contours are now derived from the admitted elevation field at
lens-dependent density and skip every cell touching no-data. Exact admitted
hydrology Polygons become terrain-following water areas made only from fully
valid source triangles; the exact outline remains retained and the filled edge
discloses its terrain-grid quantization. Reference fallback, accelerated
rendering, diagnostics, and fidelity-suite revision 2 retain the area
contract.

Fresh 1920×1080 fulfilled-transport voyages passed the complete World →
rotated Atlas → Landscape → Objects → Evidence traversal in reference manifest
`sha256:2f64841d0c3c13163cdb8b1811da0df1267ad8a96b0a1eaf04fd0cb8bf7f21eb`
and WebGL2 manifest
`sha256:68a90fbc2cdf265f9296aec21dd082e71c9127c95bb2805aa917f535813a7cc1`.
The WebGL2 Landscape retained one water area, twelve provenance-bound line
batches containing 2,481 contour and hydrology segments, zero no-data triangle
leaks, and zero tile-seam mismatches. Object and Evidence no longer mount all
6,561 exact terrain vertices; the complete WebGL2 manifest is 105,311 bytes,
down from the prior 6.3 MiB traversal artifact, while an exact selected terrain
vertex remains addressable.

The bounded Atlas prewarm and submitted-frame handshake passed a subsequent
1920×1080 fulfilled-transport WebGL2 voyage in manifest
`sha256:d242976085c5632c65d65cc9c6230bac6fe37223dc26e6c1024d6dccc65d1e21`.
The Atlas capture retained prewarm state `submitted`; every World, Atlas,
Landscape, Object, and Evidence capture bound its displayed scene snapshot to
the exact submitted renderer snapshot. Landscape still retained twelve line
batches, 2,481 segments, one water area, zero no-data leaks, and zero seam
mismatches. Its measured SwiftShader submission was 1.55 seconds versus the
earlier 2.07-second local baseline. That single-machine comparison demonstrates
that work moved off the visible entry path, but it is not a stable hardware
performance claim and the remaining stall stays open.

Those voyages close the executable base-layer item, not the named fidelity
matrix. The named `coastline-water` row still requires an admitted `river`
subtype and an accelerated admitted boundary. Regional scene admission
currently retains only the generic hydrology layer, and the exact County
boundary coincides with no-data support, so qualifying that row would require a
stronger semantic-property admission contract or a distinct matching fixture.
The suite is not weakened to make Rey County pass. The WebGL2 image also
retains blurred kilometer-scale relief, and its first Landscape submission
took about 2.07 seconds under SwiftShader; both remain material acceptance
gaps.

Landscape now enters through a north-up 88-degree map camera. The reversible
Atlas transition also eases into a 1.38x interior composition scale so valid
geography fills the canvas without detaching the selected sector. Shift-drag
retains bounded orbit inspection from 28 to 88 degrees, while reset restores
the cartographic entry. The rectangular validity mesh has been removed;
no-data is communicated by absent triangles over the terrain canvas. Landscape
feature LOD retains water and transport vectors plus the exact selection while
hiding terrain-control, district, lot, and unselected point envelopes until a
closer semantic lens. The exact County footprint remains mounted and
accessible but becomes a faint support boundary, while the detailed bearing
and evidence key collapse to one compact map-status line at Landscape.
Contours and the County boundary are subordinate to the relief rather than
bright framing lines. This corrects composition and layer hierarchy, but the
roughly one-kilometer 81x81 source still leaves the retained relief-fidelity
comparison open.

### Perceptual rebaseline — 2026-08-15

The operator-supplied 3022×1926 terrain reference and Rey's retained 1920×1080
WebGL2 Landscape capture were inspected side by side. The reference is an
acceptance target, not admitted scene evidence and not a source asset. The Rey
capture is bound to voyage manifest
`sha256:d242976085c5632c65d65cc9c6230bac6fe37223dc26e6c1024d6dccc65d1e21`.

| Dimension        | Minimum acceptance read                                      | Current Rey County read                                      | Gap   |
| ---------------- | ------------------------------------------------------------ | ------------------------------------------------------------ | ----- |
| Composition      | Continuous, overhead geography fills the map viewport.       | One attached near-north-up surface fills the usable canvas.  | Minor |
| Relief           | Fine ridges, valleys, benches, and drainage at many scales.   | Broad blurred lobes reveal roughly one-kilometer sampling.   | Major |
| Hillshade        | Crisp multiscale form without faceting or muddy smoothing.    | Stable but low-frequency shading cannot recover source form. | Major |
| Land cover       | Coherent local classes with terrain-following boundaries.     | Five broad tints read as washes rather than geography.       | Major |
| Water            | Continuous areas and terrain-following river hierarchy.       | One wetland area plus angular admitted centerlines.          | Major |
| Contours         | Scale-aware hierarchy reveals form without dominating it.     | Sparse derived lines inherit the coarse elevation field.     | Major |
| Vectors/labels   | Roads, rail, structures, and labels resolve by semantic LOD.  | Landscape exposes hydrology but no geographic network read.  | Major |

This matrix keeps the side-by-side delivery item open. The next source-fidelity
slice must first replace repeated object-per-cell browser transport with a
compact renderer-neutral field payload retaining exact artifact revision,
dimensions, bounds, value channels, material palette, and validity mask. Only
then should the geography compiler increase field density and emit a deeper
water/transport/label hierarchy against named byte, compile, resident, and
submission budgets. Interpolating or shading the current 81×81 field more
aggressively is not an acceptable substitute for admitted source detail.

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

The source-controlled generator is the first geography compiler. Its second
revision produces the 81×81 field with deterministic multi-scale relief,
hydrology response, coherent base materials, exact no-data, and explicit
synthesis metadata. Multi-package ingestion, explicit cross-package seam
resolution, raster-native field storage, and deeper vector hierarchy remain
subsequent slices of the same boundary.

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

- [x] Make the Landscape entry camera near north-up and mostly overhead while
      preserving reversible Atlas attachment, analytic pan/pick behavior, and
      an explicit bounded orbit gesture.
- [x] Remove the stage-like validity slab. Unsupported cells must remain holes,
      while canvas/background treatment communicates absence without drawing a
      rectangular world object.
- [x] Establish a cartographic layer hierarchy in which relief, water, land
      cover, and contours form the base read and semantic envelopes, markers,
      and selection cannot dominate at Landscape LOD.
- [x] Revise the Rey County geography compiler and admitted dataset to carry
      denser multi-scale landforms, drainage response, and coherent land-cover
      fields under exact deterministic lineage and validity.
- [x] Add explicit water surfaces/areas and scale-aware contour styling before
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
