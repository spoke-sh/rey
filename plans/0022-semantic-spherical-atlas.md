# Plan 0022: Semantic Spherical Atlas And World Globe

- Status: In progress
- Decision: [ADR 0047](../docs/decisions/0047-semantic-spherical-atlas.md)
- Continued by: [Plan 0029](0029-continuous-explorer-grammar.md) for the exact
  globe-to-semantic-Mercator-to-isometric-county grammar
- Extends: [Plan 0017](0017-incremental-context-topography.md), [Plan
  0020](0020-high-fidelity-projection-engine.md), and [Plan
  0021](0021-read-first-scene-editor.md)

## Outcome

Project admitted high-dimensional regions through one revision-bound semantic
sphere so World reveals global topology, Atlas becomes a wraparound chart, and
closer lenses retain local terrain and exact evidence. Keep native identity,
layout, camera, editor candidates, and future route/economic evidence as
separate authorities.

## Completion Checklist

### 1. Establish the spherical atlas contract

- [x] Define `rey.semantic-atlas.v1` with stable region identity, exact source
      patch/revision bindings, integer semantic longitude/latitude, compiler,
      layout policy, limits, completeness, omissions, and lineage.
- [x] State explicitly that the coordinate system is synthetic, has no Earth
      CRS, and makes no physical-distance or broad semantic-similarity claim.
- [x] Recluster only when the retained admitted source set or a source
      topography revision changes; keep zoom and camera state out of identity.
- [x] Implement deterministic bounded k-medoids survey-structure clustering,
      equal-area cluster centers, and polar member placement.
- [ ] Retain prior atlas revisions at the admission transition and emit a typed
      delta for inserted, removed, moved, merged, and split regions.

### 2. Expose human and structured evidence

- [x] Carry the atlas in `rey.workload-list.v1` beside its admitted regional
      topography sources.
- [x] Show atlas revision, region/cluster counts, bounds, compiler, coordinate
      authority, and the no-zoom-reclustering rule in `rey workloads list`.
- [ ] Add a dedicated `rey workloads atlas inspect <revision>` evidence ladder
      if retained atlas history makes the portfolio list too shallow.
- [ ] Expose the directed atlas delta and prior revision through human and JSON
      CLI output once admission retention exists.

### 3. Render the World lens as a globe

- [x] Bind atlas and compiler revisions into the immutable Explorer scene.
- [x] Add a deterministic accessible orthographic reference globe with
      graticule, clusters, admitted region POIs, and exact atlas revision.
- [x] Add a Three.js WebGPU-first globe surface with WebGL2 fallback, bounded
      geometry, lighting, graticule, markers, and resource disposal.
- [x] Suppress local relief, hydrology, frontier points, and flat chart
      boundaries at World when the atlas globe is active; World shows regional
      aggregates rather than every local object.
- [ ] Make drag rotate the semantic globe while preserving ordinary map pan in
      flattened lenses; keep rotation in camera state only.
- [ ] Add front/back hemisphere label culling, collision budgets, selection
      recentering, and smooth globe-to-chart transition evidence.

### 4. Project World into a wraparound semantic-Mercator Atlas

- [ ] Add ADR 0056's antimeridian-safe spherical Mercator transform with
      explicit distortion, polar cutoff/disclosure, horizontal wrap, and the
      strict absence of an Earth/Web-Mercator CRS claim.
- [ ] Place regional relief scenes in local tangent frames derived from their
      admitted atlas positions while preserving all native POI identities.
- [ ] Make Atlas pan wrap horizontally and transition continuously into the
      globe without reclustering or replacing source truth.
- [ ] Derive cluster/region/POI label budgets from retained semantic LOD rather
      than viewport accidents.

### 5. Connect the editor admission loop

- [ ] Extend Plan 0021's admitted vector scene result with the county transform,
      terrain fields, and projection-packet semantics required before making
      an editor package an atlas region.
- [ ] Define how native CRS84, raster terrain, and provider-qualified semantic
      charts enter a local region frame without conflating coordinate systems.
- [ ] Prove editor package → qualified admission → atlas revision → World POI →
      local terrain → exact source voyage through the CLI and browser.

### 6. Add global systems without inventing them

- [ ] Define separate typed contracts for discovered travel routes, constructed
      paths, and time/revision-bound trade flows; source edges remain excluded.
- [ ] Define economic observations independently from visual terrain and bind
      magnitude, direction, time, provider, completeness, and omissions.
- [ ] Add global overlay LOD only after deterministic scenario qualification and
      CLI evidence exist.

### 7. Qualification

- [x] Unit-test atlas determinism, input-revision invalidation, coordinate
      bounds, Earth-CRS rejection, and zoom exclusion.
- [x] Unit-test World scene compilation, reference globe semantics, and Three.js
      globe materialization.
- [ ] Add CLI fixture proof for multiple admitted regions, bounded folding, and
      human/JSON atlas parity.
- [ ] Add retained browser captures and WebGPU/WebGL2/reference parity evidence
      for globe selection and LOD transitions.
- [x] Run `just check` and `just test` for this slice and record exact counts in
      this plan.

## Current Implementation Checkpoint

The first slice creates a portfolio atlas only from the latest verified
topography patch retained for each admitted workload. The derivation is pure,
bounded, content-identified, and visible through the CLI and workload API. It
does not execute a survey or make an editor candidate authoritative.

World now consumes that atlas as a 3D semantic globe. Atlas and closer lenses
retain the existing local tiled relief layout; drag still pans the full scene
rather than rotating the globe. The read model does not yet retain prior atlas
revisions, so a new current revision is reproducible but has no first-class
movement delta. Those are explicit completion gaps, not implied behavior.

Repository-wide evidence for this checkpoint:

- `just check` passed Prettier, TypeScript, 75 UI tests across 24 files, the
  production browser build (including lazy Three.js globe/terrain chunks),
  Rust formatting and workspace Clippy with warnings denied, and Nix flake
  evaluation on x86_64 Linux.
- `just test` passed the same 75 UI tests, 182 Rust tests across 14 binaries,
  and all workspace documentation tests.

## Acceptance Boundary

This plan is complete only when an admission can revise a retained atlas with a
typed movement delta, a human can rotate World and flatten it into a
wraparound Atlas without identity jumps, an admitted editor region can be
traversed end to end, and global route/economic layers appear only from their
own qualified evidence.
