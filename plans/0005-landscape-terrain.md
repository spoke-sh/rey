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
separate 48 MiB CPU and 64 MiB GPU budgets.

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
`SCENE@11` and passed `scene-admission` run
`blake3:db4409678341ea7a5c407056172d58c4be139a64f4c970b2a403847b75dd7f60`
before `/explore` consumption. This improves the default project bearing
without treating terrain controls as observed height or closing the still-open
named fidelity matrix.

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

`rey.terrain.compilation-worker@5` and reference renderer revision 3 now hard-
cut admitted terrain through `rey.landscape-pyramid-envelope.v1`. The envelope
binds exact BLAKE3 height, validity-class, hillshade, salience, and tangent
content to the one-level height/relief contracts before camera tile sampling.
Both pyramids remain explicitly incomplete: current envelopes retain only the
complete finest field, report zero source gutter for every relief operator,
and list absent coarse levels, halos, border digests, MDOW, and SVF as
omissions. This closes the shared renderer contract boundary without claiming
the 8.3 hierarchy.

`rey.landscape-relief-engine@3` extends that prototype with
`rey.terrain-relief-metrics.v1` source-spacing and elevation-range metadata.
It derives local, midslope, and regional target radii in meters, explicitly
marks scales that the admitted grid cannot support, excludes unsupported
scales from composition, and exposes the exact scale basis and support through
renderer diagnostics. Its multi-azimuth light and validity-bounded local tone
are still a prototype: they are not the slope-adaptive MDOW, SVF/openness,
curvature, haloed relief-pyramid, or linear composition contracts required by
8.3–8.5.

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
- [ ] Feed only a connected conflict-free set of terrain-qualified seams into a
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
      once parity is proved. Keep `rey.landscape-relief-field.v3` labeled as an
      enabling field-wide prototype inside the incomplete envelope until 8.3
      replaces it with the haloed hierarchy.
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
- [ ] Keep land cover, water, contours, and vectors as independently
      attributed companions. A height mosaic cannot manufacture their source
      authority.

Acceptance: fixtures for adjacent patches, partial overlap, nested resolutions,
datum conflict, invalid overlap, corner contact, and gaps with and without an
admitted overview source produce deterministic mosaics with no validity gain.

#### 8.3 Build the height pyramid before camera tile materialization

- [ ] Construct the multiresolution height/validity hierarchy over the shared
      mosaic. Downsampling must be conservative at validity boundaries and must
      retain the contributing source set for each parent sample.
- [ ] Give every derivation tile a source gutter at least as wide as the
      largest active relief operator (`gutter_radius >= max(operator_support_radius_meters / sample_spacing_meters)`).
      Derive channels from the halo, crop only the render interior, and retain
      border digests for adjacent-tile proof.
- [x] Make whole-field and partitioned compilation equivalent within one named
      numeric tolerance. Moving the camera or changing the active tile
      partition must not change a sample's height, normal, or illumination.
- [ ] Select pyramid levels with stable screen-space error and compatible
      neighboring support. Preserve current cancellation and CPU/GPU residency
      bounds; add derived-channel bytes and halo work to those measurements.
- [ ] Cache by mosaic revision, pyramid level, tile identity, operator revision,
      and validity support so an Atlas prewarm can be reused without admitting
      stale or differently composed terrain.

Acceptance: an untiled reference field and every legal tiling of it have
identical valid interiors, zero no-data triangle leakage, and no internal-edge
discontinuity or crosshatch banding attributable to kernel truncation.

#### 8.4 Derive cartographic relief at explicit metric scales

- [ ] Derive metric slope, aspect, and normals from source spacing rather than
      cell count. Produce separately revisioned local, midslope, and regional
      channels whose support radii are declared in meters (e.g. local 350m,
      midslope 1,400m, regional 5,600m).
- [ ] Implement deterministic Multi-Directional Oblique Weighted (MDOW) Swiss
      hillshading. Weight illumination across multiple azimuths (NW 315° primary
      sun, SW 225° fill, NE 45° back-rim) with slope-adaptive contrast to prevent
      pitch-black shadows on steep faces and washouts on flat plains.
- [ ] Add a deterministic Sky-View Factor (SVF) / positive-and-negative
      topographic openness term to naturally darken deep gorges, cirques, and
      valleys without artificial scalar multiplier hacks.
- [ ] Blend high-pass profile/plan curvature and slope magnitude into high-frequency
      ridge salience so micro-scale crests, couloirs, and ravines separate
      crisply against macroscopic mountain mass illumination.
- [ ] Apply local contrast and cartographic tone mapping as presentation
      channels in linear color space. Keep one lighting owner so a pre-lit
      relief scalar is not lit again by a physical material.
- [ ] Share exact derived arrays, masks, parameters, and implementation identity
      between the deterministic reference and WebGPU/WebGL2 paths. Renderer
      backends may execute the math differently only under retained parity
      tolerances.
- [ ] Treat finer terrain content as source work. Any synthesized landform must
      be generated, reviewed, and admitted before rendering with explicit
      lineage; shader noise and renderer-side microrelief cannot substitute for
      absent elevation evidence.

Acceptance: named steep- and low-relief fixtures preserve fine ridges without
muddy faceting or plastic smoothing, avoid broad tonal domination, and disclose
when the admitted spacing cannot support the comparison scale.

#### 8.5 Compose a coherent terrain map

- [ ] Implement dual-tone cartographic chromatic lighting: blend warm direct
      sunlit highlights on illuminated slopes with cool, ambient-sky-tinted
      diffuse fill in shadowed aspects, replacing desaturating grayscale scalar
      multiplication (`tint * hillshade * occlusion`).
- [ ] Replace discrete flat palettes with continuous elevation- and slope-graded
      hypsometric color ramps (lush valley greens → warm mid-elevation montane
      grasslands → slate/grey alpine crests) with slope-triggered rock/cliff
      exposure on steep grades.
- [ ] Render terrain-bound water areas with crisp high-contrast polygon fills
      and distinct shorelines (e.g. alpine tarns, glacial lakes, and river
      corridors) draped seamlessly over the relief.
- [ ] Drive contour interval, weight, and opacity from semantic LOD and metric
      elevation range. Contours remain thin, crisp, and subordinate to relief,
      never concealing hillshade defects.
- [ ] Add terrain-bound roads, rail, structures, boundaries, and labels only
      from their exact admitted companions, with independent visibility and
      invalidation revisions.
- [ ] Keep selection and evidence overlays readable without changing the base
      relief assessment. Preserve the deterministic accessible reference path
      under backend loss.

Acceptance: the base map reads first as continuous geography, then as relief,
water, land cover, and semantic detail; no layer gains evidence or coverage
from visual composition.

#### 8.6 Make Atlas and Landscape sample the same terrain hierarchy

- [ ] Generate Atlas stipple position, density, salience, and reveal order from
      the exact mosaic and relief-pyramid samples that Landscape renders. Do not
      run an independent raw-field relief calculation for Atlas.
- [ ] Retain source-native sample identity and deterministic seed/order through
      every transition frame so stipples can expand into the corresponding
      Landscape support instead of dissolving into unrelated geometry.
- [ ] Use the selected Atlas member as the camera/focus anchor while allowing
      qualified neighboring and overview patches to reveal from the common
      mosaic. A primary patch is not the boundary of the visible world.
- [ ] Prewarm the exact mosaic, pyramid levels, relief revision, and material
      revision required by the predicted entry view. Keep the last compatible
      submitted terrain until its successor submits; never flash an empty or
      differently compiled field during handoff.
- [ ] Prove wheel, click, back-navigation, interrupted traversal, and backend
      loss in both directions without a semantic field swap, tile flash, or
      validity expansion.

Acceptance: Atlas ↔ Landscape is one reversible content-derived morph over
one source lineage, including when several regional patches contribute to the
entry viewport.

#### 8.7 Retain the qualification matrix and close the fidelity bar

- [ ] Add deterministic fixtures for one patch with holes, touching patches,
      partial overlap, nested resolutions, a rejected datum, a gap, an admitted
      overview gap fill, steep relief, low relief, water/coastline, dense
      vectors, stale input, and backend loss.
- [ ] Assert no validity gain, whole-field/tile derivation equivalence, border
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
