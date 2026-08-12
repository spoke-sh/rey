# ADR 0047: Admission-Revisioned Semantic Spherical Atlas

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0044](0044-explorer-projection-engine.md) and [ADR
  0046](0046-read-first-scene-editor.md)
- Extended by: [ADR
  0056](0056-continuous-globe-mercator-county-grammar.md), which selects the
  exact synthetic semantic-Mercator Atlas grammar and local isometric County
  posture
- Supersedes: the non-spherical standalone World placement in [ADR
  0042](0042-world-geometry-and-probe-navigation.md)

## Context

One anchor-derived relief scene is legible at Atlas and closer lenses, but a
larger collection of admitted scenes still behaves like tiles on an expanding
plane. It has no higher-dimensional reference surface, no wraparound world,
and no stable way to say that multiple surveyed regions belong to one larger
projection. Shrinking that plane cannot reveal world geometry.

Latitude and longitude provide a useful spherical addressing grammar. Using
those names carelessly would confuse a language-space layout with Earth,
geographic distance, OGC CRS84, or Web Mercator. Reclustering on every zoom
would also violate semantic identity: camera motion cannot move an admitted
region or silently change which regions appear related.

The world must remain read-first. Its coordinates may organize admitted
evidence but cannot turn visual proximity into source truth, execute a probe,
or admit editor candidates. Future travel, trade, route, and economic layers
need their own typed evidence instead of being inferred from survey edges.

## Decision

Rey introduces `rey.semantic-atlas.v1`, a bounded content-identified layout
over retained admitted regional sources. Its coordinate system is a
**synthetic semantic sphere**:

- axes are `semantic_longitude` and `semantic_latitude` in integer
  microdegrees;
- longitude wraps at ±180 degrees and latitude is bounded to ±90 degrees;
- `earth_crs` is always absent; the values are not OGC CRS84, Earth locations,
  physical distance, or geographic area;
- native semantic region identity and source revision remain separate from
  spherical placement; and
- the exact layout compiler, policy, limits, omissions, inputs, and revision
  travel with the atlas.

The first layout compiler derives a bounded survey-structure feature vector
from each admitted topography: anchor-kind proportions, survey coverage,
candidate density, frontier density, and completeness. Deterministic bounded
k-medoids forms world clusters. Cluster centers use an equal-area spherical
sequence; member regions use deterministic polar placement around the cluster
center. This is a projection policy, not a semantic-similarity claim.

```text
admitted regional evidence revisions
                 │
                 ▼
      rey.semantic-atlas.v1
      ├─ stable region identities
      ├─ synthetic spherical coordinates
      ├─ multiregion world clusters
      ├─ compiler + limits + omissions
      └─ exact atlas revision
                 │
                 ▼
 World globe → Atlas chart → local tangent terrain → exact evidence
```

Reclustering is admission-revisioned. A changed admitted source set or source
topography revision produces a new atlas revision and may move regions.
Camera pan, globe rotation, selection, viewport size, and zoom never enter the
atlas identity. Zoom selects retained levels of detail from one atlas revision.
Native region identity survives a layout revision so a directed atlas delta
can later report inserted, removed, moved, merged, and split regions without
pretending the camera caused them.

The semantic lens gains a projection hierarchy:

| Lens | Projection posture | Current/target geometry |
| --- | --- | --- |
| World | Understand regions and global topology | 3D semantic globe with major clusters and admitted region POIs |
| Atlas | Navigate a wraparound chart | Target spherical Mercator transform over synthetic coordinates with explicit antimeridian, pole, distortion, and scale-qualified label behavior |
| Landscape and Neighborhood | Read and curate local terrain | Local tangent relief under a bounded isometric camera, natural and constructed feature layers, validity, frontier, and POIs |
| Object and Evidence | Inspect exact basis | Native semantic coordinates, source objects, bounds, omissions, and lineage |

The WebGPU renderer materializes the World sphere and markers through the same
immutable scene boundary as continuous relief. The reference renderer owns a
deterministic accessible orthographic globe using the same atlas revision.
Three.js owns graphics mechanics only; Rey owns atlas semantics, scene
identity, LOD, evidence binding, and qualification.

`rey editor` candidates do not enter the atlas. A future scene-admission
workload must validate an exact package and emit an admitted regional result
before it becomes an atlas source. Geographic GeoJSON remains native CRS84
evidence. A qualified adapter may project it into a semantic region, but Rey
does not relabel its Earth coordinates or confuse synthetic semantic axes with
its native CRS.

Routes and flows are independent overlays. Travel routes, constructed paths,
trade flows, and economic conditions must each bind exact typed observations,
time/revision semantics, direction, capacity or magnitude where applicable,
limits, omissions, and authority. Topography seed edges remain excluded.

## Implemented Checkpoint

`rey-mining` defines and validates `rey.semantic-atlas.v1`. The workload-list
read model derives it deterministically from the latest verified topography
patch per admitted workload and exposes it in `rey.workload-list.v8`. `rey
workloads list` prints its exact revision, region/cluster counts, compiler, and
the non-Earth/recluster boundary.

Explorer binds the atlas and compiler revisions into its immutable World scene.
The reference renderer draws an accessible orthographic globe and region POIs;
the Three.js path draws a lit sphere, graticule, and admitted region markers
through WebGPU-first rendering with WebGL2 fallback. Atlas and closer lenses
continue to use the existing local relief projection.

This checkpoint recomputes the atlas as a deterministic projection of retained
admission state. Retaining prior atlas revisions and their directed movement
deltas at the admission transition, interactive globe rotation, sector
identity, editor scene admission, and typed route/economic overlays remain
Plan 0022 work. ADR 0056 and Plan 0029 own the exact semantic-Mercator unwrap,
sector interaction, and admitted isometric county grammar.

## Consequences

- Zooming out can reveal a world that contains multiple surveyed regions
  instead of only shrinking a tiled plane.
- A layout revision can change spherical positions without changing native
  region identity or source truth.
- The globe creates a natural home for later global topology, travel, trade,
  and economics while refusing to manufacture those layers today.
- Integer microdegrees keep structured identity deterministic and make the
  synthetic coordinate namespace visibly different from GeoJSON numbers.
- Spherical placement introduces distortion and cluster instability that must
  be exposed through revisions and deltas rather than hidden as animation.
- Current World rendering is a first engine slice, not proof of an Earth map,
  semantic-distance metric, global survey completeness, or admitted path.
