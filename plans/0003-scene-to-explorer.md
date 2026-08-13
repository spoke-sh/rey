# Plan 0003: Admit Scenes And Complete Explorer

- Status: Active
- Owns: scene admission, retained atlas history, World-to-Atlas-to-County
  grammar, projection-engine separation, terrain fidelity, and qualification

## Outcome

Prove one exact path from an editor candidate to admitted regional evidence,
then make the same identity navigable from the CLI through World, semantic
Mercator Atlas, and an isometric County surface. Rendering may improve
perception but may not invent geography, validity, constructed features,
activity, or authority.

```text
editor WORKING → INDEX → SCENE@n package
  → qualified scene-admission workload
  → admitted regional scene + retained atlas delta
  → projection packet + terrain program
  → World globe → semantic Mercator Atlas → County → exact evidence
```

## Current Boundary

The editor generates and validates bounded native CRS84 GeoJSON, stages exact
objects, and retains candidate-only packages. The file-backed
`scene-admission` workload now qualifies deterministic acceptance and rejection
scenarios and the CLI admits one exact current `SCENE@n` transfer envelope into
a retained regional scene and projection packet. The verified
`rey.admitted-regional-scene.v1`,
`rey.regional-projection-packet.v1`, and `rey.explore-grammar.v1` structural
contracts plus a bounded multi-region fixture define that path. `/explore`
now consumes only the latest accepted production result retained by the
workload read model: rejected results, qualification scenarios, candidate
packages, and mismatched workload/graph/package/packet/coordinate bindings
fail closed. It projects the exact synthetic point at World, the same point on
a semantic-Mercator chart at Atlas, and exact native object bounds in a bounded
County reference view. This first County view explicitly retains absent terrain,
atlas sectors, footprints, geometry reconstruction, and deep source links as
boundaries rather than inventing them. Production evidence runs now retain up to
64 verified synthetic atlas revisions and one directed
`rey.semantic-atlas-delta.v1` per revision. Accepted production regional scenes
now enter the same bounded history as separately typed atlas members whose
scene, admission, package revision, projection packet, and exact synthetic
placement are bound explicitly. The delta keeps inserted, removed, moved,
interest-changed, merged, and split states distinct across both evidence
families; qualification fixtures and read-only list/UI access cannot advance
history. Explorer binds a latest delta only when its target is the current
retained atlas and rejects mismatched regional membership. Each retained scene
also carries that atlas revision as a verified non-owning back-reference:
`scene_id` excludes the recursive link while the scene-admission result, run,
and retained state bind it exactly. Stable occupied fixed-grid sectors now bind
explicit region membership without claiming surveyed coverage or native County
footprints. A revisioned browser projection primitive now owns the exact
`360000000µ°` horizontal wrap, `±85051129µ°` cutoff, polar-cap disclosure,
analytic chart inverse, antimeridian fragment identity, shared orthographic
World endpoint, and identity-stable World-to-Atlas interpolation. Regional
Atlas points and sector fragments consume that primitive, and its compiler
revision enters the immutable scene snapshot. The snapshot also retains one
renderer-neutral transition manifest with the same region/focus and sector
identities at both endpoints. The reference renderer uses the grammar's
`0.14 → 0.24` scale band to interpolate those points and bounded sector
vertices continuously across the World/Atlas regime switch; active globe
rotation remains the World endpoint and the accelerated duplicate is withheld
during the transition. At settled Atlas the reference renderer materializes
exactly three bounded horizontal chart copies. Duplicate copies are pointer-only
and accessibility-hidden; inverse picking resolves their projected centers to
one canonical synthetic coordinate and retained region/focus identity. Drag pan
recenters modulo the rendered chart width, keeping horizontal camera state
bounded without changing selection or semantic identity. Explorer implements the fresh
orientation globe, semantic World globe rotation, local relief, procedural
terrain programs, camera-relative transient working sets, continuous TSL
material, WebGPU/WebGL2 paths, and an accessible reference path. The revisioned
deterministic label engine applies the grammar's 70-label World and 96-label
Atlas budgets across globe rotation, morph, and chart copies. Selected canonical
focus wins, then depth/copy priority and identity provide stable ordering;
collided or over-limit labels collapse without removing markers or pick targets.
Its compiler revision enters immutable scene lineage. Admitted County fabric,
render-graph completion, clipmap reuse, and retained visual/performance proof
remain open.

## Completion Checklist

### 1. Close the CLI evidence contract

- [x] Define the admitted regional-scene result and its exact relationship to
  topography patches, atlas revisions, projection packets, terrain programs,
  native objects, transforms, validity/no-data, layers, limits, omissions, and
  lineage.
- [x] Add a versioned Explorer grammar contract for projection posture,
  hysteresis, morphing, semantic/geometric LOD budgets, picking, and
  renderer-independent camera constraints.
- [x] Make workload human/JSON output distinguish native CRS84, synthetic
  semantic, Mercator chart, County-local, and camera coordinates.
- [x] Add one bounded multi-region fixture with accepted/rejected candidates,
  polar and antimeridian cases, overlapping footprints, and typed County
  features.

### 2. Admit one scene package

- [x] Add a file-backed `scene-admission` workload with deterministic scenarios
  for package/object tampering, stale parents, unsupported formats, coordinate
  mismatch, duplicate identity, missing objects, bounds, omissions, and replay.
- [x] Qualify one exact `SCENE@n` package, retain the admitted regional result,
  and emit a verified projection packet without copying candidate-only hints
  into observed truth.
- [x] Make `/explore` consume only that admitted result; keep candidate preview,
  browser generation, and browser admission absent or persistently separate.
- [x] Prove editor WORKING → INDEX → package → workload INDEX → qualification
  → human HEAD admission → Explorer through `rey ... -vv` before calling the
  slice complete.

### 3. Retain and project atlas change

- [x] Retain prior survey-atlas revisions at production admission and emit inserted, removed,
  moved, merged, split, and interest-change deltas.
- [x] Bind accepted regional scenes into the retained atlas without changing
  native/package identity or synthetic placement.
- [x] Record the exact admitted atlas revision back on each regional scene
  without creating circular scene/atlas content identity.
- [x] Add stable synthetic sector polygons and explicit region membership;
  keep sectors separate from admitted County footprints.
- [x] Implement one revisioned spherical-Mercator primitive with horizontal
  wrap, latitude cutoff, polar disclosure, antimeridian-safe draw fragments,
  analytic inverse coordinates, and identity-stable World-to-Atlas endpoints;
  consume it for regional Atlas points and sectors.
- [x] Retain one immutable renderer-neutral transition manifest and present its
  exact region/focus and sector identities continuously across the declared
  World-to-Atlas morph band.
- [x] Drive three bounded chart copies through renderer drawing, canonical
  inverse picking, pointer-only duplicate accessibility, and modulo recentering
  without replacing focus or semantic identity.
- [x] Preserve focus, selection, and one semantic identity through globe
  rotation, deterministic label culling, collisions, and camera changes.

### 4. Enter a bounded County

- [ ] Require one selected admitted footprint and expand its revision-bound
  local tangent frame into the bounded isometric camera.
- [ ] Add independently typed terrain, hydrology, boundary, highway, road,
  district, lot, structure, utility, POI, label, beacon, construction, and
  connector layers only where admitted evidence supports them.
- [ ] Deep-link every selected object to its native source, admission result,
  revision, delta, validity, limits, omissions, and lineage.
- [ ] Extend deterministic editor authoring only after the admission path can
  preserve the resulting native identity and reviewable change set.

### 5. Complete the projection engine and terrain

- [ ] Finish evidence-adapter, immutable-scene, camera, invalidation,
  render-graph, renderer, picking, and React ownership separation.
- [ ] Replace full CPU window rebuilds with bounded crack-free transient
  patches or clipmaps, including stable absolute sampling, hydrology halos,
  seam behavior, cache keys, and byte/GPU budgets.
- [ ] Qualify GPU height/normal/material evaluation against deterministic CPU
  reference samples and preserve visible fallback on context/device loss.
- [ ] Compose LOD-aware terrain, contours, hydrology, validity boundaries,
  features, labels, collision/culling, selection, and accessibility while
  keeping the base landform independently legible.

### 6. Qualify the complete voyage

- [ ] Retain named World → Atlas → County → Evidence voyages at 1920×1080 and
  3840×2160 through WebGPU, forced WebGL2, and the reference renderer.
- [ ] Prove focus retention, no hidden survey/admission, unknown masks,
  pole/antimeridian behavior, footprint collision, terrain transitions,
  stable picking, context loss, last-good scene, and exact links.
- [ ] Record compilation, residency, evaluation/upload, draw, geometry, label,
  memory, frame-time, and interaction budgets on a named reference machine
  before making fidelity or frame-rate claims.
- [ ] Pass CLI output/exit fixtures, browser accessibility and parity tests,
  `just check`, `just test`, embedded assets, packaged Nix, and cargo-dist
  release checks.

## Deferred

Unrestricted free orbit, first-person navigation, physics, multiplayer, a
general ECS, inferred routes/trade/economics, automatic browser mining or
admission, proprietary map inputs/styles, and unqualified GeoPackage,
GeoTIFF/COG, DEM, or Arrow adapters remain outside this plan.
