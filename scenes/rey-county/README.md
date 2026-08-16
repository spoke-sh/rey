# Rey County

Rey County is a source-controlled authored world for exercising the complete
Explorer journey. It translates the project's architecture into a geographic
grammar without pretending to be an Earth survey or a measurement of the
repository.

The native files are editor candidates. They become Explorer evidence only
after exact editor retention and the separately qualified `scene-admission`
workload. Admission preserves their authored authority; it does not turn the
metaphor into physical geography, observed code activity, or proof.

## First Principles

The County is built from five rules:

1. **Topology precedes decoration.** Foundations, runtime, mining, proof, and
   Explorer have distinct spatial bearings before labels or material styling.
2. **Terrain is a dataset, not a hint.** `terrain-controls.geojson` remains the
   reviewable authoring basis. Only the exact rectilinear points in
   `terrain.geojson` can become admitted height.
3. **Unknown remains a hole.** Grid vertices outside the exact County boundary
   and inside Unexplored Scrub are explicit `no_data`; they have no height or
   material for the renderer to interpolate.
4. **Channels retain separate meaning.** Elevation, validity, material,
   hydrology, boundaries, districts, and markers enter the scene independently.
   Evidence River can shape the authored field without becoming a route or an
   observed watershed.
5. **The whole artifact is bounded and reproducible.** One deterministic
   generator produces a grid that fits editor, scene-admission, workload-state,
   terrain-tile, CPU, and GPU limits.

## Project Topography

The landforms summarize the ownership and dependency shape found across the
foundational contracts, twelve Rust crates, `@rey/agent`, and `@rey/explorer`:

| Landform               | Project bearing                                                   |
| ---------------------- | ----------------------------------------------------------------- |
| Anchor Range           | Constitution, instructions, exact context, and shared foundations |
| Architecture Highlands | Ownership boundaries and the one-way system dependency shape      |
| Explorer Terraces      | Globe, Atlas, Landscape, semantic projection, and rendering       |
| Runtime Basin          | Workload execution, nested loops, scheduling, and convergence     |
| Mining Ridge           | Source/relational mining, locators, frames, and directed deltas   |
| Proof Escarpment       | Qualification, exact evidence, limits, and staleness              |
| Unexplored Scrub       | Explicitly unsupported or not-yet-surveyed space                  |

The terrain model combines the retained control geometry with deterministic,
domain-warped macro, meso, ridge, and micro relief. Existing hydrology alone
carves the authored channels, Explorer receives a subtle terrace response, and
the five renderer-recognized land-cover materials follow coherent elevation,
moisture, exposure, meadow, wetland, and water-distance fields. These are
authored semantic choices recorded in the native source, not facts inferred by
the renderer.

## Native Sources

| File                       | Editor role       | Meaning                                                           |
| -------------------------- | ----------------- | ----------------------------------------------------------------- |
| `boundary.geojson`         | `boundary`        | Exact County footprint and validity boundary                      |
| `terrain.geojson`          | `terrain`         | 81×81 row-major elevation/material dataset with explicit validity |
| `terrain-controls.geojson` | `terrain_control` | Candidate-only named landform influences; never observed height   |
| `hydrology.geojson`        | `hydrology`       | Authored rivers, streams, runoff, and wetland geometry            |
| `features.geojson`         | `features`        | Districts, meadow, and the explicit unexplored region             |
| `markers.geojson`          | `markers`         | Semantic points of interest with independent label LOD            |

The terrain grid contains 6,561 vertices at exact integer-microdegree spacing:

- 4,623 valid vertices;
- 1,825 no-data vertices outside the County footprint;
- 138 no-data vertices in Unexplored Scrub (some exterior vertices satisfy both
  predicates, producing 1,938 unique no-data vertices);
- 88.77–1,721.67 meters of authored relief; and
- `granite`, `rock`, `sand`, `soil`, and `vegetation` material identifiers.

Eighty intervals per axis preserve the County's exact bounds at approximately
1.04–1.15-kilometer sample spacing and cross the renderer’s 32-interval tile
boundary twice in both directions. The resulting 3×3 leaf working set exercises
tiled evaluation and residency while the complete admitted result remains
inside the bounded local workload store. This is a material improvement over
the correctness fixture, not the final resolution target; a raster-native
pyramid is required for sub-kilometer authored fields without turning GeoJSON
points into a bulk terrain format.

The embedded `rey.agent-geography.rey-county@2` compiler record states the
topology, elevation, hydrology, land-cover, and stitching contracts. This
revision owns one County-wide authoring domain and therefore reports zero
seams and conflicts while explicitly omitting cross-package seam resolution.
It does not imply that multiple editor packages have already been stitched.

## Regeneration And Verification

Regeneration reads only the checked-in boundary, feature, hydrology, and
terrain-control files. Their SHA-256 identities, derivation principles, grid
shape, and summary are embedded in the GeoJSON foreign metadata. Same inputs
produce the same bytes.

```sh
node scenes/rey-county/generate-terrain.mjs
node scenes/rey-county/generate-terrain.mjs --check
pnpm --filter @rey/agent exec vitest run \
  scripts/rey-county-terrain.test.mjs
```

The Vitest contract verifies row-major coordinates, exact bounds, footprint
and internal no-data, distinct landforms, relief, bounded materials, and
byte-for-byte agreement with the checked-in artifact.

## Editor And Admission

A fresh local editor store can register the source-controlled fixture without
copying or rewriting it:

```sh
rey editor source add scenes/rey-county/boundary.geojson \
  --id rey-county-boundary --role boundary --scene-id rey-county
rey editor source add scenes/rey-county/features.geojson \
  --id rey-county-features --role features
rey editor source add scenes/rey-county/hydrology.geojson \
  --id rey-county-hydrology --role hydrology
rey editor source add scenes/rey-county/markers.geojson \
  --id rey-county-markers --role markers
rey editor source add scenes/rey-county/terrain-controls.geojson \
  --id rey-county-terrain-controls --role terrain_control
rey editor source add scenes/rey-county/terrain.geojson \
  --id rey-county-terrain --role terrain
rey editor add
rey editor diff --staged
rey editor commit -m "Admit Rey County"
rey workloads run scene-admission --scene SCENE@n
```

`SCENE@n` is the exact scene label printed by the editor commit. The final run
re-inspects frozen native bytes, rejects divergent grid metadata or validity,
and retains `rey.regional-terrain-grid.v1`. `/explore` consumes only that latest
accepted production result; a checkout containing these files but no local
editor/workload history correctly remains unadmitted.
