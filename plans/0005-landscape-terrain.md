# Plan 0005: Deliver Geographic Landscape Terrain

- Status: Active
- Owns: admitted regional elevation datasets, validity-safe multi-region
  mosaics and relief pyramids, terrain tiling and residency, bounded 3D County
  camera, Atlas-to-Landscape continuity, geographic render passes, and
  fidelity/performance voyages

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
  → shared-datum validity-bounded mosaic
  → haloed renderer-neutral height and relief pyramids
  → camera-selected render tiles
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
incomplete grids, proves every expanded row-major vertex against its source
object, then retains the lossless compact result as
`rey.regional-terrain-grid.v2`. Bounds and dimensions derive positions while
parallel channels retain exact cell/source identities, artifact and object
revisions, validity, elevation, and material. The grid is carried by
`rey.regional-terrain-program.v2`; terrain-program v1 retains its original
point-only, no-interpolation meaning.

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
separate 64 MiB CPU and GPU budgets.

A selected regional Atlas member now retains one
`rey.atlas-landscape-transition.v3` binding from its exact synthetic sector
fragment to its exact primary terrain patch and shared Landscape mosaic.
One reversible projector drives
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
through a tilted object-view composition. The current packed 501×501 geography-
compiler output and map-first composition materially improve the base read at
roughly 167–185-meter spacing without renderer interpolation. Local geographic
form remains visibly below the required multi-scale fidelity bar. No imagery
provider or license authority is admitted, so the engine renders admitted
material and discloses the absence instead of fabricating familiar map imagery.

Fidelity qualification is now executable but has not yet been retained for the
complete matrix. `rey.explorer.landscape-fidelity@3` names steep relief, low
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

The normal Rey County fixture now carries a deterministic packed 501×501
authored regional grid. Its exact County footprint and Unexplored Scrub become
70,722 explicit no-data cells; 180,279 valid cells retain five bounded
materials and 32–1,784.12 meters of authored semantic relief. Revision
`rey.agent-geography.rey-county@7` resolves the named landform controls into
bounded orographic backbones, branching ridge networks, and incised ravines
before exact authored hydrology carves a preliminary field. A second
validity-contained pass priority-floods the surface, selects steepest-descent
receivers, accumulates drainage, and applies a broad, bounded incision without
exposing the D8 receiver tree as visible trenches. It also records
band-limited domain-warped macro, meso, ridge, and fine relief, coherent
land-cover inputs, a district/transport/label hierarchy, and the explicit
absence of cross-package stitching. The source is generated reproducibly from
checked-in boundary, district, feature, highway, hydrology, label, railway,
road, and terrain-control inputs. It traveled through editor admission as
`SCENE@12` and passed the hard-cut `rey.scene-admission.validate@2` run
`blake3:4bdbfb50717ebc9f2c63c8a6902dcd25f7ae13eaec9cc9fdbca4551b3bb5b0fd`
as `rey.scene-admission-result.v2` before `/explore` consumption. Pre-v2
admission and atlas history was discarded rather than adapted. This improves
the default project bearing without treating terrain controls as observed
height or closing the still-open named fidelity matrix.

That admission exposed a transport boundary. Retaining three 251,001-element
identity arrays in the v2 result exceeded the 64 MiB workload-state limit and
was rejected before state mutation. The derivation-compact v3 result reduced
the complete retained workload state to 9,955,778 bytes. The compatible
browser projection is still 27,294,879 uncompressed bytes because transport
v2 carries packed cell and source-revision digests. On the qualification
machine, an optimized cold server projection took about 18.2 seconds while an
exact cache hit took about 65 ms. The UI binds that cache to the exact workload,
catalog, ignore, environment, and Git dependencies, so an unrelated Channel or
conversation revalidation tick cannot trigger a rebuild loop.

`rey.ui-workload-transport.v1` removes the duplicated latest accepted scene
when the canonical active-scene set is present and encodes a gridded terrain
source once. Its first `rey.regional-terrain-grid.transport.v1` representation
removed geometry repetition; `transport.v2` also packs canonical BLAKE3 cell
and source-revision identities as concatenated fixed-width bytes and
prefix-compresses exact source-object IDs. One exact source artifact, validity
bytes, elevations, and palette-indexed materials remain lossless; coordinates
and grid positions derive only from admitted dimensions and bounds. The browser
validates the compact representation before projection, compiles directly from
typed value columns, and reconstructs a complete identity-rich cell only for an
exact evidence route. Browser compatibility retains v1 decoding while the
server emits v2. Point-feature scene admissions remove the same per-vertex
repetition from retained workload state through `rey.regional-terrain-grid.v2`.
Packed-source admissions use `rey.regional-terrain-grid.v3` to retain one exact
source-feature identity and derive the cell locator, source revision, and cell
identity from row/column instead of storing three repeated identity columns.
Verification reconstructs exact cells and fails closed on channel or
derivation tampering. Legacy retained v1/v2 grids remain verifiable. Cold
workload projection at substantially higher source density remains open. The
browser validates large Base64 columns in one bounded linear pass, defers byte
decoding until exact Evidence, and validated the actual SCENE@11 transport in
about 200 ms under Node. A derivation-aware browser transport remains open to
remove the remaining wire repetition.

The source side now has a matching compact admission path. A single GeoJSON
feature may carry `rey.packed-terrain-grid.v1` beside its exact Polygon grid
envelope. Editor and scene admission independently validate the declared shape,
integer-microdegree spacing, sorted material palette, byte-exact validity,
little-endian centimeter elevation, material indices, and at least one fully
supported triangle. Admission derives stable row/column cell locators and
revisions from the frozen packed feature, retains the derivation-compact v3
grid, and marks the source encoding as `geojson_packed_grid_v1` through browser
transport and exact evidence. The existing one-million native-coordinate limit
bounds packed cells independently from the native-object limit because those
cells are not counterfeit GeoJSON Point features. This is the required
high-density source adapter; generated artifacts remain incomplete until
separately reviewed and admitted.

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

Those voyages closed the executable base-layer item, not the named fidelity
matrix. At that point the named `coastline-water` row still required an
admitted `river` subtype and an accelerated admitted boundary: regional scene
admission retained only the generic hydrology layer, and the exact County
boundary coincided with no-data support. The later typed-hydrology cutover now
retains exact source `water_class` values and geometry compatibility instead
of weakening the suite or guessing from feature IDs. A fresh named capture is
still required before the row can close. The WebGL2 image also retained blurred
kilometer-scale relief, and its first Landscape submission took about 2.07
seconds under SwiftShader; both remain material acceptance gaps.

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
bright framing lines. The current 501×501 source enters the renderer-neutral
field without presentation refinement; fully supported source triangles remain
exact and cells touching explicit no-data remain absent. An independent
validity mask survives the complete pipeline. Lower-density admitted sources
may still receive bounded validity-safe refinement and presentation-only
microrelief, but this source does not. This corrects composition, layer
hierarchy, and source density while leaving the retained relief-fidelity
comparison open.

The refined field now feeds one deterministic geography chain:
`admitted elevation → depression-safe receivers → basin accumulation →
validity-bounded moisture/erosion potential → normals/curvature → land cover`.
The priority-flood topology never crosses no-data. Raw accumulation remains a
deterministic topology channel, while a smoothed copy drives material moisture
without exposing D8 raster paths. Renderer drainage does not displace the
already hydrology-conditioned authored elevation, and derived stream/river
linework is disclosed separately from exact admitted hydrology. Contours use
elevation intervals in meters—100 m at Landscape, 50 m at Neighborhoods, and
25 m at Objects/Evidence—with a hard level bound. Landscape omits supplemental
synthetic drainage linework so exact admitted waterways own its water read.

### Perceptual rebaseline — 2026-08-16

The operator-supplied 3022×1926 terrain reference and Rey's current 1920×1080
WebGL2 Landscape capture were inspected side by side. The reference is an
acceptance target, not admitted scene evidence and not a source asset. The Rey
capture is bound to voyage manifest
`sha256:36e556070f169bd0658b0a3166463e4577390d2000e1815dab26cd8c256eb023`.
The fulfilled-transport WebGL2 voyage passed World → rotated Atlas → Landscape
→ Objects → exact Evidence without browser exceptions, no-data triangle leaks,
or tile-seam mismatches. It is a traversal baseline rather than a named
Landscape-fidelity matrix row.

That capture binds source compiler `rey.agent-geography.rey-county@7`,
renderer geography `rey.terrain.regional-geography@4`, and material
`rey.terrain.tsl-continuous-relief@3`. Render-pass compiler
`rey.explorer.terrain-render-passes@3` clips water boundary triangles to the
exact admitted polygon and derives every new boundary height barycentrically
inside the same fully valid terrain triangle. Local, midslope, and regional
topographic tone contributes only where each complete sampling window is
valid, and the material retains stronger directional separation. The healthy
accelerated posture also leaves duplicate SVG vectors as invisible focus
surfaces, keeping labels and evidence semantics without drawing each road,
rail, district, and water edge twice. The Landscape frame selected 88
finest-level tiles containing 95,832 tiled field cells and 6,762,624 bytes of
active GPU input, with zero support leaks and zero seam mismatches. Residency
held 16,241,616 CPU bytes and 17,087,076 GPU bytes beneath the 48 MiB and 64 MiB
budgets. Its fulfilled-transport worker update took about 1.26 seconds and its
reported submission took 4 ms under SwiftShader; those transient values are
not hardware performance claims. The visual result remains subordinate to the
open fidelity gap.

| Dimension      | Minimum acceptance read                                      | Current Rey County read                                                            | Gap   |
| -------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------- | ----- |
| Composition    | Continuous, overhead geography fills the map viewport.       | One attached near-north-up surface fills the usable canvas.                        | Minor |
| Relief         | Fine ridges, valleys, benches, and drainage at many scales.  | Denser local ridges and valleys read, but long soft tonal bands remain.            | Major |
| Hillshade      | Crisp multiscale form without faceting or muddy smoothing.   | Higher-density normals separate form, but the image stays soft and locally banded. | Major |
| Land cover     | Coherent local classes with terrain-following boundaries.    | Five admitted classes remain coherent but read as a broad low-frequency wash.      | Major |
| Water          | Continuous areas and terrain-following river hierarchy.      | Exact clipped river/wetland areas and tributaries retain shorelines.               | Major |
| Contours       | Scale-aware hierarchy reveals form without dominating it.    | Metric contours are subordinate but lack reference-level density.                  | Major |
| Vectors/labels | Roads, rail, structures, and labels resolve by semantic LOD. | Accelerated vectors no longer receive a duplicate SVG drawing.                     | Major |

This matrix keeps the side-by-side delivery item open. Repeated object-per-cell
browser transport is now replaced by a compact renderer-neutral payload that
retains exact artifact revision, dimensions, bounds, value channels, material
palette, validity, and lazy exact-cell evidence. The next source-fidelity slice
may therefore increase authored field density and deepen the
water/transport/label hierarchy, but must do so against named retained-state,
cold-compile, wire, resident, and submission budgets. Interpolating or shading
the current 501×501 field more aggressively is not an acceptable substitute for
the next admitted multi-resolution source.

### Relief-engine rebaseline — 2026-08-21

The operator-supplied Google Maps terrain comparison (French Pyrenees around
Pic du Midi de Bigorre, Bagnères-de-Bigorre, and Barèges) and the 7:13 PM Rey
County Landscape capture were evaluated side by side. The reference is a visual
acceptance target only; it is neither admitted terrain evidence nor an asset
that Rey may redistribute. Named techniques below are Rey implementation
choices selected to close visible gaps; they are not claims about the
reference product's proprietary rendering pipeline.

The comparison proves that while Rey's accelerated pipeline executes
interactive continuous terrain, it fails the cartographic acceptance bar on six
fundamental dimensions:

| Dimension                       | Visual acceptance target                                                                                                                                                       | Rey County Landscape (7:13 PM Capture)                                                                                      | Gap   |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- | ----- |
| **Geomorphic Resolution**       | Knife-edge aretes, crags, cirques, couloirs, dendritic avalanche chutes, talus slopes, and incised ravines.                                                                    | Soft, rounded "puffy dough" mounds; sub-100m features are absent due to ~180m source grid spacing.                          | Major |
| **Multi-Scale Hillshade**       | Crisp, aspect-balanced relief remains legible across slope directions and scales without blacking out steep faces. Rey will pursue an MDOW-style operator to meet this target. | Fixed 3-point scalar illumination clamped to `[0.42, 1.14]`, producing flat, low-contrast, plastic-looking slopes.          | Major |
| **Topographic Openness**        | Deep valleys, ravines, and hollows retain ambient depth. Rey will pursue SVF / positive-negative topographic openness to meet this target.                                     | Weak low-frequency elevation difference; valley bottoms lack ambient occlusion depth.                                       | Major |
| **Color & Hypsometric Tinting** | Continuous elevation- and slope-graded hypsometric tinting (lush valley greens → warm mid-slopes → slate/grey alpine summits → cliff rock).                                    | Five discrete palette classes mix into a uniform, desaturated olive-grey wash across all elevations.                        | Major |
| **Chromatic Lighting**          | Two-tone cartographic lighting: warm sunlit highlights on illuminated faces vs. cool ambient sky-fill in shadows.                                                              | Monochromatic scalar multiplication (`tint * hillshade * occlusion`) creating dirty grey shadows and washed-out highlights. | Major |
| **Surface Continuity & Seams**  | Seamless, homogeneous terrain field across all viewports and zoom levels without visible tiling artifacts.                                                                     | Regular horizontal and vertical tonal discontinuities / crosshatch grid lines crossing the entire terrain surface.          | Major |

The visual discontinuities and fidelity shortcomings have concrete architectural causes across the pipeline:

1. **Tile-boundary kernel truncation**: Deriving multi-scale relief operators
   (local: 350m, midslope: 1,400m, regional: 5,600m) over independently
   materialized tiles truncates support at tile borders, causing neighboring
   tiles to renormalize differently and producing regular crosshatch banding.
   Relief must be derived over a unified mosaic or with source gutters satisfying
   `gutter_radius >= max(operator_support_radius_meters / sample_spacing_meters)`.
2. **Missing MDOW and Sky-View Factor operators**: The current shader executes
   a single scalar hillshade multiplier from fixed key/fill/back vectors. It
   lacks aspect-weighted multi-azimuth illumination (NW 315° primary, SW 225°
   fill, NE 45° back-rim), slope-adaptive illumination angles (avoiding shadow
   blackout on steep slopes while preserving flat terrain clarity), and true
   horizon/sky-view ambient occlusion.
3. **Monochromatic lighting model**: In `createContinuousReliefMaterial`, the
   TSL shader computes `clamp(tint * hillshade * occlusion, 0, 1)`. In
   cartography, scalar multiplication produces muddy greys; realistic shaded
   relief requires dual-tone chromatic lighting that blends a warm direct sun
   component with a cool, ambient-sky fill component.
4. **Coarse source resolution & isotropic synthesis**: Rey County's 501×501
   packed grid (~167–185m nominal spacing) cannot mathematically represent
   knife-edge ridges or dendritic ravines. Furthermore, the synthetic geography
   generator relies on isotropic value noise rather than geomorphological
   processes (fluvial incision, thermal scree deposition, structural faulting).

`rey.landscape-relief-engine@1` and `rey.landscape-patch-set@1` are enabling
prototypes, not the completed engine. The former establishes renderer-neutral
relief channels and a cartographic material path; the latter carries ordered
patch metadata and deterministic overlap depth. Neither yet provides a
shared-datum multi-region mosaic, haloed derivation, deterministic evidence-
aware overlap resolution, or an admitted overview source for terrain between
regional patches. The rendering engine must close those contracts before
additional contrast tuning can count as fidelity progress.

The next engine revision closes the known single-field tile-kernel defect.
`rey.landscape-relief-engine@2` derives `rey.landscape-relief-field.v2` once
over each complete refined regional field before camera selection, then samples
the retained hillshade, salience, and tangent arrays into render tiles by exact
source row/column identity. `rey.terrain.compilation-worker@4` retains both
shared relief-border mismatches and complete-field/partition mismatches; the
fidelity suite requires the latter to remain zero. Residency and worker CPU
budgets now include those sampled derived arrays. This is seam-safe for one
complete regional field. The following regional-mosaic slice now supplies the
shared horizontal frame; a metric relief pyramid remains open below.

`rey.terrain.compilation-worker@7` and reference renderer revision 4 now hard-
cut admitted terrain through the materialized
`rey.landscape-pyramid-envelope.v1`. The envelope binds exact BLAKE3 height,
validity-class, hillshade, salience, tangent, derivation-tile, maximum-gutter,
and border-digest content at every conservative height/relief level before
camera tile sampling. Every relief tile derives from a source window at least
as wide as the largest supported metric operator, crops only its render
interior, and must equal whole-level derivation within the named `1e-6`
tolerance. Adjacent canonical border digests and overlapping interiors must
match after tolerance quantization. The earlier one-level zero-gutter fallback
contract has been removed rather than retained as a compatibility path.

`rey.terrain.height-hierarchy@2` and
`rey.terrain.compilation-worker@6` add the first materialized 8.3 slice before
camera tile projection. They derive bounded multiresolution levels from the admitted
field, require a complete valid child window before any parent becomes valid,
retain no-data versus unsupported at every level, and encode every sample's
canonical contributing patch set as exact CSR offsets and indices. Hierarchy
bytes now participate in the worker CPU limit and both renderer paths expose
hierarchy identity and level counts. Revision 2 removes the accidental
odd-dimension requirement: non-dyadic and rectangular source grids now reach a
bounded 2×2 root through explicit conservative child windows without gaining
support. The shared height and relief envelopes carry those levels, and
`rey.terrain.dataset-tiles@2` now selects them directly with cumulative
screen-space error and exact revision-bound residency.

`rey.landscape-relief-engine@4` replaces that prototype with
`rey.terrain-relief-metrics.v1` source-spacing and elevation-range metadata.
It derives local, midslope, and regional target radii in meters, explicitly
marks scales that the admitted grid cannot support, excludes unsupported
scales from composition, and exposes the exact scale basis and support through
renderer diagnostics. Revisioned metric gradients, slope-adaptive MDOW,
SVF/openness, high-pass curvature/ridge salience, and linear local tone arrays
are now shared by both render paths. Chromatic lighting and final map
composition remain in 8.5.

`rey.landscape-relief-engine@5` replaces the final Reinhard shoulder with
`rey.landscape.linear-tone-map@2`. The prior shoulder compressed most valid
MDOW samples toward the same middle value after the metric, openness, and
local-contrast operators had separated them. Revision 5 retains the neutral
flat response while expanding locally normalized shadow and highlight
separation within a `[0.32, 1.24]` nonblack bound. It changes no elevation,
validity, metric support, source identity, or lighting ownership.

`rey.terrain.regional-mosaic@1` now establishes and executes the next
renderer-neutral contract. It compiles integer-aligned, common-scale regional fields
into `rey.landscape-mosaic.v1`, requires identical validity, elevation, and
material at qualified shared samples, rejects positive-area overlap, and
retains uncovered grid cells as unsupported. The result carries exact source
placements, revisions, authority, sample spacing, composition, coordinate and
vertical references, focus patch, limits, and omissions into every sampled
render tile. `/explore` selects only the connected conflict-free component of
qualified edge seams containing the focused region, projects all members
through one horizontal frame and component-wide elevation normalization, and
derives refinement, geography, relief, and tiles from the resulting single
mosaic. Disjoint or conflicted members remain out; a component that fails the
stricter renderer-neutral alignment contract falls back to the focused patch
with an explicit omission. Positive-area overlap and admitted overview gap
coverage remain open.

`rey.terrain.regional-mosaic@2` admits positive-area overlap on the existing
common aligned lattice. It retains a BLAKE3-identified source-contribution
raster and conflict mask, then resolves each disagreeing sample by validity,
declared authority, nominal metric spacing, and stable source identity. Reversed
input order produces the same mosaic identity, contribution map, and height
bytes. The Atlas/Landscape transition and accelerated/reference diagnostics
retain the exact contribution/conflict identities and conflict count. This does
not yet resample differently aligned or nested grids, feather a mutually valid
overlap, or admit overview coverage.

`rey.terrain.regional-mosaic@3` adds deterministic height feathering only at
two-source samples where both sources are valid and have equal declared
authority, role, and nominal metric spacing. Edge-distance weights hand the
surface from one patch interior to the other without expanding either source's
validity. The compiler retains an exact BLAKE3 feather identity,
secondary-owner raster, primary weights, and feathered count beside the primary
contribution and conflict rasters. No-data, unsupported, unequal-authority,
unequal-resolution, and three-or-more-source overlap remains a hard auditable
selection.

`rey.terrain.regional-mosaic@4` treats a `role: overview` DEM as separately
admitted supplemental evidence. Compatible overview samples may own only grid
space not already supported by a detail source; valid detail samples and
explicit detail no-data boundaries win regardless of overview priority. The
compiler retains a BLAKE3 overview-coverage identity, exact coverage raster,
source patch set, and covered count through the Atlas/Landscape transition and
both renderer diagnostics. An absent, invalid, or incompatible overview source
leaves the existing unsupported hole unchanged.

`rey.terrain.regional-mosaic@7` closes companion attribution, compacts the
exact mosaic manifest and channel content into a bounded BLAKE3 identity, and
rejects incompatible horizontal or vertical datum bindings without treating
the height result as evidence for another layer. It retains land-cover owners
in a separate BLAKE3-identified raster bound to every exact source and material
channel revision. Contours explicitly name the composed height and validity
content they derive from; water and semantic vectors remain outside the height
mosaic and keep their feature-level source revisions and authority. The shared
transition and renderer diagnostics expose the companion-attribution identity.

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

The source-controlled generator is the first geography compiler. Its seventh
revision produces the packed 501×501 field with deterministic multi-scale relief,
orographic branching, exact hydrology-conditioned height, validity-contained
source drainage, coherent base materials, exact no-data, districts, transport,
cartographic labels, and explicit synthesis metadata. Multi-package
admission now retains a canonical bounded active scene set instead of replacing
the preceding editor package whenever `scene-admission` runs. Every scene keeps
its exact original atlas back-reference while the current atlas inventories all
active scene/package/packet memberships, and the browser validates both the
historical admission binding and current membership before projecting it.
`rey.regional-geography-composition.v2` now evaluates every bounded native
package pair and retains gaps, corner contact, shared edges, overlaps, terrain
sample alignment, validity, elevation, and material conflicts under one exact
atlas revision. The human workload list exposes its package, pair, qualified
seam, conflict, and stitch-readiness counts. Version 2 additionally retains a
stable identity and canonical member/seam set for every focus-selectable
terrain component; the human workload list prints each component and its
source-level exclusions, and `/explore` consumes the same server-owned set.
This assessment grants no source merge or synthesis authority. Explorer may
project its connected qualified edge component into a renderer-neutral validity-safe mosaic, but that derived
field is not an admitted source dataset and cannot resolve an overlap, fill a
gap, or change a seam decision. Qualified geography-compiler output,
raster-native field storage, and deeper vector density remain subsequent slices
of the same boundary.

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
- [x] Replace repeated terrain-object browser transport with a bounded compact
      row-major field representation while retaining exact dataset, artifact,
      cell, source-object, revision, validity, elevation, material, and lazy
      evidence bindings.
- [x] Retain qualified regular grids in the same lossless compact form so
      higher source density does not duplicate terrain objects, validity rows,
      layer membership, and grid cells in workload state.
- [x] Add explicit water surfaces/areas and scale-aware contour styling before
      treating roads, railways, structures, or label density as fidelity
      completion.
- [ ] Retain 1920x1080 side-by-side captures and record perceptual gaps for
      composition, relief, hillshade, land cover, water, contours, and vector
      hierarchy. Do not mark this slice complete while a major gap remains.
- [x] Retain multiple admitted editor packages and derive a bounded,
      content-identified native-boundary seam/conflict assessment without
      treating package ingestion as geography.
- [x] Feed only a connected conflict-free set of terrain-qualified seams into a
      revisioned geography compiler, retain every explicit resolution, and
      admit its stitched multi-resolution output before rendering it.

### 8. Formalize the seam-safe multi-region Landscape engine

Implement this slice in dependency order. A later milestone may prototype
against an earlier contract, but it cannot be accepted until its prerequisites
and CLI evidence are complete.

#### 8.1 Freeze the renderer-neutral engine contracts

- [x] Replace patch ordering as composition authority with a versioned mosaic
      request/result contract. Bind every patch's dataset and implementation
      revisions, native coordinate identity, horizontal units, vertical
      reference, nominal source spacing, validity, authority, bounds, and
      material/vector companions.
- [x] Define `rey.landscape-mosaic.v1` as a content-identified height,
      validity, source-contribution, conflict, and omission field. Keep the
      selected Atlas member as the focus anchor, not an implicit winner for all
      overlapping terrain.
- [x] Define `rey.landscape-height-pyramid.v1` and
      `rey.landscape-relief-pyramid.v1`. Each level must retain metric sample
      spacing, dimensions, bounds, validity, source lineage, operator support,
      implementation revision, and exact parent/child identity.
- [x] Hard-cut the accelerated and reference paths to those shared contracts
      once parity is proved. The enabling field was hard-cut to
      `rey.landscape-relief-field.v4` when 8.3 supplied the haloed hierarchy
      and 8.4 supplied revisioned metric cartographic operators.
- [x] Extend the existing verbose `rey workloads run scene-admission` result
      and structured JSON with patch-set, mosaic, pyramid, conflict, omission,
      source-resolution, and renderer-budget summaries before treating the
      browser engine as verifiable.

Acceptance: the CLI can explain which exact sources contribute to every
Landscape field, why an overlap winner was selected, which gaps remain unknown,
and which implementation revisions produced each derived channel.

#### 8.2 Compile a validity-safe multi-region mosaic

- [x] Select only a connected terrain-qualified subset from
      `rey.regional-geography-composition.v2`; reject or omit coordinate,
      vertical-reference, unit, and seam relationships that are not qualified.
- [x] Transform qualified patches into one declared horizontal frame and
      vertical reference before resampling. A missing transform or datum
      relationship remains a typed omission and cannot be hidden by visual
      alignment.
- [x] Resolve overlaps deterministically from declared authority, validity,
      nominal source spacing, and stable source identity. Retain both inputs,
      the decision map, conflicts, and limits; input array order must not decide
      source truth.
- [x] Feather height or presentation channels only inside mutually valid
      overlap support. Never extend either patch's validity or blend across a
      no-data boundary.
- [x] Fill space between detailed patches only when a separately admitted,
      compatible overview DEM covers that space. Without that evidence, retain
      an explicit hole through the mosaic, pyramid, mesh, and shading paths.
- [x] Keep land cover, water, contours, and vectors as independently
      attributed companions. A height mosaic cannot manufacture their source
      authority.

Acceptance: fixtures for adjacent patches, partial overlap, nested resolutions,
datum conflict, invalid overlap, corner contact, and gaps with and without an
admitted overview source produce deterministic mosaics with no validity gain.

#### 8.3 Build the height pyramid before camera tile materialization

- [x] Construct the multiresolution height/validity hierarchy over the shared
      mosaic. Downsampling must be conservative at validity boundaries and must
      retain the contributing source set for each parent sample.
- [x] Give every derivation tile a source gutter at least as wide as the
      largest active relief operator (`gutter_radius >= max(operator_support_radius_meters / sample_spacing_meters)`).
      Derive channels from the halo, crop only the render interior, and retain
      border digests for adjacent-tile proof.
- [x] Make whole-field and partitioned compilation equivalent within one named
      numeric tolerance. Moving the camera or changing the active tile
      partition must not change a sample's height, normal, or illumination.
- [x] Select pyramid levels with stable screen-space error and compatible
      neighboring support. Preserve current cancellation and CPU/GPU residency
      bounds; add derived-channel bytes and halo work to those measurements.
- [x] Cache by mosaic revision, pyramid level, tile identity, operator revision,
      and validity support so an Atlas prewarm can be reused without admitting
      stale or differently composed terrain.

Acceptance: an untiled reference field and every legal tiling of it have
identical valid interiors, zero no-data triangle leakage, and no internal-edge
discontinuity or crosshatch banding attributable to kernel truncation.

#### 8.4 Derive cartographic relief at explicit metric scales

- [x] Derive metric slope, aspect, and normals from source spacing rather than
      cell count. Produce separately revisioned local, midslope, and regional
      channels whose support radii are declared in meters (e.g. local 350m,
      midslope 1,400m, regional 5,600m).
- [x] Implement deterministic Multi-Directional Oblique Weighted (MDOW) Swiss
      hillshading. Weight illumination across multiple azimuths (NW 315° primary
      sun, SW 225° fill, NE 45° back-rim) with slope-adaptive contrast to prevent
      pitch-black shadows on steep faces and washouts on flat plains.
- [x] Add a deterministic Sky-View Factor (SVF) / positive-and-negative
      topographic openness term to naturally darken deep gorges, cirques, and
      valleys without artificial scalar multiplier hacks.
- [x] Blend high-pass profile/plan curvature and slope magnitude into high-frequency
      ridge salience so micro-scale crests, couloirs, and ravines separate
      crisply against macroscopic mountain mass illumination.
- [x] Apply local contrast and cartographic tone mapping as presentation
      channels in linear color space. Keep one lighting owner so a pre-lit
      relief scalar is not lit again by a physical material.
- [x] Share exact derived arrays, masks, parameters, and implementation identity
      between the deterministic reference and WebGPU/WebGL2 paths. Renderer
      backends may execute the math differently only under retained parity
      tolerances.
- [x] Treat finer terrain content as source work. Any synthesized landform must
      be generated, reviewed, and admitted before rendering with explicit
      lineage; shader noise and renderer-side microrelief cannot substitute for
      absent elevation evidence.

Acceptance: named steep- and low-relief fixtures preserve fine ridges without
muddy faceting or plastic smoothing, avoid broad tonal domination, and disclose
when the admitted spacing cannot support the comparison scale.

#### 8.5 Compose a coherent terrain map

- [x] Implement dual-tone cartographic chromatic lighting: blend warm direct
      sunlit highlights on illuminated slopes with cool, ambient-sky-tinted
      diffuse fill in shadowed aspects, replacing desaturating grayscale scalar
      multiplication (`tint * hillshade * occlusion`).
- [x] Replace discrete flat palettes with continuous elevation- and slope-graded
      hypsometric color ramps (lush valley greens → warm mid-elevation montane
      grasslands → slate/grey alpine crests) with slope-triggered rock/cliff
      exposure on steep grades.
- [x] Render terrain-bound water areas with crisp high-contrast polygon fills
      and distinct shorelines (e.g. alpine tarns, glacial lakes, and river
      corridors) draped seamlessly over the relief.
- [x] Drive contour interval, weight, and opacity from semantic LOD and metric
      elevation range. Contours remain thin, crisp, and subordinate to relief,
      never concealing hillshade defects.
- [x] Add terrain-bound roads, rail, structures, boundaries, and labels only
      from their exact admitted companions, with independent visibility and
      invalidation revisions.
- [x] Keep selection and evidence overlays readable without changing the base
      relief assessment. Preserve the deterministic accessible reference path
      under backend loss.

Acceptance: the base map reads first as continuous geography, then as relief,
water, land cover, and semantic detail; no layer gains evidence or coverage
from visual composition.

#### 8.6 Make Atlas and Landscape sample the same terrain hierarchy

- [x] Generate Atlas stipple position, density, salience, and reveal order from
      the exact mosaic and relief-pyramid samples that Landscape renders. Do not
      run an independent raw-field relief calculation for Atlas.
- [x] Retain source-native sample identity and deterministic seed/order through
      every transition frame so stipples can expand into the corresponding
      Landscape support instead of dissolving into unrelated geometry.
- [x] Use the selected Atlas member as the camera/focus anchor while allowing
      qualified neighboring and overview patches to reveal from the common
      mosaic. A primary patch is not the boundary of the visible world.
- [x] Prewarm the exact mosaic, pyramid levels, relief revision, and material
      revision required by the predicted entry view. Keep the last compatible
      submitted terrain until its successor submits; never flash an empty or
      differently compiled field during handoff.
- [x] Prove wheel, click, back-navigation, interrupted traversal, and backend
      loss in both directions without a semantic field swap, tile flash, or
      validity expansion.

Acceptance: Atlas ↔ Landscape is one reversible content-derived morph over
one source lineage, including when several regional patches contribute to the
entry viewport.

#### 8.7 Retain the qualification matrix and close the fidelity bar

`rey.explorer-landscape-fidelity-assessment.v1` is the retained aggregation
boundary for this gate. It binds every workload/viewport row to complete
reference, WebGL2, and WebGPU voyages, exact screenshots, rendered parity, and
a digest-verified operator-supplied consumer-map reference. Operator labels
and pass/minor/major judgments remain explicitly self-asserted. Missing rows,
changed lineage, failed workload assertions, incomplete parity, and any major
judgment leave the assessment and this plan open.

The first retained steep-relief triad exposed a composition defect after
semantic and rendered parity passed: chromatic composition calculated SVF,
signed openness, local contrast, and material occlusion, then normalized the
candidate color back to `base luminance × hillshade`. That erased most of
their luminance contribution and produced the assessed flat, pale read.
`rey.landscape.chromatic-relief@2` retains those terms in one bounded linear
tone target shared byte-for-byte by the reference, WebGL2, and WebGPU upload
paths. A fulfilled-transport 1920×1080 WebGPU steep-relief voyage passed with
manifest `sha256:80b83f666670a4d07736a5c2b6932bfe9f90b8af84ed54c89ed36f0ff031263b`.
Its Landscape capture has measurably deeper enclosed terrain and less pale
linear-color wash, but broad tonal bands and insufficient local form remain a
major perceptual gap. `rey.landscape-relief-engine@5` then removed the
remaining Reinhard compression and passed a second 1920×1080 WebGPU voyage in
manifest `sha256:d2a3826e5c9d939ae398ed36dd56f5ee4e4d6b580cdbb30ef5d84be00377dd58`.
Across the fixed 1750×750 canvas crop, mean display luminance moved from
`0.7271` in the pre-correction capture to `0.6388`, while standard deviation
increased from `0.0394` to `0.0635`. These image statistics demonstrate that
the pale compression changed; they are not a perceptual acceptance result.
The remaining broad source-scale bands and insufficient geomorphic detail keep
the explicit major judgments open.

`rey.terrain.regional-geography@7` next rebalanced the derived land-cover ramp
in linear color and narrowed full rock exposure to genuinely steep or alpine
support. The admitted DEM, source material classes, validity, and hydrology
did not change. Its 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:a87c9564ee37ea5cba5b1de63979b6c3616a58aeeec142c02752e871659d8f7a`.
Across the same fixed canvas crop, mean display saturation increased from
`0.1293` to `0.1561` and luminance deviation increased from `0.0635` to
`0.0794`. The capture now separates vegetation, warm midslopes, and neutral
rock materially better, but the perceptual matrix remains open pending a
fresh explicit judgment and the unresolved source-scale form gap.

The same capture showed exact water triangles alternating with the base
surface because the two passes remained too close for stable depth ordering.
`rey.explorer.terrain-render-passes@5` retains the existing exact polygon clip
and barycentric height but applies a bounded presentation-only separation,
stronger polygon depth bias, explicit water render order, and a high-opacity
depth-writing base. The replacement 1920×1080 WebGPU voyage passed in manifest
`sha256:c444f264eed286cdddd7efe13cf20288792fd120795dc93cc3d4fb29c2735425`;
its river and polygon surfaces are continuous instead of crosshatched. The
following `rey.scene-admission.validate@3` /
`rey.scene-admission-result.v3` and
`rey.explorer.terrain-render-passes@6` slice hard-cuts classless hydrology from
current scene verification, retains each exact authored `water_class` through
editor transfer, admission, mining, and browser topology, and separates
open-water, wetland, river, stream, and seasonal-runoff presentation without
widening terrain validity. V2 admissions and their atlas bindings are excluded
rather than adapted. This closes the missing semantic mechanism; the named
coastline/water capture and explicit perceptual assessment remain open.

The hard cut was exercised through the human CLI boundary before browser
capture. Workload revision 3 passed all 11 frozen scenarios, then editor
`SCENE@13` retained the exact `SCENE@12` source snapshot with an equal content
delta while advancing package and request lineage to
`rey.scene-admission.validate@3`. Production result
`blake3:90948654c829f57edfa441e5bf690910bf124985af641304f5231d3ac26785d9`
accepted 12 geometry-compatible typed hydrology objects. The 1920×1080 named
`coastline-water` voyages passed for reference
(`sha256:dd6a57a6dd0059666240d9be86ed96373c25ebd8d7ad7ce1bf548bdbf22c0964`),
WebGL2
(`sha256:09c3f3f78c7ed9b3929ec2c39a3a457d0d431117b239cf35403e5b7cfa4e8762`),
and WebGPU
(`sha256:ae6992a9b1674e2696ebb6d6ebf7dc8873928a8553c4e973820f8fef497870aa`).
Each retained two distinct terrain-bound areas, the required admitted
boundary, river, stream, seasonal-runoff, water, and wetland kinds, zero
no-data triangle leaks, and zero tile or relief seams. Rendered parity manifest
`sha256:a3b3d2209e7c5383cb7df700406c898f7a2d908be7bd8326d4743c58619bf92c`
passed semantic equality and bounded accelerated WebGL2/WebGPU image
difference at maximum normalized RMSE `0.012372` against the `0.02` limit.
The captured wetland is now a subordinate green tint rather than counterfeit
open water. This closes the 1920×1080 coastline/water backend triad, not its
3840×2160 row, direct-transport proof, or the complete fidelity matrix.

The next source-fidelity slice is implemented as the deterministic
`rey.agent-geography.rey-county@8` candidate. It replaces the 501×501 lattice
with a bounded rectangular 705×626 lattice whose 131–133-meter metric spacing
remains exact in integer microdegrees and below the one-million-cell admission
limit. The source compiler narrows the oversized authored channel cuts, adds
sharp directional crest support, and replaces fixed drainage broadening with
validity-contained slope-aware stream-power incision and bounded inner/outer
valley widths. This is authored source geometry, not renderer noise. The
5.1 MiB candidate retains 317,297 valid and 124,033 no-data cells and records
its drainage metrics under `rey.county-source-drainage.v2`. This remains
incomplete enabling work until a new editor scene, workload result, browser
capture, and explicit perceptual assessment bind the exact candidate bytes;
the existing major geomorphic judgment therefore stays open.

`SCENE@14` and production admission result
`blake3:0796c5c7826eaf6e1b3ebd1463202b0617d91cd71da43f664f9a2e6e0955995b`
now bind those exact candidate bytes through the v3 human CLI path. The first
WebGPU prewarm retained failure manifest
`sha256:202e0ec2a94d59b811521aae2b708d0df1aea499854cde0b0d46ea3a778d5d8e`
before Landscape entry: the complete denser hierarchy plus selected-tile
output exceeded the prior 96 MiB transient compilation ceiling. This is a
real bounded working-output increase rather than resident cache growth, so
`rey.terrain.compilation-worker@10` raises only that ceiling to 112 MiB and
exposes the terrain-surface failure detail to qualification diagnostics. The
48 MiB CPU and 64 MiB GPU resident budgets remain unchanged pending the next
voyage.

That next WebGPU attempt materialized the hierarchy and selected 88 entry
tiles, but retained failure manifest
`sha256:dcce4c3b764af1e5ee9e2ac7711971cfa8255af05deb4c05da09fa5e6da422f6`
showed the Atlas prewarm stuck in `initializing`: a late report from the canvas
hidden when prewarm-only mode began overwrote the explicit prepared report.
`rey.explorer.atlas-terrain-prewarm@2` suppresses those stale hidden-canvas
callbacks while still publishing the exact prepared result. This corrects a
handoff state race; it does not relax any source, validity, or residency bound.

The replacement 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:a282d173be4ae339bc8318b2f83aeb3d2a0cfe10044115a75cb999a0d65bd20b`.
Its reverse Atlas, wheel-interrupted Atlas, Landscape re-entry, and same-page
back-navigation samples retained one mosaic, height hierarchy, relief pyramid,
source counts, and terrain fabric with no empty terrain or validity change.
The Landscape frame selected 88 tiles under 40,898,000 CPU and 32,362,388 GPU
resident bytes with zero support leaks or relief seams. The voyage also
exposed an unacceptable 16-second per-view hierarchy projection on the
fulfilled main-thread fallback: the exact 72.5 MiB materialized hierarchy was
larger than the 48 MiB hierarchy cache and therefore re-derived for every
semantic view. `rey.terrain.compilation-worker@11` raises that separately
bounded exact cache to 80 MiB so this one admitted hierarchy can be retained;
the 48 MiB tile-residency budget remains unchanged.

Worker revision 11 passed the same 1920×1080 WebGPU steep-relief voyage in
manifest
`sha256:c3e82298c5e73341d6e355adb8986f4653c01cf68832863a9bf826dc91dbb75d`.
Objects and Evidence both recorded one exact hierarchy-cache hit and zero
misses, reducing their main-thread update from roughly 16 seconds to 3.1
seconds. Inspection showed that a hit still paid to refine the admitted field
and derive regional geography before hashing the completed hierarchy key.
`rey.terrain.compilation-worker@12` now content-identifies the admitted input
with exact channel bytes plus refinement/geography revisions, then retains the
corresponding derived field beside its materialized pyramid. This preserves
the exact cache boundary while moving expensive derivation behind the miss.

Revision 12 passed the same voyage in manifest
`sha256:874b14294456ca1f1e3c6c4edfde076ee57ea0da573b14e6090d29af215a39fa`.
Exact cache hits now include the derived regional field; Objects and Evidence
updates measured 2.74 and 2.64 seconds respectively on the fulfilled
main-thread fallback. The remaining hit cost is bounded source-key hashing,
view-specific linework, tile materialization, and mesh preparation, not a
hierarchy miss. This is improvement but not completion of the performance
matrix.

Visual inspection of that exact Landscape capture found a new actionable
source defect: the higher-density crests are present, but exact-waterway
conditioning still cuts broad grey trenches through the map and overwhelms
the subtler ridge network. The next source candidate,
`rey.agent-geography.rey-county@9`, narrows main and tributary conditioning to
bounded three-to-four-cell corridors, reduces their direct depth, and
strengthens only source-authored directional crests and ridge octaves below
the 705×626 Nyquist limit. It changes exact elevation/material bytes under a
new dataset revision and leaves admitted water geometry, terrain validity,
renderer operators, and overlap policy untouched. The major perceptual result
remains open until this candidate is admitted and captured.

`SCENE@15` and scene-admission result
`blake3:46146bddf1cb542a0344d44858f4eb9f84e1a8e1c406200d31bc1ce25857b0c9`
now bind the exact revision-9 source through admission
`blake3:6200328f51c577026562e47477eab91fd53ecc292b8e26b82ec870eba4badf78`.
The corresponding 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:bc2b1b777ffd78e0ac5f2452d27b959080044840a8deaee80343d8f617cd40be`.
Landscape retained 317,297 valid and 124,033 no-data source vertices across 88
tiles with zero tile seams, relief seams, or unsupported triangles. The first
view remained a 16.45-second main-thread cache miss; Objects and Evidence were
exact derived-field hits at 2.69 and 2.61 seconds. The narrower water
conditioning no longer dominates the full corridor width, and directional
crests are more legible, but comparison with the operator reference still
finds a major geomorphic-resolution gap: broad repeated source-scale bands do
not yet read as sharply separated mountain mass, dendritic valleys, and local
ravines. The perceptual and performance gates therefore remain open.

The separately named WebGPU device-loss voyage passed in manifest
`sha256:57ab8ecc3eebd03e23bd41134df6604dfb86b0acd6f4061f952d27a172480bde`.
It induced loss before Atlas, observed the deterministic reference fallback,
kept renderer degradation out of the Explorer footer, and recovered WebGPU by
Landscape. Wheel entry, canonical click controls, reverse Atlas traversal,
wheel interruption, Landscape re-entry, and same-document back navigation all
retained one mosaic, height hierarchy, relief pyramid, terrain fabric, and
validity count with no empty terrain. Together with the normal voyage this
closes the 8.6 transition-mechanism proof; the assessment still omits direct
browser transport and the complete backend/viewport matrix.

The next performance slice, `rey.terrain.compilation-worker@13`, preserves the
revision-12 byte-derived hierarchy key while moving the reusable exact field
identity to `rey.terrain.regional-mosaic@8`. The mosaic compiler hashes every
composed typed channel, owner map, grid/frame parameter, and compiler revision
once; camera-only jobs consume that identity after either an object reuse or a
dedicated-worker structured clone. A new composition must establish a new
byte-derived identity before it can hit the hierarchy cache. In parallel,
`rey.terrain.regional-linework@4` keys retained contour/drainage output by its
actual interval and content profile, allowing Objects and Evidence to share
their identical 25-meter profile instead of deriving it twice under different
lens names.

The resulting 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:bc2b11a580f92a6dc9523a85ad3afbd094220a016ed217f6eb090b91eb065aa7`.
It retained the same 317,297 valid and 124,033 no-data source counts, one exact
mosaic/hierarchy/relief identity through the reverse and interrupted handoff,
and zero tile seams, relief seams, or unsupported triangles. The cold
Landscape miss measured 15.16 seconds, Objects' first 25-meter profile measured
1.46 seconds, and Evidence's exact shared-profile hit measured 165
milliseconds, down from 2.61 seconds in the revision-12 voyage. This closes the
repeated content-key and identical-linework defect, but not the cold hierarchy,
first-profile, total-voyage, or full performance-matrix gates.

The first matching reference voyage retained incomplete manifest
`sha256:3cf485f3f2beba98cec47419a90bbcc1e2450603e71c99a3261dd2978cff82a2`.
Its exact Landscape field was ready with zero seams and leaks, but mounting
589,999 fallback terrain polygons plus 59,595 line segments made Chrome's main
thread miss the subsequent continuity evaluation deadline. This is a real
reference-renderer scale defect, not permission to omit reference parity.
`rey.reference-regional-terrain@5` therefore rasterizes the same deterministic
triangle choice, validity test, barycentric linear color, and finest verified
relief field into one bounded 2D canvas. It retains accessible image and
hierarchy metadata while exact vector descriptions, focus targets, labels, and
evidence links remain independent DOM overlays. The change remains enabling
work until the reference voyage completes and its pixels enter rendered
parity.

The first rasterized rerun completed the voyage but retained incomplete
manifest
`sha256:b732972d60d187db103558e5f66f746ec9181bcfc83fcd8c086daa3c54c351b9`:
the browser correctly exposed reference terrain as `initializing`, while the
qualification driver still skipped the exact-terrain readiness wait for that
backend and captured zero hierarchy and render-pass lineage. The reference
surface now remains initializing until exact compilation exists, and the
driver requires complete height/relief hierarchies plus a bound pass set before
any reference terrain capture. No requirement was weakened.

The corrected 1920×1080 fulfilled-transport reference voyage passed in
manifest
`sha256:3a79ad8cb65872a401e3a89d5def2b44acbb1b287c1b540671f084c776b17a5c`,
and the matching WebGL2 voyage passed in manifest
`sha256:d0f699ffb345b4fff397c1f2435e64dabbce2634837872773e7a666d4e6f52d8`.
Both retain 317,297 valid and 124,033 no-data source vertices, exact mosaic,
height, relief, and render-pass identity, 118,458 Landscape line segments, two
terrain-bound areas, and zero tile seams, relief seams, partition mismatches,
or unsupported-triangle leaks. Together with the prior WebGPU voyage, rendered
parity passed in manifest
`sha256:cb9432939f3741c3d012e41c9a7bf354024fead3718d4cc9fc1f5b92bc50ad8d`;
WebGL2/WebGPU normalized RMSE was 0.00744 at Landscape and stayed below the
0.02 accelerated-backend limit at every stage. Reference-to-accelerated pixels
remain an observational comparison because the reference and accelerated
materials intentionally differ.

Visual inspection of the exact retained reference Landscape confirms that
the raster path preserves the same terrain and removes the DOM-scale failure,
but it does not close the perceptual gate. Broad repeated relief bands still
read as scalar mud instead of sharply separated mountain mass, dendritic
valleys, and local ravines. The 3840×2160 and remaining workload/backend rows,
direct browser transport, formal consumer-reference assessment, and
multi-region geography-compiler output therefore remain open.

That major result directs the next source slice rather than more palette
tuning. Candidate `rey.agent-geography.rey-county@10` replaces the dominant
single-posture ridge signal with cross-oriented, domain-warped hybrid
multifractal mountain mass inside the same named-control envelopes; raises
bounded local relief below source Nyquist; reduces exact-waterway height
conditioning to narrow shallow corridors; and retains a dendritic drainage
summary with heads, branch junctions, and Strahler order. Its exact validity
count remains 317,297 valid / 124,033 no-data vertices. At a declared
528.85-meter sample radius, the authored-source local-relief summary records
33.27 meters median and 64.46 meters at the 90th percentile across 19,281
supported samples. These are deterministic source diagnostics, not Earth DEM
observations or perceptual proof. The revision remains a candidate until a new
hard-cut scene admission and browser capture bind its exact bytes.

`SCENE@16` and accepted scene-admission result
`blake3:55081f5edc158b9a2bd83224eff3e96fb4e2e2c8eed42a96e9e8617e0fa40de5`
now hard-cut the browser to that exact revision through admission
`blake3:87debf32d5ab1515de7b75b78c74dc103bb4e39b503a395f8dc369d4872f0e9b`.
The 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:94cc758d1cc206646c5233fb2e91178433a8ed89e343c1ba427f5bf9c22e3514`
with 88 tiles, 169,081 line segments, two areas, stable handoff identity, and
zero tile seams, relief seams, partition mismatches, or unsupported triangles.
The capture shows materially stronger local relief and the shallow waterway
conditioning no longer dominates the entire corridor. It also exposes the
next source defect clearly: the globally repeated hybrid ridge signal reads as
fingerprint bands, while low-slope priority-flood escape paths leave
rectilinear/diagonal scars across Runtime Basin. The perceptual result remains
major; the next candidate must localize ridge emphasis, restore irregular
non-ridged mountain mass, and make incision respond to physical local slope so
flat drainage topology cannot become visible height.

Candidate `rey.agent-geography.rey-county@11` makes that source correction.
Global and named-control mass is now dominated by domain-warped non-ridged
fractal relief; cross-oriented hybrid ridges and sharp crests are subordinate
local channels rather than the County-wide signal. Drainage still retains its
complete deterministic topology, 467 heads, 384 junctions, and maximum
Strahler order 5, but height incision is multiplied by the unfilled local
terrain slope. Priority-flood escape paths across a flat basin therefore
remain topology rather than rendered trenches. The exact validity mask is
unchanged. At the same 528.85-meter sampling radius, median local relief is
26.12 meters and the 90th percentile is 50.53 meters: lower than revision 10's
band-inflated values by design, and still bounded source diagnostics rather
than perceptual proof.

`SCENE@17` and accepted scene-admission result
`blake3:ebd0fa454becd58f694cf8447428559c4ac0538c38e0f93009e63c5ac0e01d2d`
now hard-cut the browser to those exact bytes through admission
`blake3:f76159218de59316db676d55f3b88ab8bc7dd643dc927ecd4e8ae612e4159640`.
The 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:02c5caf9b11f6960617d973ca1f772b69910aa670ce22d997a8257d45883412d`.
It retained 88 tiles, 130,223 line segments, two terrain-bound areas, exact
Atlas/Landscape hierarchy identity, and zero tile seams, relief seams,
partition mismatches, or unsupported triangles. Landscape entry reused the
prewarmed hierarchy and completed its terrain update in 588.3 milliseconds on
the fulfilled main-thread fallback. Visual inspection confirms that the
rectilinear Runtime Basin trenches are absent and the County-wide fingerprint
signal is substantially reduced. The remaining broad, low-contrast, rounded
relief still lacks the sharply separated ridges, dendritic valleys, and local
ravines of the operator reference. The explicit geomorphic and multi-scale
relief judgment therefore remains major and keeps the fidelity gate open.

Candidate `rey.agent-geography.rey-county@12` is the next bounded source
response. Before drainage, it separates convex and concave form at 311,343
source vertices whose complete five-by-five neighborhoods are valid, while
leaving 5,954 valid boundary vertices unchanged. Raise and lowering are
bounded to 47.79 and 51.19 meters. Its stronger slope-conditioned fluvial pass
retains 475 heads, 396 junctions, maximum Strahler order 5, and no flat-path
incision while increasing maximum supported incision to 55.39 meters. The
exact grid and 317,297-valid / 124,033-no-data mask do not change. This is
authored source geometry rather than renderer sharpening and remains a
candidate until a new scene admission and capture bind the bytes.

`SCENE@18` and accepted scene-admission result
`blake3:a5d4d21586751260b7600d6edce92be932c8bcc9d4b1ca476ec577ecea038ecd`
now hard-cut revision 12 through admission
`blake3:5db0ccd4afbbbf2101abb351f5aac8c3df88597fb5d5a38335847b5586b7acb8`.
Its 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:021199353b50cca2ce57ff09cc2370f4ff5a0d5f74ed2d8f093fd24a262262e4`
with 88 tiles, 148,331 line segments, two areas, and zero tile seams, relief
seams, partition mismatches, or unsupported triangles. The cold Landscape
hierarchy miss took 15.30 seconds. The source pass visibly sharpens local
terrain, but inspection also shows that pre-drainage convex/concave separation
amplifies short-wavelength random texture rather than organizing the surface
around drainage divides. It therefore does not close the major geomorphic
judgment. The next source revision must let a validity-contained fluvial model
form valleys first and sharpen that resulting landform, while reducing
unstructured fine-noise amplitude.

Candidate `rey.agent-geography.rey-county@13` implements that ordering. It
reduces the global fine-noise and ridge amplitudes, derives a stronger
slope-conditioned drainage field first, and only then separates the resulting
validity-contained divides and valleys. The retained source derivation has
26,377 channel cells, 474 heads, 384 junctions, maximum Strahler order 5, and a
78.07-meter maximum incision. Post-fluvial shaping changes only the 311,343
cells with fully valid five-by-five support and remains bounded to +48.98 /
-57.85 meters. The 317,297-valid / 124,033-no-data mask is unchanged. This
candidate must still cross scene admission and visual capture before it can
replace the major assessment.

`SCENE@19` and accepted scene-admission result
`blake3:98728f184f1d473f803abdd181d389c8ca35de68fb8c593040c0a0b6f85ea532`
hard-cut revision 13 through admission
`blake3:9df6fea04b8276505cbcb499b52fb55daf7f057fd8292f25ef51b3d4b0f46edb`.
Its 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:a74ccd0e77bc118fbb9c306e8a4c89d0c4d1dd85b314a2eb34b57bbd16024522`
with 88 tiles, 153,062 line segments, two areas, and zero validity or seam
failures. Landscape reused the prewarmed hierarchy in a 547.8-millisecond
terrain update. The frame nevertheless exposes vertical D8/priority-flood
tree scars across the central basin. This is a major regression despite the
valid lifecycle and means the fluvial ordering alone is not acceptable.

Candidate `rey.agent-geography.rey-county@14` makes the missing semantic
distinction explicit. Priority flood may retain an escape parent to prove
bounded topology, but height displacement is admitted only for the 221,973
cells with a genuine downhill receiver; the retained count for flat-escape
incision is exactly zero. No source sample or validity class changes. A fresh
scene and capture must prove that the visible tree scars are gone.

`SCENE@20` and accepted scene-admission result
`blake3:951636433b8f6fb845f7acb3d656046ef8404ba789405ff6744fbb9871ee704d`
hard-cut revision 14 through admission
`blake3:83947535f66ce31222b7e067a94969152437bbbe46c8460c97ba8a8c1baa2a61`.
Its 1920×1080 WebGPU steep-relief voyage passed in manifest
`sha256:92fbade9e245e11d1e03f0218627f949370c4342f07d5fc73fb83bae916eeef7`
with 88 tiles, 153,321 line segments, two areas, and zero validity or seam
failures. The vertical tree bands remain. Because source revision 14 records
zero flat-escape height displacements, this falsifies the source-elevation
hypothesis. Inspection of `rey.terrain.regional-geography@7` identifies the
remaining mechanism: browser land cover consumes accumulation propagated
directly through its priority-flood traversal tree. That derived presentation
channel, not the admitted DEM, is making escape topology visible. The next
smallest engine commit must hard-cut accumulation to genuine downhill flow and
leave sinks as sinks.

`rey.terrain.regional-geography@8` and
`rey.terrain.regional-linework@5` make that hard cut. The browser now chooses
only the steepest neighbor with a positive admitted-elevation drop, propagates
rainfall in exact descending source-height order, and leaves local minima with
no receiver. Flow direction, accumulation, erosion potential, land cover,
linework, worker cache identity, and diagnostics all bind the new revision.
A deterministic flat-field fixture proves every flow vector remains zero and
the derived authority discloses retained sinks. This is enabling engine work
until a new exact v14 capture demonstrates removal of the presentation scars.

The freshly rebuilt browser bundle passed the matching 1920×1080 WebGPU
steep-relief voyage in manifest
`sha256:d4201b52a27f50391a0875512cb5bb55015279af0977a4a8b2c2c53cae08da96`.
It retained 88 tiles, 161,762 line segments, a 553.3-millisecond hierarchy-cache
hit, and zero validity leaks or relief seams. The accumulation-dependent color
read changed, proving that revision 8 executed, but the grid-aligned vertical
cliff silhouettes remain. The source-side flat-escape displacement and the
browser-side escape-flow presentation hypotheses are therefore both
falsified. The remaining attribution to the admitted v14 field's aggressive
D8-conditioned incision and post-fluvial shaping is an inference from those
eliminations, not a new observation. This remains a major fidelity result; do
not resume scalar tuning against this synthetic source. The next accepted
terrain-source slice must provide materially stronger admitted geomorphic
resolution, while the next engine slice must exercise the already-frozen
multi-region contracts with an actual connected, conflict-free source set.

Candidate `rey.agent-geography.rey-county@15` adds one exact, grid-aligned
eastern boundary segment without changing the rectangular source lattice.
Candidate `rey.agent-geography.rey-eastern-uplands@1` independently authors a
193×168 neighboring field while copying only the 168 shared validity,
centimeter-elevation, and material samples. A deterministic source fixture
reports zero conflicts across all three seam channels and retains 27,432 valid
and 4,992 no-data neighbor vertices. Neither candidate is runtime evidence yet:
both exact packages must pass scene admission, the server-owned composition
must retain their qualified connected component, and a browser capture must
prove a two-patch mosaic before the multi-region delivery item can close.

Those admissions now retain composition
`blake3:7094aba7492a099c4ffbe9ad774e7ffafab8ea7f0b2b44e78f60295516f86af9`
as `READY`: two packages, one 168-sample terrain-qualified seam, one connected
component, and zero conflicts. The first fulfilled-transport engine attempt
compiled the exact 897×626 mosaic with 352,003 valid and 121,583 no-data
vertices, but its roughly 92 MiB materialized hierarchy exceeded the 80 MiB
cache and was re-derived during Landscape entry. Candidate
`rey.terrain.compilation-worker@14` raises only that exact hierarchy cache to
the existing 112 MiB compilation-output bound; the 48 MiB CPU and 64 MiB GPU
tile-residency budgets do not change. A completed capture is still required.

The cold two-region proof then exposed three independent contracts rather than
one cache defect. `rey.terrain.relief-hierarchy@3` uses bounded 256-interval
halo partitions to reduce redundant derivation work while retaining exact
whole-field/partition and border equality. The resulting hierarchy plus
selected Landscape tiles is 153,877,859 bytes, so
`rey.terrain.compilation-worker@17` gives transient compilation output a
separate 160 MiB ceiling while leaving the exact hierarchy cache at 112 MiB
and resident tiles at 48 MiB CPU / 64 MiB GPU. Budget-aware selection now
chooses the finest uniform materialized level inside those residency bounds
and discloses any excess screen error. `rey.landscape-relief-pyramid.v2` and
`rey.landscape-pyramid-envelope.v2` retain every level's complete relief-field
identity, allowing that exact coarser level to be verified as a legitimate
sampled consumer instead of recognizing only the finest field.

This schema change is a hard cut through
`rey.scene-admission.validate@4` / `rey.scene-admission-result.v4`; pre-v4
workload, atlas, and regional admission state is removed before verification.
Fresh `SCENE@22` Rey County result
`blake3:4f75c206aaa09a4cec31618178dcb7f87c0ab68b5c8cb165461ce5745dd7d832`
and fresh `SCENE@2` Eastern Uplands result
`blake3:8dcedded17c429633ed6ac568f96fd612fe4411e5863914ef3e71f2203967c03`
produce READY composition
`blake3:1500034418656c2857c539b0fbb9e3b87753f09360873f2d8e9cc4e5d0381dc4`:
two members, one connected component, one 168/168 valid terrain-qualified
seam, and zero elevation, material, or composition conflicts. Fulfilled WebGPU
voyages `sha256:112a3fceb5fd3eeb10321d389776189c5ebac9e0c29748ef4fa29f947c6106d4`
and `sha256:920a139fd464b0382e9ff363ada8ad72df6589aee619be9f686f95303573d10b`
did not reach Landscape: the pre-terrain World↔Atlas sampler observed only one
reverse dissolve frame amid roughly 690 ms presentation gaps. That transition
stall remains open, and no multi-region browser-capture or fidelity checkbox
advances from these attempts.

`rey.semantic-globe.tsl-stippled-atmosphere@39` addresses that pre-terrain
submission bottleneck without changing either endpoint or sample authority.
It orders the same coordinate-identified stipples into a deterministic,
spatially distributed progressive sequence, retains the complete fabric at
stable World and Mercator, and eases moving frames down to a bounded 32
percent. Repeat charts apply the same fraction only after their connected-seam
prefix is selected. The live fraction is retained in browser transition
diagnostics; forward and reverse WebGPU qualification must still demonstrate
enough distinct dissolve/depth frames before the stall is considered closed.

The exact-revision replacement voyage passed as
`sha256:4e5bab7db47db1d98aa2fe58701531d3e86b07edeb6abd533715de4efec63c17`.
World ↔ Atlas retained ten distinct intermediate projection frames, five
entering and five exiting repeat-dissolve/depth frames, and a 119.4 ms maximum
sampled transition gap instead of the prior roughly 690 ms stall. The complete
fulfilled-transport WebGPU journey then reached Evidence and proved stable
mosaic, composition, primary-patch, height-hierarchy, relief-pyramid, terrain
source, validity, and content-derived Atlas-fabric identities across forward,
reverse, interrupted, re-entry, and same-document back-navigation samples,
with no empty terrain frame. Its 1920×1080 steep-relief Landscape capture is
`sha256:392df80283ec9aa15d6d1381f61127629d11da6ef09004e2ca4f49a942914ce7`:
12 level-9 tiles, zero screen error, zero tile, relief-seam, partition, and
no-data-leak mismatches, 62,172,000 resident CPU bytes, and 46,851,876 resident
GPU bytes. This closes the projection-stall and WebGPU handoff mechanism; it
does not close direct transport, the remaining matrix rows, visual fidelity,
or performance. The 193,289 ms voyage and 516.6/1,583.3 ms Landscape median/p95
presentation cadence remain failures, while the capture still exhibits broad
source-scale bands, muddy low-contrast County form, and oversmoothed Uplands
landforms relative to the operator reference.

Candidate `rey.agent-geography.rey-county@16` next removes single-receiver D8
incision as height authority. Priority flood still supplies a bounded escape
surface, but accumulation is distributed across every downhill neighbor with
hydraulic-slope weights, actual unfilled source-height slope owns displacement,
and a dominant receiver remains only for channel-order reporting. Centerline,
inner-valley, and outer-valley profiles are smoothed independently inside
validity before the existing source-scale divide/valley separation. Across
321,748 fully supported source neighborhoods, mean absolute second difference
falls from 3.941 m to 2.864 m horizontally, 3.977 m to 3.510 m vertically, and
6.060 m to 5.138 m on the first diagonal, while 528.85 m local relief remains
25.74 m at p50 and 53.07 m at p90. Those measurements show that the grid-scale
scar signal changed without flattening the named relief; they are not a
perceptual acceptance result. The County validity boundary remains unchanged,
and companion `rey.agent-geography.rey-eastern-uplands@4` copies the new exact
168-sample seam and first interior slope before the same bounded low-pass
transition. County `SCENE@23` was accepted by the hard-cut v4 admission as
result
`blake3:d6ccc796ca569ee4d65b5d30de71ffde01cfefa4c2dccf8081da5304744ded7d`;
its exact 705×626 source retains 324,739 valid and 116,591 no-data vertices.
Eastern Uplands `SCENE@5` was accepted as result
`blake3:14c84ad5dc114a3a1e260f1d0cc53984fde833c710fa8c9aeb9173bd8fafe75b`;
its exact 193×168 source retains 27,432 valid and 4,992 no-data vertices.
Both human CLI runs expose the current v4 workload, package, snapshot,
projection, validity, omission, and source-resolution lineage. Server-owned
composition
`blake3:e95a7638f1362a4b7bc0541a0d569d7d145230fddc812ca8956c459972ae34f8`
is READY over those exact admissions. Its qualified seam
`blake3:e9eec322cfc288ec46b15223f60b161639115beda1a27ae1c4c41876cab347a1`
compares all 168 shared vertices with zero elevation, material, validity,
coordinate-gap, or no-data conflicts. Accelerated pixels remain required
before the source correction is perceptually accepted.

The exact fulfilled-transport WebGL2 voyage passed as
`sha256:d9c02eb2db9407c660608eb3cda232049d0344896f01003a21b5d8c765060a63`.
Its Landscape capture
`sha256:32027992f8c152472fff639b99a881792d4ec0c4d685e11275160fdad357f579`
retains all 12 level-9 tiles, zero screen error, zero relief-partition,
relief-seam, tile-seam, and no-data-leak mismatches, 62,172,000 resident CPU
bytes, and 46,851,876 resident GPU bytes. Side-by-side inspection against the
preceding County v15 capture accepts the distributed-flow source correction:
the isolated vertical single-receiver incision scars are absent and central
drainage reads as a connected distributed surface. It does not accept the
overall fidelity gate. County still has broad low-contrast, source-scale banding
and Eastern Uplands still reads as oversized smooth forms rather than the
multi-scale ridges and valleys in the operator reference. The 207,457 ms
voyage and 600/1,483.2 ms Landscape median/p95 presentation cadence also remain
explicit performance failures.

Candidate `rey.agent-geography.rey-eastern-uplands@5` replaces the neighbor's
three broad sinusoidal folds with an explicitly retained authored source
network: eight ridge/spur segments, nine branching-valley segments, and four
bounded source-scale noise octaves. Independent displacement still enters only
after the 24-column seam trend corridor through a 36-column zero-slope
envelope, so all 168 exact County boundary samples and first interior slopes
remain unchanged. The 4-cell local-relief distribution across supported
interior samples moves from 48.83/97.57 m at p50/p90 to 68.85/218.91 m, while
the source elevation span moves from 416.62 m to 827.72 m without changing the
27,432 valid / 4,992 no-data mask. Eastern Uplands `SCENE@6` was accepted by
the current v4 admission as result
`blake3:ea16999158ef8b573770323f3996e5846827443cc31f7e7e0f325da703acca3a`.
Server-owned composition
`blake3:0668ad86dbda781010669df118c78e0cee49e6c1c9ddcb18821e8c8b1302b86c`
is READY with all 168 vertices qualified and zero elevation, material,
validity, coordinate-gap, or no-data conflicts at seam
`blake3:28a0d26ffb37d71786d91638620c3e00e94375e4bd583ea1de1d348a33fb6548`.
These remain source/admission measurements, not perceptual acceptance;
accelerated pixels are still required.

The corresponding WebGL2 voyage passed its structural workload as
`sha256:c5005f0cb560f92cbe7eb5dc36d91bca142fc11788bc06bcfffb149d2f70746a`,
with zero partition, relief-seam, tile-seam, and no-data-leak mismatches across
all 12 level-9 tiles. Its Landscape capture
`sha256:3ca965974db2402ec4d1df62e83bd89ca97b5508f96ffe05b7726d453270485a`
rejects the v5 source: the straight segment-distance ridges remain visibly
literal as large crossing strokes even after the shared cartographic relief
operators. This demonstrates that increased local-relief amplitude is not a
sufficient geographic-form fixture. The next source revision must remove
linear segment distance as height authority and test continuous ridge and
drainage form before another admission.

Candidate `rey.agent-geography.rey-eastern-uplands@6` removes every
segment-distance elevation term. Three source-domain warp octaves feed five
continuous ridge and four branching-valley octaves, with a separate bounded
four-octave source-scale channel; the same 24-column seam corridor and
36-column zero-slope entry envelope remain. Its supported 4-cell local relief
is 53.68/94.73 m at p50/p90 rather than v5's 68.85/218.91 m, deliberately
returning amplitude near the v4 source while changing form. Twelve gradient
direction buckets across the independent interior all retain 946–1,940
samples, preventing one or two authored line directions from dominating the
candidate fixture. The generator and 13 County/seam fixtures pass with the
same 27,432 valid / 4,992 no-data mask. Eastern Uplands `SCENE@7` was accepted
as result
`blake3:c5c9efefd219f768009a86ee1e17921653823878277c8b4868939ca14c61151f`;
server-owned composition
`blake3:59e8b65ed944833043120e9639eb8185b40324003543f730b95126ec1bf254c0`
is READY with all 168 vertices and zero elevation, material, validity,
coordinate-gap, and no-data conflicts at seam
`blake3:c9cbc46bcf9e656351832aef24304768a1f724e1a227cdf3b6628f671dc3a5dc`.
Accelerated pixel checks remain required before acceptance.

The exact v6 WebGL2 voyage passed structurally as
`sha256:18ad395feaaeeca4820e75d312e49f60ecd2641bdb42caabe0d303cec5d84e0a`.
Landscape capture
`sha256:a9b56292c22a9a72bb944132b03533ae57971664496cd99d4aa20d21f32c8343`
removes v5's literal crossing strokes and retains zero partition,
relief-seam, tile-seam, and no-data-leak mismatches across all 12 level-9
tiles. It does not accept v6: the separate 36-column independent-relief ramp
ends as a visible vertical detail front inside Uplands. The source needs one
continuous full-width zero-slope envelope rather than a second transition
boundary. The 208,655 ms voyage and 566.6/1,500 ms Landscape median/p95
presentation cadence remain failures.

Candidate `rey.agent-geography.rey-eastern-uplands@7` removes the separate
zero-then-ramp envelope. The same continuous domain-warped relief now enters
from the exact seam through one sine-squared smootherstep envelope over 120
columns, with zero first derivative at the boundary and no interior switch-on
coordinate. Mean row second difference falls from 1.393 m at the exact first
interior slope to 0.140 m at corridor column 24 before independently authored
form grows continuously; supported interior 4-cell relief remains bounded at
52.34/90.53 m p50/p90. All 13 County/seam fixtures and the byte-stable
generator pass over the unchanged 27,432 valid / 4,992 no-data mask. Eastern
Uplands `SCENE@8` was accepted as result
`blake3:ad9375ed4c01aae2144700d76dfb6d3da99780b9075c4edfd8a1eb31fa887f9a`.
Server-owned composition
`blake3:e7cd51691bce338d5bb070d6d15e9ef41810631f429955cd990d70e5ca1d670a`
is READY with all 168 vertices and zero elevation, material, validity,
coordinate-gap, and no-data conflicts at seam
`blake3:bc0453a3d7ecd21c91b0e9e0cfabeb9510821b6e683303f315519680339c375c`.
Accelerated pixels remain required.

The exact v7 WebGL2 voyage passed structurally as
`sha256:22fef15c205bf28314b9704d8c30a8b53cf137cf803c008d90ed5453f47e981a`.
Landscape capture
`sha256:a6788d52971a95e7437b21c88b3b168ece4ee61ca42a549e418ac9d5e0683eae`
retains all 12 level-9 tiles and zero partition, relief-seam, tile-seam, and
no-data-leak mismatches. Side-by-side with v6, independent detail now grows
progressively instead of appearing after a binary zero-to-full switch-on; this
accepts the internal detail-front correction. It does not accept Uplands or
overall fidelity. Relief remains concentrated toward the eastern validity
edge and reads as procedurally mottled rather than as coherent source-scale
drainage; County remains low-contrast and broadly banded. The 208,449 ms
voyage and 783.2/1,516.6 ms Landscape median/p95 presentation cadence remain
failures.

Candidate `rey.agent-geography.rey-eastern-uplands@8` addresses that exact
source-scale drainage gap before any renderer tuning. It runs the same bounded
priority-flood and slope-weighted multiple-flow-direction mechanism proven in
County over the Uplands' explicit validity, but owns a separately revisioned
`rey.uplands-source-drainage.v1` result. Accumulation distributes through
101,441 downhill receiver edges at 25,518 multi-receiver vertices; only actual
source-height slopes may displace terrain, so flood-parent escape topology
still contributes zero incision. The smootherstep source envelope protects the
first 24 seam columns and admits incision over the following 96 columns,
reaching at most 35.74 m across 1,737 derived channel vertices. The candidate
retains the 27,432 valid / 4,992 no-data mask and exact County seam. This is
covered by 14 passing County/Uplands source fixtures and the byte-stable
generator. Eastern Uplands `SCENE@9` was accepted as result
`blake3:4fb78b312da548563a8cadf16b3bcd275daefc747ca27d2d70f2d27f87c39e34`.
Server-owned composition
`blake3:9d422f2d21d8f1abc1eb1d26c099027f8773d13954ce04c92b18fb6cef08e2c4`
is READY with all 168 vertices and zero elevation, material, validity,
coordinate-gap, and no-data conflicts at seam
`blake3:bf2e7afa06aced901c0d534cc9292f7e926ff6ad2987ac36fab23a6d3416f5e8`.
This remains source-controlled enabling work until an accelerated capture
binds and perceptually assesses the exact admitted bytes.

The exact v8 WebGL2 voyage passed structurally as
`sha256:492d9919ab08f1d3e4170a12cd00f8921277f4637420d54221372407cc1a6c85`.
Landscape capture
`sha256:76830b0110de6f559ea6006e22d5fa4d8a300f5237a5c3cc546a5d32390685ad`
retains all 12 level-9 tiles, zero screen error, and zero partition,
relief-seam, tile-seam, and no-data-leak mismatches. Side-by-side with v7, the
admitted drainage slightly deepens branching form without introducing linear
D8 scars or changing the qualified seam. This accepts the bounded source
drainage mechanism only. It does not accept Uplands or overall fidelity: the
relief is still concentrated toward the eastern validity edge and its broad
independent-relief entry still reads as a tonal zone rather than one regional
landform; County remains low-contrast and broadly banded. The 203,964 ms voyage
and 1,133.2/2,233.2 ms Landscape median/p95 presentation cadence remain
explicit failures.

Candidate `rey.agent-geography.rey-eastern-uplands@9` follows that visual
finding by replacing the single relief ramp and zero-height eastern edge
envelope with separately disclosed 48-column local and 120-column regional C2
smootherstep entries, weighted 0.32/0.68, plus a bounded 0.58 sine-squared edge
floor. The exact seam displacement and first derivative remain zero, while
supported 4-cell relief across columns 20–59 rises from v8's 16.18/36.33 m
p50/p90 to 31.10/60.17 m and terminal columns 140–179 rise from 35.05/72.94 m
to 65.43/123.08 m. This removes the source's artificial flat terminal strip
without expanding its unchanged 27,432 valid / 4,992 no-data support. The
terrain-derived drainage recomputes over the changed source at 24.64 m maximum
incision and still grants flood-parent escape topology zero displacement. All
14 County/Uplands source fixtures and the byte-stable generator pass. Eastern
Uplands `SCENE@10` was accepted as result
`blake3:748f70492535eb2c226e17249f525d48ddbb3cc348d4017f9c35a83f1ac7670f`.
Server-owned composition
`blake3:6fbeff0898c53e922f8c05c306ecb676919969913d030d752b75c24efd5d454e`
is READY with all 168 vertices and zero elevation, material, validity,
coordinate-gap, and no-data conflicts at seam
`blake3:2cc9bd1ac71ae8a8b001975c1fcb18353bc8e348b095f95aaac0791842a1daf8`.
An accelerated capture must still decide whether the broader form removes the
observed tonal zone without introducing another visible front.

The exact v9 WebGL2 voyage passed structurally as
`sha256:6290d2d0b26cc1b87bcf4d812fc8e8450b94d46b8fb20c0dec6216d34b1a25cf`.
Landscape capture
`sha256:5655514af8137dd27f99f0f91d024904d2cef7e9d10bbcd547a30fa73cc8ebea`
retains all 12 level-9 tiles, zero screen error, and zero partition,
relief-seam, tile-seam, and no-data-leak mismatches. Side-by-side with v8,
source relief now grows through the western half of Uplands and remains present
at its unsupported eastern boundary rather than appearing as one late tonal
zone and collapsing into a flat strip. This accepts the relief-distribution
correction. It does not accept Uplands or overall fidelity: the admitted form
still reads as procedural, similarly sized ridges rather than a hierarchy of
coherent drainage basins, and County remains low-contrast and broadly banded.
The 210,502 ms voyage and 583.3/1,499.9 ms Landscape median/p95 presentation
cadence remain explicit failures.

The exact v9 `steep-relief@1920x1080` matrix row is now retained across
reference
`sha256:9ede0d7927f9661df3eb53fdf77e835320a37bee088ba3fffe98c7a993240b57`,
WebGL2
`sha256:6290d2d0b26cc1b87bcf4d812fc8e8450b94d46b8fb20c0dec6216d34b1a25cf`,
and WebGPU
`sha256:66e4052f12a42abd71f20faf81cc1192769d04a1a1aa60e10801fd3cca5821a1`.
Rendered parity
`sha256:0638f2555e9765cb10696086fddc3e957af3a8e63d1cc572bab46f19513e7ea8`
passes semantic equality and bounds accelerated maximum normalized RMSE to
0.0130586 under the 0.02 limit. Perceptual assessment
`sha256:ed736384a60dcafc18419df57f5abfeb213a7daa2f4b7689733123cb0e30376c`
binds the operator-supplied comparison as
`sha256:2888cb6bf3b18b639cc0d1130cdfecda5dc579242dd95040ef55dff5d2a4cac1`
and remains explicitly INCOMPLETE at 1/14 rows: one pass, six minor results,
and five major results for multi-scale MDOW read, SVF valley depth,
hypsometric land-cover separation, flat scalar mud, and plastic smoothing.
All three voyages used fulfilled retained-document transport, so this advances
neither direct-transport proof nor the remaining workload/viewport rows.

Reference voyage
`sha256:fb22c58cc67c239eded46b7353de3d3fab6ea27ad060a748c084f0ef14679d8f`
then completed the full World → Atlas → Landscape → Object → Evidence
traversal over the exact two-region mosaic. It retained zero partition, relief,
tile-seam, and no-data-leak mismatches, but did not qualify: the 48 MiB CPU
resident ceiling forced uniform hierarchy level 8, 15,543,000 CPU bytes, and
244.376 screen-error pixels. The next finer exact level is approximately four
times that CPU footprint and remains below 64 MiB. The named local ceiling,
`rey.terrain.tile-residency@3`, and `rey.terrain.compilation-worker@18`
therefore raise only resident CPU capacity to 64 MiB, matching the existing
GPU ceiling; the 112 MiB exact hierarchy cache and 160 MiB transient output
ceiling do not change. A replacement voyage must still prove the 1.5-pixel
fidelity requirement before any capture or matrix item closes.

That replacement fulfilled-transport reference voyage passed as
`sha256:17b8b1a1524ccd4f631649401f90b642e3dc6d42845a322a71ed8292a63a8e22`.
The exact two-region mosaic selected all 12 level-9 tiles at 62,172,000 CPU
bytes and 46,851,876 GPU bytes, reported zero screen error, and retained zero
partition, relief-seam, tile-seam, and no-data-leak mismatches through the full
World → Atlas → Landscape → Object → Evidence traversal. Its Landscape
capture is
`sha256:d65306e2a86dcd67f71921934b63094c608c1b050a26dc593304bd25e621ca81`.
This closes the connected conflict-free multi-region compiler/admission item,
not the backend/viewport matrix: the 199,538 ms fulfilled voyage and
149.9/733.3 ms median/p95 presentation cadence also remain explicit
performance failures, and accelerated pixels plus operator perceptual
judgments are still required.

The matching fulfilled-transport WebGL2 voyage passed as
`sha256:b5e08ba0f0977a9f579dcf2acbcbf7207f9e90bb47a63ffb070e0a785a9e8228`
with Landscape capture
`sha256:07c722cc12b38ec5fba270a573d0569a4a464a2dfe3dcb68dd0f655da1c2ce34`.
It retained the same exact mosaic, 12 level-9 tiles, zero screen error, and
zero partition, relief-seam, tile-seam, and no-data-leak mismatches. The
operator-supplied cropped Google Maps terrain reference is bound for this
assessment as
`sha256:3a756185922d5293240570f4e0148288636e85f0d40d92d50a421f1dcd0a1635`;
it is an acceptance reference only, not scene evidence or a redistributable
asset. Side-by-side inspection keeps major judgments open for multi-scale
form, hillshade continuity, valley/SVF depth, chromatic lighting, hypsometric
coherence, flat scalar mud, plastic smoothing, and the visible regional edge.
Terrain-bound hydrology is materially clearer and remains a minor gap; contour
and vector hierarchy remain subordinate but incomplete. The accelerated
backend therefore proves that full-detail LOD alone does not close fidelity.
Its 211,260 ms voyage and 750/1,783.2 ms Landscape median/p95 presentation
cadence also fail the retained performance bar.

The accelerated capture's exact eastern edge was not a renderer-tile seam.
Source inspection found a C⁰-only package join: all 168 boundary elevations
matched, but the one-sided first derivative differed by 3.36 m at p50,
10.64 m at p99, and 10.97 m at maximum over one roughly 131 m cell; 46 first
interior material samples also changed immediately. Candidate
`rey.agent-geography.rey-eastern-uplands@2` retains one exact County interior
context column, continues its edge slope through the first Uplands column,
and introduces independent relief through a zero-slope envelope. Its source
fixture now reports zero elevation-gradient and material discontinuities at
all 168 seam samples. This is a source-contract correction rather than a
renderer feather; a fresh scene admission and accelerated capture are still
required before the visible-edge major judgment can change.

Fresh Eastern Uplands `SCENE@3` admission result
`blake3:977e56a1b03fbd84607985cdda60902f6989b6c1c4b583b0898166d35aa986ee`
produced READY regional composition
`blake3:e462743f3526922c7bfcc99bf4babef7c3d36f5a4f415438c8fb1208b6ed04ff`.
Its WebGL2 voyage passed structurally as
`sha256:e07c76af678add0ea9fe072ac8bf6d76b6212da7a746a4fba03dba8ec5623c98`,
but capture
`sha256:9089000fff2efda1f5f13b67c49b5a122802d5d8a8534050c3cb09e8e3ec4c0f`
showed that carrying each exact row slope deep into the neighbor created
unacceptable horizontal bands. Candidate
`rey.agent-geography.rey-eastern-uplands@3` confines exact C¹ matching to the
first interior column, then transitions over 24 columns into an eight-row
low-pass boundary height/slope trend before independent landforms take over.
The mean row second difference at corridor column 8 falls from 4.845 m to
1.251 m and reaches 0.170 m at column 24 while all 168 seam derivative and
material differences remain zero. Fresh Eastern Uplands `SCENE@4` admission
result
`blake3:64aad48572f5d6254b3f6bddcee0fe9366a0fcb4d7f441eba1975b845ac68362`
produced READY regional composition
`blake3:7d7a5b0f9e21f3da21c4f1513b263bfb5d435cee184d3ef949b16210a6964d73`.
Its fulfilled-transport WebGL2 voyage passed structurally as
`sha256:3ffe1440e582d4b9e5c784bcc467dae3dfd86e22618cea3295bd09c3270927f4`;
Landscape capture
`sha256:a6f578ecc43115f469cc493f5ed70c0cceebaa08a64be3928aa63c361b25bf08`
removes the rejected v2 horizontal bands while retaining zero screen error,
zero relief-partition, relief-seam, tile-seam, and no-data-leak mismatches.
The thin shared edge remains an explicit admitted regional boundary vector,
not a height seam. The neighbor's broad low-detail forms and the County's
source-scale bands remain major fidelity gaps, so this accepts only the C1
source-continuity correction and does not advance the overall perceptual gate.

The Mercator entry hard cut now follows the continuous-lens contract rather
than requiring a regional card selection. `3d81884`, `3901bae`, and `d7761ae`
derive the Landscape focus from the connected Atlas member as wheel scale
crosses the boundary, retain one predicted source/mosaic submission, and keep
one dedicated terrain worker alive across prewarm, movement, reversal, and
settled refinement. Moving frames use a half-resolution level-6 ceiling and a
fixed predicted compilation view (9,979 triangles in the named voyage), while
the resulting bounded mesh is presented through the same live pan, scale,
orbit, and model transform as the County footprint on every morph frame. After
a 300 ms settled delay the exact level-9 working set replaces it with zero
screen error. Main-thread regional contour and full-pyramid derivation no
longer run during accelerated movement.

`rey.terrain.compilation-worker@20` now projects the exact finest admitted
relief samples into the Atlas fabric with retained source sample, field,
hierarchy, relief, salience, tangent, brightness, and reveal-order identity.
The accelerated morph paints 16 bounded prefixes into one lifetime-stable
canvas and changes only compositor opacity on intervening wheel frames; the
reference path retains its deterministic SVG projection. Fulfilled-transport
WebGL2 voyage
`sha256:386b608a60af9f1da47b0d0af5b693f6902199c6f8137987a98f3d6697ff8d1a`
entered Landscape with zero regional clicks and retained stable mosaic,
terrain-source, validity, height-hierarchy, relief-pyramid, and fabric
identities through forward, reverse, interrupted, re-entry, and same-document
back-navigation samples, with no empty terrain frame. Its maximum sampled
wheel-frame gap was 1,121.8 ms under the disclosed 2,000 ms fulfilled
SwiftShader tolerance. This is continuity evidence, not hardware frame-rate
proof: the 250 ms direct-transport budget and the full retained performance
matrix remain open.

`d8868bc` hard-cuts the shared Atlas/Landscape projection contract to
`rey.atlas-landscape-projector@3`. The County footprint's CSS matrix is now
recovered from the same terrain point projector used by the accelerated scene,
and the accelerated surface no longer freezes its presentation at the
zero-pan endpoint while pointer-anchored zoom moves the footprint. The exact
source/mosaic compilation remains fixed; only the bounded moving mesh camera
submission changes. Fulfilled-transport WebGL2 voyage
`sha256:7993841f6590edcad25b0dc9a4cd47e4183ca0a83cfd9a4a17c68d0e734bc72c`
entered Landscape off-center with zero regional clicks and retained identical
border/terrain model transforms and pan values at every sampled intermediate
frame. Pan moved from `0,0` to `-351.77,75.38` while the moving set remained
level 6 at 9,979 triangles; the maximum sampled wheel-frame gap was 1,798.2 ms
under the disclosed 2,000 ms fulfilled SwiftShader tolerance. Landscape
capture
`sha256:f6e2caa27324aef3bfde6cb39a3bb800c8c0a1df59e2f7f1fbdec353a1adad98`
binds the settled result. This qualifies the shared projection/camera
connection under fulfilled transport, not direct hardware frame rate.

The first responsiveness deep dive uses that exact voyage as its
uninstrumented baseline. Despite dedicated-worker execution, each camera
successor still structured-cloned the full immutable DEM source into the
worker and returned about 92 MiB of materialized height/relief hierarchy that
the page never read. The baseline retained a 1,798.2 ms maximum moving-frame
gap, 559 ms worker update, 333 ms tile projection, 223 ms geometry compilation,
and 624 ms render submission for the named Landscape handoff. Full-voyage CPU
sampling was rejected as qualification evidence because its observer overhead
stalled the SwiftShader run; the retained uninstrumented manifest remains
authoritative. `rey.terrain.compilation-worker@21` hard-cuts the transport
contract: the first exact source identity is registered once in the long-lived
worker, compatible view successors omit source fields, complete hierarchies
remain worker-local, and only the active working set plus compact lineage
summaries crosses back by transferable `ArrayBuffer` ownership. Browser
diagnostics disclose source and result payload kinds, transferred
buffers/bytes, and retained hierarchy bytes. This closes the avoidable
hierarchy-clone mechanism, not the open direct-hardware frame-rate gate; the
next named performance slice bounds main-thread cartographic draping to the
feature's terrain support before rerunning the same voyage.

`rey.explorer.terrain-render-passes@7` implements that follow-up without
changing validity or source authority. Exact polygon bounds now select the
smallest intersecting cell window and reuse the canonical validity-aware
terrain diagonal rule; the pass set retains complete-field, candidate-cell,
candidate-triangle, and transient-time diagnostics. The accelerated surface
also retains an unchanged world-coordinate pass set across camera-only worker
successors by derived-line source/presentation identity. Focused fixtures prove
that concatenated bounded cell windows are index-identical to complete-field
triangulation and that exact clipped water edges remain unchanged. Fresh
voyage evidence is still required before attributing an interaction-level
improvement or closing the direct-transport performance gate.

The first attempted rerun was stopped rather than retained when the new
diagnostics exposed a remaining 4,134.7 ms main-thread render-pass compile.
The worker cutover itself behaved as intended: a registered-source successor
kept 91,705,859 hierarchy bytes worker-local and transferred 1,726,608 active
bytes. Area bounds reduced the candidate set from 1,120,000 complete-field
cells to 155,124 cells, but exact clipping still visited 307,014 fine-source
triangles—far beyond an interaction-sized cartographic mesh.
`rey.terrain.compilation-worker@22` therefore derives a distinct
level-6-or-coarser cartography support set from the same height/validity
hierarchy. Conservative coarse validity may remove supported detail but cannot
gain support, and exact polygon clipping still uses the canonical diagonal
rule inside that support. The page waits for this bounded worker result rather
than compiling overlays against the full source while prewarm is pending, and
keys reuse by exact cartography tile identity. This is an implemented enabling
slice; a completed retained voyage is still required.

That faster prewarm exposed a readiness race in the next attempted voyage:
the hidden terrain submission completed before the Atlas wheel animation's
last zoom tick, then the idle-timer effect cleared `prepared` even though
neither source nor backend had changed. Preparation invalidation is now scoped
to exact source/backend eligibility while zoom movement only reschedules the
idle mount timer. This keeps a completed predicted entry submission valid and
prevents a performance improvement from stranding the harness/operator at a
false `mounted` state. The stopped run is diagnostic only, not retained proof.

A subsequent cold run reached Landscape in 2,661.8 ms but timed out for five
minutes waiting for the monolithic settled submission: the retained moving
surface remained level 6 with 9,979 triangles. The settled hierarchy projects
many non-overlapping tiles, while `ContinuousReliefScene` previously built an
independent but identical TSL node material for every tile. That multiplied
shader-graph compilation at the exact level-of-detail handoff.
`rey.terrain.material-binding@2` now binds one immutable material graph across
all non-overlapping hierarchy tiles; only legacy uncomposed patch identities
named by an overlap pair retain separate depth-biased materials. A focused R3F
fixture proves shared object identity for disjoint meshes and distinct binding
for overlap. This addresses redundant shader compilation; it does not yet
qualify the full settled handoff.

Repeating the cold workload with shared material identity still saturated
SwiftShader at the 638,262-triangle finest submission while the level-6 result
remained responsive. The engine had coupled exact relief sample density to
geometry density; redundant shader compilation was not the only cause.
At that diagnostic point, `LANDSCAPE_SETTLED_TERRAIN_MAXIMUM_LEVEL` hard-capped
the interactive settled surface at level 7 (one hierarchy step above the
level-6 moving surface). The
reported screen error remains authoritative, so this bounded result is not
described as full detail and cannot pass the open fidelity gate. The required
engine follow-up is explicit: retain high-resolution hillshade, MDOW, SVF,
hypsometry, and material as tiled sampled textures while geometry LOD remains
bounded independently. That separation—not another unbounded triangle
increase—is the route to Google Maps-class fidelity and responsive motion.

The responsiveness follow-through implements that separation as a first
source-raster slice. `9f4a416` adds exact renderer content identity beside the
semantic scene snapshot, so a completed LOD/texture successor invalidates the
R3F scene without corrupting the snapshot identity exposed to the operator.
`3983089` bounds terrain raster resolution to one half of CSS resolution while
preserving camera, model, border, and native-coordinate math. `8dbf033` removes
four-sample hardware MSAA from the multiply overdrawn terrain canvas (the globe
retains it); a later one-pass screen-space AA treatment remains open.

`rey.terrain.compilation-worker@23`,
`rey.terrain.cartographic-texture@1`,
`rey.terrain.tsl-cartographic-relief@7`,
`rey.terrain.material-binding@3`, and
`rey.terrain.cpu-mesh-upload-parity@4` now pack the finest exact admitted
chromatic relief into one RGBA source raster, transfer it by ownership, bind
stable source-normalized UVs to every selected hierarchy tile, and sample one
shared texture/material graph across non-overlapping tiles. The texture is
derived from the already qualified complete-field hillshade, MDOW, SVF,
openness, local contrast, hypsometry, and material result; it does not add
support or source geography. Both moving and settled interactive surfaces now
remain at level 6. Geometry error remains separately authoritative, so the
texture cannot counterfeit a finer height surface.

The unchanged fulfilled-transport WebGL2 continuity gate passed after dynamic
resolution as incomplete voyage
`sha256:8f86daac88e3d3fe7981fabc67a70527a9806bcd0a95aa9bf49dd6f42b28451a`:
the maximum moving-frame gap fell from the 1,798.2 ms baseline to 1,587.3 ms
under the disclosed 2,000 ms SwiftShader tolerance. The first exact textured
run retained incomplete manifest
`sha256:80661351bd9c6fa69a8e4470eda58e52b9330a479df334f09a3de8f4aeb1f123`.
It transferred a 561,522-texel / 2,246,088-byte raster as part of a 4,044,112
byte active working set, retained 91,705,859 hierarchy bytes worker-local,
reported a 220.6 ms worker update, 199.9 ms tile projection, 0.3 ms CPU render
submission, 9,979 triangles, and a 1,570.6 ms maximum moving-frame gap. It no
longer requests the 638,262-triangle settled frame. The voyage remains
correctly incomplete because level-6 geometry reports 950.75 px maximum
screen error against the unchanged 1.5 px fidelity requirement. A retained
full voyage, direct-hardware timing, tiled texture residency/mip selection,
screen-space AA, and adaptive geometry that meets the error bound without a
monolithic submission all remain open.

The 2026-08-27 first-load responsiveness slice separates canvas availability
from the expensive retained portfolio projection. `/explore` now starts that
read beside its renderer chunk and mounts an explicitly evidence-pending World
coordinate scaffold without a provisional region, terrain, count, or coverage
claim. The admitted projection reuses the root operator shell rather than
re-reading it. `rey.explorer.pending-canvas` and
`rey.explorer.scene-ready` provide stable browser marks, and the named
`rey.explorer-first-load-measurement.v1` workload fails if the pending scene
misses its 2,000 ms budget or displays Rey County before admission.

The same slice hard-cuts the regional mosaic compiler to
`rey.terrain.regional-mosaic@10`. Admitted transport has already verified each
regional dataset's BLAKE3 content identity, so the deterministic mosaic now
derives channel, field, and mosaic identities as a Merkle graph over those
exact source digests, composition inputs, spatial frame, authority decisions,
and compiler revision. It no longer re-hashes roughly 34 MiB of derived typed
channels synchronously during first paint. Input order remains normalized;
changed admitted source identity, composition, authority, placement, or
compiler revision still produces a different derived identity.

On the named local fulfilled-transport cold workload, the evidence-neutral
canvas appeared at 543.2 ms while a fresh Rey process spent 6,959.3 ms building
the exact server projection. The admitted scene became ready at 7,978.5 ms,
with 454.8 ms in client compilation: 161.8 ms in regional field generation and
234.7 ms in mosaic composition. Against the pre-cutover warm measurement,
client compilation fell from 3,604.6 ms to 470.6 ms and admitted-scene readiness
from 4,547.6 ms to 1,406.2 ms. These are fulfilled-transport diagnostics, not a
direct-browser or GPU timing claim. The cold server projection remains the
dominant delay to exact evidence and the retained direct-transport performance
gate remains open.

- [x] Add deterministic fixtures for one patch with holes, touching patches,
      partial overlap, nested resolutions, a rejected datum, a gap, an admitted
      overview gap fill, steep relief, low relief, water/coastline, dense
      vectors, stale input, and backend loss.
- [x] Assert no validity gain, whole-field/tile derivation equivalence, border
      digest agreement, deterministic overlap decisions, zero unsupported
      triangles, zero tile-boundary seam mismatches (`relief_seam_mismatches == 0`),
      bounded resident/working bytes, stable picking, and exact revision lineage
      in unit and integration tests.
- [ ] Retain reference, WebGL2, and WebGPU captures at 1920×1080 and 3840×2160
      for every required row. Compare against the untiled derived-field
      reference numerically and against the operator-supplied consumer-map
      reference (e.g. Google Maps Pyrenees DEM) perceptually; assert absence
      of crosshatch grid lines, flat scalar mud, and plastic smoothing.
- [ ] Record composition, multi-scale relief (MDOW), hillshade continuity,
      Sky-View Factor valley depth, dual-tone chromatic lighting, hypsometric
      land-cover coherence, terrain-bound hydrology, contours, and vector
      hierarchy with explicit pass/minor/major results. Any major result leaves
      this plan open.
- [ ] Repeat World → Atlas → Landscape → Object → Evidence through
      direct browser transport on one exact machine and retain source, mosaic,
      pyramid, operator, backend, omission, limit, performance, and capture
      lineage.

The delivery gates are therefore:

1. seam correctness and halo-safe derivation on one admitted region;
2. deterministic validity-safe composition across multiple regions;
3. source-resolution-aware multi-scale MDOW relief and Sky-View Factor occlusion;
4. dual-tone chromatic lighting, continuous hypsometric tinting, and coherent hydrology;
5. one shared Atlas/Landscape field and reversible handoff;
6. cartographic composition and the complete retained qualification matrix.

Do not advance a gate by tuning color around a known seam, hiding unknown
terrain beneath a skirt, choosing an overlap by draw order, or synthesizing
detail in the renderer.

## Incremental Implementation And Commit Plan — 2026-08-21

Advance the remaining checklist in the following dependency order. Each
numbered batch ends in one or more logical commits; a commit must bind one
reviewable contract or end-to-end proof slice, update the owning documentation
and checklist, and pass its focused tests before the next batch begins. Visual
tuning may be captured while an earlier contract is still an enabling
prototype, but it cannot close a later delivery gate.

1. **Land the current seam-safe metric-relief prototype.** Finish the
   renderer-neutral metric-spacing metadata, complete-field relief derivation,
   exact render-tile sampling, sampled-field verification, scale-support
   disclosure, and WebGL2 capture proof. Keep MDOW, SVF, and the pyramid
   contracts open. Commit the contract/tests/docs separately from retained
   qualification artifacts.
2. **Close the shared contract and CLI boundary (8.1).** Define the height and
   relief pyramid schemas, their parent/child and operator-support identities,
   then expose patch, mosaic, source-resolution, pyramid, omission, fallback,
   and renderer-budget summaries through verbose and structured
   `rey workloads run scene-admission`. Hard-cut browser paths only after the
   CLI and renderer consume the same typed contracts.
3. **Close deterministic composition (8.2).** Add evidence-aware overlap
   decisions independent of input order, retain decision/conflict maps, permit
   feathering only inside mutually valid overlap, and fill a gap only from a
   separately admitted compatible overview DEM. Keep material, water,
   contours, and vectors independently attributed. Commit each overlap,
   feather, and overview policy with its own fixtures.
4. **Close halo-safe hierarchy and residency (8.3).** Build conservative
   height/validity levels over the shared mosaic, derive every relief tile from
   a metric source gutter, retain cropped-interior border digests, add derived
   bytes to LOD/residency accounting, and cache only by exact mosaic, level,
   tile, operator, and support identity. Whole-field/partition equivalence and
   zero internal-edge mismatch are required before this gate closes.
5. **Close the cartographic relief operators (8.4).** Implement separately
   revisioned metric slope/aspect channels, deterministic slope-adaptive MDOW,
   SVF/positive-negative openness, high-pass profile/plan curvature and ridge
   salience, and local contrast/tone mapping in linear space. Each operator
   receives steep-, low-relief-, hole-, and too-coarse-source fixtures plus
   reference/WebGL2 parity before being composed with the next operator.
6. **Close cartographic composition (8.5).** Replace scalar grey multiplication
   with warm-direct/cool-ambient chromatic lighting, replace discrete material
   classes with continuous elevation- and slope-graded hypsometry, then
   qualify crisp terrain-bound water, shoreline, contour, route, boundary, and
   label hierarchy. Keep every absent source and visibility boundary explicit.
7. **Close the shared Atlas/Landscape hierarchy (8.6).** Generate Atlas
   stipples and reveal order from the same mosaic/relief-pyramid samples,
   retain native sample identity through the morph, prewarm the predicted exact
   hierarchy, and keep the last compatible submission until its successor is
   ready. Prove forward, reverse, interrupted, and backend-loss transitions.
8. **Close the retained fidelity matrix (8.7).** Run every named fixture at
   1920×1080 and 3840×2160 through reference, WebGL2, and WebGPU on one exact
   machine. Retain numeric seam/parity/budget results and explicit perceptual
   pass/minor/major assessments for MDOW, SVF, chromatic lighting, hypsometry,
   hydrology, contours, vectors, and transition continuity. Any major result
   keeps this plan active and directs the next smallest commit.

## Open Choices

- A qualified Cloud-Optimized GeoTIFF (COG), TileDB, or native raster pyramid
  adapter remains future work; the current GeoJSON grid is the smallest
  CLI-verifiable admission slice, not the long-term bulk-elevation format.
  Resolving sub-100m geomorphic features (knife-edge aretes, talus, cirques)
  requires high-density raster sources (e.g., 10m–30m DEM datasets).
- Synthetic geography compilers may incorporate physical geomorphological
  models (hydraulic fluvial incision, thermal weathering, slope-dependent
  scree accumulation, and tectonic fault lines) rather than relying on
  isotropic noise octaves.
- Tile dimensions, geometric-error metric, worker topology, and camera bounds
  must be selected against named workloads rather than by drive-by dependency.
- Imagery and material inputs require their own provider and license authority;
  the renderer must not infer them from elevation or familiar map styling.

## Non-Goals

This plan does not authorize automatic locator execution, ambient downloads,
unbounded caches, invented terrain outside validity, a general ECS, a plugin
framework, physics, first-person navigation, or a new persistence engine.
