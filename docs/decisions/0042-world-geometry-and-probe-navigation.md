# ADR 0042: World Geometry And Probe Navigation

- Status: Accepted
- Date: 2026-08-10
- Extends: [ADR 0041](0041-continuous-coordinate-topography.md)
- Amended: the transport and curation-path projection is superseded by [ADR
  0043](0043-emergent-natural-features-and-separate-paths.md)
- Amended: the flat standalone World placement is superseded by the synthetic
  semantic sphere in [ADR 0047](0047-semantic-spherical-atlas.md)

## Context

Anchor relief makes one surveyed scene legible, but its outer edge is still a
canvas edge. Zooming out shows a smaller local map rather than the larger world
that contains it. Exact relationships also arrive only at Neighborhood scale,
even though broad connectivity and the unknown crossings at its boundary are
the most useful facts at a far scale.

The real-world map analogy provides a useful visual grammar: land and survey
horizons establish extent; roads, rivers, and passages expose different forms
of connection; trailheads mark where knowledge ends; and points of interest
change density with scale. The analogy must not turn layout into evidence.
Rey's current standalone coordinates are not geographic or high-dimensional
semantic coordinates, and a promising-looking shape cannot authorize a read,
probe, mining operation, or workload.

## Decision

### World before Atlas

Explorer adds **World** as the farthest of six deterministic levels:

```text
World → Atlas → Landscape → Neighborhood → Object → Evidence
```

The continuous camera range becomes `0.05..=5.4`; World has a canonical stop
at `0.1`. The selected coordinate remains stable across every transition.
World is a semantic world projection, not Web Mercator or an Earth map. ADR
0047 now gives admitted regions explicit synthetic semantic longitude and
latitude on a revision-bound sphere; those axes have no Earth CRS and make no
physical-distance claim.

For each admitted patch, the projection derives two geometries:

- **charted land** is a bounded envelope over the patch's displayed admitted
  anchors; and
- the **survey horizon** is a wider envelope that may include retained
  unresolved frontier points.

The charted envelope says where displayed evidence exists. The horizon says
where the current survey stops. Space outside remains unexplored with no
inferred global boundary, area, coverage percentage, terrain, or similarity.
Neither envelope becomes retained source truth.

### Transport geometry (superseded)

ADR 0043 removes every corridor below from the current relief projection.
These entries preserve the historical decision; exact edges now remain deep
inspection evidence while natural features emerge from admitted survey-field
conditions.

The far map exposes connectivity through redundant labeled corridor classes:

| Map grammar | Evidence basis | Claim |
| --- | --- | --- |
| Containment road | Exact admitted `contains` edge | Directed containment only |
| Reference flow | Exact admitted `references` edge | Directed reference only |
| World passage | The same exact coordinate occurs in more than one admitted chart | Shared coordinate identity only |
| Probe trail | Retained unresolved frontier row from its exact source anchor | Candidate crossing; not a resolved edge |

Road and flow curvature is deterministic presentation. Distance, crossings,
line weight, and visual adjacency do not add a relationship. Probe trails are
always dashed and visually distinct from exact corridors.

### Prospecting before mining

Explorer may expose evidence-backed reasons to investigate without inventing a
mining recommendation:

- anchor degree identifies a charted terminus, connected anchor, or junction;
- an exact anchor coordinate is an eligible bounded workload input, but grants
  neither read authority nor a recommendation to mine;
- a frontier status names the prerequisite before mining: expand a truncated
  bound, revalidate stale evidence, admit a missing resolver, obtain authority,
  curate malformed input, or verify a missing reference; and
- a selected anchor or frontier derives one bounded directed route from the
  workspace survey origin. The route separates exact steps from an unresolved
  probe crossing.

This route is a camera and interpretation overlay. Selecting or curating a path
does not deform admitted relief. Only a subsequent admitted patch with changed
anchors, relationships, coverage, or frontier can change the terrain.

The Relief, Routes, and Probes controls are lens visibility controls only. They
cannot change source assessment, scope, or authority. Panning, zooming,
selecting, following a deep link, or toggling a layer still executes nothing.

### Human and agent verification

The browser consumes the retained `rey.topography-patch.v1` already exposed by
`GET /api/v1/workloads`. The CLI projects the same basis as:

- admitted chart and surveyed/probe-horizon counts;
- containment-road and reference-flow counts;
- exact anchor and connected-junction counts; and
- a prerequisite action for each retained probe row.

Connectivity is explicitly disclosed as neither a mining recommendation nor
read authority. No new store, resolver, scheduler, or probe executor is
introduced by this decision.

## Consequences

- Zooming out now changes the semantic field of view instead of only shrinking
  one relief scene.
- Boundary uncertainty becomes navigable and actionable without being filled
  in as terrain.
- Broad exact connectivity appears before object detail, matching the operator's
  posture at far scale.
- Curated paths can be inspected without allowing selection state to rewrite
  evidence.
- A future high-dimensional provider can replace the standalone layout only by
  binding its coordinate, model, projection, revision, and omissions; this
  world projection makes no such semantic-distance claim.
