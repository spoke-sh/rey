# ADR 0044: Explorer Projection Engine

- Status: Accepted
- Date: 2026-08-11
- Extended by: [ADR
  0056](0056-continuous-globe-mercator-county-grammar.md), which supersedes the
  single top-down 2.5D camera restriction with bounded globe, Mercator, and
  isometric-county postures
- Extends: [ADR 0041](0041-continuous-coordinate-topography.md) and [ADR
  0043](0043-emergent-natural-features-and-separate-paths.md)
- Extended by: [ADR 0045](0045-threejs-webgpu-renderer.md), which selects
  Three.js `WebGPURenderer` and TSL with WebGPU-first and WebGL2 compatibility
  backends

## Context

Explorer began as a bounded React/SVG topology projection. It now has a
continuous camera, semantic levels of detail, stable points of interest,
anchor-derived relief, weather, hydrology, erosion, selection, and read-only
evidence traversal. Those are no longer isolated visualization widgets. They
are the early mechanisms of a spatial engine.

Two supplied captures from 2026-08-11—a 3840×2160 Rey World view and a
3022×1926 Google Maps terrain view—make the current fidelity gap clear. Rey
draws legible nested isolines and natural-feature strokes around the surveyed
anchors, but the surface between them is visually flat. A mature terrain map
composes a continuous material from multiscale height, slope, aspect, ridge and
valley occlusion, hypsometric tint, water, contours, labels, and overlays. Its
landforms remain legible across the viewport and across zoom instead of
appearing as linework placed on a plane.

Closing that gap by adding more SVG paths would further entangle evidence
derivation, field simulation, camera state, React rendering, and visual style
inside `topology.ts`, `explore.tsx`, and one StyleX module. The required change
is an engine boundary.

## Decision

### Product identity

`/explore` is Rey's **high-fidelity spatial game engine for evidence-bound
projections of high-dimensional context**. It owns coordinate transforms, an
immutable scene, data-oriented fields, a camera, semantic level of detail,
deterministic simulation, a render graph, materials, compositing, labels,
picking, and incremental invalidation.

This architecture does not make Explorer a game product, evidence store,
resolver, scheduler, or mutation plane. The engine projects admitted evidence;
it cannot promote a shader result, visual proximity, simulated feature, picked
object, or camera gesture into source truth or authority.

The first fidelity target in this decision was a top-down **2.5D semantic
terrain**. ADR 0056 supersedes that camera restriction with bounded globe,
semantic-Mercator, and isometric-county postures while continuing to defer
unrestricted free camera pitch/orbit, volumetric worlds, physics, and a general
entity-component system.

### Engine pipeline

The target data flow is:

```text
admitted evidence + exact projection basis + limits
                         │
                         ▼
                 projection packet
       coordinates · fields · validity · lineage
                         │
              ┌──────────┴──────────┐
              ▼                     ▼
       scene compiler          CLI inspector
              │
       field + simulation passes
              │
       immutable scene snapshot
              │
       camera + LOD + culling
              │
          render graph
              │
     reference or accelerated backend
              │
             pixels
```

A projection packet is a target typed boundary, not a new durable store. It
binds the evidence identities, coordinate or embedding basis and revision,
scalar and vector channel semantics, surveyed-validity mask, normalization,
world bounds, layer inventory, effective limits, completeness, omissions, and
projection implementation revision.

The scene compiler turns that packet into stable scene identities and
data-oriented field buffers. The renderer consumes the compiled scene but does
not receive raw workload documents or decide semantic assessment. React owns
route integration, controls, accessibility, and evidence panels; it does not
act as the per-feature simulation or drawing engine.

### High-dimensional boundary

The engine is dimension-agnostic, not dimension-inventing. A provider or
admitted projection operation may map a high-dimensional space into render
coordinates only when it binds:

- the input dimensions and exact source revisions;
- the projection or embedding algorithm and implementation revision;
- parameters, normalization, random seed when applicable, and effective
  limits;
- neighborhood, distance, density, scalar-field, and uncertainty semantics;
- completeness, distortion, unsupported dimensions, and omissions; and
- stable coordinate identity across incremental updates, or an explicit
  projection delta when coordinates move.

The current standalone anchor layout remains an explicitly synthetic
orientation projection. It cannot claim that visual distance is language or
semantic distance.

### Terrain field and validity

Terrain is a bounded multiresolution field, not a collection of contour paths.
Every field channel names its semantics, units or normalization, revision, and
validity. Unknown, surveyed-empty, omitted, stale, unsupported, truncated, and
frontier cells remain distinct from sampled cells. Visual feathering may blend
the edge of a validity mask into the background, but it cannot fill unknown
cells with inferred evidence.

Contours, drainage, erosion, normals, slope, aspect, curvature, shading, and
materials are derived render or simulation channels. Each declares its source
field and implementation revision. Derived natural features remain projections
under ADR 0043 rather than discovered paths or retained natural facts.

### Terrain-fidelity target

“Google Maps-level fidelity” means comparable perceptual terrain legibility and
continuous multiscale composition, not copying Google data, colors, labels, or
proprietary algorithms. The target render graph supports:

1. bounded multiresolution height and validity evaluation;
2. stable normals derived from the height field;
3. multidirectional hillshade rather than one hard light direction;
4. local ambient, ridge, valley, and curvature shading;
5. a restrained evidence-aware base tint or hypsometric material;
6. contour lines whose interval, weight, and opacity vary by semantic LOD;
7. water, weather, boundary, POI, label, selection, and evidence overlays as
   separate ordered passes;
8. antialiasing, device-pixel-ratio awareness, and tonal control; and
9. graceful degradation to a reference renderer with the same scene semantics.

The surface must read continuously before overlays are added. A high-fidelity
shader cannot compensate for a low-resolution field, unstable coordinates, an
unbounded scene, or missing validity information.

### Runtime and renderer boundaries

The engine uses an immutable scene snapshot plus data-oriented field and
transient geometry buffers. A generic entity-component system is deferred until a concrete dynamic
entity workload requires it. Render passes form an explicit directed graph so
dependencies, invalidation, ordering, and budgets are testable.

The renderer runs only when the admitted scene, camera, viewport, active layer,
or presentation setting changes. It does not fabricate animation to appear
alive. Passive revalidation may compile a new scene snapshot; camera state and
selection survive only when their coordinate identities remain valid.

ADR 0045 selects Three.js `WebGPURenderer` and TSL as the production rendering
boundary. WebGPU is preferred; the renderer's WebGL2 backend is the compatibility
path rather than a separate Rey engine. Exact dependency adoption still
requires bounded qualification. A deterministic reference backend remains able
to verify field, scene, ordering, omission, and fallback behavior without
making GPU pixels authoritative evidence.

### Identity, limits, and proof

Projection basis, scene compiler, field simulation, material, render-graph,
LOD, and renderer revisions are distinct. Semantic scene identity excludes
ephemeral camera motion and measured frame time. Changed evidence or a changed
semantic projection invalidates the scene; a changed material invalidates
render proof but not source evidence.

Budgets cover source objects, scene objects, field cells, tiles, field bytes,
GPU or canvas resources, labels, draw calls, compilation time, and frame time.
Reaching a budget exposes degradation and omissions rather than silently
dropping evidence.

Human verification remains CLI-first for exact engine inputs and boundaries
and browser-first for spatial fidelity. The CLI must expose the projection
basis, scene/field/material revisions, validity and LOD bounds, natural-feature
derivations, omissions, and source lineage. Browser proof adds named viewport,
device-pixel-ratio, interaction, screenshot, and performance evidence. A static
image or frame-rate number alone cannot prove semantic correctness.

## Consequences

- Explorer becomes a first-class architectural plane with a narrow evidence
  input and a narrow rendered output instead of a growing React drawing module.
- High-dimensional providers can improve coordinates and fields without
  replacing camera, LOD, picking, or rendering mechanics.
- Terrain fidelity can advance through field resolution and render passes
  without changing the underlying survey claim.
- Backend choice, GPU adoption, and visualization dependencies require explicit
  qualification and fallback evidence.
- Current SVG relief remains an incomplete reference implementation until the
  engine and fidelity plan closes its CLI, browser, and performance proof.
