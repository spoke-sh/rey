# ADR 0056: Continuous Globe, Semantic Mercator, And County Grammar

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0044](0044-explorer-projection-engine.md), [ADR
  0046](0046-read-first-scene-editor.md), and [ADR
  0047](0047-semantic-spherical-atlas.md)
- Supersedes: ADR 0044's restriction of the first target to one top-down 2.5D
  camera and ADR 0047's unspecified Atlas-chart transform

## Context

Explorer has the beginnings of a semantic sphere, a flat local relief scene,
and a read-first scene-editor boundary. Those pieces do not yet define one
spatial language. World is a globe only at a hard semantic boundary, Atlas is
not a qualified spherical-to-planar transform, and the closer view does not
have a county-scale level-editor grammar for terrain, roads, lots, and placed
artifacts.

The desired interaction is closer to moving through one map than switching
between dashboards:

1. far away, the operator sees the topology of the whole admitted context
   world on a high-fidelity spherical object;
2. zooming in unwraps that same world into a horizontally wrapping Mercator
   chart where sectors and admitted regions are easy to compare; and
3. entering one region resolves into a stylized isometric county surface where
   detailed terrain and admitted constructed features become a reasoning
   surface.

The analogy creates two risks. Calling synthetic coordinates latitude and
longitude can be mistaken for Earth geography. Rendering roads, buildings,
beacons, or construction can also make authored candidates or inferred source
relationships appear admitted. The grammar must preserve identity and
authority while its geometry changes radically.

## Decision

### One lens, three geometric postures

`/explore` has one continuous semantic lens with three coupled projection
postures:

| Posture | Operator question | Geometry | Primary visible material |
| --- | --- | --- | --- |
| **World globe** | What regions exist and how is the admitted world organized? | Lit 3D semantic sphere viewed as a polar/world overview | Atlas sectors, region aggregates, major admitted POIs, validity, and omissions |
| **Atlas chart** | Which sector or county is worth entering? | Horizontally wrapping semantic Mercator chart | Admitted scene-region footprints, coarse terrain, sector state, major natural and constructed systems |
| **County surface** | What exists here, how is it arranged, and where should work be proposed? | Local tangent frame with a stylized isometric camera and true height displacement | Detailed terrain, hydrology, highways, roads, districts, lots, structures, and admitted artifacts |

These are projection postures, not three stores or routes. A selected semantic
coordinate, source identity, atlas revision, and admission lineage survive the
transition. Camera center, scale, rotation, pitch, viewport, hover, selection,
and morph progress remain view state.

World is called a polar projection in the product grammar because it exposes
the poles and full spherical world. Mathematically, the accelerated surface is
a lit 3D sphere under an orthographic or bounded-perspective camera, not a
two-dimensional polar map projection. The reference renderer remains an
accessible orthographic representation of the same scene.

### Projection posture and semantic detail are independent

The existing six semantic regimes continue as a detail ladder across the three
geometric postures:

| Semantic regime | Projection posture | Detail grammar |
| --- | --- | --- |
| World | World globe | Clusters, sectors, admitted-region aggregates, major POIs, atlas revision, global omissions |
| Atlas | Atlas chart | Sector polygons, admitted county footprints, coarse multiresolution terrain, major hydrology and qualified inter-county connectors |
| Landscape | County surface | One county's continuous relief, boundary, watersheds, highways, districts, major roads, and landmarks |
| Neighborhood | County surface | Local roads, lots, structures, utilities, beacons, construction state, labels, and unresolved stations |
| Object | County surface | One parcel, feature, artifact, workload object, or exact scene object with its typed state |
| Evidence | County surface plus evidence overlay | Native object, source span or tile, admission result, delta, validity, limits, omissions, and lineage |

A renderer may blend geometry and layers around regime boundaries, but it may
not disclose an object before its semantic LOD permits it or hide a known
omission during a transition.

### Logical coordinates and transforms

The abstract atlas uses `semantic_longitude` and `semantic_latitude` from
`rey.semantic-atlas.v1`. They are integer microdegrees on a synthetic sphere,
not Earth coordinates. `earth_crs` remains absent. Their meaning is bound to an
exact atlas compiler, policy, source set, parameters, limits, omissions, and
revision.

The coordinate stack is explicit:

```text
provider-qualified object identity
        + admitted region identity
        + semantic longitude/latitude at one atlas revision
        + reversible projection transform
        + county-local east/north/up frame when entered
        + camera and viewport presentation
```

The Atlas posture selects a versioned **semantic Mercator** transform. For
longitude `lambda` and latitude `phi` in radians, its normalized chart
coordinates follow the spherical Mercator transform:

```text
x = (lambda + pi) / (2 * pi)
y = 1/2 - ln(tan(pi/4 + phi/2)) / (2 * pi)
```

`x` wraps. Mercator is singular at the poles, so the chart transform clamps at
the declared latitude limit of `atan(sinh(pi))` (approximately
`±85.05112878°`). The globe retains the polar caps. During unwrapping, clipped
cap contents remain explicitly disclosed as polar omissions or appear in
bounded cap insets; they never vanish silently. Sector polygons crossing the
antimeridian are split for drawing but preserve one semantic identity.

This is Mercator mathematics over Rey's synthetic sphere. It is not Web
Mercator, EPSG:3857, OGC CRS84, WGS 84, physical distance, or geographic area.
Native RFC 7946/CRS84 scene data retains its Earth-coordinate semantics. A
qualified admission adapter must bind the native-to-semantic region transform;
copying GeoJSON longitude and latitude into semantic longitude and latitude is
invalid.

Each entered county has a revision-bound local tangent transform from the
semantic sphere to `east`, `north`, and `up` scene units. The isometric camera
is a presentation transform over that frame. Its heading, pitch, vertical
exaggeration, and lighting are versioned material/camera policy, not coordinate
identity or evidence.

### Sectors, counties, and interest

A **sector** is a revision-bound atlas partition used to organize admitted
regions. Its polygon is synthetic layout geometry and is labeled as such. A
**county footprint** is the admitted boundary of one detailed scene region.
The two may overlap visually but are not interchangeable.

Sector prominence may be derived only from named typed evidence such as
retained workload attention, admitted-region count, completeness, frontier,
staleness, or a qualified domain-specific measure. The UI must disclose the
measure behind “interest”; proximity, polygon area, and renderer height do not
create importance.

In Atlas, hover or keyboard focus may lift a sector by a bounded presentation
offset, strengthen its outline, and reveal its label and interest basis. The
lift is transient picking feedback. It does not alter terrain height, sector
identity, admission state, or semantic distance. Selection pins the sector;
zoom alone does not mutate it.

### Continuous transitions

The projection grammar is driven by continuous logarithmic camera scale with
hysteresis and screen-space error budgets:

- World drag rotates the sphere; Atlas and County drag pan their projected
  frame.
- Pointer-centered zoom preserves the semantic coordinate under the pointer.
- World-to-Atlas morphs the same sector and region vertices from sphere to
  semantic Mercator positions while aggregate labels and geometry cross-fade
  under explicit LOD rules.
- Atlas-to-County requires exactly one admitted county footprint under the
  focus or an explicit selection. It expands that county's local tangent frame,
  introduces the isometric camera, and replaces aggregate tiles with finer
  retained levels from the same admitted scene.
- If no admitted county is available, zoom stops at Atlas detail and exposes
  the missing admission. It does not generate terrain, execute a survey, or
  choose an unrelated county.
- If footprints collide, the operator must select one stable identity; z-order
  or apparent height cannot choose semantic focus.

Transition thresholds and blend widths belong to a versioned Explorer grammar
and are qualified against named viewports. They do not enter atlas or scene
identity. A geometry morph may temporarily render both source and target
meshes, but picking must resolve to one exact semantic object throughout.

### Admitted scenes are the map fabric

The Atlas and County fabric is composed primarily from exact editor packages
that passed a qualified scene-admission workload. The admission result must
bind the candidate package, native objects, normalized region and local-frame
transforms, terrain/feature layer contracts, validity and no-data behavior,
limits, omissions, and resulting projection-packet revision.

```text
editor WORKING → INDEX → SCENE@n candidate package
                                  │
                                  ▼ qualified scene-admission workload
                         admitted regional scene
                                  │
                                  ├─ atlas sector/county footprint
                                  ├─ multiresolution terrain fields
                                  ├─ natural and constructed feature layers
                                  └─ exact source/admission lineage
```

Candidate packages never appear in the admitted world. A future explicit
editor-preview surface may show them with persistent `UNADMITTED` treatment,
but `/explore` remains read-first.

Survey-only topography remains useful as an evidence overlay for anchor relief,
weather, hydrology, frontier, and omissions. It does not become detailed solid
county fabric unless an admission operation produces the required scene and
terrain contracts.

### Terrain and cartographic fidelity

Google Maps-level fidelity means comparable terrain legibility and LOD
coherence using Rey-owned or admitted inputs, not copied data, style, or
algorithms. A detailed county needs more than a shader over sparse control
points. The admitted scene contract must support:

- a bounded multiresolution height/validity tile pyramid with crack-free LOD
  transitions and explicit no-data semantics;
- independently revisioned normal, slope, aspect, curvature, occlusion,
  roughness, tint, wetness, and material channels where the source warrants
  them;
- continuous multidirectional lighting, cast/contact shadows where qualified,
  valley/ridge legibility, LOD-aware contours, and restrained atmospheric
  perspective;
- ordered hydrology, land-cover, boundary, road, lot, structure, POI, label,
  selection, validity, and accessibility layers;
- screen-space-error terrain selection, tile residency and byte budgets,
  seam stitching, label collision, picking, and visible degradation; and
- exact source, compiler, material, renderer, viewport, device, limits,
  omissions, and performance evidence for every fidelity claim.

The surface must read as landform before contours, roads, or labels are added.
Unknown, omitted, stale, unsupported, truncated, and frontier support cannot be
filled by interpolation, normal generation, texture filtering, or shadow.

### County as a reasoning and authoring surface

Roads, highways, lots, structures, utilities, beacons, construction, and other
placed artifacts are typed scene layers. Their visual analogy grants no
authority:

- roads and highways require admitted constructed-feature geometry; survey
  graph edges never become roads;
- a highway crossing a county boundary requires exact compatible connector
  identities on both admitted regions;
- lots and structures retain stable feature identity and their native/admitted
  geometry;
- a rendered beacon binds an admitted beacon or scene-artifact observation; it
  does not imply that polling or relay is currently authorized or running; and
- construction is an admitted stateful artifact or proposal, not decorative
  animation presented as observed work.

Agents use the CLI to inspect the same county coordinates and evidence, author
or fine-tune candidate feature files in editor WORKING, stage exact changes,
and commit candidate packages. A qualified workload remains the only path back
into `/explore`. Browser navigation, hover, selection, and placement previews
do not write scene state.

### Renderer boundary and COBE reference

COBE establishes a useful quality and interaction reference: a compact,
responsive lit globe, smooth rotation, dense sampled surface, markers, and DOM
label binding. It is not the selected Rey renderer. Its official implementation
is a standalone WebGL globe and does not provide Rey's terrain tile pyramid,
semantic scene, TSL material graph, county surface, or WebGPU-first lifecycle.

Rey retains the pinned Three.js `WebGPURenderer` and TSL boundary from ADR
0045. One engine scene and render graph drive the globe, Mercator chart, and
county surface. WebGPU remains preferred, Three.js WebGL2 remains the
compatibility backend, and the deterministic accessible reference renderer
remains the semantic fallback. COBE-class describes the World experience, not
a dependency or permission to render unadmitted arcs.

## Current Implementation Boundary

The repository currently implements a lit World sphere with region markers, a
synthetic semantic atlas, a flat local relief mesh, and a candidate-only
GeoJSON scene editor. It does not yet implement interactive globe rotation,
semantic Mercator unwrapping, sector polygons and hover lift, scene admission,
raster/multiresolution editor terrain, Atlas-to-County morphing, an isometric
county camera, or admitted road/lot/artifact layers. This ADR formalizes the
target grammar; [Plan 0029](../../plans/0029-continuous-explorer-grammar.md)
owns its implementation and proof.

## Consequences

- The operator moves through one stable semantic world while projection and
  information density change around the focus.
- Mercator supplies a familiar wrapping chart without making an Earth or
  physical-distance claim; its pole behavior and distortion become explicit.
- Admitted editor scenes, rather than survey edges or browser-generated
  geometry, become the primary detailed map fabric.
- County isometric view expands Explorer beyond one top-down 2.5D camera while
  still deferring unrestricted free orbit, physics, and a general game ECS.
- Detailed construction and communication metaphors become inspectable scene
  layers without bypassing editor and workload admissions.
- Terrain fidelity now depends on admitted source resolution, tile/feature
  contracts, and retained qualification evidence as much as GPU shading.

## References

- [COBE official repository](https://github.com/shuding/cobe)
- [PROJ Mercator projection](https://proj.org/en/stable/operations/projections/merc.html)
