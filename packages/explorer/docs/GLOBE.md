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

Projection progress, presentation progress, and camera scale are separate.
The surface can keep morphing while posture-specific presentation exits on a
faster curve and semantic content crosses independent LOD thresholds.

## Geometry And Scene

The current scene uses:

- a `1.72` unit sphere with `160 × 96` segments;
- an orthographic camera;
- one shared yaw/pitch orientation in the projector;
- two directional lights and one ambient light;
- three transparent atmosphere shells; and
- depth-tested surface markers with optional beacon halos.

Atmosphere, lighting, scaffold, and stipple are presentation. They imply
neither terrain nor coverage nor an Earth coordinate reference system.

## Presentation Choreography

The transition deliberately does not drive every layer with the same visual
curve:

| Layer                        | Globe behavior                                 | Transition behavior                                          | Mercator behavior                                             |
| ---------------------------- | ---------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------- |
| Indexed surface              | Spherical and oriented.                        | Unfurls and expands continuously.                            | Fills the intended planar canvas.                             |
| Attached geometry            | Fixed to semantic coordinates.                 | Uses the exact surface projector.                            | Retains coordinate and identity.                              |
| Atmosphere                   | Clearly visible around the sphere.             | Contracts while opacity falls on a faster fifth-power curve. | Absent before it can appear inside the map.                   |
| Reference spherical scaffold | Establishes the fallback globe body.           | Keeps its diameter while fading in `@rey/agent`.             | Absent; no residual gray circle.                              |
| Stipple                      | Subtle curvature texture.                      | Material response changes with posture.                      | Darker subdued texture remains legible on the flat surface.   |
| Polar fabric                 | Sparse unlabeled caps in the stipple language. | Travels with the shared projector.                           | Does not claim geography outside the disclosed chart support. |

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
