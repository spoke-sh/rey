# Plan 0020: High-Fidelity Projection Engine

- Status: In progress
- Decisions: [ADR 0044](../docs/decisions/0044-explorer-projection-engine.md),
  [ADR 0045](../docs/decisions/0045-threejs-webgpu-renderer.md)
- Extends: [Plan 0017](0017-incremental-context-topography.md) and [Plan
  0019](0019-emergent-context-features.md)

## Outcome

Turn the current Explorer read model into a logically separated,
evidence-bound projection engine and raise terrain rendering from SVG contour
linework to a continuous, Google-class 2.5D relief surface. Preserve semantic
identity, surveyed validity, omissions, deep evidence links, and read-only
navigation through every fidelity improvement.

## Observed Gap

The supplied 2026-08-11 3840×2160 Rey capture proves the current implementation
can place stable anchors, contours, water, weather, and boundaries in one
semantic World scene. It also shows a mostly uniform plane, a small contour
island, no continuous normal or light field, little ridge/valley separation,
and no multiscale surface material. The supplied 3022×1926 Google Maps terrain
capture shows continuous hillshade, occlusion, tint, hydrology, contours,
labels, and overlays remaining legible together across the viewport.

The target is comparable terrain legibility and compositional fidelity using
Rey's own admitted evidence and visual language. The reference screenshots are
design evidence only and must not be copied into repository fixtures or treated
as a license to reproduce proprietary map data or styling.

## Target Code Shape

The current `topology.ts` (~2,300 lines), `explore.tsx` (~1,000 lines), and
Explorer StyleX module (~800 lines) mix source adaptation, scene derivation,
simulation, camera mechanics, DOM/SVG rendering, labels, and material styling.
The hard-cut target is:

```text
apps/rey-ui/src/explore/
  contracts.ts                 projection packet + scene identities
  projection/
    portfolio-projector.ts     typed workload fallback adapter
    topography-projector.ts    admitted patch adapter
    validity.ts                surveyed/unknown/omitted masks
  engine/
    camera.ts                  world/screen transforms and camera invariants
    scene.ts                   immutable scene compiler and stable ordering
    fields.ts                  bounded scalar/vector grids and tiles
    invalidation.ts            evidence/camera/material dirty sets
    lod.ts                     semantic and geometric LOD budgets
    render-graph.ts            ordered pass dependencies and resources
    picking.ts                 screen hit → exact scene coordinate
  terrain/
    elevation.ts               anchor or provider height channels
    hydrology.ts               rainfall, flow accumulation, erosion
    normals.ts                 slope/aspect/curvature derivation
    materials.ts               tint and shading parameters
  renderers/
    reference.ts               deterministic semantic fallback
    three-webgpu.ts            Three.js WebGPURenderer + TSL adapter
  react/
    explore-page.tsx           route and evidence-bound shell
    controls.tsx               camera/layer/fullscreen controls
    overlays.tsx               accessible labels and evidence panels
```

Names may tighten during extraction, but ownership may not collapse back into
one topology or React module. The engine uses an immutable scene graph and
data-oriented field buffers; do not introduce a generic ECS without a qualified
dynamic-entity requirement.

## Completion Checklist

### 1. Freeze contracts and current behavior

- [x] Accept ADR 0044 and establish the projection-engine identity, evidence
  boundary, 2.5D scope, and terrain-fidelity target.
- [x] Define `rey.projection-packet.v1` as a browser/CLI typed
  contract binding evidence, projection basis, fields, validity, layers,
  revisions, limits, completeness, omissions, and lineage.
- [x] Add a synthetic admitted terrain fixture with named World, Atlas,
  Landscape, and Neighborhood view envelopes; do not use external map imagery
  or unbound high-dimensional coordinates.
- [ ] Record current semantic scene manifests and bounded visual baselines
  before moving code, including unknown-mask and no-source-edge invariants.

### 2. Extract the engine without visual drift

- [ ] Split portfolio and admitted-topography adapters from the generic engine
  contracts.
- [ ] Extract camera transforms, semantic-regime hysteresis, focus retention,
  fit bounds, and picking into framework-independent modules.
- [ ] Compile one immutable, stably ordered scene snapshot whose identity
  excludes camera state and whose objects retain exact evidence links.
- [ ] Replace ad hoc rerender coupling with explicit evidence, scene, camera,
  material, label, and viewport invalidation sets.
- [x] Keep the existing SVG/DOM result as the first reference renderer until
  scene and camera parity tests pass.

### 3. Build bounded multiresolution terrain

- [ ] Replace nested-number grids with typed scalar/vector buffers and an
  explicit per-cell surveyed-validity mask.
- [ ] Build a bounded tile pyramid or equivalent multiresolution field whose
  LOD changes do not move stable coordinates or fill unknown cells.
- [ ] Separate elevation, rainfall, flow, erosion, normal, curvature, and
  material channels with independent implementation revisions.
- [ ] Preserve deterministic hydrology and natural-feature semantics from ADR
  0043 while testing that erosion changes relief but never source assessment.
- [ ] Expose cell, tile, byte, compilation-time, and omission budgets in both
  structured output and the human CLI.

### 4. Qualify the renderer boundary

- [ ] Implement a renderer interface over immutable scene snapshots and an
  explicit render graph.
- [x] Pin Three.js `0.185.1` and add a narrow lifecycle adapter that proves
  asynchronous initialization, preferred WebGPU selection, forced WebGL2
  selection, viewport bounds, failure, and disposal without replacing the live
  reference surface.
- [ ] Qualify a pinned Three.js `WebGPURenderer` and TSL dependency across its
  preferred WebGPU and forced-WebGL2 paths for bundle size, browser support,
  asynchronous initialization, resource ownership, accessibility, determinism,
  performance, and Nix packaging.
- [x] Record the selected production boundary in ADR 0045: Three.js owns GPU
  mechanics behind a narrow adapter while Rey retains its semantic scene and
  deterministic reference path.
- [ ] Move high-cardinality terrain and natural-feature drawing out of React
  elements while keeping accessible labels, controls, status, and exact links
  in the React shell.
- [ ] Handle device-pixel ratio, resize, context loss, resource disposal, and
  fallback without losing the last good scene or evidence boundary.

### 5. Reach continuous terrain fidelity

- [ ] Render a continuous base terrain material before contours and overlays.
- [ ] Add height-gradient normals, multidirectional hillshade, ambient/valley
  occlusion, ridge/curvature enhancement, and restrained evidence-aware tint as
  separately testable passes.
- [ ] Make contour interval, weight, labeling, and opacity scale with semantic
  and geometric LOD instead of scaling one SVG path uniformly.
- [ ] Composite water, weather, validity boundaries, POIs, selection, labels,
  and evidence overlays in an explicit order with redundant non-color meaning.
- [ ] Add bounded label collision, decluttering, culling, and stable picking so
  terrain remains legible as object density grows.
- [ ] Blend unexplored and unsupported validity edges into the background
  visually while retaining their exact masks and disclosures.

### 6. Prove semantic, visual, and performance behavior

- [ ] Extend `rey workloads ... -vv` with a human-readable projection-engine
  block covering basis, scene, field, material, LOD, validity, limits,
  omissions, and lineage before relying on browser diagnostics.
- [ ] Add golden tests for scalar fields, masks, normals, hydrology, erosion,
  render-pass ordering, picking, LOD transitions, and deterministic scene
  manifests.
- [ ] Add browser tests for stable focus, no LOD popping, unknown masking,
  context loss/fallback, layer composition, accessible controls, and exact
  evidence links.
- [ ] Capture and inspect named 1920×1080 and 3840×2160 World → Neighborhood
  voyages over the synthetic and retained-project fixtures.
- [ ] Define and preserve a named performance result before making a frame-rate
  claim: warm camera interaction should target 60 Hz on the declared reference
  machine, with field compilation, tile memory, draw-call, label, and frame
  budgets reported separately.
- [ ] Verify the packaged Nix build, embedded assets, CLI/browser parity, and
  zero-Spoke fallback through `just check` and `just test`.

## Acceptance Boundary

This plan is complete only when:

1. an agent can inspect the exact projection inputs and limitations through the
   established workload CLI;
2. a human can traverse the same retained scene in `/explore` and perceive a
   continuous shaded terrain surface rather than contour lines on a plane;
3. World, Atlas, Landscape, and Neighborhood preserve coordinate identity and
   do not pop, invent terrain, or expose source edges as geography;
4. renderer loss or unavailable acceleration degrades visibly to the reference
   path without changing semantic assessment; and
5. fidelity and performance claims cite named fixtures, viewports, hardware,
   revisions, limits, and retained comparison results.

## Explicit Deferrals

- free-orbit 3D, pitch, volumetric rendering, physics, and multiplayer state;
- a generic ECS, asset marketplace, editor, scripting runtime, or plugin API;
- a universal high-dimensional embedding algorithm;
- inferred terrain outside surveyed validity;
- browser-triggered probes, mining, path construction, or action admission;
- proprietary map data, styles, or algorithms; and
- WebGPU-only behavior without the Three.js WebGL2 compatibility path, Rey's
  deterministic reference path, and a qualified deployment contract.
