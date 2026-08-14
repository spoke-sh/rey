# `@rey/explorer`

`@rey/explorer` is Rey's reusable accelerated rendering package. It owns the
React Three Fiber canvas, declarative globe and terrain scenes, pure GPU-data
compilers, bounded Three.js renderer lifecycle, and renderer diagnostics.

For the product model, spatial journey, evidence rules, and desired geospatial
experience, start with [Explorer](../../docs/EXPLORER.md). This document is the
technical guide to the package as it exists today and the rendering
capabilities it is intended to grow into.

## Package Boundary

The dependency is deliberately one-way:

```text
@rey/agent
  evidence adapters · projection · immutable scenes · fields
  camera controls · render graph · picking · labels · reference renderer
                              │
                              │ typed compiled inputs
                              ▼
@rey/explorer
  pure GPU compilers · R3F scenes · Three.js lifecycle · canvas reports
                              │
                              ▼
                 WebGPU or WebGL2 pixels
```

The package accepts already admitted, application-compiled inputs. It does
not:

- fetch or resolve evidence;
- interpret workload, survey, or scene-admission documents;
- choose semantic coordinates or levels of detail;
- generate terrain working sets from source evidence;
- own application camera controls, labels, picking policy, or evidence links;
- render the accessible reference fallback; or
- qualify, mutate, or persist what it renders.

Those responsibilities remain in `@rey/agent`. The application keeps its
deterministic reference renderer mounted until this package submits a valid
accelerated frame and reveals it again after renderer failure.

## Component Map

| Component | Source | Responsibility |
| --- | --- | --- |
| Canvas boundary | [`src/canvas.tsx`](src/canvas.tsx) | Creates and configures the R3F root, selects content, applies frame invalidation, and reports readiness and draw submission. |
| Renderer contracts | [`src/renderer.ts`](src/renderer.ts) | Defines renderer lifecycle, status, frame identity, invalidation, and bounded viewport sizing. |
| Backend adapter | [`src/three-webgpu.ts`](src/three-webgpu.ts) | Initializes Three.js `WebGPURenderer`, selects WebGPU or WebGL2, observes loss, instruments draw submission, and disposes resources. |
| Structural inputs | [`src/types.ts`](src/types.ts) | Defines the renderer-facing globe, marker, camera, and terrain field shapes. |
| Globe compiler | [`src/three-globe.ts`](src/three-globe.ts) | Converts semantic globe input into deterministic sample buckets, polar patterns, marker statistics, and material identity. |
| Globe fabric | [`src/globe-samples.ts`](src/globe-samples.ts) | Generates revision-seeded spherical stipple and subtle north/south cap patterns. |
| Terrain compiler | [`src/three-terrain.ts`](src/three-terrain.ts) | Converts valid field grids into bounded indexed meshes, verifies CPU/upload parity, accounts bytes, and builds the TSL relief material. |
| Declarative scenes | [`src/fiber-scenes.tsx`](src/fiber-scenes.tsx) | Expresses globe and continuous-relief cameras, lights, materials, instancing, geometry, and evidence-named scene objects. |
| Shared Three graph | [`src/three-fiber-runtime.ts`](src/three-fiber-runtime.ts) | Exposes the modular WebGPU Three.js graph plus the legacy renderer required by the R3F test harness. |
| Public surface | [`src/index.ts`](src/index.ts) | Exports the canvas, compilers, contracts, revisions, limits, and renderer status. |

## Runtime Data Flow

The application supplies one immutable frame identity and one of two content
variants:

```ts
type ExplorerCanvasContent =
  | {
      kind: "globe";
      compiled: CompiledContextGlobe;
      view: GlobeCameraView;
      world: { width: number; height: number };
    }
  | {
      kind: "terrain";
      compiled: CompiledContinuousRelief;
      view: TerrainCameraView;
      world: { width: number; height: number };
    };
```

The full frame path is:

```text
typed content + frame identity
  → bounded canvas viewport
  → backend initialization
  → demand-driven R3F root
  → declarative globe or terrain scene
  → Three.js render submission
  → lifecycle, backend, draw-call, and CPU-submission report
```

`ExplorerCanvas` uses a demand-driven frame loop. It reconciles a new scene
only when one of these exact identities changes:

| Identity | Meaning |
| --- | --- |
| `snapshot_id` | Immutable semantic scene changed |
| `camera_revision` | Camera or viewport changed |
| `material_revision` | GPU material contract changed |
| `render_graph_id` | Ordered pass availability changed |

An identical frame is quiet. Measured timings are not part of frame identity.

## Globe Pipeline

The accelerated globe path is:

```text
ExplorerGlobe
  → compileContextGlobe
  → revision-seeded stipple buckets + polar caps + marker accounting
  → ContextGlobeScene
  → orthographic camera + lit sphere + atmosphere + instanced dots + markers
```

### Input contract

`ExplorerGlobe` contains the globe/source/compiler identities, coordinate
authority, clusters, admitted regions, and workload beacons. Regions and
beacons carry semantic longitude/latitude in degrees plus stable identities
and presentation tone.

The package treats those coordinates as already qualified. It does not decide
whether they are native geography, synthetic semantic coordinates, or another
spherical basis.

### Surface and atmosphere

The current scene uses:

- a `1.72` unit sphere with `160 × 96` segments;
- two directional lights and one ambient light;
- three transparent atmosphere shells;
- an orthographic camera;
- quaternion-based yaw and pitch over the complete globe group; and
- depth-tested surface markers and optional beacon halos.

The atmosphere, lighting, and stipple are presentation. They do not imply
terrain, coverage, or an Earth coordinate reference system.

### Deterministic stipple

`contextGlobeSamples` evaluates 26,000 golden-angle candidates and retains a
revision-seeded subset. Coherent brightness terms reveal curvature, while
admitted region emphasis can brighten existing candidates without creating
new support.

The compiler partitions retained samples into three color/opacity buckets for
instanced rendering. North and south use sparse deterministic golden-angle cap
patterns in the same dot language. The caps identify the spherical frame
visually; they are not labels or geographic evidence.

### Current omissions

The package globe does not yet render the World-to-Atlas morph, sector
polygons, vector geography, labels, or picking. `@rey/agent` currently owns
those projection and accessible overlay paths.

## Terrain Pipeline

The accelerated terrain path is:

```text
TerrainFieldSetInput[]
  → buildTerrainMeshData
  → verifyTerrainMeshParity
  → enforce GPU byte budget
  → CompiledContinuousRelief
  → ContinuousReliefScene
  → orthographic camera + TSL material + indexed relief meshes
```

### Field input

Each `TerrainFieldSetInput` supplies one bounded regular grid:

| Channel | Representation | Use |
| --- | --- | --- |
| Grid | columns, rows, and local bounds | Maps cells into local scene coordinates |
| Validity | `Uint8Array` | Controls whether a triangle has admitted support |
| Elevation | `Float32Array` plus scale | Becomes the mesh Y coordinate |
| Normal | `Float32Array` or `Int8Array` | Drives lighting in Three.js coordinates |
| Curvature | `Float32Array` | Enhances ridges and valleys |
| Tint | RGB `Float32Array` | Supplies the base material color |
| Occlusion | `Float32Array` | Darkens bounded valleys and ambient support |
| Roughness | `Float32Array` | Controls the TSL surface response |

The application currently derives these fields from admitted survey terrain
programs and camera-relative patch requests. Field generation, hydrology,
validity classification, LOD band selection, halo evaluation, and patch
caching remain outside this package.

### Mesh construction

Grid X/Y becomes mesh X/Z and admitted elevation becomes mesh Y. Each grid
quad is triangulated with alternating diagonals. A triangle is emitted only
when all three vertices have valid support, so the GPU surface cannot bridge an
invalid or unexplored cell.

Compilation creates separate upload arrays for positions, normals, tint,
occlusion, roughness, curvature, and indices. The reference CPU fields remain
unchanged if a renderer mutates an upload buffer.

### Upload parity and limits

`verifyTerrainMeshParity` checks every uploaded field sample after the
coordinate transform and rejects any index that touches invalid support. The
current parity identity is `rey.terrain.cpu-mesh-upload-parity@1`.

The compiler measures exact typed-array byte length before rendering and
rejects a compilation above `MAX_ACCELERATED_TERRAIN_GPU_BYTES`, currently
64 MiB. Statistics retain field-set, vertex, triangle, source-byte, GPU-byte,
budget, parity-sample, and geometry-compilation counts.

### Material and lighting

`createContinuousReliefMaterial` builds a `MeshStandardNodeMaterial` with TSL.
The material combines:

- source tint;
- world-space multidirectional hillshade;
- explicit occlusion;
- curvature-based ridge brightening and valley darkening; and
- bounded roughness.

`ContinuousReliefScene` shares that material across compiled meshes and adds
warm and cool directional lights plus ambient fill. The current accelerated
terrain camera is bounded and orthographic from overhead. A unified bounded
County-isometric 3D camera remains part of the engine direction rather than a
claim about the package today.

### Current omissions

The terrain package does not yet upload or draw contours, water, weather,
boundaries, roads, structures, labels, selection, or evidence overlays. Those
passes exist in the application render graph and reference renderer. The
accelerated package currently owns the base continuous-relief surface only.

## Renderer Lifecycle

`ReactThreeFiberRendererAdapter` wraps Three.js's asynchronous
`WebGPURenderer` behind a small facade.

| Preference | Behavior |
| --- | --- |
| `auto` | Prefer WebGPU and accept Three.js's WebGL2 compatibility backend with visible degraded status |
| `webgpu` | Require WebGPU; fail to reference status if Three.js selects another backend |
| `webgl2` | Force Three.js's WebGL2 backend, primarily for compatibility qualification |
| `reference` | Skip accelerated initialization and report that the application reference renderer is selected |

Lifecycle states are `idle`, `initializing`, `ready`, `failed`, and
`disposed`. The adapter publishes immutable status snapshots and reports the
selected backend and renderer revision.

WebGPU device loss disposes the renderer and reports a degraded reference
status. WebGL context loss unmounts the R3F root and does the same. The package
does not draw fallback pixels itself; `@rey/agent` keeps the deterministic
reference surface available beneath the accelerated canvas.

Renderer submission timing measures only the synchronous CPU call boundary.
Draw-call counts come from Three.js renderer information. Neither value is GPU
execution time or a frame-rate measurement.

## Viewport Bounds

`boundedViewport` prevents accidental unbounded pixel work. Its defaults are:

| Limit | Default |
| --- | ---: |
| Maximum device pixel ratio | 2 |
| Maximum logical dimension | 2048 px |
| Maximum physical pixels | 8,388,608 |

The function preserves aspect ratio while reducing width and height to satisfy
all three limits. Internal callers may provide stricter bounds.

## Public API

The package root exports four groups.

### Canvas and renderer

- `ExplorerCanvas`
- `ExplorerCanvasContent`
- `ExplorerCanvasProps`
- `ExplorerCanvasReport`
- `RendererPreference`
- `RendererStatus`
- `THREE_RENDERER_REVISION`
- `WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT`

### Globe

- `compileContextGlobe`
- `CompiledContextGlobe`
- `contextGlobePolePatterns`
- `GlobePole` and `GlobePolePattern`
- globe geometry, sample, pole-pattern, and material revision constants

The pure globe fabric is also available through the
`@rey/explorer/globe-samples` subpath so the application reference renderer can
share exactly the same presentation pattern without importing Three.js.

### Terrain

- `buildTerrainMeshData`
- `compileContinuousRelief`
- `createContinuousReliefMaterial`
- `terrainCameraProjection`
- `terrainMeshByteLength`
- `verifyTerrainMeshParity`
- compiled mesh, camera, parity, material, and GPU-budget contracts

### Structural types

- `ExplorerGlobe`, `ExplorerGlobeRegion`, and `ExplorerGlobeBeacon`
- `GlobeCameraView`
- `TerrainFieldSetInput` and `TerrainCameraView`

## Render Graph Boundary

The immutable render graph currently lives in `@rey/agent`. It orders these
logical passes:

```text
validity_background
base_terrain
height_normals_hillshade
ambient_valley_occlusion
contours
water_weather_boundary
features_labels_selection
evidence_accessibility
```

The application passes the exact graph identity into `ExplorerCanvas`, but the
package currently implements only the accelerated globe and base-terrain
portions. Growing the accelerated renderer means adding typed, bounded pass
inputs—not letting a scene component fetch application state or invent its own
graph.

## Geospatial Engine Direction

The next rendering components should extend the same declarative architecture:

| Component | Desired package capability |
| --- | --- |
| Projection scenes | Reconcile globe, globe-to-map morph, wrapped map, and bounded local 3D postures through one stable identity. |
| Terrain residency | Consume multiresolution camera working sets with crack-free seams, explicit no-data, frustum culling, and bounded GPU residency. |
| Raster primitives | Upload provider-qualified elevation, imagery, and material tiles without losing source revision, resolution, attribution, or no-data semantics. |
| Vector primitives | Batch points, lines, polygons, holes, extrusions, and connectors while preserving feature and layer identity. |
| Semantic/geometric LOD | Select geometry, material, labels, and features independently from the semantic lens and report all omissions. |
| Render passes | Materialize the application's compiled pass graph as explicit R3F components with authority and availability retained. |
| Picking | Return renderer-neutral semantic picks for globe, map, terrain, and vector geometry while preserving analytic inverse-coordinate checks. |
| Labels and selection | Supply stable projected anchors and occlusion information to the accessible application overlay. |
| Backend parity | Keep WebGPU, WebGL2, and reference output semantically aligned and visibly disclose unsupported accelerated features. |

This direction does not imply a general entity-component system, unrestricted
free-flight camera, physics engine, or browser-owned evidence store. New scene
types remain bounded projections over application-owned semantic inputs.

## Adding A Rendering Capability

A new accelerated capability should normally include:

1. A renderer-neutral typed input owned by the correct semantic adapter.
2. A pure compiler that validates shapes, validity, limits, and byte cost.
3. Explicit source, compiler, material, and parity revisions.
4. Disposable upload arrays that cannot mutate authoritative CPU fields.
5. A declarative R3F scene or pass component with deterministic object names.
6. Frame invalidation keyed to exact scene, camera, material, and graph
   identity.
7. Lifecycle and resource reporting through `ExplorerCanvasReport` or the
   application report adapter.
8. A deterministic reference projection and accessibility path in
   `@rey/agent`.
9. Vitest coverage for compilation, bounds, parity, scene structure,
   initialization, degradation, and disposal.
10. A named browser qualification that exercises the real backend and retains
    exact omissions without claiming GPU execution time.

## Testing And Development

From the repository root:

```sh
pnpm --filter @rey/explorer test
pnpm --filter @rey/explorer typecheck
pnpm --filter @rey/explorer build
pnpm check
```

The package test suite covers:

- viewport bounds and frame invalidation;
- WebGPU/WebGL2 initialization, forced backend selection, loss, and disposal;
- globe determinism, polar patterns, compilation statistics, and declarative
  object identity;
- terrain mesh construction, validity holes, parity, byte accounting, camera
  projection, TSL material creation, and the GPU budget; and
- R3F globe and terrain scene structure through the React Three test renderer.

Real-backend output, fallback continuity, browser interaction, and retained
qualification manifests are exercised by the `@rey/agent` Explorer voyage.

## Related Documentation

- [Explorer concept](../../docs/EXPLORER.md)
- [Architecture](../../docs/ARCHITECTURE.md)
- [Mining and visualization](../../docs/MINING.md)
- [Interfaces](../../docs/INTERFACES.md)
- [Plan 0003](../../plans/0003-scene-to-explorer.md)
