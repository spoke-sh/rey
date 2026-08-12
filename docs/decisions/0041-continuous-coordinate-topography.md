# ADR 0041: Continuous Coordinate Topography

- Status: Accepted; workload admission path superseded by ADR 0049
- Date: 2026-08-10
- Extends: [ADR 0017](0017-mining-capability-model.md), [ADR
  0026](0026-context-topology-explorer.md), and [ADR
  0032](0032-seed-discovery-survey-and-live-communications.md)
- Supersedes: the complete matrix-style Explorer coordinate grammar in [ADR
  0030](0030-operator-cadence-agents-and-explorer-coordinates.md)
- Extended by: [ADR 0042](0042-world-geometry-and-probe-navigation.md), which
  adds the World level and probe-first navigation; [ADR
  0043](0043-emergent-natural-features-and-separate-paths.md) replaces its
  literal transport projection with emergent natural features and separate
  path evidence; [ADR 0044](0044-explorer-projection-engine.md) formalizes the
  high-dimensional projection-engine and terrain-fidelity boundary

## Context

The first Explorer proves that smooth pointer-centered motion can move one
bounded workload projection through landscape, neighborhood, and object
grammars. Its world is still a fixed canvas derived from
`rey.workload-list.v5`. It cannot zoom far enough out to show an expanding
context, distinguish surveyed from unexplored terrain, or explain how exact
anchors entered the map.

The existing term *Explorer coordinate* also combined two different things:
an address in context and browser presentation state. The former matrix route
binds a Rey object kind, identity, revision, lens regime, and sometimes agent
role. Zoom and lens are not part of an object's semantic identity, and a
browser route is not the Spoke coordinate system Rey intends to retrieve and
project.

Rey already has the pieces of an incremental map: process-owned discovery
seeds, agent-generated workloads, native locators, bounded mining, directed
deltas, and retained workload evidence. They need one explicit topography
contract before the canvas or a generic scheduler grows.

## Decision

### Coordinates, locators, and views

A **coordinate** is a typed, provider-qualified address in the Spoke
coordinate model. It identifies an object or bounded region independently of
how a browser draws it. Its exact provider, space or namespace, native locator
payload, identity class, and source/version binding remain available to the
resolver and projection. Provider-owned payloads stay opaque.

The current Spoke repository publishes exact resource locators, but not yet a
general public coordinate algebra for Rey to import. This decision records the
required cross-project contract rather than minting counterfeit global Spoke
identities. In zero-Spoke operation Rey may retain an explicitly local
coordinate binding over native locators. That binding carries standalone
provider and retention guarantees and never claims Spoke durability, global
resolution, or federation.

A **locator** is a candidate address emitted during survey. Locator resolution
is a separate bounded operation. It may return a coordinate binding, or a
typed missing, stale, unsupported, unauthorized, malformed, or truncated
outcome. Finding a string that looks like a URI neither resolves it nor grants
authority to read it.

The implemented standalone coordinate family is:

```text
rey+local://{kind}/{identity}?revision={revision}[&role={agent-role}]
```

`kind` is currently `agent`, `attention`, `cluster`, `portfolio`, or
`workload`. Every coordinate is revision-bound. `role` is required only for an
agent and is `coding_harness`, `human`, or `rule`. The canonical serializer
orders `revision` before `role`, percent-encodes the identity and values, and
rejects duplicate, unknown, empty, or non-canonical dimensions. This local
family is a provider-qualified carrier with explicit zero-Spoke guarantees; it
does not pretend to be a globally resolved Spoke resource.

An **Explorer view** is a presentation envelope over a selected coordinate:

```text
Explorer view = coordinate + camera center + continuous scale + viewport
                + projection revision + optional selection
```

Camera center, scale, viewport, selection, and semantic lens are not coordinate
identity. The canonical deep-link envelope is:

```text
/explore?coordinate={percent-encoded-coordinate}&scale={canonical-number}
```

The selected semantic coordinate is the first-slice camera anchor. Free pan
offset and viewport size remain ephemeral until admitted topography supplies a
stable spatial-coordinate contract; they are not silently serialized as
resource identity.

The pre-alpha cutover is hard. The matrix route and parser are removed; old
`/explore/{kind}/{identity};...` links resolve as missing rather than being
reinterpreted. Journal contracts advance to `rey.journal-entry-proposal.v2`,
`rey.journal-entry.v2`, `rey.journal-log.v2`, and
`rey.journal-admission.v2`. Their binding stores the semantic coordinate,
numeric scale, and matching source revision as separate fields. Existing v1
Journal state requires explicit regeneration; Rey provides no dual reader or
automatic migration.

### A continuous semantic lens

Explorer owns one continuous camera scale. Pointer-centered wheel zoom retains
the coordinate under the pointer; keyboard and control zoom retain the current
focus. Semantic level of detail is a deterministic projection of that
continuous scale with hysteresis around boundaries, so small input changes do
not flicker between grammars.

The target five-level vocabulary is:

| Level | Projection |
| --- | --- |
| Atlas | Discovered spaces, providers, repositories, coverage contours, survey boundaries, and unexplored regions |
| Landscape | Bounded regions, corpora, workloads, and concentrations of typed attention |
| Neighborhood | Exact anchors and classified relationships around one coordinate |
| Object | Exact files, documents, symbols, workloads, graphs, scenarios, artifacts, and deltas |
| Evidence | Exact source spans, rows, graph nodes, diff hunks, omissions, and lineage |

These are level-of-detail projections, not five different graphs. A source
identity and its selected coordinate survive every transition. A projection
may aggregate, cluster, label, or omit under declared bounds; it may not invent
terrain, change assessment, or imply a relationship from spatial proximity.
This decision's first hard-cut implementation accepted a `0.12..=5.4` camera
scale and projected those five levels. ADR 0042 extends the current bound to
`0.05..=5.4` and adds World before Atlas; the continuous numeric `scale` remains
present in every exact view link.

The Atlas and Landscape visual primitive is a terrain-style relief layer, not
a dashboard of region summaries. Rey derives a bounded scalar height field
from admitted anchor prominence and exact classified edges, then extracts
nested contour isolines. Anchors remain stable points of interest over that
relief. Zoom adds labels, boundaries, classified relationships, objects, and
evidence to the same scene rather than replacing it with an unrelated layout.
As additional admitted patches enlarge the bounded world, zooming out reveals
additional survey scenes.

The current zero-Spoke layout is relational, not semantic: deterministic
positions come from admitted anchor and edge topology. Relief therefore means
anchor concentration and classified connectivity only. It does not claim that
screen distance is language similarity. A high-dimensional language or
embedding provider must bind its coordinate system, model and implementation
revision, projection, limits, and omissions before semantic distance may
become an observed relief input.

### Incremental topography

**Context topology** is the bounded typed anchors and classified relationships
Rey can currently explain. **Context topography** adds scale, coverage,
density, survey boundaries, frontier, and explicit unexplored space. It is a
sparse, evidence-backed map, not a claim that Rey has observed the whole
project or context universe.

An admitted survey workload emits a content-identified **topography patch**.
Each patch binds:

- exact input seeds, source revisions, providers, capability snapshot,
  workload/graph/scenario revisions, run identity, and limits;
- located candidates and their native locators;
- resolution outcomes and exact coordinate bindings where available;
- typed anchors and classified directed relationships;
- surveyed regions, coverage units, omissions, and completeness;
- unresolved boundary coordinates that may enter the portfolio frontier; and
- a directed patch delta against the selected prior topography revision.

The visible map is a deterministic bounded projection of admitted patches.
Overlaps and conflicts remain inspectable; later evidence does not silently
erase earlier lineage. Staleness is derived from changed bound inputs. Empty
space is labeled either surveyed-empty or unexplored—never filled by visual
interpolation and presented as evidence.

### The first survey voyage

A **survey voyage** is a bounded series of admitted survey-workload runs over a
declared frontier. It is a derived operational description, not a new
scheduler, retention store, or implicit recursive crawler.

For a newly encountered project, the first voyage begins from the process-owned
`PWD` seed and a small process-owned seed-name inventory that includes
`AGENTS.md` and README variants such as `README.md`. These names are survey
inputs, not configuration files and not discovery truth. An agent-generated
workload may locate the bounded files, mine URI and reference candidates,
classify declared relationships, and emit the first topography patch.

The graph and its scenario suite enter through the existing workload creation
and admission path. They are not hard-coded scenarios hidden in the runtime.
Resolution and any subsequent retrieval use declared capabilities and limits.
The resulting unresolved boundary may propose another survey run, but only the
normal admission and scheduling contracts may execute it.

Opening, panning, or zooming Explorer performs bounded retrieval and projection
of already admitted map evidence. It never authorizes a provider, executes a
locator, starts a workload, or recursively expands the map as a hidden browser
side effect.

### Human and agent surfaces

The agent-facing proof remains workload-centered:

```text
rey workloads create context-anchor-survey --intent <bounded-intent>
rey workloads test context-anchor-survey -vv
rey workloads run context-anchor-survey --source AGENTS.md --source README.md
rey workloads status context-anchor-survey
```

Exact generated package contents and source arguments remain project-specific.
The human output and structured contracts must expose seed coverage, located
candidates, resolution classes, produced anchors and relationships, patch
delta, frontier, omissions, limits, and lineage. `/explore` consumes the same
admitted topography evidence. A private canvas endpoint or a hidden scan does
not satisfy this interface.

## Consequences

- The matrix coordinate grammar no longer exists in current interfaces or
  retained Journal schemas.
- The current fixed canvas and three implemented regimes are a valid narrow
  topography projection, not the final map contract.
- Explorer can zoom farther out without reducing zoom to geometric scaling or
  treating aggregates as new truth.
- Survey becomes an ongoing workload family that incrementally expands an
  evidence frontier; it is not a one-time bootstrap phase.
- Unexplored, surveyed-empty, omitted, stale, and unsupported regions remain
  distinct.
- Rey needs a coordinate/locator conformance boundary with Spoke before it can
  claim connected coordinate semantics.
- The first implementation slice must prove one seed-to-patch-to-Explorer path
  through `rey workloads ...` before generic topographic scheduling or broad
  crawling is considered.
