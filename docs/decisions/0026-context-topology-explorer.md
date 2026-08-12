# ADR 0026: Context Topology Explorer

- Status: Accepted
- Date: 2026-08-09
- Extends: [ADR 0025](0025-local-operator-ui.md)
- Extended by: [ADR 0041](0041-continuous-coordinate-topography.md), which
  separates semantic coordinates from view state and adds incremental
  topography plus Atlas and Evidence levels to the continuous lens
- Extended by: [ADR 0042](0042-world-geometry-and-probe-navigation.md), which
  adds World geometry and probe horizons; [ADR
  0043](0043-emergent-natural-features-and-separate-paths.md) replaces literal
  edge corridors and curation bearings with emergent natural features and
  separate path evidence; [ADR 0044](0044-explorer-projection-engine.md)
  formalizes the high-fidelity projection-engine boundary

## Context

The initial Rey UI proved a persistent workload projection, but its default
dashboard still treated the browser as an alternate report. The intended
collaboration plane is stronger: the human operator should spend most of their
time in the UI, while agents use the CLI as their primary runtime interface.
Humans should normally descend to the CLI only for exact diagnosis.

Rey's context is high-dimensional. A useful operator map cannot render every
record at once or treat zoom as simple magnification. As scale changes, the
operator needs a different object grammar: a bounded landscape, meaningful
neighborhoods, and then exact runtime objects. Those representations must not
invent facts or lose their source identity.

## Decision

`/explore` is the default human route. `GET /` redirects to it. The previous
Instrument route and label hard-cut to Environment at `/environment`; no
`/instrument` compatibility alias is retained. `/workloads` and exact workload
routes remain.

The Explorer implements a context-topology canvas using explicit React
boundaries: `ContextCanvas`, `SemanticLens`, topology regions, topology
objects, and classified relationship edges. Its deterministic
`buildTopologyScene` projection consumes the existing
`rey.workload-list.v5` document rather than fetching or constructing an
independent runtime graph.

The semantic lens has three initial regimes:

1. landscape/telescope: context, portfolio, workload, evidence, request, and
   attention aggregates;
2. neighborhoods/mesoscopic: individual workload/request and attention
   neighborhoods with directed relationships; and
3. objects/microscope: graph, scenario, evidence, dependency, delta, and
   bearing objects for the selected coordinate.

Wheel and control zoom traverse these regimes in order. Selection advances the
lens while retaining a typed focus. Drag pans the map, keyboard controls are
available, and the canvas can own the native full-screen viewport. Every
projection reports known omissions caused by its declared bounds.

Explore uses a single scroll plane. The route locks its application root to
`100dvh`, prevents document overflow, and flexes the canvas into the exact
space left by application chrome and the route heading. Wheel input on Explore
therefore always changes the semantic lens. Environment and Workloads retain
normal document scrolling.

The browser removes manual Refresh. It passively invalidates and reloads the
same read-only workload projection every 5000 ms. The `rey ui` startup document
reports `/explore` as the human entry and reports the exact passive
revalidation interval. No refresh cycle executes, mutates, admits, tests, runs,
or schedules work.

The first canvas is a bounded portfolio/context projection, not a claim that
the complete environment graph is available. Full environment objects,
scenario deltas, proof evidence, URL-addressable focus, and new Explorer
windows require explicit typed inputs and later slices.

## Consequences

- Humans land on a topology map rather than a command-shaped portfolio report.
- The UI and CLI have explicit primary personas without splitting evidence
  authority.
- Zoom can change visualization dimension while tests preserve semantic order,
  exact identities, bounds, and visible omissions.
- Full screen improves spatial investigation without widening data or action
  authority.
- Passive revalidation makes the UI useful as an ongoing surface and removes a
  routine manual control.
- The initial map is useful but incomplete: environment mapping nodes and exact
  scenario/delta evidence remain concrete next inputs.
