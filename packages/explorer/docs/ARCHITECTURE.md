# Explorer Package Architecture

This document defines the technical ownership and runtime flow of
`@rey/explorer`. Product semantics and the visual fidelity bar live in the
[Explorer concept](../../../docs/EXPLORER.md).

## Ownership Boundary

```text
@rey/agent
  evidence adapters · semantic projection · immutable scenes · fields
  tile LOD · workers/residency · camera · picking · reference renderer
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
- choose semantic coordinates or semantic levels of detail;
- generate terrain working sets from source evidence;
- own application camera controls, labels, picking policy, or evidence links;
- render the accessible reference fallback; or
- qualify, mutate, or persist what it renders.

Those responsibilities remain in `@rey/agent`. The application keeps its
deterministic reference renderer mounted until this package submits a valid
accelerated frame and reveals it again after renderer failure.

## Component Map

| Component          | Source                                                           | Responsibility                                                                                             |
| ------------------ | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Canvas boundary    | [`../src/canvas.tsx`](../src/canvas.tsx)                         | Configure the R3F root, select content, invalidate frames, and report readiness and draw submission.       |
| Renderer contracts | [`../src/renderer.ts`](../src/renderer.ts)                       | Define lifecycle, status, frame identity, invalidation, and bounded viewport sizing.                       |
| Backend adapter    | [`../src/three-webgpu.ts`](../src/three-webgpu.ts)               | Initialize `WebGPURenderer`, select WebGPU/WebGL2, observe loss, report submission, and dispose resources. |
| Structural inputs  | [`../src/types.ts`](../src/types.ts)                             | Define renderer-facing globe, camera, terrain-field, and executable-pass shapes.                           |
| Globe compiler     | [`../src/three-globe.ts`](../src/three-globe.ts)                 | Compile semantic globe input into deterministic fabric, marker statistics, and material identity.          |
| Globe fabric       | [`../src/globe-samples.ts`](../src/globe-samples.ts)             | Generate revision-seeded stipple and subtle north/south patterns.                                          |
| Globe projection   | [`../src/globe-projection.ts`](../src/globe-projection.ts)       | Project one indexed surface and its attached sectors and markers from sphere to Mercator.                  |
| Terrain compiler   | [`../src/three-terrain.ts`](../src/three-terrain.ts)             | Compile valid grids into bounded meshes, verify parity, and execute gated TSL material stages.             |
| Declarative scenes | [`../src/fiber-scenes.tsx`](../src/fiber-scenes.tsx)             | Express cameras, lights, terrain, draped geographic passes, instancing, and named scene objects.           |
| Shared Three graph | [`../src/three-fiber-runtime.ts`](../src/three-fiber-runtime.ts) | Expose modular WebGPU Three.js and the legacy renderer needed by the R3F test harness.                     |
| Public surface     | [`../src/index.ts`](../src/index.ts)                             | Export canvas, compilers, contracts, revisions, limits, and renderer status.                               |

## Immutable Frame Flow

The application supplies one exact frame identity and one content variant:

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

```text
typed content + frame identity
  → bounded canvas viewport
  → backend initialization
  → demand-driven R3F root
  → declarative globe or terrain scene
  → Three.js render submission
  → lifecycle, backend, draw-call, and CPU-submission report
```

`ExplorerCanvas` uses a demand-driven frame loop. It reconciles when one of
four exact identities changes:

| Identity            | Meaning                            |
| ------------------- | ---------------------------------- |
| `snapshot_id`       | Immutable semantic scene changed.  |
| `camera_revision`   | Camera or viewport changed.        |
| `material_revision` | GPU material contract changed.     |
| `render_graph_id`   | Ordered pass availability changed. |

An identical frame is quiet. Measured timings are observations, never frame
identity.

## Regional Terrain Flow

Regional terrain reinforces the package boundary:

```text
frozen native terrain source
  → Rust editor index + independent scene-admission verification
  → content-identified regional grid + explicit valid/no-data cells
  → @rey/agent regional field compiler
  → TerrainFieldSetInput
       ├─ @rey/agent reference triangles
       └─ @rey/agent conservative tile pyramid + worker/residency
            └─ @rey/explorer bounded GPU meshes
```

The application owns what a row, column, height, material, and validity value
mean. The package owns deterministic field-to-mesh conversion, parity, resource
accounting, and R3F presentation. Both rendering paths call the same
`terrainTriangleIndices` rule, so backend selection cannot change the admitted
support boundary.

The application also supplies the bounded terrain orbit and optional
Atlas-to-Landscape model transform. The package applies them declaratively to
one orthographic camera and one terrain group. Executable line, point, and
validity-background inputs are children of that same group, so a projection
transition cannot detach overlays from relief. It does not choose transition
timing, semantic LOD, source/target frames, focus, or native-coordinate inverse
policy.

## Render-Graph Boundary

The immutable render graph lives in `@rey/agent`:

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

`@rey/agent` compiles this graph into `rey.terrain-render-pass-set.v1`. Each
executable pass retains its implementation revision, exact input revision, and
authority. Dependencies fail closed: a child cannot execute if its declared
parent is unavailable. The application drapes native line geometry against the
same terrain fields, sampling every crossed grid cell and splitting at no-data.

`@rey/explorer` consumes the typed pass set. Its TSL material gates base tint,
hillshade, and ambient/valley response independently; its R3F terrain group
draws the validity background, conservative line segments, and point/selection
anchors. Labels, links, descriptions, and interaction remain in the mounted
reference overlay, so accelerated pixels never become the semantic or
accessibility authority. An R3F scene never fetches application state or
invents its own graph.
