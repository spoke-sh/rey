# ADR 0045: Three.js WebGPU Renderer

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0044](0044-explorer-projection-engine.md)

## Context

ADR 0044 deliberately deferred the accelerated renderer until Rey could
qualify a concrete engine boundary. Its initial plan compared a direct WebGL2
implementation with a rendering library. That comparison put an API backend
and an engine abstraction at the same decision level.

Explorer needs modern GPU rendering and eventually compute, but it also needs
one maintained scene/material abstraction, an explicit render graph, graceful
compatibility, resource lifecycle management, and a narrow adapter that keeps
GPU objects outside semantic evidence. Maintaining those mechanisms directly
over WebGL2 and later WebGPU would duplicate engine work that is not distinctive
to Rey.

Three.js provides a `WebGPURenderer` that selects WebGPU by default and falls
back to its WebGL2 backend when WebGPU is unavailable. Its Three.js Shading
Language (TSL) expresses node materials and render/post-processing composition
independently of WGSL or GLSL. The renderer is still documented as experimental,
so revision pinning, compatibility proof, and a non-GPU semantic reference path
remain necessary.

## Decision

Explorer's production renderer targets **Three.js `WebGPURenderer` with TSL**.
The backend posture is:

```text
immutable Rey scene + typed field buffers
                 │
                 ▼
       narrow Three.js adapter
                 │
                 ▼
      Three.js WebGPURenderer + TSL
          │                  │
          ▼                  ▼
   WebGPU backend     WebGL2 compatibility backend

deterministic reference renderer ── semantic and degraded fallback proof
```

WebGPU is the preferred production backend. WebGL2 is a compatibility backend
inside the same Three.js renderer contract, not Rey's primary rendering
architecture and not a separately maintained scene implementation. The
deterministic reference renderer remains independent of Three.js so semantic
scene, field, validity, ordering, omission, and picking behavior can be tested
without a GPU.

Rey owns its immutable scene, camera semantics, field buffers, validity masks,
LOD, invalidation, render-pass declarations, selection, picking identity, and
resource budgets. A narrow adapter may materialize Three.js scenes, geometries,
textures, node materials, render targets, and passes, but those objects do not
become Rey's semantic scene or source of truth. React continues to own routes,
controls, accessibility, HTML labels, evidence panels, and exact links.

Terrain materials and post-processing use TSL rather than backend-specific
WGSL, GLSL, `ShaderMaterial`, `RawShaderMaterial`, or `onBeforeCompile`
customization. This keeps the same declared material graph available to the
WebGPU and WebGL2 backends. A backend-specific escape hatch requires a recorded
capability boundary, fallback behavior, and proof that it does not change
semantic assessment.

Initial elevation, validity, hydrology, erosion, normal, and curvature fields
remain deterministic CPU/reference computations uploaded through typed buffers
or textures. WebGPU compute may later accelerate derived-field work only after
fixtures prove parity, budgets expose GPU limits, and the WebGL2/reference path
retains a bounded equivalent or visible degradation. GPU results never become
the sole copy of a field or authoritative evidence.

The renderer is demand-driven. Three.js's animation-loop facility may be used
to initialize and schedule its asynchronous renderer safely, but Rey requests
frames only for invalidated scene, camera, material, viewport, or interaction
state. It does not fabricate continuous animation. Device/context loss,
initialization failure, unsupported features, and backend fallback remain
visible operator state while the last valid semantic scene is retained.

The first implementation pins Three.js `0.185.1` and its TypeScript declarations
at `0.185.4` in the frontend package and lockfile. Qualification
must cover the WebGPU and forced-WebGL2 paths, asynchronous initialization,
TSL material compilation, output quality, bundle and Nix closure size, resource
disposal, device/context loss, browser support, licensing, and named performance
evidence before the SVG reference surface is displaced.

## Consequences

- Rey does not build and maintain parallel direct WebGL2 and WebGPU engines.
- Three.js supplies graphics mechanics; Rey retains semantic scene and evidence
  ownership behind a narrow adapter.
- One TSL material graph can target WGSL or GLSL through the selected backend.
- WebGPU compute remains an optimization boundary rather than a prerequisite
  for deterministic field semantics.
- WebGL2 and the reference renderer preserve bounded operation where WebGPU is
  unavailable, while visibly reporting the active backend and degradation.
- Three.js experimental API drift is controlled through exact version pins,
  upgrade qualification, and backend-independent fixtures.

## References

- [Three.js WebGPURenderer](https://threejs.org/manual/en/webgpurenderer)
- [Three.js TSL specification](https://threejs.org/docs/TSL.html)
- [WebGPU API](https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API)
