# AGENTS.md

Shared guidance for AI agents working on Rey.

## Project Direction

Rey is a diff-directed compute runtime for agents. It observes high-dimensional
spaces as bounded typed frames, computes directed deltas, uses unresolved
deltas to choose subsequent work, and emits scoped proof artifacts.

Rey first inventories its environment and remains useful without Spoke. When
available, Spoke is Rey's durable reasoning and compute plane. Rey must not
duplicate Spoke storage, query, document, stream, table, tool, run, or capture
ownership or pretend standalone providers offer those guarantees. The runtime
remains deterministic without an LLM; an agent is one policy that may propose
actions through the same validated interface as rules or humans.

Rey is Spoke's first external runtime application. Preserve a two-way
improvement loop: Rey exposes public-contract gaps in Spoke, and new Spoke
capabilities become discoverable Rey providers. Do not create a build, package,
storage, or startup cycle between the repositories.

The repository currently contains foundational documents, a pinned Rust
development shell, and a ten-crate Cargo workspace. Executable behavior remains
narrow: inspect current files and tests before inferring that a provider,
adapter, scheduler loop, action executor, or Spoke integration exists.

## Read This First

1. `README.md`
2. `CONSTITUTION.md`
3. `INSTRUCTIONS.md`
4. `docs/ARCHITECTURE.md`
5. `docs/RUNTIME.md`
6. `docs/FRONTIER.md`
7. `docs/ENVIRONMENT.md`
8. `docs/GIT.md`
9. `docs/DIFFS.md`
10. `docs/PROOFS.md`
11. `docs/INTERFACES.md`
12. `docs/DEVELOPMENT.md`
13. `plans/README.md` and the active plan
14. `docs/decisions/README.md`

## Core Principles

- Treat the delta as a scheduler input, not only an output artifact.
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

The provisional ownership map is:

```text
rey                 CLI and local composition
rey-core            identities, revisions, limits, and shared contracts
rey-dataframe       frame schemas, Polars helpers, and Arrow codecs
rey-environment     bounded capability discovery and local context providers
rey-git             repository snapshots, semantic index, polling, and activation
rey-diff            typed deltas, alignment, summaries, and renderings
rey-runtime         observations, actions, transitions, and bounded loop
rey-frontier        invalidation, dependencies, prioritization, convergence
rey-proof           claims, evidence manifests, certificates, and staleness
rey-policy          policy proposal contract; no provider-specific agent loop
rey-spoke           optional Spoke provider, source bindings, runs, and persistence
```

This is a target ownership map, not a claim that these crates exist or must each
become a process. The active plan may refine it before scaffolding.

## Decision Resolution

Resolve ambiguity in this order:

1. `CONSTITUTION.md`
2. `docs/ARCHITECTURE.md`, `docs/RUNTIME.md`, `docs/FRONTIER.md`,
   `docs/ENVIRONMENT.md`, `docs/GIT.md`, `docs/DIFFS.md`, `docs/PROOFS.md`, and
   `docs/INTERFACES.md`
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
