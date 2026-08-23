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
   hydrology, boundaries, districts, transport, markers, and labels enter the
   scene independently.
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
domain-warped macro, ridge, meso, and fine relief below the source grid's Nyquist
limit. Smooth authored drainage constraints carve the named channels, then a
depression-safe source-drainage pass derives receivers and accumulation only
inside exact validity and incises a bounded dendritic valley network. Explorer
receives a subtle terrace response, and
the five renderer-recognized land-cover materials follow coherent elevation,
moisture, exposure, meadow, wetland, and water-distance fields. These are
authored semantic choices recorded in the native source, not facts inferred by
the renderer.

## Native Sources

| File                       | Editor role       | Meaning                                                                  |
| -------------------------- | ----------------- | ------------------------------------------------------------------------ |
| `boundary.geojson`         | `boundary`        | Exact County footprint and validity boundary                             |
| `terrain.geojson`          | `terrain`         | Packed 705×626 elevation/material grid with explicit validity            |
| `terrain-controls.geojson` | `terrain_control` | Candidate-only named landform influences; never observed height          |
| `hydrology.geojson`        | `hydrology`       | Authored typed rivers, streams, runoff, river area, and wetland geometry |
| `features.geojson`         | `features`        | Meadow land cover and the explicit unexplored region                     |
| `markers.geojson`          | `markers`         | Semantic points of interest with independent label LOD                   |
| `districts.geojson`        | `district`        | Subordinate administrative and semantic boundaries                       |
| `highways.geojson`         | `highway`         | Primary and secondary authored transport hierarchy                       |
| `roads.geojson`            | `road`            | Terrain-aware local route candidates                                     |
| `railways.geojson`         | `railway`         | Regional and industrial rail candidates                                  |
| `labels.geojson`           | `label`           | Geographic names with exact zoom and collision policy                    |

The terrain grid contains 441,330 cells at exact integer-microdegree spacing:

- 324,739 valid cells;
- 108,750 no-data cells outside the County footprint;
- 9,471 no-data cells in Unexplored Scrub (some exterior cells satisfy both
  predicates, producing 116,591 unique no-data cells);
- 197.31–1,826.03 meters of authored relief; and
- `granite`, `rock`, `sand`, `soil`, and `vegetation` material identifiers.

Seven hundred four longitudinal and 625 latitudinal intervals preserve the
County's exact bounds with near-square 131–133-meter cells. One
`rey.packed-terrain-grid.v1` GeoJSON feature carries byte-exact validity,
little-endian centimeter elevation, and palette-indexed material channels
beside its exact Polygon grid envelope. The 5.1 MiB artifact replaces 441,330
counterfeit Point features and remains under the one-million-cell packed-grid
admission limit. This is a source-native density improvement, not the final
resolution target; a tiled raster adapter is still required beyond the bounded
in-memory regional grid.

The embedded `rey.agent-geography.rey-county@16` compiler record states the
topology, elevation, hydrology, land-cover, and stitching contracts. This
revision owns one County-wide authoring domain and therefore reports zero
seams and conflicts while explicitly omitting cross-package seam resolution.
Its exact grid-aligned eastern boundary segment allows a separately admitted
neighbor to be assessed without an implicit merge. It does not imply that
multiple editor packages have already been stitched.

Its elevation compiler resolves each rough named landform into bounded,
domain-warped irregular mountain mass with restrained structural ridges and
substantially reduced unstructured fine-noise amplitude. Exact authored
waterways condition only narrow shallow corridors instead of dominating the
landform. A priority-flood pass starts only at exact validity boundaries and
derives source drainage. Accumulation is distributed across every downhill
neighbor with hydraulic-slope weights instead of making one D8 traversal tree
own height. The actual unfilled source-height slope owns incision; a
separately retained dominant receiver exists only for channel-order reporting,
and flood-parent escape cells contribute exactly zero displacement. The
retained derivation reports 315,512 multiple-receiver vertices, 1,212,983
weighted flow edges, 21,964 channel cells, 1,942 heads, 1,290 branch junctions,
maximum Strahler order 6, maximum stream power `0.817548`, a 70.01-meter
maximum incision, and a maximum contributing area of 230,779.75 valid-cell
units. It applies nonzero incision at 235,931 genuinely downhill cells and at
zero flat escape cells. Only after those profile-smoothed valleys exist, a
bounded source-scale pass separates drainage divides and valleys at the
318,781 cells with fully valid five-by-five neighborhoods; 5,958 valid boundary
cells remain unchanged. Its displacement is bounded to a 47.64-meter raise and
a 53.36-meter lowering and is authored source geometry, not renderer detail. The
separate authored-source local-relief summary samples a 528.85-meter radius:
the median is 25.74 meters, the 90th percentile is 53.07 meters, and neither is
an Earth DEM observation or perceptual fidelity result. The main river and
wetland are exact admitted areas;
tributaries remain exact paths. This is source geography rather than renderer
noise, and every no-data vertex remains absent.
Its cartographic hierarchy currently retains four highways, twelve local
roads, four railway paths, and sixteen independently bounded labels. Its water
hierarchy retains one exact river surface, one wetland, the river centerline,
and nine exact tributary or runoff paths. These are source features with exact
evidence routes, not renderer-generated decoration. Every feature carries the
required `water_class` admission property; line and polygon classes are
geometry-checked and remain self-asserted authored semantics rather than a
claim about current water conditions.

## Regeneration And Verification

Regeneration reads only the checked-in boundary, district, feature, highway,
hydrology, label, railway, road, and terrain-control files. Their SHA-256
identities, derivation principles, grid shape, and summary are embedded in the
GeoJSON foreign metadata. Same inputs produce the same bytes.

```sh
node scenes/rey-county/generate-terrain.mjs
node scenes/rey-county/generate-terrain.mjs --check
pnpm --filter @rey/agent exec vitest run \
  scripts/rey-county-terrain.test.mjs
```

The Vitest contract verifies row-major coordinates, exact bounds, footprint
and internal no-data, distinct landforms, relief, bounded materials, packed
channel shape and encoding, and byte-for-byte agreement with the checked-in
artifact.

## Editor And Admission

A fresh local editor store can register the source-controlled fixture without
copying or rewriting it:

```sh
rey editor source add scenes/rey-county/boundary.geojson \
  --id rey-county-boundary --role boundary --scene-id rey-county
rey editor source add scenes/rey-county/features.geojson \
  --id rey-county-features --role features
rey editor source add scenes/rey-county/districts.geojson \
  --id rey-county-districts --role district
rey editor source add scenes/rey-county/highways.geojson \
  --id rey-county-highways --role highway
rey editor source add scenes/rey-county/hydrology.geojson \
  --id rey-county-hydrology --role hydrology
rey editor source add scenes/rey-county/labels.geojson \
  --id rey-county-labels --role label
rey editor source add scenes/rey-county/markers.geojson \
  --id rey-county-markers --role markers
rey editor source add scenes/rey-county/railways.geojson \
  --id rey-county-railways --role railway
rey editor source add scenes/rey-county/roads.geojson \
  --id rey-county-roads --role road
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
and retains `rey.regional-terrain-grid.v3` with derivable, cell-addressable
packed-source lineage. `/explore` consumes only that latest
accepted production result; a checkout containing these files but no local
editor/workload history correctly remains unadmitted.
