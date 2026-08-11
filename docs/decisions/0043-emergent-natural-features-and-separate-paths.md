# ADR 0043: Emergent Natural Features And Separate Paths

- Status: Accepted
- Date: 2026-08-10
- Amends: the transport and curation-path projection in [ADR
  0042](0042-world-geometry-and-probe-navigation.md)
- Extended by: [ADR 0044](0044-explorer-projection-engine.md), which makes the
  field and natural-feature projection part of a high-fidelity engine boundary

## Context

Projecting admitted `contains` and `references` edges as roads and flows made
the survey's extraction scaffolding look like world geometry. Those edges are
exact evidence, but an edge found in a seed file is not a road through context
space. A path is something an agent must separately discover, propose, build,
and eventually re-survey.

The terrain analogy is more useful when the survey creates local conditions.
Anchor stations sample the admitted context, unresolved boundaries change the
projected atmosphere, and accumulated flow over relief may produce natural
features. The map can then show consequences of the sampled field without
drawing the source graph on top of it.

## Decision

### Source relationships remain inspection evidence

Survey `contains` and `references` edges remain retained in
`rey.topography-patch.v1`, the verbose CLI, and exact Object or Evidence
inspection. They do not:

- contribute ridge height to the relief field;
- appear as roads, rivers, passages, probe trails, or curation paths; or
- establish visual connectivity between map points.

Shared coordinate identity likewise remains identity evidence rather than a
rendered passage.

### Survey conditions produce the projected field

The standalone projection treats admitted anchors as survey stations. Station
prominence comes from admitted local sampling: surveyed seed candidate counts,
resolved observations, and the explicit workspace origin. Retained frontier
rows and omissions contribute bounded unresolved-condition pressure. They do
not supply a spatial relationship back to their source edge.

The browser derives three classes of natural geometry:

| Natural grammar | Admitted basis | Projection claim |
| --- | --- | --- |
| Relief | Anchor station positions and sample prominence | Deterministic scalar field; no semantic-distance claim |
| Weather front | Retained unresolved frontier condition | Boundary atmosphere only; no crossing or route |
| Stream or river | Rainfall and eight-neighbor downslope accumulation over the anchor field | Projected runoff only; no retained hydrology or path |

Accumulated runoff erodes the displayed scalar field before contour extraction,
so the water projection and relief are causally coherent. The rainfall,
drainage tilt, stream thresholds, and erosion formula are deterministic browser
presentation parameters. Their values are not observed climate, semantic
similarity, mining value, or source truth.

Natural features stop inside the admitted chart. Unknown space is never filled
with an invented watershed, river, or atmosphere. Frontier points retain their
status-specific probe prerequisite, while their weather fronts remain visually
and contractually distinct from probes and paths.

### Paths are a separate future evidence family

A context path may later be admitted only through a separate typed contract
that distinguishes at least discovered from constructed paths and binds its
coordinates, method or author, revision, authority, cost, effects, omissions,
and evidence. Camera selection, source edges, hydrology, and a visually
convenient line cannot create that contract.

The map therefore exposes Relief, Water, Weather, and Probes visibility
controls. None performs a locator resolution, probe, mining operation, path
discovery, path construction, or patch admission.

### Human and agent verification

The verbose workloads CLI reports:

- admitted world and unresolved horizon counts;
- sampled atmospheric inputs, boundary fronts, and omissions;
- anchor stations and the number of retained seed edges excluded from relief
  and path rendering; and
- the hydrology/erosion projection boundary and explicit absence of a
  discovered or built path claim.

Browser tests verify that natural features include projected water and weather,
that the topographic scene contains no seed-edge lines, and that a selected
frontier reports its prerequisite without implying a route.

## Consequences

- The relief reads as an emergent environment rather than a decorated source
  graph.
- Rivers can reveal field convergence while remaining explicitly hypothetical
  projection geometry.
- Exact source relationships remain available when the operator zooms into
  evidence, without leaking into the far map.
- Agent-curated paths can later become first-class, revision-bound artifacts
  instead of being inferred from whichever edges happened to seed the survey.
