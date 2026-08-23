# Terrain Pipeline

The accelerated terrain path turns admitted, bounded field grids into a
continuous-relief R3F scene. It never converts missing support, control
geometry, or application cards into terrain.

## Pipeline

```text
TerrainFieldSetInput[]
  → complete-field cartographic relief derivation
  → verified haloed height/relief pyramid envelope
  → camera-qualified tile descriptors + bounded worker
  → exact row/column sampling of height and relief into render tiles
  → buildTerrainMeshData(fields, sampled relief)
  → verifyTerrainMeshParity
  → enforce GPU byte budget
  → CompiledContinuousRelief + revisioned render-pass set
  → ContinuousReliefScene
  → orthographic camera + TSL stages + indexed relief + draped geography
```

## Field Contract

Each `TerrainFieldSetInput` supplies one bounded regular grid:

| Channel        | Representation                  | Use                                                            |
| -------------- | ------------------------------- | -------------------------------------------------------------- |
| Grid           | Columns, rows, and local bounds | Maps cells into local scene coordinates.                       |
| Validity       | `Uint8Array`                    | Determines whether a triangle has admitted support.            |
| Validity class | `Uint8Array`                    | Distinguishes valid, source no-data, and unsupported vertices. |
| Elevation      | `Float32Array` plus scale       | Becomes the mesh Y coordinate.                                 |
| Normal         | `Float32Array` or `Int8Array`   | Drives lighting in Three.js coordinates.                       |
| Curvature      | `Float32Array`                  | Enhances ridges and valleys.                                   |
| Tint           | RGB `Float32Array`              | Supplies base material color.                                  |
| Occlusion      | `Float32Array`                  | Darkens bounded valleys and ambient support.                   |
| Roughness      | `Float32Array`                  | Controls the TSL surface response.                             |

`rey.landscape-relief-field.v4` binds renderer-neutral metric slope/aspect,
MDOW illumination, sky-view factor, signed openness, profile/plan curvature,
local contrast, tone-mapped hillshade, ridge salience, and tangent arrays to
the exact field identity. `rey.terrain-relief-metrics.v1`, when present, binds
source sample spacing in both axes, admitted elevation range, and the authority
of that presentation transform. Each local, midslope, and regional scale names
its metric-gradient, MDOW, and openness revisions and declares its support in
meters and cells.

MDOW combines a 315° northwest primary, 225° southwest fill, and 45° northeast
rim with slope-adaptive contrast and a nonblack ambient floor. Directional
positive/negative horizons derive sky-view and openness from admitted metric
elevation. High-pass profile/plan curvature, metric slope, and local position
produce ridge salience; a validity-safe local statistic then applies
linear-space contrast and deterministic tone mapping.
`rey.landscape.linear-tone-map@2` keeps the flat MDOW neutral point stable but
expands locally normalized shadows and highlights between a nonblack floor and
a bounded highlight ceiling; it does not add another light or synthesize
surface detail. The complete support radius includes this contrast
neighborhood as well as the largest metric operator.

Regional terrain derives every relief array over the complete refined field
before camera tile materialization. Each render tile samples that result by
original row/column identity. Relief is never re-evaluated on a cropped render
tile, so a kernel cannot lose neighbors or renormalize merely because the
camera selected a different partition. A target scale that is finer than the
admitted spacing, larger than the bounded kernel, or wider than the complete
grid is marked unsupported and does not contribute. Renderer diagnostics
expose the scale basis and exact support decisions. The worker retains both
shared-border equality and complete-field/partition equality across every
derived array as distinct zero-mismatch diagnostics.

Admitted regional fields also carry
`rey.terrain-validity-classification.v1`. Its values distinguish supported
terrain (`1`), source-declared no-data (`2`), and space with no admitted source
(`0`). The geometry mask remains authoritative for triangle admission, and a
contract verifier rejects either channel when they disagree. Classification
is retained through mosaic composition, validity-safe refinement, conservative
tile projection, and tile materialization. A BLAKE3 validity identity binds
the exact classification bytes and implementation revision for pyramid use.

The metric relief operator derives slope-adaptive MDOW, directional
SVF/openness, profile/plan curvature, ridge salience, and bounded local contrast
inside the haloed hierarchy. `rey.landscape.chromatic-relief@2` then composes
those renderer-neutral arrays with warm direct and cool ambient light in
linear color. The remaining fidelity limit is the admitted source's spatial
resolution and geomorphic form, not a license for a backend shader to invent
surface detail.

## Height And Relief Pyramid Contracts

`rey.landscape-height-pyramid.v1` and
`rey.landscape-relief-pyramid.v2` are executable, content-identified contract
schemas. Contract revision 3 requires every relief level to retain the exact
complete relief-field identity accepted by downstream sampled consumers in
addition to its derivation-tile count, maximum source gutter, border digest,
and derived channels. Their finalizers canonicalize lineage,
channels, operators, and omissions before assigning BLAKE3 identities. Their verifiers require
every level to retain metric x/y spacing, dimensions, common bounds,
conservative valid/no-data/unsupported counts, byte cost, exact source
lineage, and deterministic parent/child level identities.

Height levels additionally retain an exact BLAKE3 height-channel identity and
their vertical range. Relief levels bind one
exact height level and independently content-identify each operator revision,
metric target and support radius, required source gutter, support decision,
validity policy, and derived channel set. A supported operator is invalid when
its gutter is narrower than its kernel support. Relief geometry and validity
must match the bound height level exactly.

`rey.landscape-pyramid-envelope.v2` binds the complete admitted mosaic and its
materialized height/relief hierarchy to those schemas. The envelope
content-identifies exact height, validity-class, hillshade, salience, tangent,
derivation-tile, maximum-gutter, and border-digest content at every level. Both
the accelerated compiler and deterministic reference path verify the same
envelope before sampling camera tiles; diagnostics retain its identities,
completion, level/byte counts, gutters, border digests, and zero-mismatch
results.

`rey.terrain.relief-hierarchy@3` partitions every level into bounded
256-interval interiors, expands each source window by the largest supported
metric operator, derives from that halo, and then crops the interior. The
assembled result must equal whole-level derivation within the named `1e-6`
numeric tolerance; overlapping interiors and tolerance-canonicalized adjacent
border digests must agree. The larger bounded derivation partition reduces
redundant halo overlap without changing validity, operator support, or the
whole-field equivalence proof. The earlier one-level, zero-gutter fallback has
been removed. `rey.terrain.dataset-tiles@2` now projects camera tiles directly
from those materialized levels instead of striding over the finest field.
Selection uses conservative cumulative screen-space error and one uniform
level per view, so adjacent support remains compatible without mixed-edge
cracks.

The application-side `rey.terrain.height-hierarchy@2` materializes
conservative height and validity levels before camera tiling. It
uses explicit bounded child windows for dyadic, non-dyadic, square, and
rectangular grids, retains the canonical contributing source set for every
sample, and reaches a 2×2 root within the declared level bound. Every level
also carries renderer-neutral normals and presentation companions sampled
without widening validity. Tile cache identities bind the exact mosaic,
height level, relief field and operator, validity support, and border digest.
Derived-channel bytes, halo source cells, and selected CPU/GPU bytes enter the
worker measurements and retained browser diagnostics.

Several admitted regional fields may first enter
`rey.landscape-mosaic.v1`. The application-owned compiler requires a common
coordinate reference, vertical reference, projected sample spacing, elevation
scale, and integer-aligned grid origin. Qualified adjacent patches must carry
identical validity, elevation, and material at every shared sample when no
selection is needed. Positive-area overlap on that common lattice is resolved
validity-first, then by declared authority, finer nominal metric spacing, and
stable source identity. Input array order and draw depth never decide the
winner. `rey.landscape-source-contribution.v1` retains the final source index
for every vertex, while `rey.landscape-overlap-conflicts.v1` retains every
sample where the candidate height, validity class, or material disagreed; both
arrays have exact BLAKE3 identities in the compact mosaic binding and browser
diagnostics. Differently aligned or differently projected grids remain
rejected pending the nested-resolution resampling contract, and no overlap
feather is applied yet. The output is one regular field with explicit invalid
cells wherever no admitted patch contributes, so subsequent
refinement, normals, drainage, relief, tiling, and rendering operate across
qualified seams without turning gaps into geography. The compact mosaic
binding on derived tiles preserves the exact composition, source-patch, focus,
coordinate, vertical-reference, contribution, conflict, overlap, and gap
identities.
The application selects the connected component containing the focused region
only across retained, terrain-qualified, conflict-free edge seams. A disjoint
or conflicted region stays outside the field; a component that fails the
stricter renderer alignment contract falls back to the focused patch and
retains that omission for the footer, diagnostics, and qualification report.
`rey.explorer.regional-terrain-grid@5` evaluates every member's nominal metric
sample spacing at the exact shared component-center latitude. This removes a
false per-patch spacing conflict for grid-aligned regions whose native latitude
extents differ, while retaining each source's exact angular sample interval.
It is a deterministic presentation-frame scale, not a geodetic transform or
permission to resample an unqualified seam.

`@rey/agent` derives these fields from two admitted sources:

- survey terrain programs produce camera-relative, haloed procedural fields;
- regional terrain datasets produce exact bounded elevation fields with
  source-declared per-vertex validity/no-data.

Field generation, dataset interpretation, hydrology, classification
derivation, LOD selection, worker orchestration, and tile residency remain
application-owned. Validity-class verification and identity, renderer-neutral
relief derivation, and exact relief sampling are package-owned so reference,
WebGL2, and WebGPU compilation use one contract.
`@rey/explorer` sees the same structural field contract in both cases and
cannot upgrade either source's authority.

The source-controlled `rey.agent-geography.rey-county@14` field is 705×626 and
uses no renderer refinement. `SCENE@20` and scene-admission result
`blake3:951636433b8f6fb845f7acb3d656046ef8404ba789405ff6744fbb9871ee704d`
qualify its exact native bytes as the current working field. This revision
reduces unstructured fine octaves, derives slope-supported fluvial valleys
first, and applies validity-contained divide/valley separation only after that
drainage structure exists. It further requires a genuine downhill receiver for
any drainage height displacement; priority-flood escape topology contributes
exactly zero. Later source edits remain candidates until the editor and
workload repeat that boundary.
The density policy targets at least 320 intervals per axis, so lower-density
admitted fields may refine by a bounded integer factor while this source stays
exact.
In a refined field, fully supported cell
interiors use bilinear sampling; a cell touching no-data uses only fully
supported source triangles. Every refined point carries an independent
validity value and its retained no-data or unsupported class. Band-limited
deterministic microrelief exists only on those
presentation refinements, is constrained to zero at admitted source vertices,
and remains explicitly presentation-only. Diagnostics always report the
original admitted cell counts and elevation range. The reference renderer
rasterizes the verified finest complete-field relief into one bounded 2D
canvas instead of mounting hundreds of thousands of fallback polygons.

That dense working field is then treated as a causal geography graph, not a
bag of independent visual channels. A priority-flood pass resolves local
depressions without crossing no-data, every valid cell receives one bounded
downstream receiver, and a height-ordered accumulation pass carries rainfall
through the complete admitted basin. The raw accumulation channel retains that
topology; a validity-bounded smoothed copy drives moisture so D8 paths do not
become visible material bands. Non-displacing erosion potential
remains a separate derived channel, while the authored elevation and its
normals/curvature stay intact. Continuous elevation, moisture, and forest
support grade lush valley greens through warm montane grassland into slate
alpine crests; metric slope exposes rock without a discrete height palette.
Metric contour intervals tighten by semantic lens, with index weights and
opacity remaining subordinate to relief. Supplemental synthetic stream and
river linework is reserved for closer lenses and remains distinctly qualified
from exact admitted hydrology.

## Validity-Safe Mesh Compilation

Grid X/Y becomes mesh X/Z; admitted elevation becomes mesh Y. For each quad,
`terrainTriangleIndices` deterministically chooses the diagonal that retains
the most fully valid triangles; equal choices alternate to avoid a directional
bias. A triangle is emitted only when all three vertices have valid support.
This lets a supported half-cell survive next to no-data without bridging the
invalid vertex. `rey.reference-regional-terrain@5` applies the same diagonal
choice and barycentric interpolation per raster pixel, so fallback cannot fill
a hole that the GPU path omits. The canvas retains one accessible image label
and the exact hierarchy diagnostics; labels, geometry descriptions, focus
targets, and evidence links remain in the separate DOM reference overlay.

Compilation creates separate upload arrays for positions, normals, base tint,
the final cartographic linear color, occlusion, roughness, curvature, relief,
and indices. Upload storage is disposable and cannot mutate the authoritative
CPU fields.

`verifyTerrainMeshParity` checks every uploaded field sample after the
coordinate transform, checks the supplied relief channels, and rejects any
index touching invalid support. The current parity identity is
`rey.terrain.cpu-mesh-upload-parity@3`.

## Material And Lighting

`createContinuousReliefMaterial` produces a `MeshBasicNodeMaterial` with TSL.
The renderer-neutral relief engine and
`rey.landscape.chromatic-relief@2` own illumination and final linear color; the
material therefore does not apply a second physical light response. Warm
direct light and cool sky ambient remain chromatic. The final luminance target
retains hillshade, local contrast, sky-view factor, signed openness, and
material occlusion instead of normalizing those operators back out of the
result. A bounded linear power curve keeps the display encoding from turning
the terrain into a pale wash. Its separately revisioned pass inputs gate:

- source tint;
- field-wide multidirectional hillshade;
- explicit occlusion;
- and the exact retained cartographic-color array.

`ContinuousReliefScene` shares the material across compiled meshes without
adding another lighting owner. Regional geography continuously blends the
admitted material class with deterministic moisture, elevation, slope,
exposure, and validity-bounded drainage fields without changing elevation or
source support. `rey.terrain.regional-geography@8` uses a bounded linear-color
hypsometric ramp with separated valley, montane, subalpine, alpine, forest,
and rock anchors. Its rainfall accumulation follows only genuine downhill
receivers from the admitted elevation; a retained sink remains a sink rather
than borrowing a priority-flood escape tree for presentation. Only sufficiently
steep or alpine samples receive full rock exposure, preventing the slope
channel from neutralizing the complete map. Its
orthographic camera is a bounded target/orbit view: the application supplies a
near-north-up,
mostly-overhead cartographic entry, pitch and yaw are clamped, screen-axis pan
resolves to one ground target, and an optional model transform keeps the
terrain attached during projection changes. R3F owns the declarative camera and
terrain-group lifecycle; `@rey/agent` owns the explicit Shift-drag orbit
interaction, the semantic projection curve, and the reversible interior
composition scale used to make Landscape fill the viewport. Evidence and
validity boundaries remain mounted while their visual weight is independently
controlled by the application lens. `terrainCameraProjection` returns this
camera's pose and ortho frustum bounds; `projectTerrainCoordinate` is its
point-projection counterpart, resolving one world-space terrain point to a 2D
screen offset through the same orbit basis (right/up vectors derived the way
`camera.lookAt` derives them). The DOM reference renderer's Atlas→Landscape
tilt transform (`@rey/agent`'s `atlasLandscapePresentation`) and its
click-to-focus panning (`panForTerrainTarget`) both derive their 2D CSS
matrices from `projectTerrainCoordinate` rather than independently
hand-rolled trigonometry, so they cannot silently diverge from the
accelerated camera they approximate. That presentation also holds yaw
constant through the whole tilt — only pitch animates — since sweeping pitch
and yaw together makes off-pivot screen points move non-monotonically; with
yaw fixed, every flat (zero-elevation) point's screen position is guaranteed
monotonic across the transition.

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

Before tile projection, the shared relief field derives local, midslope, and
regional light/openness from admitted elevation. A scale contributes only when
its complete sampling window is valid, so an internal hole or no-data edge
cannot cast presentation relief into supported terrain. The chromatic
composition never changes elevation, validity, source material identity, or
geographic authority, and the reference renderer converts that same linear
array to CSS sRGB rather than reconstructing a separate lighting formula.

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
and clipping each boundary triangle to the exact even-odd rings. Every clipped
vertex receives a barycentric height from that same fully valid terrain
triangle. `rey.explorer.terrain-render-passes@6` applies one bounded
presentation-only vertical separation, and the water material uses a stable
depth bias/order, so the exact clipped surface does not alternate with the
underlying terrain through depth conflict. The exact admitted `water_class`
selects distinct open-water, wetland, river, stream, and seasonal-runoff
presentation while preserving one source-bound path and authority. A
high-contrast open-water fill or subordinate wetland tint and separately
styled boundary therefore retain the exact admitted polygon without extending
support; no triangle touching a no-data vertex can enter the surface. The
surface, areas, lines, and point anchors share one R3F terrain group and one
Atlas-to-Landscape model transform. Validity is represented by missing
triangles over the canvas background, not by a rectangular mesh that can read
as geographic support. Disconnected valid line intervals remain independent
endpoint pairs but batch once per source feature into an R3F `LineSegments`
object. Batching changes draw mechanism, never the validity cuts or source
identity.

Text labels, evidence links, descriptions, and pointer semantics deliberately
remain in `@rey/agent`'s deterministic reference overlay. It stays mounted
with the accelerated surface and becomes visible on backend failure. While an
accelerated frame is healthy, exact SVG feature geometry remains an invisible
focus/hit surface instead of drawing a second road, rail, boundary, or water
outline over the corresponding 3D pass. Labels, descriptions, focus semantics,
and exact evidence routes remain present in either posture. No imagery source
or license authority is currently admitted, so the package does not synthesize
an imagery layer from elevation or styling.

## Tiling, Workers, And Residency

`@rey/agent` projects the admitted `rey.landscape-height-pyramid.v1` and
`rey.landscape-relief-pyramid.v2` into `rey.terrain-tile-pyramid.v1`. Tile
identities bind the mosaic, exact height and relief levels, operator revision,
validity support, and border digest. Every level shares exact edge samples and
validity borders. Coarse validity is conservative: a no-data source sample may
remove coarse support but cannot become a valid coarse vertex. Camera
selection chooses a uniform level from cumulative hierarchy error, preventing
mixed-level edge cracks while retaining screen-space control. If the target
screen error would exceed the retained tile budgets, selection falls back to
the finest visible uniform level that fits and discloses the resulting error
instead of failing residency or silently raising a budget.

`rey.terrain.compilation-worker@17` runs hierarchy projection, haloed relief
derivation, exact relief sampling, procedural field evaluation, partition and
border parity checking, and mesh preparation in a cancellable dedicated
worker. `rey.terrain.regional-mosaic@8` hashes every composed typed channel,
grid/frame parameter, source owner, and compiler revision into an exact field
content identity before the field reaches that worker. Its complete hierarchy
plus selected-tile output has a separate
160 MiB bound; that transient compilation-output bound is not the 48 MiB
resident-tile budget. The deterministic reference field remains visible while work is
pending or after failure. A disclosed main-thread fallback exists where
`Worker` is unavailable. `rey.terrain.tile-residency@2` retains compiled tiles
under independent 48 MiB CPU and 64 MiB GPU budgets, rejects a tile whose
compiled relief differs from its cache identity, and evicts the oldest
unrequested exact identity first. The worker also retains a separately bounded
112 MiB exact materialized-pyramid cache keyed from every contributing typed
array and the mosaic, hierarchy, relief, validity, and material revisions; its
hits and misses are exposed in browser diagnostics. Cache identity is derived
from the admitted field bytes plus the refinement and regional-geography
revisions before those expensive derived channels are computed, so a hit
reuses both the exact derived field and its hierarchy rather than rebuilding
the field merely to discover that the hierarchy was already retained. The
worker consumes that exact field content identity rather than hashing the full
payload again after every structured clone, and memoizes the resolved hierarchy
key for the lifetime of an immutable topology field object. A newly composed
field must establish a new byte-derived identity before it can reuse anything.
Revisioned regional linework is likewise retained by exact derived-field and
content profile. Objects and Evidence deliberately share the same 25-meter
contour/drainage profile identity instead of recompiling identical linework
under two lens names.

When an Atlas contains exactly one admitted regional terrain field, the
application may mount an invisible `rey.explorer.atlas-terrain-prewarm@2`
terrain canvas after the Atlas camera has remained stable for 600 milliseconds
and before selection. Any camera movement cancels and reschedules that idle
work. Prewarm uses the exact predicted Landscape entry scale, viewport, pitch,
yaw, mosaic identity, hierarchy/operator revisions, and material revision. The
source key is shared with the visible renderer, while view-only successor jobs
retain the last compatible submitted terrain until they submit. A late
callback from a canvas that was hidden as prewarm-only cannot replace the
explicit prepared report or stall the handoff. It compiles
only that already admitted field; it does not select the
County, change the camera, render coverage, execute a locator, or widen
evidence. The same field identity and resident compilation remain available
when Atlas-to-Landscape traversal begins, avoiding an avoidable
first-visible-frame setup stall without racing an active projection morph. The
application exposes `scheduled`, `mounted`, and `prepared` states so
qualification can require actual renderer submission instead of inferring
warmth from elapsed time.

`rey.landscape-terrain-fabric@2` projects Atlas stipples from exact valid
vertices of the same finest materialized height/relief hierarchy used by
Landscape. A stipple retains field, relief, row, column, and sample identity;
its salience, tangent, brightness, length, density, and reveal order come from
that bound relief sample. The planar sequence only chooses a deterministic
bounded subset. It cannot create a second raw-field relief calculation or a
free-floating point unrelated to Landscape support. The selected Atlas member
anchors the projection, but connected qualified neighboring patches and
separately admitted compatible overview coverage remain visible through the
shared mosaic outside the primary patch frame.

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
surface interpolation. Scene admission first verifies the expanded
`rey.regional-terrain-grid.v1` against one unique native-object identity index,
then retains the same cells as `rey.regional-terrain-grid.v2`: bounds and
dimensions carry regular positions, hexadecimal bytes carry validity and
material indices, and parallel row-major arrays carry every cell/source
identity, source revision, elevation, and material. The retained projection no
longer repeats one native Point, validity row, and layer membership per terrain
vertex. Re-expansion is exact and verification remains linear in objects plus
cells; increasing terrain density must not reintroduce a per-cell scan of every
native object. Terrain-control geometry never becomes observed elevation.

The browser receives packed-source grids through
`rey.regional-terrain-grid.transport.v3`. This is a lossless transport view,
not a new terrain authority: it binds the original dataset, source artifact,
exact source feature and revision, packed row-major values, and explicit
validity. Native coordinates and grid positions derive only from admitted
bounds and dimensions. Cell locators, source revisions, and BLAKE3 cell
identities derive from the exact source feature plus row and column only when
an individual cell is inspected. Rust and TypeScript share an executable
identity vector so this derivation cannot silently drift across runtimes.

Repeated terrain Point objects, per-object validity rows, terrain-layer
membership, fixed-width digest columns, and source-object suffix arrays are
absent from both compact retention and the v3 workload payload. Field
compilation decodes only typed value columns and never allocates an
identity-rich object for every vertex. Retained v1/v2 point grids continue to
use compatible v1/v2 browser transport; exact historical identities are not
rewritten. The CLI remains the full human verification surface and reports the
semantic terrain-vertex count even though those vertices are not stored or
transported as duplicate objects.

For the 501×501 Rey County field, a measured cold unoptimized operator read
fell from roughly 200 seconds of repeated packed-cell expansion plus 34 seconds
of transport work to 1.3 seconds with optimized pure-compute development
dependencies. The gzip workload transfer is about 718 KiB and a warm retained
projection returns in about 15 ms. These are named local development
measurements, not universal deployment guarantees. The structural invariant is
more important: retained verification and wire size are proportional to packed
value/validity columns, not multiplied by derivable per-cell identity objects.

Source admission also accepts one bounded GeoJSON foreign member with schema
`rey.packed-terrain-grid.v1`. Its Polygon geometry is the exact CRS84 grid
envelope; the foreign member binds dimensions, integer-microdegree bounds,
compiler revision, byte-exact validity, little-endian centimeter elevation,
and palette-indexed material channels. Editor indexing validates every channel,
requires an exact supported triangle, freezes the native bytes, and exposes the
same source through `rey editor`. Scene admission independently reparses those
bytes and derives a unique cell locator and revision from the packed feature
revision plus row/column. The retained result is
`rey.regional-terrain-grid.v3`; its derivation inputs reproduce the exact cell
identity without storing per-cell hash and locator arrays, and its
`geojson_packed_grid_v1` browser source encoding prevents any Point-feature
authority claim. Legacy point-grid v1/v2 identities remain byte-compatible.
Lazy browser evidence reconstructs an exact packed source cell, not a
fictitious native Point object. The adapter is bounded to one million cells and
cannot mix packed and point-grid bindings in one terrain source.

Each admitted regional field remains one bounded in-memory grid rendered by a
bounded 3D orthographic terrain camera through a tiled accelerated working set.
The application may now retain and validate multiple active editor packages,
including each scene's historical admission-atlas binding and its membership
in the current atlas. The runtime derives an exact
`rey.regional-geography-composition.v2` assessment over every bounded package
pair. It distinguishes gaps, corner contact, shared edges, overlaps, missing
terrain support, sample misalignment, and validity/elevation/material
conflicts. `ready` requires a connected conflict-free graph of
terrain-qualified edges; the assessment itself grants no merge or synthesis
authority. Its canonical `terrain_components` give every connected qualified
member set a content identity and exact member/seam lists. The CLI and browser
consume those same components; the browser does not independently infer graph
connectivity.

Selection binds one synthetic Atlas member to its exact server-owned terrain
component and compiles that component into the transient shared-frame mosaic
described above. The application drives the same reversible model transform
through surface, passes, and reference paths. Native raster/imagery streaming,
qualified geography-compiler stitching, and retained Landscape fidelity
voyages remain work tracked by [Plan
0005](../../../plans/0005-landscape-terrain.md).
