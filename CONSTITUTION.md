# Rey Constitution

This document defines Rey's durable values and invariants. Product details and
provisional syntax belong in architecture, interface, and plan documents. The
constitution should change rarely.

## Decision Hierarchy

Resolve conflicts in this order:

1. **Constitution** — values and invariants.
2. **Architecture, mining, environment, Git, diffs, proofs, and interfaces** —
   ownership and contracts.
3. **Accepted decisions** — consequential choices within those boundaries.
4. **Active plans** — delivery sequence and acceptance criteria.
5. **Code and tests** — current implementation facts.
6. **External documentation** — upstream constraints and standards.

When a higher-level decision intentionally changes, update stale lower-level
documents and behavior in the same change.

## Project Values

### 1. Mine Context Into Evidence

Rey turns explicitly selected context into bounded, addressable evidence before
policy proposes work. Relational and source mining are peer capability
families: typed query, grouping, traversal, and comparison coexist with text
search, parsing, indexing, structural analysis, and visualization. Derived
structure never severs its lineage to exact sources or hides incomplete work.

### 2. Deltas Direct Compute

A delta is a first-class runtime value, not presentation added at the end. Rey
uses changed and unresolved state to invalidate observations, prioritize a
frontier, and select the next bounded computation.

### 3. Bind Exact Inputs And Capabilities

Every observation, action, and proof binds exact source revisions, lens
definitions, capability snapshots, tool identities, and relevant runtime
policy. A mutable name or unresolved ambient environment alone is not
reproducible evidence.

### 4. DataFrames Are The Local Coordinate System

Polars DataFrames are the canonical in-process representation for typed
collections, observations, mined relations, frontiers, and query results.
Apache Arrow is the preferred typed interchange family. DataFrames remain
bounded working state; they do not replace durable content, ordered source
text, native artifacts, trees, graphs, or resource identity.

### 5. Observe The Environment Explicitly

Rey discovers useful local and remote context surfaces through bounded,
versioned providers. Discovery is an observation, not permission to execute
arbitrary tools. The capability snapshot is evidence and changes to it can
invalidate actions and proofs.

### 6. Let Spoke Amplify, Not Gate, Rey

Rey remains useful with zero Spoke capabilities. When present, Spoke is Rey's
durable reasoning and compute plane. Rey integrates through public Spoke
contracts and does not imitate stronger Spoke storage, query, or execution
claims in standalone mode.

### 7. Degrade Capabilities Visibly

A missing provider removes declared capabilities; it never silently weakens a
claim. A run records which guarantees were available, which were absent, and
whether that makes an action unavailable or a proof inconclusive.

### 8. Separate Policy From Mechanism

The deterministic runtime owns validation, effects, comparison, invalidation,
limits, lineage, and proof assembly. An agent, rule, or human policy may propose
a compute-graph revision or action but cannot redefine evidence, qualify its
own proposal, or bypass admission.

### 9. Make Effects Explicit

Read-only lenses and probes are safe and replayable. Mutations use explicit
resource operations or admitted compute actions with declared effect classes.
A query never hides a write.

### 10. Scope Every Proof

A passing proof states exactly what was compared, under which keys and
normalizers, with which coverage and limits. Missing evidence, unsupported
controls, and budget exhaustion remain visible and can make a result
inconclusive.

### 11. Preserve Direction And Meaning

Every delta names its source and target. Insertions, deletions, and
modifications must be interpretable without color or surrounding prose. A
rendering may simplify presentation but cannot silently discard types, keys,
revisions, or comparison semantics.

### 12. Bound The Loop

Frames, mining requests/results, queries, probes, actions, queues, traces, and
proofs have explicit row, byte, time, memory, depth, concurrency, and iteration
limits where applicable. Stopping because a bound was reached is observable;
it is never represented as convergence.

### 13. Co-Evolve Without A Dependency Cycle

Rey is a reference external application of Spoke and should turn real client
friction into conformance evidence that improves both systems. Rey must still
run when Spoke is absent or unhealthy, and Spoke must build and start without
Rey. Cross-project improvement follows public contracts and versioned evidence,
not private imports or circular bootstrapping.

### 14. Evidence Beats Aspiration

Correctness, parity, convergence, determinism, incrementality, and performance
claims require tests or generated evidence. A design document is not proof that
the runtime implements its target contract.

### 15. Git State Activates Through Deltas

For software spaces, commit, ref, index, and declared worktree observations are
first-class frames. Polling compares frozen snapshots; it does not assume refs
are append-only or the index is immutable. Git deltas may activate workload
graph entry points, but they never bypass normal action admission.

### 16. Project High-Dimensional Context As An Evidence-Bound World

Explorer is a high-fidelity spatial game engine specialized for evidence-bound
projections of high-dimensional context. Its scene compilation, cameras, level
of detail, field simulation, materials, render passes, picking, and accelerated
graphics remain semantically subordinate to admitted evidence. A rendered
surface, simulated feature, visual distance, animation, or user gesture cannot
mint source truth, authority, coverage, or progress.

Projection fidelity is a correctness concern for the human interface, not
license to interpolate unknown evidence. Stable coordinates, exact source
links, validity masks, semantic levels of detail, omissions, limits, and
revision lineage must survive every rendering backend and visual lens. The
engine should degrade visibly when fidelity or acceleration is unavailable
without changing the underlying assessment.

### 17. Author Worlds As Candidates, Admit Them As Evidence

Explorer is the read-first projection of an admitted scene, not its level
editor. Surveys, agents, and humans may assemble native terrain, feature,
marker, label, hydrology, and boundary artifacts through a separate editor
candidate plane, but a candidate cannot write an admitted topography patch,
projection packet, or browser scene directly. Exact native artifacts,
coordinates, identities, changes, limits, and omissions must pass through an
explicit qualified workload before they can affect `/explore`.

Standard formats retain their native semantics. Geographic coordinates cannot
be relabeled as semantic embeddings, vector lines cannot silently become
relationships or paths, and bounded indexes cannot replace the source
artifact. Editor staging and packaging make candidates reviewable and
reproducible; they grant no read, probe, action, or admission authority.

## Frame And Delta Invariants

- A frame has a stable logical schema, source bindings, lens revision,
  evaluation bounds, and content identity.
- A frame records the capability snapshot and provider guarantees needed to
  interpret how it was produced.
- Keyed comparison requires declared key columns and proves their uniqueness in
  each compared input.
- Comparison direction and labels are explicit and survive every encoding.
- Typed before/after values remain available even when a text or Tabular Diff
  rendering combines them for display.
- Normalization is versioned, deterministic, reviewable, and included in the
  delta identity.
- An incompatible schema, missing key, duplicate key, truncated input, or
  failed probe produces an explicit non-passing outcome rather than a guessed
  diff.
- Re-evaluating identical frozen inputs with the same implementation and limits
  produces the same semantic delta.

## Mining Invariants

- Mining is ongoing at two levels: workloads mine their declared domains, and
  the runtime mines the workload portfolio for qualification, staleness,
  capability, dependency, ownership, and coverage evidence.
- Portfolio attention is a typed derived relation, not a scheduler or agent
  assertion. Ready, blocked, inconclusive, and policy-excluded work remain
  distinct, and a clean portfolio is represented by typed empty attention.
- A scheduler may select admitted attention but does not invent its reasons.
  A proposer may act on bounded selected evidence but cannot declare its own
  workload or attention row resolved.

- A mining request binds exact source or input-artifact identities, operation
  contract and implementation revision, canonical parameters, capability
  snapshot, effective limits, and the workload/frontier rationale for the work.
- Relational mining retains schemas, keys, ordering, types, and contributing
  scope. Source mining retains native content addresses, encoding and language
  rules, exact spans, and derivation links for syntax, indexes, graphs, and
  metrics.
- Reading mutable state or invoking an external mining tool is an explicit
  probe. Discovering `rg`, a parser, compiler, index, or query provider does not
  grant permission to execute it.
- Every mining result distinguishes complete, partial, truncated, unsupported,
  unavailable, and failed evidence and records omissions and effective bounds.
- A derived relation, metric, summary, patch, tree, graph, or visualization is
  never the sole authoritative copy of user-authored source.
- Visualization preserves evidence direction, scope, aggregation, elision,
  completeness, and deep links. It cannot change delta assessment, coverage,
  progress, confidence, or proof status.
- Spatial projections bind their coordinate or embedding basis, field and
  material revisions, validity masks, level-of-detail rules, render limits,
  and omissions. Visual smoothing, shading, or simulation never fills an
  unknown region with evidence.

## Transition Invariants

- An action names the frame and source revisions against which it was proposed.
- An action names the capability snapshot against which it was admitted and is
  rejected if required tools or guarantees have changed.
- Admission revalidates frozen preconditions before an effect begins.
- An observation is read-only. An effectful action declares its mutation
  boundary and cannot be executed by a query path.
- A completed process is evidence, not by itself a successful semantic
  transition; post-action lenses determine the observed result.
- Retry never erases an earlier attempt or rewrites its lineage.
- Frontier updates derive from committed observations and deltas, not from an
  agent's unsupported assertion about what changed.
- A poll cursor advances only after its deltas, activations, and required
  evidence reach the claimed retention boundary.
- Trigger replay is idempotent. Rey does not claim exactly-once activation from
  a mutable Git repository.
- Ref rewrites, incomplete history, index conflicts, and unsupported index
  semantics remain explicit rather than being flattened into append events.

## Proof Invariants

- A proof names a claim, scope, expected predicate, evaluated observations,
  coverage, omissions, and limits.
- Proof status is one of `pending`, `passed`, `failed`, `inconclusive`, or
  `stale`.
- A passing proof contains no failed required check and no unacknowledged
  missing evidence.
- A proof becomes stale when any bound source, provider, capability, workload,
  compute graph, scenario, lens, mining operation, parser/index, normalizer,
  policy, candidate, fixture, tool, guarantee, or evaluator implementation
  changes.
- Similarity and progress scores help navigate evidence; neither is a parity
  proof.
- Evidence is content-addressed or bound to the strongest immutable source
  revision available. A certificate states whether evidence is local-only or
  Spoke-backed and never claims stronger durability than its provider.

## Collaboration Rules

- Keep target architecture separate from current repository truth.
- Record consequential choices before coupling implementation broadly to them.
- Prefer the smallest end-to-end slice that proves a runtime invariant in
  standalone mode, then proves the same semantic contract with Spoke
  amplification where applicable.
- Use Rey's observations of Spoke as actionable compatibility evidence, and
  feed newly implemented Spoke capabilities back into Rey's capability probes.
- Treat commit and index deltas in Rey and Spoke checkouts as pollable
  activation sources without making either repository a boot dependency.
- Update documents, decisions, plan checklists, examples, and tests with the
  behavior they describe.
- Make hard cutovers during pre-alpha development unless a plan explicitly
  defines a migration.
- Keep credentials, local Spoke data, generated traces, and large proof
  artifacts out of source control unless they are intentional bounded fixtures.
