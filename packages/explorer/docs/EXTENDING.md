# Extending And Qualifying The Renderer

New accelerated capabilities extend the declarative architecture without
moving evidence authority into the rendering package.

## Capability Checklist

A rendering capability normally includes:

1. A renderer-neutral typed input owned by the correct semantic adapter.
2. A pure compiler validating shape, validity, limits, and byte cost.
3. Explicit source, compiler, material, and parity revisions.
4. Disposable upload arrays unable to mutate authoritative CPU fields.
5. A declarative R3F scene or pass with deterministic object names.
6. Invalidation keyed to exact scene, camera, material, and graph identity.
7. Lifecycle and resource reporting through `ExplorerCanvasReport` or its
   application adapter.
8. A deterministic reference projection and accessible path in `@rey/agent`.
9. Vitest coverage for compilation, bounds, parity, scene structure,
   initialization, degradation, and disposal.
10. A named browser qualification exercising the real backend and retaining
    exact revisions, limits, and omissions.

## Proof Layers

The proof boundary has three layers:

| Layer                       | What it establishes                                                                                                      |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Pure Vitest contracts       | Projection math, deterministic compilation, validity, bounds, parity, and byte accounting.                               |
| React Three test renderer   | Declarative scene structure, stable object names, attachment, and update/disposal behavior.                              |
| `@rey/agent` browser voyage | Real input, intermediate animation frames, backend output, reference continuity, accessibility, and device/context loss. |

Globe transitions are sampled between endpoints. Qualification checks that
surface, fabric, sectors, markers, atmosphere, and scaffold progress according
to their own contracts while semantic identity stays fixed. A final flat map
alone does not prove a continuous projection.

Measured CPU submission time and Three.js draw-call counts are retained only
under their exact meanings. They are not GPU execution or frame-rate evidence.

The versioned `rey.explorer.landscape-fidelity@2` suite defines seven separately
admitted browser workloads at 1920×1080 and 3840×2160: steep relief, low relief,
coastline/water, dense vectors, explicit holes, stale data, and backend loss.
Selecting one with `--landscape-workload` binds its source-controlled suite
digest into the voyage. Qualification fails unless the live admitted Landscape
satisfies its relief, pass, no-data, seam, label, resource, omission, and loss
requirements. The harness does not inject or relabel a fixture as admitted
evidence.

## Current Test Coverage

The package suite covers:

- viewport bounds and frame invalidation;
- WebGPU/WebGL2 initialization, forced selection, loss, and disposal;
- globe determinism, polar fabric, compiler statistics, and declarative object
  identity;
- sphere-to-Mercator endpoints and shared sector attachment;
- terrain meshes, validity holes, CPU/upload parity, byte accounting, camera
  projection, TSL material creation, and GPU budget;
- conservative tile seam verification and explicit no-data triangle-leak
  accounting;
- separately revisioned TSL and geographic terrain passes; and
- globe and terrain/pass structure through the React Three test renderer.

Run it from the repository root:

```sh
pnpm --filter @rey/explorer test
pnpm --filter @rey/explorer typecheck
pnpm --filter @rey/explorer build
pnpm check
```

## Engine Direction

The next components should preserve the same boundaries:

| Component            | Desired package capability                                                                                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Projection scenes    | Continue the shared globe/map surface into wrapped maps and bounded local 3D postures through one stable identity.             |
| Terrain residency    | Extend the implemented conservative uniform-level tiles toward qualified mixed-level crack repair and native streaming.        |
| Raster primitives    | Upload qualified elevation, imagery, and material tiles while retaining source revision, resolution, attribution, and no-data. |
| Vector primitives    | Batch points, lines, polygons, holes, extrusions, and connectors while preserving feature and layer identity.                  |
| Independent LOD      | Select geometry, material, labels, and features independently from semantic lens and report omissions.                         |
| Render passes        | Batch and extend the executable R3F pass set while retaining authority, dependency, input revision, and fallback.              |
| Picking              | Return renderer-neutral semantic picks across globe, map, terrain, and vector geometry with analytic inverse checks.           |
| Labels and selection | Supply stable projected anchors and occlusion to the accessible application overlay.                                           |
| Backend parity       | Keep WebGPU, WebGL2, and reference output semantically aligned and disclose unsupported acceleration.                          |

This direction does not imply a general entity-component system, unrestricted
free-flight camera, physics engine, or browser-owned evidence store. Every new
scene remains a bounded projection over application-owned semantic input.
