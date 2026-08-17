# Globe Projection

The accelerated globe is one declarative indexed surface that changes from an
oriented sphere into a full-canvas semantic Mercator map. It is not a swap
between unrelated globe and map scenes.

## Pipeline

```text
ExplorerGlobe
  → compileContextGlobe
  → revision-seeded stipple + polar fabric + sectors + marker accounting
  → ContextGlobeScene
  → orthographic camera + projected surface + presentation layers
```

`ExplorerGlobe` contains exact globe/source/compiler identities, coordinate
authority, clusters, admitted regions, and workload beacons. Regions and
beacons carry stable identity and semantic longitude/latitude in degrees.

The package treats those coordinates as qualified input. It does not decide
whether they are native geography, synthetic semantic coordinates, or another
spherical basis.

## Shared Projection Contract

One coordinate projector applies orientation and projection progress to:

- the indexed surface;
- ordinary stipple and north/south fabric;
- occupied sectors and admitted regions;
- workload markers and optional halos; and
- renderer-facing anchors later used for picking and accessible overlays.

This contract prevents attached geometry from sliding over or detaching from
the world during the morph. A new globe-bound primitive must consume the same
projector instead of recreating sphere math or using screen coordinates.

The coordinate facing the camera becomes the Mercator view center. The
indexed surface moves its longitude seam to the back of that view before
unfurling, and bounded sectors split against the same view-relative seam. A
rotated globe therefore opens around the operator's current bearing instead of
twisting back toward the canonical antimeridian.

A sector that straddles that seam splits into multiple fragments so no
triangle crosses the discontinuity, and that same split reruns unchanged for
every horizontal repeat copy, regardless of which one is actually connecting.
Each fragment's own chart position — not the sector's naive bounds-average
center, which sits unstably right at the seam for a straddling sector —
decides its own repeat-copy visibility, so the fragment that isn't adjoining
the connected seam stays suppressed instead of doubling the boundary.

The flat Mercator chart's "north is up" convention has no analogue for
camera pitch: it can only recenter around a bearing's longitude and latitude,
never its tilt. Pitch is therefore not part of the shared projector at all —
it lives entirely in the camera (`globeCameraPose`), as a screen-relative
tilt about a fixed world-horizontal axis, independent of yaw. It eases from
the raw pitch to level over the same projection progress that drives the
rest of the morph, so the camera genuinely re-levels while the globe
flattens, arriving at exactly the same static, level pose the Mercator
endpoint has always used — rather than a per-vertex correction simulating
that motion under a fixed lens. Yaw stays in the projector: it recenters the
indexed surface (and, with it, the unfurl seam) around the operator's
current bearing, a content decision independent of viewing angle. Because
pitch never touches vertex data, every consumer of the shared projector
(indexed surface, sectors, markers, pole fabric, the deterministic stipple's
GPU-blended positions) sees a consistent, un-rolled sphere with no
correction needed to keep cached sphere/Mercator endpoints from shearing
mid-transition.

Projection progress, presentation progress, and camera scale are separate.
The surface can keep morphing while posture-specific presentation exits on a
faster curve and semantic content crosses independent LOD thresholds.

## Geometry And Scene

The current scene uses:

- a `1.72` unit sphere with `160 × 96` segments;
- an orthographic camera that itself orbits through pitch (`globeCameraPose`);
- yaw orientation baked into the shared projector, independent of camera pitch;
- two directional lights and one ambient light;
- three transparent atmosphere shells; and
- depth-tested surface markers with optional beacon halos.

Atmosphere, lighting, scaffold, and stipple are presentation. They imply
neither terrain nor coverage nor an Earth coordinate reference system.

## Presentation Choreography

The transition deliberately does not drive every layer with the same visual
curve:

| Layer                      | Globe behavior                                 | Transition behavior                                                                                                                  | Mercator behavior                                             |
| -------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| Indexed surface            | Spherical and oriented.                        | Unfurls and expands continuously.                                                                                                    | Spans the intended planar canvas for projected attachments.   |
| Attached geometry          | Fixed to semantic coordinates.                 | Uses the exact surface projector.                                                                                                    | Retains coordinate and identity.                              |
| Atmosphere                 | Projected shell outside the same surface.      | One reversible shell and opacity response; traversal direction cannot select a brighter material state.                              | Absent before it can appear inside the map.                   |
| Neutral spherical scaffold | Establishes the globe body in both renderers.  | Keeps its extent but finishes its color fade before repeated fabric becomes prominent; depth-only geometry may briefly mask overlap. | Absent; no residual warm or gray plane.                       |
| Stipple                    | Subtle curvature texture.                      | Material response changes with posture.                                                                                              | Darker subdued texture remains legible on the flat surface.   |
| Polar fabric               | Sparse unlabeled caps in the stipple language. | Travels with the shared projector.                                                                                                   | Does not claim geography outside the disclosed chart support. |

The accelerated atmosphere reuses the exact projected surface geometry. Its
TSL material offsets that surface along the retained rotated spherical normal
by a bounded shell thickness. Shell extent follows the square root of remaining
spherical posture while opacity follows its square, so geometry can retain a
soft exterior outline without carrying bright light into the planar posture.
The radial attribute keeps the halo visible in screen space even while the
projected surface normal still points toward the camera. Each layer renders
its rear shell face and derives fragment opacity from a smoothstep over the
retained spherical normal. The falloff is normalized to the layer's live shell
extent, including while that extent shrinks, so every layer reaches zero before
its outer silhouette instead of ending at a constant-opacity ring. Globe depth masks
the interior at the spherical endpoint. During intermediate postures, a
colorless double-sided silhouette pass writes one stencil value for the live
canonical and wrapped projections, and every atmosphere layer accepts only
pixels outside that union. This leaves a clear rim without tinting the surface
or drawing an atmosphere band across Mercator. A clear warm-yellow inner shell,
pale gold middle shell, and cream outer shell overlap across widening radii to
create a continuous warm falloff without the prior dark green edge or visible
brightness steps. The bands use additive blending so
they emit a luminous yellow rim instead of reading as translucent pigment over
the gray canvas. The same progress value now produces the same shell extent and
opacity while entering or exiting Atlas. Every band retains its settled color
and relative opacity, so reversing the wheel cannot switch visual modes, jump
toward full brightness, overshoot the final extent, or introduce an
independently scaled sphere inside the wider projection.

The indexed globe body uses neutral ambient and directional light. Warm light
belongs to the exterior atmosphere bands only; returning surface opacity cannot
recolor the projected interior yellow and be mistaken for a bright halo.

During the interval where horizontal Atlas copies still exist, the same
projected rear-face shells repeat at chart offsets `-1` and `1`. They share the
canonical projected geometry and radial band falloff, so no glow can fill a
chart interior: it exists only beyond the current globe silhouette. A
repeated copy's overall envelope is the bounded product of the repeat
dissolve and its complement, so it therefore follows the same path in either
direction, peaks only while charts are actively dissolving, and is
transparent at both World and Atlas endpoints. Within that envelope, the
glow additionally sweeps using the same per-vertex chart-position weight
that governs occupied sectors, stipple, and markers, but in the opposite
direction from those layers: dim at the copy's connected seam, brightest at
its outer edge, so the glow reads as a directional cue toward the content
still ahead if the operator keeps unfurling rather than mirroring the
seam-outward reveal everything else on that copy uses.

The accelerated renderer is the sole atmosphere owner. Its transparent SVG
reference scaffold does not render the fallback radial-gradient circle, so a
late legacy halo cannot replace or compound the projected shell near the World
endpoint. The fallback atmosphere remains available only when the reference
renderer owns the surface.

As the projection approaches the planar endpoint, the accelerated scene
dissolves projected sectors, stipple, regions, and beacons into horizontal
chart indexes `-1` and `1` around the canonical `0` chart. The repeat dissolve
begins at projection progress `0.58` and follows a smoothstep curve to full
opacity. Because opacity is derived only from projection progress, exiting
Mercator evaluates the same curve in reverse and dissolves both repeated copies
before the spherical endpoint. A side copy is always evaluated in planar
Mercator coordinates; it must not inherit the canonical chart's intermediate
spherical vertex warp. Its center offset instead places the planar copy's
inner edge on the current eased seam. The offset begins at half a chart width
when the repeat is invisible and reaches one chart width at the planar
endpoint. Only a narrow connection band bends from the planar chart onto the
canonical seam, so the copies stay joined without buckling their interiors.

Within each side copy, a mirrored smoothstep field keeps stipple and projected
attachments darkest at the connected seam, then lightens them continuously
toward the outer edge. The reversible temporal dissolve expands that spatial
field away from a full-strength seam instead of multiplying the whole side
chart. The joint therefore retains the canonical chart's visual weight while
the lighter fabric grows or contracts around it. While charts still overlap,
the same field also forms a depth wedge. A bounded band beside the connected
seam stays coplanar so the nearest retained samples form a visibly continuous
joint; only after that band does the repeat recede smoothly along negative Z
toward its outer edge. The canonical unfurling surface writes the temporary
depth boundary, preventing receded dots from accumulating over its stipple.
Depth returns to zero with overlap at the planar endpoint and reverses from
the same projection state on exit. This Z offset is presentation geometry,
not a semantic altitude or evidence axis.

The visible neutral scaffold uses its own bounded fade window from projection
progress `0.38` through `0.62`. Its color is therefore gone before the repeated
charts are materially legible, avoiding a bright canonical rectangle whose
vertical bounds could read as disconnected map edges. The same geometry may
continue writing depth while fully transparent; color opacity and overlap
occlusion are intentionally separate mechanisms.

The canvas establishes all three chart extents and prewarms transparent side
copies while the globe is still spherical. Spherical and planar instance
matrices, Atlas positions, and closed-seam morph offsets are cached against
exact orientation, viewport, and source-bucket identity. TSL interpolates the
stable instance attributes from one scalar projection uniform; a morph frame
does not rewrite thousands of instance matrices, reproject every repeated
sample, rebuild node materials, or recreate atmosphere geometry.

The canonical (non-repeated) stipple field blends this same cached spherical
instance transform toward its cached Atlas position entirely inside the
vertex shader, one scalar `mix()` between two fixed endpoints.

The indexed surface's own sphere/Mercator endpoints are cached the same way
and blended per vertex every frame with a cheap array lerp rather than full
per-vertex reprojection. Because pitch lives in the camera (`globeCameraPose`)
rather than in either cached endpoint's vertex data, this plain lerp is
exact — a real camera rotation never shears interpolated geometry the way a
naive blend between two differently-view-rotated vertex sets would, so no
correction step is needed here at all. Repeat
instances are ordered from the joined seam toward the outer edge, and each
frame submits only the prefix that the dissolve can make visible. Transparent
cached dots are not draw work.
Entry and exit therefore add no per-frame instance reprojection, material
rebuild, GPU canvas resize, scale jump, or framing jump. Raster and submission
cost for the visible repeated fabric remains measurable and bounded separately.
The agent retains that compiled globe across the World-to-Atlas regime handoff
only when the next scene binds the same exact atlas transition revision; a
different revision or direct Atlas entry compiles its own declared surface.
Horizontal panning exposes no artificial edge. Wrapped copies remain view
geometry: they keep canonical semantic identity and grant no additional
evidence or coverage.

The accelerated package owns the surface, fabric, and atmosphere behavior;
`@rey/agent` owns the matching reference scaffold. Their independent curves
preserve perceptual continuity. Reusing one opacity or scale function for all
layers is a regression risk even when the projection math remains correct.

## Deterministic Fabric

`contextGlobeSamples` evaluates 26,000 golden-angle candidates and keeps a
revision-seeded subset. Coherent brightness terms reveal curvature. Admitted
region emphasis may brighten existing candidates but cannot create support.

The compiler partitions retained samples into three color/opacity buckets for
instancing. North and south use sparse deterministic golden-angle cap patterns
in the same dot language. They make the spherical frame legible without `N`
or `S` labels and do not constitute geographic evidence.

The pure pattern is exported through `@rey/explorer/globe-samples`, allowing
the application reference renderer to share it without importing Three.js.

## Current Boundary

The package keeps its surface, fabric, occupied sectors, and markers in one
World-to-Atlas projection. The application still owns semantic chart wrapping,
labels, picking policy, accessibility, vector geography, wheel integration,
and semantic LOD.

Projection tests cover deterministic endpoints and shared attachment. Browser
qualification must also sample intermediate frames because correct endpoints
do not prove a correct journey. See [Extending and qualifying](EXTENDING.md).
