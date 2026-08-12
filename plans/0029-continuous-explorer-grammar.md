# Plan 0029: Continuous Explorer Projection Grammar

- Status: In progress
- Decisions: [ADR
  0056](../docs/decisions/0056-continuous-globe-mercator-county-grammar.md),
  [ADR 0057](../docs/decisions/0057-procedural-terrain-program.md), and [ADR
  0058](../docs/decisions/0058-consent-first-explorer-orientation.md)
- Extends: [Plan 0020](0020-high-fidelity-projection-engine.md), [Plan
  0021](0021-read-first-scene-editor.md), [Plan
  0022](0022-semantic-spherical-atlas.md), and [Plan
  0023](0023-fresh-v1-rey-county.md)

## Outcome

Make `/explore` one continuous, evidence-bound spatial language: a COBE-class
World globe unwraps into a semantic Mercator Atlas, then an admitted region
opens into a high-fidelity isometric county surface. Preserve semantic focus,
admission authority, validity, omissions, and exact source lineage while
projection geometry and visible layer density change.

## Current Gap

World currently renders a lit sphere but drag still pans the scene. Atlas and
closer regimes use a flat local relief layout rather than a qualified Mercator
chart, and there are no stable sector polygons or globe-to-chart morph. The
editor produces candidate-only CRS84 GeoJSON; no scene-admission workload can
turn a package into an admitted county. The accelerated terrain proves typed
fields and a TSL material boundary. Projection packets now carry procedural
frequency bands and a bounded camera working-set contract instead of stored
terrain levels, but evaluation remains CPU-bound and still lacks reusable
clipmap geometry, cartographic layers, or the isometric county scene needed for
the reference fidelity.

The fresh-user bearing now has a separate implemented pre-survey posture:
`/explore` renders an unmapped orientation globe from exact request,
HEAD/INDEX/WORKING workload file state. Workload beacons direct the operator
to exact inspection and consensual qualification/admission without claiming an
atlas, terrain, semantic distance, or active agent work. The initial
`context-anchor-survey` beacon closes the visual handoff into the existing Feed
admission control. Running the admitted survey and advancing from retained
topography to an admitted semantic atlas remain explicit later steps.

## Completion Checklist

### 1. Freeze the projection grammar contract

- [x] Accept ADR 0056 and distinguish projection posture from semantic detail.
- [x] Define World globe, semantic Mercator Atlas, and local isometric County
      as the three continuous geometric postures.
- [x] Define the synthetic longitude/latitude, Mercator cutoff/wrap, polar-cap
      disclosure, antimeridian split, county-local tangent frame, and native
      CRS separation.
- [ ] Add a versioned `rey.explore-grammar.v1` contract containing posture
      bands, hysteresis, morph policy, semantic LOD layer budgets, picking
      policy, and renderer-independent camera constraints.
- [ ] Hard-cut internal regime names only where required; preserve the public
      six-step World/Atlas/Landscape/Neighborhood/Object/Evidence evidence
      ladder while giving each step an exact projection posture.

### 2. Close the CLI evidence path first

- [x] Make `/explore` the `rey ui` human entry and replace the clean-state
      portfolio-card fallback with an unmapped orientation globe whose exact
      workload beacons retain request/WORKING/INDEX/admitted state, source,
      digest, producer, mapping role, and next consent step.
- [x] Keep orientation coordinates presentation-only, keep all pre-survey lens
      levels on that World posture, and route inspection and consent to the
      exact workload record and Feed admission control without executing work.
- [ ] Extend the existing workload human/JSON projection to inspect the exact
      atlas, sector, scene-admission, projection-grammar, terrain-program,
      layer, LOD, validity, limit, omission, and lineage identities.
- [ ] Add a named multi-region fixture with a staged editor county, a rejected
      package, an admitted package, polar-cap content, an antimeridian sector,
      colliding footprints, and a county with roads/lots/artifacts.
- [ ] Make the CLI distinguish native CRS84 coordinates, synthetic semantic
      coordinates, semantic Mercator chart coordinates, and county-local
      coordinates without flattening them into one longitude/latitude claim.
- [ ] Prove stdout, stderr, JSON, human verbosity, and exit semantics before
      treating any browser lens as complete.

### 3. Retain atlas sectors and revision deltas

- [ ] Extend `rey.semantic-atlas.v1` or hard-cut its successor with stable
      sector identities, deterministic spherical partition polygons, explicit
      region membership, interest inputs, bounds, limits, and omissions.
- [ ] Retain atlas revisions at admission time and emit inserted, removed,
      moved, merged, split, and interest-changed deltas.
- [ ] Keep synthetic sector boundaries separate from admitted county
      footprints and disclose both in CLI and browser picking.
- [ ] Split antimeridian geometry only in the projection layer and preserve one
      sector/region identity across all draw fragments.

### 4. Reach a COBE-class World experience

- [x] Move World drag from planar pan to direct globe rotation while keeping
      rotation and camera framing outside semantic identity. Inertial release
      and optional idle motion remain future presentation work.
- [ ] Render bounded sector surfaces, admitted-region markers, lighting,
      atmosphere, antialiasing, front/back culling, collision-managed labels,
      selection recentering, and visible validity/omission state through the
      Three.js WebGPU/TSL graph. Dense deterministic stipple, atmosphere,
      lighting, marker occlusion, and shared reference/WebGPU projection are
      implemented; sectors, collision management, and recentering remain.
- [ ] Keep the reference globe keyboard navigable and semantically equivalent;
      respect reduced motion and never require automatic spin.
- [ ] Refuse untyped arcs. Routes, trade, and travel remain absent unless their
      own qualified admitted layers exist.

### 5. Unwrap World into semantic Mercator

- [ ] Implement the exact spherical Mercator transform, horizontal wrap,
      latitude cutoff, polar-cap disclosure/insets, and bounded inverse picking.
- [ ] Morph sector and region geometry from sphere to chart without changing
      stable identity or the semantic coordinate under the pointer.
- [ ] Raise hovered or keyboard-focused sectors as transient presentation,
      expose the exact interest basis, and keep selection stable through the
      morph.
- [ ] Add screen-space-error and semantic-LOD budgets for sector polygons,
      county footprints, coarse terrain, major hydrology/connectors, POIs, and
      labels.
- [ ] Stop at Atlas with an explicit missing-admission state when no admitted
      county exists under focus.

### 6. Admit editor packages as regional fabric

- [ ] Implement and scenario-qualify the scene-admission workload from Plan
      0021. It must bind one exact package, native objects, coordinate
      transforms, normalized layers, validity/no-data, limits, omissions, and
      an admitted regional-scene identity.
- [ ] Define and implement the Rey-native abstract-scene manifest so semantic
      longitude/latitude and county-local coordinates do not misuse RFC 7946
      CRS84 positions.
- [x] Hard-cut survey projection packets to a bounded
      `rey.terrain-program.v1` with deterministic evaluator, frequency bands,
      absolute-coordinate and validity rules, and a transient working-set
      budget.
- [ ] Admit standard vector/container formats behind explicit adapters and
      treat GeoTIFF/COG/DEM as optional native sources that a bounded adapter
      can compile into terrain controls; preserve native files without making
      persistent tiles Rey's terrain model.
- [ ] Project only admitted regional scenes into Atlas and County. Candidate
      preview, if added, must use a separate persistently marked surface.
- [ ] Prove editor WORKING → INDEX → SCENE@n → staged workload → qualified
      workload commit → atlas delta → browser region through exact CLI output.

### 7. Reach Google-class terrain legibility

- [x] Derive a bounded camera-relative field window from the admitted terrain
      program, snap it to absolute scene coordinates, select representable
      macro/meso/micro bands, and expose its exact current and maximum
      cell/byte allocations.
- [x] Keep the accelerated canvas in viewport space and drive its
      orthographic terrain camera from Explorer pan, scale, viewport, and
      pointer-centered zoom instead of scaling a pre-rendered scene canvas.
- [ ] Replace whole-window CPU rebuilds with crack-free camera-centered
      geometry clipmaps or equivalent transient patches, including snapped
      recentering, transition fixups/geomorphing, an explicit hydrology halo,
      invalid support, GPU and byte budgets, and disposable cache keys.
- [ ] Port absolute-coordinate height, normal, and material evaluation into
      the pinned WebGPU/TSL graph while retaining deterministic CPU reference
      samples and backend tolerance qualification.
- [ ] Separate and qualify height, normal, slope, aspect, curvature, roughness,
      tint, wetness, occlusion, material, contour, and validity channels.
- [ ] Compose multidirectional hillshade, ridge/valley legibility, qualified
      shadows, restrained atmospheric perspective, LOD-aware contours,
      hydrology, land cover, and unknown-space blending before feature labels.
- [ ] Add stable culling, collision, picking, and independent vector LOD for
      boundaries, highways, roads, lots, structures, POIs, and labels.
- [ ] Demonstrate that the base landform remains readable with contours,
      roads, POIs, and labels disabled.

### 8. Enter the isometric county surface

- [ ] Require an exact selected/admitted county before Atlas-to-County entry;
      resolve footprint collisions explicitly.
- [ ] Expand the selected region into its local east/north/up frame and blend
      from the flat chart to a stylized isometric camera without focus jumps or
      terrain LOD pops.
- [ ] Add Landscape-level county boundary, terrain, watershed, highway,
      district, major-road, and landmark grammar.
- [ ] Add Neighborhood-level local roads, lots, structures, utilities,
      admitted beacons, construction, unresolved stations, and label grammar.
- [ ] Add Object/Evidence picking and deep links from every rendered artifact
      to its native object, admission result, exact revision, delta, validity,
      limits, omissions, and lineage.
- [ ] Treat cross-county highways as typed connector pairs, beacons as admitted
      observations rather than implied execution, and construction as explicit
      admitted state rather than decorative activity.

### 9. Author county detail through the editor loop

- [ ] Extend deterministic editor generation with a county-scale recipe that
      can tune terrain frequency bands, drainage, road hierarchy, district and
      lot subdivision, structure density, material regions, and POI placement
      while retaining every effective hyperparameter.
- [ ] Define bounded agent authoring primitives for placing and revising typed
      scene artifacts in WORKING; do not let them write admitted Explorer
      state.
- [ ] Make status, diff, add, commit, and log render terrain-program,
      vector-layer, road/lot, and placed-artifact changes with exact
      native-object identity.
- [ ] Regenerate and fine-tune Rey County through that surface, then admit it
      through the scene workload instead of wiring a fixture into the UI.

### 10. Browser and performance qualification

- [ ] Retain named World → Atlas → County voyages at 1920×1080 and 3840×2160
      through WebGPU, forced WebGL2, and the reference renderer.
- [ ] Test focus retention, globe rotation, wrap, pole and antimeridian
      behavior, hover/focus lift, collision selection, terrain geomorphing,
      unknown masks, label LOD, context/device loss, last-good scene, and exact
      evidence links.
- [ ] Record program compilation, transient working-set residency,
      evaluation/upload, draw call, triangle, label, memory, frame-time, and
      interaction-latency budgets on a
      named reference machine before making a 60 Hz or fidelity claim.
- [ ] Compare the retained county capture with the 2026-08-11 Google Maps
      terrain reference for landform legibility, scale coherence, feature
      separation, and label density without copying proprietary data or style.
- [ ] Run packaged Nix, embedded UI, nextest, accessibility, and cargo-dist
      release checks for the completed slice.

## Acceptance Boundary

This plan is complete only when:

1. the CLI can explain one exact admitted region from editor package through
   atlas placement, every coordinate transform, terrain/layer compilation, and
   omission;
2. a human can rotate World, unwrap it into Atlas, focus/lift a sector, enter an
   admitted county, and inspect a lot or artifact without an identity jump;
3. the county base terrain reaches the retained Google-class legibility target
   under declared data, viewport, device, LOD, and performance bounds;
4. candidates, inferred source edges, unsupported space, and browser gestures
   never appear as admitted terrain, roads, beacons, construction, or paths;
   and
5. WebGPU loss and unavailable detail degrade visibly without changing the
   semantic scene or hiding validity and omissions.

## Explicit Deferrals

- unrestricted free-orbit camera, first-person navigation, physics, multiplayer,
  or a general ECS;
- automatic browser probing, mining, scene generation, admission, or mutation;
- Earth geography, geographic distance, or area claims for semantic
  longitude/latitude;
- inferred roads, travel, trade, economics, or beacon activity from visual
  proximity or survey/source edges; and
- proprietary Google data, styles, tiles, or algorithms.
