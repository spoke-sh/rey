# Plan 0021: Read-First Scene Editor

- Status: In progress
- Decision: [ADR 0046](../docs/decisions/0046-read-first-scene-editor.md)
- Extends: [Plan 0017](0017-incremental-context-topography.md) and [Plan
  0020](0020-high-fidelity-projection-engine.md)

## Outcome

Give agents, surveys, and eventually humans a bounded level-editor plane for
authoring terrain, feature, marker, label, hydrology, boundary, and material
inputs without turning `/explore` into a mutation surface. Freeze exact native
artifacts into immutable scene packages, admit them only through qualified
workloads, and project the admitted result through the same CLI-visible
topography and projection contracts consumed by Explorer.

## Completion Checklist

### 1. Freeze the editor and admission boundary

- [x] Accept ADR 0046 and separate editor candidates from topography patches,
      projection packets, browser scene state, and admission authority.
- [x] Define `HEAD → INDEX → WORKING` as editor comparison state while stating
      that each `SCENE@n` commit's package remains an unadmitted candidate.
- [x] Define `rey.editor-project.v1`, `rey.scene-candidate-snapshot.v1`,
      `rey.scene-change-set.v1`, `rey.scene-package.v1`, and
      `rey.scene-admission-request.v1`.
- [x] Keep `/explore` unchanged when a package is created and require a later
      explicit workload result before the browser can consume it.
- [ ] Define the admitted scene-layer result and its exact relationship to
      `rey.topography-patch.v1` and `rey.projection-packet.v1` without creating
      a second projection authority.

### 2. Establish the agent-first editor CLI

- [x] Keep one authoring entry point: `rey editor generate terrain` bootstraps
      the project and emits tunable native WORKING artifacts; agents may then
      fine-tune those exact bytes directly.
- [x] Keep the project declaration in the selected Rey state store
      (`.rey/editor/project.json` by default), remove the ambient
      `rey.scene.json`/`--project` contract, and make uninitialized `status` a
      read-only successful observation with absent WORKING.
- [x] Implement only the public revision loop `generate`, `status`, `diff`,
      `add`, `commit`, and `log` with human and structured output; remove the
      redundant public `init`, `import`, and `validate` surfaces.
- [x] Keep human `status` Git-shaped and concise, expose immutable
      commit/package evidence through `log`, retain complete typed state through
      JSON, and render validation evidence as part of successful `commit`.
- [x] Reject workspace escapes, symlinked inputs, non-files, malformed JSON,
      custom GeoJSON CRS declarations, unstable feature identities, invalid
      geometries, out-of-range coordinates, and explicit bounds violations.
- [x] Make `add` freeze exact verified native objects and make `commit`
      revalidate only the staged INDEX rather than ambient WORKING state,
      failing without advancing HEAD when the gate does not pass.
- [x] Retain linear `SCENE@n` history with messages, timestamps, parents,
      immutable packages, requests, and optional log patches.
- [ ] Add partial feature/source staging after a CLI design can preserve native
      artifact integrity without inventing a second editable geometry store.
- [ ] Add an explicit restore/reset workflow only with clear treatment of
      user-authored project files and retained content-addressed objects.

### 3. Support standard vector and marker interchange

- [x] Implement bounded RFC 7946 Feature/FeatureCollection ingestion for Point,
      MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon, and
      GeometryCollection geometry in OGC CRS84.
- [x] Require stable source and feature identities; retain native bytes and
      content/property/feature revisions rather than flattening GeoJSON into a
      replacement table.
- [x] Index explicit feature, marker, terrain-control, hydrology, and boundary
      roles without interpreting ordinary lines as paths or source edges.
- [x] Index bounded marker title/category/symbol, zoom interval, and collision
      priority for later POI and label qualification.
- [ ] Add fixtures from representative standards-compliant OSM-derived GeoJSON
      while retaining the exporter and original relation limitations.
- [ ] Qualify a GeoPackage adapter for multiple vector layers and retained
      layer/geometry metadata.

### 4. Add detailed terrain and semantic-chart sources

- [x] Add a deterministic terrain-control generator with retained seed, CRS84
      bounds, geometry/effect hyperparameters, overwrite ownership, and exact
      replay through the ordinary WORKING → INDEX → HEAD loop.
- [ ] Specify a Rey-native multiresolution terrain manifest that references
      exact typed height, validity, normal/material, and optional provider field
      artifacts without embedding large arrays in JSON.
- [ ] Qualify GeoTIFF/Cloud Optimized GeoTIFF elevation, no-data/validity, CRS,
      units, overview, tile, and byte semantics.
- [ ] Qualify typed Arrow feature/property artifacts where tabular semantics are
      genuine, while keeping raster and native geometry bytes outside frames.
- [x] Define provider-qualified semantic chart coordinates separately from
      geographic CRS84: ADR 0047 uses namespaced synthetic semantic
      longitude/latitude with no Earth CRS and retains the native source CRS.
- [ ] Preserve source-native spatial indexes or build reproducible bounded
      indexes with exact implementation revisions and omissions.

### 5. Close the admission loop

- [ ] Add a workspace `scene-admission` workload package with deterministic
      scenarios for package/object tampering, unsupported formats, coordinate
      mismatch, duplicate identity, missing objects, limits, omissions, stale
      parents, and deterministic replay.
- [ ] Expose package inputs, operation/provider/implementation revisions,
      capability snapshot, limits, progress, result, directed delta, lineage,
      and omissions through `rey workloads test|run|status ... -vv`.
- [ ] Emit one admitted scene-layer/topography result whose exact identity is
      retained by the workload store and projected into a verified
      `rey.projection-packet.v1`.
- [ ] Make `/explore` consume that retained result through its evidence adapter;
      panning, zooming, selection, and opening a link remain read-only.
- [ ] Show editor-origin provenance at Evidence scale without exposing the
      package's candidate-only hints as observed semantic truth.

### 6. Build a human editor without weakening authority

- [ ] Start any browser editor from an explicit `rey` CLI exposure and bind it
      to the same project/snapshot/change-set/package contracts.
- [ ] Use a distinct route and persistent `UNADMITTED` treatment for candidate
      previews; never layer WORKING or INDEX directly into `/explore`.
- [ ] Provide map-grade feature selection, layer ordering, POI/label preview,
      validation, diff review, staging, and commits while retaining exact
      CLI parity and keyboard/accessibility behavior.
- [ ] Keep the browser from executing admission workloads unless a separate
      authenticated, authorized action contract is accepted.

### 7. Proof and qualification

- [x] Add unit proof that native bytes are frozen at staging, later WORKING
      changes do not alter retained commits/packages, history remains linear,
      and missing feature IDs fail closed.
- [x] Add CLI proof covering generate → agent fine-tune → status → add →
      commit-time validation → log → working diff, including rejection of
      the removed commands, human evidence, JSON contracts, stderr, and exit
      behavior.
- [ ] Add malformed geometry nesting, every geometry family, duplicate IDs,
      source/feature/property/coordinate/byte limits, symlink, tampering,
      historical package, and concurrent writer fixtures.
- [ ] Prove one full editor → admission workload → projection packet →
      `/explore` voyage with exact source links and no candidate-authority leak.
- [ ] Run `just check` and `just test` after the admission slice and retain the
      exact end-to-end command evidence in this plan.

## Current Implementation Checkpoint

The first slice is intentionally incomplete enabling work. The `rey` crate now
owns a dependency-light editor module and CLI. A JSON editor project declares
bounded GeoJSON sources from `.rey/editor/project.json`; native authored
sources remain workspace files. `generate terrain` bootstraps that internal
project and a deterministic native source, after which an agent may fine-tune
WORKING bytes. An untouched workspace reports an uninitialized editor without
creating local state.
Observation constructs a deterministic feature and POI index while retaining
exact source bytes as native artifacts. `add`
publishes those artifacts and the snapshot atomically under `.rey/editor`;
`commit` revalidates only the frozen INDEX, advances linear `SCENE@n` history,
retains its immutable package and directed change set, then emits a separate
request whose status is `requires_workload` and whose `admitted` field is
false. The embedded generation recipe reproduces the base output; exact later
agent edits remain bound by the source digest and scene delta.

No scene-admission workload exists yet. The current `context-anchor-survey`
remains the only executable producer of admitted topography, and creating an
editor package does not alter the workload store, projection packet, UI API, or
`/explore`. GeoPackage, GeoTIFF/COG, Arrow, raster terrain, semantic charts,
partial staging, and browser editing remain explicitly unsupported.

Repository-wide evidence for this checkpoint:

- `just check` passed UI formatting/typechecking, 79 UI tests, the production
  browser build, Rust formatting and workspace Clippy with warnings denied, and
  Nix flake evaluation on x86_64 Linux.
- `just test` passed the same 79 UI tests, 196 Rust tests across 14 binaries,
  and all workspace documentation tests.

## Acceptance Boundary

This plan is complete only when an agent can create and inspect a detailed
scene candidate through `rey editor`, a qualified workload can admit or reject
that exact package with complete CLI evidence, and a human can traverse the
same admitted result in `/explore` without any browser navigation or editor
preview widening read or action authority.
