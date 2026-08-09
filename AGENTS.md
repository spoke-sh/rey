# AGENTS.md

Shared guidance for AI agents working on Rey.

## Project Direction

Rey is a diff-directed mining and compute runtime for agents. It inventories
explicit context, mines bounded relational and source evidence, computes
directed deltas, uses unresolved deltas to choose subsequent work, and emits
scoped proof artifacts. Mining spans typed retrieval/grouping/traversal and
text search/parsing/indexing/metrics/visualization without flattening native
artifacts into artificial tables.

Rey first inventories its environment and remains useful without Spoke. When
available, Spoke is Rey's durable reasoning and compute plane. Rey must not
duplicate Spoke storage, query, document, stream, table, tool, run, or capture
ownership or pretend standalone providers offer those guarantees. The runtime
remains deterministic without an LLM; an agent is one policy that may propose
compute-graph revisions or actions through the same validated interface as
rules or humans.

Rey is Spoke's first external runtime application. Preserve a two-way
improvement loop: Rey exposes public-contract gaps in Spoke, and new Spoke
capabilities become discoverable Rey providers. Do not create a build, package,
storage, or startup cycle between the repositories.

The repository currently contains foundational documents, a pinned Rust
development shell, and an eleven-crate Cargo workspace. Executable behavior
remains narrow: inspect current files and tests before inferring that a
provider, adapter, scheduler loop, action executor, or Spoke integration
exists.

## Read This First

1. `README.md`
2. `CONSTITUTION.md`
3. `INSTRUCTIONS.md`
4. `docs/ARCHITECTURE.md`
5. `docs/MINING.md`
6. `docs/WORKLOADS.md`
7. `docs/RUNTIME.md`
8. `docs/FRONTIER.md`
9. `docs/ENVIRONMENT.md`
10. `docs/GIT.md`
11. `docs/DIFFS.md`
12. `docs/PROOFS.md`
13. `docs/INTERFACES.md`
14. `docs/DEVELOPMENT.md`
15. `plans/README.md` and the active plans
16. `docs/decisions/README.md`

## Core Principles

- Treat the delta as a scheduler input, not only an output artifact.
- Treat mining as the bounded bridge from context to evidence: bind exact
  sources, operation revisions, parameters, completeness, lineage, and limits.
- Keep relational and source mining as peer capability families. Preserve
  native text/artifacts while projecting genuinely relational structure into
  DataFrames.
- Treat visualization as an evidence projection that retains direction, scope,
  omissions, and exact source links; it cannot change semantic assessment.
- Treat the workload as the public unit of computation: one versioned graph,
  scenario suite, policy boundary, qualification contract, and total budget.
- Let agents propose compute-graph revisions through the same validated
  contract as rules or humans; deterministic scenarios decide qualification.
- Preserve failing scenario results as directed typed deltas from expected to
  observed output.
- Treat the capability snapshot as a first-class, changing runtime input.
- Treat Git commit, ref, and semantic index snapshots as first-class software
  activation inputs.
- Support a zero-Spoke standalone profile with explicitly narrower guarantees.
- Bind observations, actions, and proofs to exact source and implementation
  revisions.
- Use Polars DataFrames as canonical bounded in-process state and Arrow as the
  preferred typed interchange family.
- Keep a typed delta authoritative; Tabular Diff is a portable projection.
- Separate deterministic runtime mechanism from agent policy.
- Keep reads safe and mutations explicit.
- Distinguish process success from semantic convergence.
- Scope proof claims and expose missing, ignored, unsupported, and truncated
  evidence.
- Make staleness a derivation from changed inputs, not a manually maintained
  label.
- Prefer generated evidence and focused tests over prose-only claims.

## Target System Shape

The implemented and target ownership map is:

```text
rey                 workload CLI, catalog composition, and local orchestration
rey-core            identities, revisions, limits, and shared contracts
rey-mining          mining operations, requests/results, artifacts, and views
rey-dataframe       frame schemas, Polars helpers, and Arrow codecs
rey-environment     bounded capability discovery and local context providers
rey-git             repository snapshots, semantic index, polling, and activation
rey-diff            typed deltas, alignment, summaries, and renderings
rey-runtime         workloads, graph/scenario transitions, actions, and bounded loop
rey-frontier        invalidation, dependencies, prioritization, convergence
rey-proof           claims, evidence manifests, certificates, and staleness
rey-policy          policy proposal contract; no provider-specific agent loop
rey-spoke           optional Spoke provider, source bindings, runs, and persistence
```

This is an ownership map, not a requirement that every boundary become a
process. The workspace proves which crates exist; provider implementations and
the optional `rey-spoke` boundary remain plan-owned target work.

## Decision Resolution

Resolve ambiguity in this order:

1. `CONSTITUTION.md`
2. `docs/ARCHITECTURE.md`, `docs/MINING.md`, `docs/WORKLOADS.md`,
   `docs/RUNTIME.md`, `docs/FRONTIER.md`, `docs/ENVIRONMENT.md`, `docs/GIT.md`,
   `docs/DIFFS.md`, `docs/PROOFS.md`, and `docs/INTERFACES.md`
3. accepted ADRs
4. active plans
5. code and tests
6. external standards and libraries

If an intentional higher-level decision conflicts with a lower-level artifact,
update the stale artifact in the same change.

## Project Conventions

- Make hard cutovers during pre-alpha development unless a plan explicitly
  defines migration behavior.
- Probe only declared environment surfaces with bounded read-only discovery;
  finding a tool does not grant permission to execute it.
- Freeze provider, path, version, digest/provenance, trust, and supported
  operations before a discovered tool participates in an action.
- Bind every mined artifact to its request, exact inputs, operation and
  implementation revision, capability snapshot, effective limits,
  completeness, omissions, and derivation lineage.
- Treat a mutable read or external miner invocation as an explicit probe;
  pure projection over frozen evidence may remain deterministic in-process
  compute.
- Use semantic Git index entries for staged-change triggers; raw index byte or
  mtime changes alone are not semantic activation.
- Classify ref movement as fast-forward, rewind, rewrite/divergence, or unknown.
  Never fabricate append events across a rebase or incomplete history.
- Advance Git poll cursors only after retained transition evidence and make
  activation replay idempotent.
- Never silently substitute standalone evidence for a required Spoke-backed
  claim.
- Keep raw bytes and native artifacts out of DataFrame wrappers when tabular
  semantics add no value.
- Require unique keys before comparing unordered relations.
- Retain source/target labels, types, schemas, keys, revisions, and normalizers
  through every delta representation.
- Reject stale action preconditions before effects.
- Keep `QUERY` paths read-only; use explicit local actions or Spoke resource
  methods/admitted compute for mutation according to the active provider.
- Never let an agent directly declare its own proof successful.
- Keep similarity, confidence, coverage, progress, and proof status separate.
- Apply limits before optimization work is accepted.
- Do not select a persistence engine, policy protocol, or deployment topology
  in a drive-by dependency change.
- Update documentation and plan checklists with public behavior.

## Plans And Proof

- Implement the smallest end-to-end slice that proves a diff invariant with no
  Spoke, then exercise the same contract through Spoke when available.
- Environment work needs fixtures for missing tools, version drift, path
  changes, timeouts, malformed version output, trust classification, and
  capability degradation.
- Mining work needs fixtures for exact source drift, empty and bounded search,
  invalid encoding, tool failure/drift, typed empty relations, text/structural
  direction, traversal limits, derivation lineage, visualization omissions,
  completeness, and deterministic replay.
- Workload work needs fixtures for invalid and cyclic graphs, missing graph
  policy, passing/failing/inconclusive scenarios, graph-revision invalidation,
  qualification, progress counts, and test/run parity.
- Git work needs fixtures for ref rewrites, merges, shallow history, detached
  HEAD, semantic index changes, conflicts, linked worktrees, bounded traversal,
  and cursor replay.
- Diff changes need fixtures for insertion, deletion, modification, schema
  change, null handling, duplicate keys, typed empty frames, and determinism.
- Runtime changes need cancellation, timeout, budget, retry, stale-input, and
  partial-failure tests.
- Proof changes need tampering, changed-input, changed-evaluator, missing
  evidence, inconclusive, and stale verification tests.
- Spoke changes need direct and routed contract evidence with exact revision and
  run lineage.
- Cross-project work needs a conformance artifact that can direct the next
  change in Rey or Spoke without importing either repository's internals.
- CLI changes need stdout, stderr, structured output, and exit-code tests.
- Performance claims need named workloads and preserved comparison results.

## Hygiene

- Preserve user changes and unrelated work in a dirty worktree.
- Keep credentials, local Spoke state, private codebase snapshots, and generated
  proof artifacts out of source control.
- Do not treat a cache, frame, delta rendering, or projection as the sole copy
  of user-authored data.
- Keep examples synchronized with current implementation status.
