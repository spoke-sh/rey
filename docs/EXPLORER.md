# Context Topology Explorer

The Rey Explorer is the human operator's primary collaboration surface. It
maps the bounded context Rey can currently explain, lets the operator move
between semantic scales, and preserves exact runtime identities while the
visual grammar changes. Agents continue to use the `rey` CLI as their primary
execution and diagnostic interface.

The Explorer is a read-only projection. It does not become a second runtime,
scheduler, evidence store, or assessment authority.

## Operator Model

The intended division of labor is:

| Persona | Primary surface | Normal use |
| --- | --- | --- |
| Human operator | `rey ui` and `/explore` | Orient on context, traverse attention, inspect workload neighborhoods, and understand the next bearing |
| Agent or coding harness | `rey` CLI and structured output | Create, test, run, diagnose, and revise workloads through admitted contracts |
| Human diagnosing a problem | `rey` CLI | Drop beneath the visual projection to inspect exact command evidence, verbosity layers, stderr, and exit semantics |

The UI and CLI are different projections over the same typed facts. The UI is
not required to mimic a terminal document, but it must retain enough identity,
scope, direction, completeness, limits, and lineage to reach the exact CLI
evidence when investigation requires it.

## Presentation Concepts

The Explorer formalizes seven presentation concepts. They are React/read-model
concepts, not new runtime entities:

```text
Context topology = bounded typed objects + classified relationships
Canvas           = spatial map over one bounded topology projection
Lens             = semantic projection(topology, focus, zoom)
Regime           = one object grammar on the lens continuum
Neighborhood     = bounded objects around one meaningful coordinate
Focus            = selected coordinate retained while changing scale
Omission         = evidence that the current projection folded or excluded
                   known objects because of declared limits
```

Relationships are always labeled. The first slice uses `contains`, `directs`,
`produces`, `observes`, and `depends`; line placement or proximity alone does
not assert causality, ownership, or authority.

## Semantic Lens

Zoom is a semantic operation, not only a CSS transform:

| Regime | Operator posture | Current object grammar |
| --- | --- | --- |
| Landscape / telescope | Survey the bounded field and find concentration or unresolved direction | Context, workload, evidence, request, portfolio, and attention aggregates |
| Neighborhoods / mesoscopic | Compare the local structures that may need attention | Individual admitted workloads, creation requests, surface-attention rows, and directed workload-attention relationships |
| Objects / microscope | Inspect the machinery within a selected coordinate | Package/context binding, compute graph, scenarios, evidence, dependencies, directed delta, and next-bearing objects |

The canvas supports pointer-centered wheel zoom, discrete semantic zoom
controls, drag-to-pan, keyboard `+`, `-`, and `0`, selection-driven traversal,
and a native full-screen mode. A control step cannot skip a semantic regime.
Selecting a landscape coordinate advances to neighborhoods; selecting a
neighborhood advances to its object view.

`/explore` owns exactly the browser space remaining below Rey's application
chrome. The route is height-locked to `100dvh`, the document cannot scroll,
and the canvas flexes into the remaining space. Wheel input therefore has one
meaning on this route: move the semantic lens. `/environment` and `/workloads`
remain ordinary scrollable documents.

## Projection Invariants

- Source identities and assessments survive a lens transition. Representation
  and information density may change; source truth may not.
- Every projection is bounded. The current neighborhood view renders at most
  eight workload/request and eight attention objects and reports known folded
  objects in the canvas footer.
- Object views disclose folded evidence and dependency references rather than
  pretending one displayed reference is complete.
- The selected focus remains a typed coordinate. It cannot grant access,
  execute a workload, admit an action, or resolve its own attention row.
- Relationship labels carry meaning; geometry is a navigation aid.
- Color is redundant with family, label, state, and relationship text.
- Full screen changes only viewport ownership. It does not change scope,
  authority, limits, or the underlying topology.
- Do not introduce a second scroll plane around the canvas. If application
  chrome or explanatory copy grows, the canvas must still fit the remaining
  viewport rather than causing document scroll and making the wheel ambiguous.
- Passive revalidation may replace the source snapshot, but it cannot silently
  mutate runtime state. The UI identifies itself as live and read-only.

## Implemented Routes

`GET /` redirects to `/explore`. The application routes are:

- `/explore`: the context-topology canvas and default human entry;
- `/environment`: a Kinetic Precision workbench over the exact typed
  `HEAD → INDEX → WORKING` environment delta;
- `/workloads`: admitted workload and creation-request catalog; and
- `/workloads/$workloadId`: exact workload or request detail.

The Refresh control has been removed. The root workload and environment
projections passively revalidate every 5000 ms from `GET /api/v1/workloads`
and `GET /api/v1/environment`. Revalidation changes only the browser
projection; it does not invalidate the route, reset viewport or scroll state,
test, run, create, add, commit, or schedule work. Failed background reads retain
the last good projection and remain visible as delayed revalidation.

The global footer displays the shortened Rey implementation Git revision and
links it through the complete revision to the canonical GitHub commit. This is
separate from the BLAKE3 portfolio-attention identity: semantic evidence
digests must never be presented as source commits.

The Explorer topology is intentionally narrow. It is derived from
`rey.workload-list.v5`: exact workload packages, drafts, graph/scenario/mining
counts, portfolio attention, and mapped-surface coverage counts. The separate
`/environment` route now consumes `rey.environment-status.v3` and renders its
exact variable, application, input, and reference operator projection. The
Explorer does not yet contain those exact environment nodes, source spans,
scenario deltas, or proof manifests. Aggregates are labeled as aggregates; the
Explorer must not imply that unavailable objects have been rendered.

## React Boundaries

`ExplorePage` owns route composition. `ContextCanvas` owns zoom, pan, focus,
fit, keyboard, and full-screen state. `SemanticLens` renders a pure
`TopologyScene`; regions, objects, and the SVG edge layer are independent
components. `buildTopologyScene` is a deterministic read-model projection over
`rey.workload-list.v5` and is tested separately from browser mechanics.

Future windows and lenses should add typed topology inputs rather than fetch or
invent a second graph inside a visualization component.

## Next Boundaries

The next useful expansion is to connect the exact environment projection now
available at `/environment` to the Explorer topology, followed by
scenario/delta object routes that preserve the CLI `-v`/`-vv` evidence ladder.
URL-addressable focus, search, and bounded
neighborhood filters should arrive before a high-cardinality topology.

Browser mutation, workload campaign controls, authentication, multi-user
scope, remote deployment, and Spoke-backed streams remain separate decisions.
