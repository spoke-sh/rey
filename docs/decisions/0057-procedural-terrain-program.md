# ADR 0057: Procedural Terrain Program And Transient Working Sets

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0044](0044-explorer-projection-engine.md), [ADR
  0045](0045-threejs-webgpu-renderer.md), and [ADR
  0056](0056-continuous-globe-mercator-county-grammar.md)
- Supersedes: ADR 0056's native terrain-tile-pyramid, tile-residency, and
  raster-first wording

## Context

The first terrain renderer retained three complete CPU field grids, selected
one by semantic regime, built one mesh, rendered one frame, and allowed CSS to
scale the resulting canvas. That proved typed fields and the Three.js
WebGPU/TSL boundary, but it treated level of detail as stored map data rather
than engine evaluation. It also coupled frequency detail to six semantic
regime thresholds and spent memory on off-camera samples.

Rey's admitted artifacts describe an abstract world. The engine should
materialize the current view from those artifacts rather than require an agent
to render, package, and admit a set of image or height tiles. Imported raster
terrain can be useful source evidence, but it is not Rey's native terrain
model.

## Decision

The projection packet carries a bounded `rey.terrain-program.v1`, not a
terrain-field pyramid. The program binds:

- one deterministic evaluator identity and seed;
- macro, meso, and micro frequency bands with wavelength, amplitude, octave,
  sampling, and authority declarations;
- absolute-coordinate evaluation, validity, and detail-selection rules;
- a maximum transient working-set shape, cell/byte allocation, target
  screen-space sample spacing, overscan, and recenter rule; and
- the existing independent field-channel implementations and source lineage.

The browser compiles admitted anchor samples, atmosphere, bounds, and the
packet into an immutable terrain program. Camera center, scale, pan, viewport,
and device state remain outside that program. For each view, the engine snaps a
bounded working-set envelope to absolute scene coordinates, selects only
frequency bands that the sampling density can represent, evaluates the fields,
and renders the derived geometry. Moving or zooming the camera may replace all
derived buffers without changing semantic scene identity.

Generated field buffers, GPU resources, mesh patches, caches, and future
clipmap rings are disposable projection state. They are not admitted artifacts
and must not be the only copy of authored or observed data. Their cache key
must include the exact terrain program, coordinate envelope, sample spacing,
and implementation revisions.

Macro relief remains a deterministic projection of admitted anchor influence.
Meso modulation may improve derived ridge and valley legibility only inside
that support. Micro detail is presentation-only unless a future admitted
source grants it stronger authority. No band may expand validity, invent a
semantic relationship, change source assessment, or turn unknown space into
terrain.

The current CPU evaluator is the deterministic reference path. The production
order is:

1. prove the terrain-program and camera-working-set contract through the CLI,
   structured output, reference evaluator, and browser;
2. keep the canvas in viewport space and drive its orthographic/isometric
   camera directly from Explorer camera state;
3. replace whole-working-set rebuilds with camera-centered geometry clipmaps
   or equivalent crack-free transient patches;
4. evaluate height, normals, material, and microdetail through the pinned
   WebGPU/TSL graph, retaining reference samples and tolerances for backend
   qualification; and
5. add hydrology, constructed features, contours, labels, and picking as
   independently admitted or derived layers.

GeoTIFF, COG, DEM, and related standard formats remain optional bounded import
sources. An adapter may sample them or compile control fields into a terrain
program, while preserving the native object and lineage. They do not require
Rey to publish or render persistent tiles.

## Current Implementation Boundary

`rey.projection-packet.v1` now retains `rey.terrain-program.v1` with three
declared bands and a 255×255 / 65,025-cell / 3,576,375-byte maximum transient
working set. The CLI exposes the evaluator, seed, bands, working-set budget,
authority, and recenter rule. `/explore` materializes a camera-derived field
window, keeps the accelerated canvas in viewport space, and drives a Three.js
orthographic camera from the same pan and scale as the Explorer lens.
The packaged `rey ui` server now embeds and serves stable-name globe, terrain,
WebGPU-adapter, and Three.js renderer chunks, so this accelerated path is
reachable through the required CLI surface rather than only a Vite preview.

This is a contract and camera-loop slice, not Google-class completion. The
field evaluator still runs on the CPU, regenerates a whole working set after a
snapped camera change, builds one mesh, and performs one render for that view.
Clipmap reuse, WebGPU field evaluation, continuous isometric projection,
qualified shadows, feature composition, visual baselines, and retained frame
budgets remain open in [Plan
0029](../../plans/0029-continuous-explorer-grammar.md).

## Consequences

- Admitted artifacts remain compact descriptions and exact native features;
  generated render data remains disposable.
- Optical zoom can reveal finer surface structure continuously without
  changing semantic regime or loading an authoritative tile level.
- Absolute-coordinate sampling makes terrain stable across camera movement and
  gives seam and backend parity tests an exact reference.
- Runtime cost is explicit and bounded by visible working-set cells and bytes,
  not by the area of the admitted world.
- Neighborhood-dependent hydrology and erosion require a declared evaluation
  halo or separately admitted global control source before transient patching
  can replace the current whole-window reference calculation.
- Renderer implementations may optimize evaluation, but neither WebGPU nor a
  cache becomes semantic authority.
