# ADR 0030: Operator Cadence, Agent Index, And Explorer Coordinates

- Status: Accepted
- Date: 2026-08-09
- Extends: [ADR 0026](0026-context-topology-explorer.md)
- Superseded: the `/agents` registry projection is replaced by [ADR
  0034](0034-agent-runtime-inventory-and-derived-task-plane.md); v1 Explorer
  coordinates remain current.

## Context

The Explorer has retained typed focus while zooming, but that focus has lived
only in mounted React state. Other human surfaces cannot point to an exact
topology object, a browser reload loses the coordinate, and an operator cannot
share a location with another human or agent.

Two new operator views need that addressing boundary. Cadence must expose the
ticks Rey can actually observe without flattening independent clocks into a
fictional total-order event stream. Agents needs a familiar registry over the
coding harnesses, rules, and humans already named by admitted workload
provenance, with a direct route from each record into its Explorer
neighborhood.

The matrix-URI design note describes a useful model: a hierarchical resource
path followed by unique, equally significant named dimensions separated with
semicolons. It also explicitly says relative matrix-URI processing was never
generally implemented. Rey can adopt the named-dimension model without
claiming a new Web standard or depending on relative matrix references.

## Decision

Explorer coordinates use this v1 shape:

```text
/explore/{kind}/{identity};at={revision};lens={regime}[;role={agent-role}]
```

The current kinds are `portfolio`, `cluster`, `workload`, `attention`, and
`agent`. The current regimes are `landscape`, `neighborhoods`, and `objects`.
`at` binds the strongest revision present in the source projection: workload
or request identity, portfolio-attention identity, source snapshot, or agent
producer revision. `role` is required for agent coordinates and distinguishes
`coding_harness`, `rule`, and `human` provenance.

Matrix parameters are semantically unordered, unique, and non-empty. `at` is
required for every non-cluster coordinate. The
parser rejects duplicates, unknown parameters, invalid regimes/roles, and
ambiguous agent coordinates. The canonical serializer percent-encodes values
and orders parameters lexically as `at`, `lens`, then `role`. Rey does not
implement relative matrix references or semicolon-only navigation.

Examples are:

```text
/explore/portfolio/current;at=blake3%3A...;lens=landscape
/explore/workload/rey.portfolio.label-normalization;at=blake3%3A...;lens=objects
/explore/attention/blake3%3A...;at=blake3%3A...;lens=objects
/explore/agent/codex;at=gpt-5;lens=objects;role=coding_harness
```

A coordinate resolves against the currently loaded bounded source document.
When `at` no longer matches, the Explorer labels the coordinate `STALE` and
shows the current binding. When the identity is absent, it labels the
coordinate `MISSING`. The first slice does not reconstruct historical source
documents and must not silently show a current object as though it satisfied an
older binding.

`/agents` is a deterministic traditional registry derived from
`rey.workload-list.v5`. One row represents one exact generation tuple of kind,
producer, and producer revision. It aggregates only that tuple's admitted
workload package sources, workload ids, scenario counts, and attention counts.
Creation requests without a materialized package remain visible as unassigned
handoffs; Rey does not invent an agent identity for them. Every identified row
emits the exact agent Explorer coordinate above. Agent object scenes preserve
the boundary that a generator proposes outputs but cannot qualify its own
graph or resolve runtime attention.

`/cadence` consumes `GET /api/v1/cadence`, schema `rey.ui-cadence.v1`. Cadence
is a partial-order projection made of explicit lanes:

- the Git lane is the newest 24 commits reachable from the currently observed
  `HEAD`, read through bounded direct Git argv with exact OIDs, parents,
  committer times, subjects, object format, limit, and omissions;
- the Rey admission lane is the newest 24 verified environment commits plus
  the current admission index when present, ordered by environment sequence;
  environment commit v1 has no wall time, so the UI says `ORDER ONLY`; and
- scan schedules describe the already accepted five-second portfolio,
  environment, and cadence passive browser revalidation contracts, including
  their route activation, read-only browser authority, and last-good-document
  retention.

The Git lane is reachable-history evidence, not an append log, ref-movement
classifier, poll cursor, or activation stream. A shallow or truncated history
is incomplete. Every displayed Git commit SHA is itself a link to the exact
commit on Rey's canonical GitHub repository, but only when the selected
workspace HEAD equals the running Rey implementation revision. Without that
binding, cadence shows the repository boundary instead of an inert SHA; an
arbitrary workspace is never mislabeled as Rey source. The schedules are
mounted browser projection behavior, not the
generic runtime scheduler. Cadence never interleaves Git and environment ticks
unless later retained evidence supplies an explicit cross-clock edge.

## Consequences

- Humans, agent records, and future evidence views can share stable Explorer
  locations without moving topology authority into the URI.
- Exact revision qualifiers make stale links visible, while historical
  reconstruction remains a separate storage/evidence decision.
- The Agents page exposes only identities supported by admitted provenance and
  makes unassigned work explicit.
- Cadence is useful now but honestly partial: it cannot claim every Git
  transition, globally order an environment admission, or present passive UI
  reads as admitted runtime work.
- Git SHA presentation is actionable by construction without confusing
  semantic digests or unbound repositories for GitHub commits.
- Complete Git ref polling, activation replay, runtime scheduling, and durable
  event streams remain owned by their existing plans and contracts.
