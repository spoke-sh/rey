# Plan 0006: Mining Strategy And First Executable Slice

## Outcome

Formalize mining as Rey's bounded bridge from environment context to
delta-directed evidence, then implement the smallest zero-Spoke workload that
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
- [ ] Version the minimum mining operation, request, result, artifact,
  completeness, dependency, limit, and visualization contracts.
- [x] Decide narrow crate ownership and add `rey-mining` only if the accepted
  contracts justify it without a dependency cycle.
- [ ] Freeze one exact local source-binding and read-only source-search
  provider contract, including admitted `rg` behavior or a deterministic
  built-in baseline.
- [ ] Project bounded search matches as a typed frame while retaining native
  source/context artifacts and exact deep links.
- [ ] Implement authoritative ordered text delta semantics and reuse the
  generic relational delta contract where it is actually complete.
- [ ] Render bounded ANSI-independent table and patch projections with explicit
  grouping, context, elision, completeness, and omissions.
- [ ] Add one built-in mining conformance workload through `list`, `test`,
  `run`, and `status` without adding a peer top-level mining resource.
- [ ] Derive a workload frontier from retained failing mining evidence and
  construct one verified delta-directed reasoning surface.
- [ ] Prove deterministic zero-Spoke behavior, hard bounds, provider/source
  drift, malformed/unsupported input, failure, truncation, and staleness.
- [ ] Keep JSON/stdout/stderr/exit behavior and human `-v`/`-vv` evidence
  contracts covered by CLI fixtures.
- [ ] Run focused tests, full workspace tests, Clippy, build, Nix checks, link
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

The repository does not yet execute `rg` as a mining provider, compare
arbitrary ordered text, derive a
frontier from workload scenario deltas, retrieve provider evidence into a
reasoning surface, parse syntax, build a semantic index, or render general
tree/graph visualizations.

`rey-mining` now implements canonical v1 operation, request, and result
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

- [ ] Select an explicit fixture workspace/corpus beneath a canonical allowed
  root and define exact source identity before retrieval.
- [ ] Implement bounded native text retrieval with file, byte, line,
  context-window, encoding, and symlink/path-escape rules.
- [ ] Implement one source-search operation with canonical literal/regex,
  case, path, context, and limit parameters.
- [ ] If using `rg`, freeze path, version, digest/provenance, trust, exact argv,
  cwd, environment, output format, time/capture limits, and parser behavior
  before admission.
- [ ] Provide a deterministic in-process baseline or reviewed fixtures so tool
  behavior can be tested without confusing availability with semantics.
- [ ] Detect source drift between binding and retrieval and emit stale or
  inconclusive evidence rather than combining revisions.
- [ ] Make ignored/generated/binary/invalid-encoding and unsupported cases
  explicit.

## Milestone 3 — Typed Match Relation And Grouping

- [ ] Define a match relation with reversible path/source identity, byte and
  line spans, pattern/capture identity, context artifact references, and
  provider/result lineage.
- [ ] Preserve a typed empty match frame with exact schema and source binding.
- [ ] Define stable unique keys and canonical order without relying on lossy
  display paths or nondeterministic tool order.
- [ ] Add one bounded grouping/summary projection whose contributing match
  scope remains traceable.
- [ ] Enforce file, match, row, string, context, and encoded-byte limits before
  accepting the result.
- [ ] Prove deterministic parity for repeated frozen inputs and for any two
  providers that claim the same search semantics.

## Milestone 4 — Text And Relational Deltas

- [ ] Freeze `SOURCE` to `TARGET` text comparison semantics for encoding,
  newlines, segmentation, normalization, spans, hunks, context, and limits.
- [ ] Preserve authoritative structured hunks plus exact native source
  addresses; a rendered patch is not the sole delta.
- [ ] Compare match relations under typed schema/key rules and preserve typed
  before/after values.
- [ ] Keep text, relational, structural, and claim evidence as peer delta/fact
  shapes rather than one mega-table.
- [ ] Cover insertions, deletions, replacements, empty inputs, long lines,
  Unicode, binary/invalid encoding, context elision, overflow, incompatible
  inputs, incomplete evidence, and deterministic replay.

## Milestone 5 — Visualization

- [ ] Define the minimum visualization projection identity over source
  artifact/delta, contract revision, selection, grouping, ordering, context,
  elision, limits, and omissions.
- [ ] Render the match relation as an evidence-linked table and the text delta
  as an evidence-linked patch.
- [ ] Keep exact source/context/delta deep links in `-vv`; keep plain failure
  output immediately actionable and `-v` match evidence reviewable.
- [ ] Make redirected/structured output stable and ANSI-free, and ensure color
  never carries unique meaning.
- [ ] Do not add a general visualization library until table/patch fixtures
  prove a missing capability that warrants one.

## Milestone 6 — Workload And Reasoning-Surface Slice

- [ ] Add a reviewed built-in mining conformance workload that uses the same
  operation contracts in scenario test and admitted run paths.
- [ ] Include passing, failing, empty, truncated/inconclusive, and stale
  scenarios with exact qualifications and semantic exits.
- [ ] Derive canonical frontier work from the failing relational/text evidence
  under a versioned workload-specific derivation contract.
- [ ] Select one bounded work row through the existing deterministic scheduler
  and replay-verify the decision.
- [ ] Construct a reasoning surface that cites the mining request/result,
  native context, match relation, directed delta, visualization, provider,
  completeness, omissions, and effective limits.
- [ ] Prove that unrelated ambient workspace content is absent from the surface
  and that a mutable read/tool invocation cannot bypass the probe transition.
- [ ] Stop after producing verified policy input; do not add a provider-specific
  agent adapter or graph-revision loop in this plan.

## Milestone 7 — Verification And Documentation

- [ ] Add unit/property fixtures for contract identity, canonical order,
  bounds, completeness, tampering, and staleness.
- [ ] Add provider fixtures for missing/change/timeout/non-zero/malformed/
  oversized `rg`, path drift, source drift, symlink escape, and partial output.
- [ ] Add diff fixtures for typed empty relations and all required text-change
  shapes.
- [ ] Add runtime fixtures for stale preconditions, budget, cancellation,
  timeout, evidence failure, and partial mining results where the slice reaches
  those paths.
- [ ] Add CLI fixtures for table/JSON, plain/`-v`/`-vv`, pass/fail/
  inconclusive/stale, stdout/stderr, ANSI independence, and exit codes.
- [ ] Synchronize README, architecture, mining, environment, diff, workload,
  runtime, interface, proof, roadmap, decision, and plan truth.
- [ ] Capture `just check`, `just test`, `just build`, Nix, link, and repository
  truth evidence.

## Boundaries

- Mining composes provider capabilities; it does not create a second storage,
  query, document, stream, table, tool, run, capture, or index service beside
  Spoke.
- The public CLI remains environment plus workloads. Focused diagnostics may be
  added only when needed to explain the workload slice.
- The first slice is read-only except for explicit local result/evidence
  retention already covered by its provider contract.
- No AST/CST framework, language server protocol, semantic resolver,
  code-quality metric catalog, durable index, general graph renderer, learned
  ranking, agent transport, recurring scheduler, or service is selected here.
- Native source remains authoritative; frames, summaries, deltas, and
  visualizations are bounded derived evidence.
- Spoke amplification follows only after the standalone semantic contract is
  proved and must use public Spoke capabilities.

## Exit Bearing

Plan 0006 is complete when one scenario-qualified workload can mine exact local
source through admitted operations, retain native and relational evidence,
produce directed text and relational deltas, render reviewable linked views,
derive one bounded frontier and reasoning surface from failure, and prove the
same deterministic contracts with zero Spoke under explicit limits.

The next bearing may then choose one richer source-mining rung—CST/AST parsing,
semantic symbol/reference indexing, or derived code-quality/dependency metrics
—based on the unresolved deltas produced by this slice rather than selecting a
tool stack in advance.
