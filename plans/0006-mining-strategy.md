# Plan 0006: Mining Strategy And First Executable Slice

- Status: Complete
- Completed: 2026-08-08
- Decision: [ADR 0018](../docs/decisions/0018-first-mining-workload.md)

## Outcome

Formalize mining as Rey's bounded bridge from environment context to
delta-directed evidence, then implement the smallest local-only workload that
combines relational and source mining. The slice must retain native text,
project typed match relations, compute directed relational and text deltas,
render evidence-linked human views, and feed a failing result into a bounded
reasoning surface without implementing generic scheduling, AST/CST breadth, a
durable index, or an agent loop.

## Completion Checklist

- [x] ADR 0017 accepts the relational/source mining capability model,
  visualization boundary, workload placement, and ownership constraints.
- [x] `README.md`, the constitution, agent/contributor guidance, and
  foundational architecture documents use the mining model consistently.
- [x] Version the minimum mining operation, request, result, artifact,
  completeness, dependency, limit, and visualization contracts.
- [x] Decide narrow crate ownership and add `rey-mining` only if the accepted
  contracts justify it without a dependency cycle.
- [x] Freeze one exact local source-binding and read-only source-search
  provider contract, including admitted `rg` behavior or a deterministic
  built-in baseline.
- [x] Project bounded search matches as a typed frame while retaining native
  source/context artifacts and exact deep links.
- [x] Implement authoritative ordered text delta semantics and reuse the
  generic relational delta contract where it is actually complete.
- [x] Render bounded ANSI-independent table and patch projections with explicit
  grouping, context, elision, completeness, and omissions.
- [x] Add one built-in mining conformance workload through `list`, `test`,
  `run`, and `status` without adding a peer top-level mining resource.
- [x] Derive a workload frontier from retained failing mining evidence and
  construct one verified delta-directed reasoning surface.
- [x] Prove deterministic local-only behavior, hard bounds, provider/source
  drift, malformed/unsupported input, failure, truncation, and staleness.
- [x] Keep JSON/stdout/stderr/exit behavior and human `-v`/`-vv` evidence
  contracts covered by CLI fixtures.
- [x] Run focused tests, full workspace tests, Clippy, build, Nix checks, link
  review, and repository-truth audit.

## Current Repository Truth

Implemented foundations already provide:

- bounded capability discovery and allowlisted `rg`/`git` identity probes;
- exact workspace and partial Git source context;
- Polars/Arrow frame contracts and a capability-specific relational delta;
- a narrow UTF-8 scenario-output delta;
- deterministic bounded workload DAG execution and scenario qualification;
- local workload result state and high-fidelity workload CLI projections;
- formal runtime, frontier/progress/scheduling, and reasoning-surface
  contracts; and
- local-only scoped proof bundles.

The first built-in provider now binds an explicit checked-in or caller-selected
file corpus beneath one canonical local root, retains exact native bytes,
searches case-sensitive UTF-8 literals, and emits `rey.source-matches` version
`1` with reversible path, source, pattern, byte/line span, context, provider,
request, and result lineage. Capability discovery advertises the operation
separately from the unadmitted `rg` identity probe.

The repository now executes that provider through
`rey.fixture.source-search`, compares its canonical ordered output and typed
match relation, retains complete and incomplete evidence, derives and selects
one failing frontier row, and projects one verified reasoning surface. All four
workload commands expose the evidence through tested human and JSON paths.

The repository does not execute `rg` as a mining provider, support regex/
case-folded search, parse syntax, build a semantic index, render general tree/
graph visualizations, execute a graph-revision proposal, or run recurring
scheduling.

`rey-mining` now implements canonical v1 operation/request/result
manifests. They bind typed parameters, exact artifact/provider/capability
identity, effective limits, completeness and omissions, consumption, realized
implementation lineage, invalidation dependencies, and workload/frontier
rationale. Constructors canonicalize ordering and replay verification rejects
tampering and stale request, provider, capability, or implementation bindings.

## Milestone 1 — Freeze Common Contracts

- [x] Define operation family/kind, exact identity, implementation revision,
  determinism, effect class, input/output artifact contract, typed parameters,
  provider requirements, completeness, invalidation, and limits.
- [x] Define mining request identity over workload, graph, scenario/campaign,
  transition, frontier rationale, exact inputs, operation, parameters,
  capability snapshot, provider, and effective limits.
- [x] Define mining result identity over request, realized provider/tool/query/
  parser lineage, artifacts, schemas/media types, completeness, omissions,
  consumption, and dependency edges.
- [x] Define native, relation, tree, graph, metric, delta, and visualization
  artifact references without embedding raw artifacts in a generic envelope.
- [x] Define complete, partial, truncated, unsupported, unavailable, and failed
  result states and their legal artifact/omission shapes.
- [x] Keep semantic identity free of timestamps and display-only layout while
  including every parameter and limit that can change evidence meaning.
- [x] Decide whether a narrow `rey-mining` crate improves ownership; do not put
  the contracts into `rey-core` merely for convenience.

## Milestone 2 — Source Binding And Search

- [x] Select an explicit fixture workspace/corpus beneath a canonical allowed
  root and define exact source identity before retrieval.
- [x] Implement bounded native text retrieval with file, byte, line,
  context-window, encoding, and symlink/path-escape rules.
- [x] Implement one source-search operation with canonical literal/regex,
  case, path, context, and limit parameters.
- [x] Keep `rg` unadmitted in this slice; any later adapter must freeze path,
  version, digest/provenance, trust, exact argv, cwd, environment, output
  format, time/capture limits, and parser behavior before admission.
- [x] Provide a deterministic in-process baseline or reviewed fixtures so tool
  behavior can be tested without confusing availability with semantics.
- [x] Detect source drift between binding and retrieval and emit stale or
  inconclusive evidence rather than combining revisions.
- [x] Make ignored/generated/binary/invalid-encoding and unsupported cases
  explicit.

## Milestone 3 — Typed Match Relation And Grouping

- [x] Define a match relation with reversible path/source identity, byte and
  line spans, pattern/capture identity, context artifact references, and
  provider/result lineage.
- [x] Preserve a typed empty match frame with exact schema and source binding.
- [x] Define stable unique keys and canonical order without relying on lossy
  display paths or nondeterministic tool order.
- [x] Add one bounded grouping/summary projection whose contributing match
  scope remains traceable.
- [x] Enforce file, match, row, string, context, and encoded-byte limits before
  accepting the result.
- [x] Prove deterministic parity for repeated frozen inputs and for any two
  providers that claim the same search semantics.

## Milestone 4 — Text And Relational Deltas

- [x] Freeze `SOURCE` to `TARGET` text comparison semantics for encoding,
  newlines, segmentation, normalization, spans, hunks, context, and limits.
- [x] Preserve authoritative structured hunks plus exact native source
  addresses; a rendered patch is not the sole delta.
- [x] Compare match relations under typed schema/key rules and preserve typed
  before/after values.
- [x] Keep text, relational, structural, and claim evidence as peer delta/fact
  shapes rather than one mega-table.
- [x] Cover insertions, deletions, replacements, empty inputs, long lines,
  Unicode, binary/invalid encoding, context elision, overflow, incompatible
  inputs, incomplete evidence, and deterministic replay.

## Milestone 5 — Visualization

- [x] Define the minimum visualization projection identity over source
  artifact/delta, contract revision, selection, grouping, ordering, context,
  elision, limits, and omissions.
- [x] Render the match relation as an evidence-linked table and the text delta
  as an evidence-linked patch.
- [x] Keep exact source/context/delta deep links in `-vv`; keep plain failure
  output immediately actionable and `-v` match evidence reviewable.
- [x] Make redirected/structured output stable and ANSI-free, and ensure color
  never carries unique meaning.
- [x] Do not add a general visualization library until table/patch fixtures
  prove a missing capability that warrants one.

## Milestone 6 — Workload And Reasoning-Surface Slice

- [x] Add a reviewed built-in mining conformance workload that uses the same
  operation contracts in scenario test and admitted run paths.
- [x] Include passing, failing, empty, truncated/inconclusive, and stale
  scenarios with exact qualifications and semantic exits.
- [x] Derive canonical frontier work from the failing relational/text evidence
  under a versioned workload-specific derivation contract.
- [x] Select one bounded work row through the existing deterministic scheduler
  and replay-verify the decision.
- [x] Construct a reasoning surface that cites the mining request/result,
  native context, match relation, directed delta, visualization, provider,
  completeness, omissions, and effective limits.
- [x] Prove that unrelated ambient workspace content is absent from the surface
  and that a mutable read/tool invocation cannot bypass the probe transition.
- [x] Stop after producing verified policy input; do not add a provider-specific
  agent adapter or graph-revision loop in this plan.

## Milestone 7 — Verification And Documentation

- [x] Add unit/property fixtures for contract identity, canonical order,
  bounds, completeness, tampering, and staleness.
- [x] Add provider fixtures for path/source drift, symlink escape, malformed
  input, unsupported content, hard bounds, and partial/truncated output. The
  unadmitted external `rg` cases remain intentionally outside this slice.
- [x] Add diff fixtures for typed empty relations and all required text-change
  shapes.
- [x] Add runtime fixtures for stale preconditions, budget, cancellation,
  timeout, evidence failure, and partial mining results where the slice reaches
  those paths.
- [x] Add CLI fixtures for table/JSON, plain/`-v`/`-vv`, pass/fail/
  inconclusive/stale, stdout/stderr, ANSI independence, and exit codes.
- [x] Synchronize README, architecture, mining, environment, diff, workload,
  runtime, interface, proof, roadmap, decision, and plan truth.
- [x] Capture `just check`, `just test`, `just build`, Nix, link, and repository
  truth evidence.

## Boundaries

- Mining composes provider capabilities; it does not create a second storage,
  query, document, stream, table, tool, run, capture, or index service beside
  an external service.
- The public CLI remains environment plus workloads. Focused diagnostics may be
  added only when needed to explain the workload slice.
- The first slice is read-only except for explicit local result/evidence
  retention already covered by its provider contract.
- No AST/CST framework, language server protocol, semantic resolver,
  code-quality metric catalog, durable index, general graph renderer, learned
  ranking, agent transport, recurring scheduler, or service is selected here.
- Native source remains authoritative; frames, summaries, deltas, and
  visualizations are bounded derived evidence.
- Additional providers follow only after the standalone semantic contract is
  proved and a concrete workload requires them.

## Exit Bearing

Plan 0006 is complete: one scenario-qualified workload mines exact local
source through admitted operations, retains native and relational evidence,
produces directed text and relational deltas, renders reviewable linked views,
derives one bounded frontier and reasoning surface from failure, and proves the
same deterministic contracts locally under explicit limits.

The next bearing may then choose one richer source-mining rung—CST/AST parsing,
semantic symbol/reference indexing, or derived code-quality/dependency metrics
—based on the unresolved deltas produced by this slice rather than selecting a
tool stack in advance.
