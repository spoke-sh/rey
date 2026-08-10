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

The Explorer formalizes eight presentation concepts. They are React/read-model
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
Coordinate       = shareable kind + identity + revision + lens location that
                   resolves against the same bounded source projection
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

`/explore` and its exact coordinate routes own exactly the browser space
remaining below Rey's application chrome. The route is height-locked to
`100dvh`, the document cannot scroll, and the canvas flexes into the remaining
space. Wheel input therefore has one meaning on this route: move the semantic
lens. `/cadence`, `/agents`, `/environment`, and `/workloads` remain ordinary
scrollable documents.

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
- The fixed footer is a live communications channel. Its mailbox contains only
  typed attention or revalidation failure evidence; zero messages explicitly
  means no operator attention is requested. `MAILBOX` selects that history
  axis; the center chevrons select the separate operator/Rey/agent conversation
  axis. Selecting the active axis closes the plane, selecting the other axis
  switches it, and either Escape or a click on the background closes it.
- A coordinate whose `at` revision no longer matches is stale. A coordinate
  whose identity is absent is missing. Neither may silently drift to a current
  object while retaining the old URI.

## Explorer Coordinate URIs

The canonical v1 coordinate shape is:

```text
/explore/{kind}/{identity};at={revision};lens={regime}[;role={agent-role}]
```

The hierarchy selects an object family and identity. Semicolon parameters are
unique, unordered named dimensions inspired by the
[Matrix URI design note](https://www.w3.org/DesignIssues/MatrixURIs.html).
Rey's serializer emits the stable lexical order `at`, `lens`, `role`; the
parser accepts any order and rejects duplicates, unknown dimensions, empty
values, missing non-cluster `at` bindings, invalid regimes, and ambiguous agent
roles. Relative matrix references are not part of the contract.

Current kinds are `portfolio`, `cluster`, `workload`, `attention`, and `agent`.
Current lens values are `landscape`, `neighborhoods`, and `objects`. Agent
coordinates require `role=coding_harness|rule|human`. For example:

```text
/explore/agent/codex;at=gpt-5;lens=objects;role=coding_harness
```

`at` binds the revision supplied by the current source projection; it is not a
request for unimplemented historical reconstruction. See [ADR
0030](decisions/0030-operator-cadence-agents-and-explorer-coordinates.md).

## Implemented Routes

`GET /` redirects to `/explore`. The application routes are:

- `/explore`: the context-topology canvas and default human entry;
- `/explore/{kind}/{identity};...`: an exact matrix-style Explorer coordinate;
- `/cadence`: partially ordered Git, Rey-admission, and passive-scan clocks;
- `/agents`: evidence-ranked system recommendations and an observed-work ledger;
- `/environment`: three stacked Kinetic Precision evidence sections over the
  exact typed `HEAD → INDEX → WORKING` environment delta—directed text,
  bounded search, and the reference plane;
- `/workloads`: admitted workload and creation-request catalog; and
- `/workloads/$workloadId`: exact workload or request detail.

The Refresh control has been removed. The root workload and mounted environment
projections passively revalidate every 5000 ms from `GET /api/v1/workloads`
and `GET /api/v1/environment`. Revalidation changes only the browser
projection; it does not invalidate the route, reset viewport or scroll state,
test, run, create, add, commit, or schedule work. Failed background reads retain
the last good projection and remain visible as delayed revalidation.

`GET /api/v1/cadence` returns `rey.ui-cadence.v2`. Its leading repository-state
plane separates working-tree attention from the exact local-upstream push
relation. The remaining lanes keep newest-first Git reachability and
environment sequence separate, report truncation and shallow boundaries, and
describe existing browser scan contracts without claiming server-side or
runtime scheduling. Git tick publication is relative to a retained local
tracking-ref OID and never implies a network fetch. Environment commit v1 has
no wall time, so those ticks explicitly render as order-only.

The global footer displays a typed-attention history mailbox, chevrons that
open the traditional conversation axis of the same plane, and the shortened Rey implementation Git revision linked through
the complete revision to the canonical GitHub commit. This is separate from
the BLAKE3 portfolio-attention identity: semantic evidence digests must never
be presented as source commits. Mailbox history is currently only the mounted
projection. The conversation transcript is empty and its composer disabled
because no transport, agent session, message admission, or retention contract
exists yet.

The Explorer topology is intentionally narrow. It is derived from
`rey.workload-list.v5`: exact workload packages, drafts, graph/scenario/mining
counts, portfolio attention, and mapped-surface coverage counts. The separate
`/environment` route now consumes `rey.environment-status.v5` and renders its
exact variable, application, input, and reference operator projection.
`/agents` consumes the workload-list document at a higher semantic level: it
ranks current requests and attention as recommendations, then summarizes work
supported by retained test, run, mining, delta, and revision evidence. Agent
runtime discovery remains on `/environment`. Generator provenance still
supplies the current v1 agent neighborhoods in Explorer, but it is not
presented as runtime availability, live activity, or assignment. The Explorer
does not yet contain exact environment nodes, Git commit objects, source spans,
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
Search and bounded neighborhood filters should arrive before a
high-cardinality topology.

Browser mutation, workload campaign controls, authentication, multi-user
scope, remote deployment, and Spoke-backed streams remain separate decisions.
