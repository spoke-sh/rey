# ADR 0027: Environment Operator Delta

- Status: Accepted
- Date: 2026-08-09
- Extends: [ADR 0021](0021-environment-admission-index.md),
  [ADR 0025](0025-local-operator-ui.md)
- Supersedes the mapping capture and default human-status projection portions
  of: [ADR 0020](0020-environment-mapping-graph.md)

## Context

The implemented `rey env status` exposes snapshot, delta, provider, and
capability bookkeeping before it exposes the environment a programmer
recognizes. A fresh mapping graph consequently appears as a long list of graph,
node, and edge capability rows. `/environment` does not project environment
evidence at all; it repeats workload-portfolio information already owned by the
workload routes.

Rey needs its first native diff-directed operator surface. The relevant object
is the bounded environment document: selected `NAME=value` variables,
applications searched for and found or not found, declared inputs, and exact
reference edges. The authoritative capability snapshots and deltas remain
necessary evidence, but they are not the default human grammar.

Literal variable history is impossible under `rey.env-map.v1`, which retains
only presence or a value digest. A digest can establish drift but cannot render
the reviewed source and target values. Literal capture therefore requires an
explicit new retention contract rather than a presentation trick.

## Decision

### Mapping Hard Cut

The mapping schema advances to `rey.env-map.v2`. A non-sensitive variable may
select `capture: value`, `capture: digest`, or `capture: presence`. Value
capture accepts only bounded UTF-8 and retains the exact value inside the
mapping provider's typed observation provenance so committed environment
history can reproduce a directed text diff. Sensitive variables are restricted
to presence capture; both value and digest capture are rejected.

Value capture is opt-in. It can retain local paths, endpoints, flags, or other
plain configuration in `.rey/env` history. Mapping authors must not select it
for credentials or values whose retention is unsafe. Human rendering escapes
line-breaking and control characters without changing the retained structured
value.

Executable declarations also retain the bounded search scope used for their
observation. `not found` means not found in those exact captured search paths;
it is never a claim about the whole host.

### Shared Operator Projection

`rey.environment-status.v3` retains the complete working capability snapshot,
the optional admission index, and authoritative `HEAD → INDEX` and
`INDEX → WORKING` capability deltas from v2. It adds one deterministic operator
projection derived from those same three frozen planes:

```text
HEAD → INDEX → WORKING
  variables
  applications
  inputs
  references
```

Each mapped object retains its stable mapping node identity, declaration,
optional observation in all three planes, and explicit staged, unstaged, and
overall change classification. The projection summary keeps searched, found,
not-found, error, changed, and complete counts separate.

The default `rey env status` document leads with `HEAD → WORKING`, renders the
tracked variable relation as an env-shaped directed text diff, then groups
applications into `FOUND`, `SEARCHED, NOT FOUND`, and observation errors.
Admission state remains visible without printing generic capability rows,
semantic digests, or local storage paths by default. Structured JSON retains
all lower-level evidence.

### UI Projection

`GET|HEAD /api/v1/environment` returns the same freshly derived
`rey.environment-status.v3` document used by the CLI. `/environment` renders
its operator projection with Hifi's Kinetic grammar and Precision theme. The
primary surface is the variable diff; application search results, input
identities, reference topology, completeness, and admission state remain
separate inspectable dimensions.

The endpoint and page are read-only. Passive revalidation performs a fresh
bounded observation but does not add, commit, execute, admit, or schedule
anything. It uses the environment store already owned by the selected
workspace and does not create a UI-specific state model.

## Consequences

- Humans and agents inspect the same typed environment delta through different
  projections rather than separate inventory implementations.
- Literal diffs are honest only for explicitly value-captured, non-sensitive
  variables; digest and presence modes remain visibly non-literal.
- Missing applications stay first-class evidence with exact bounded search
  scope.
- Internal capability identity remains available in structured and diagnostic
  evidence without dominating ordinary status output.
- `rey.env-map.v1` files are rejected by the v2 loader. Existing verified
  capability-history snapshots remain readable, and the operator projection
  degrades legacy mapping provenance to digest/presence evidence where exact
  values were never retained.
- Browser admission controls, environment mutation, application invocation,
  dynamic mining proposals, and tick scheduling remain later decisions.
