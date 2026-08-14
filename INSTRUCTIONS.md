# Contributor Instructions

Procedural guidance for humans and agents working on Rey.

## Read First

1. `README.md` for scope, model, and current status.
2. `CONSTITUTION.md` for durable values and invariants.
3. `docs/README.md` for the documentation map.
4. `docs/ARCHITECTURE.md` for ownership and data flow.
5. `docs/GLOSSARY.md` for canonical project terminology and semantic
   distinctions.
6. `docs/CLI.md` before changing command behavior, revision loops, human
   rendering, structured output, streams, or exit semantics.
7. `docs/MINING.md` before changing retrieval, search, parsing, indexing,
   grouping, traversal, metrics, text/structural diffs, or visualization.
8. `docs/WORKLOADS.md` before changing workloads, compute graphs, scenarios,
   qualification, progress, or the workload CLI.
9. `docs/RUNTIME.md` before changing transitions, reasoning surfaces,
   convergence, or scheduling.
10. `docs/FRONTIER.md` before changing frontier, progress, prioritization, or
   scheduling contracts.
11. `docs/ENVIRONMENT.md` before changing providers, discovery, tools, profiles,
   or capability admission.
12. `docs/LOCATORS.md` before changing locator syntax, resolution, anchors, or
   survey behavior.
13. `docs/GIT.md` before changing repository identity, commit/ref/index polling,
   cursors, triggers, or workload activation.
14. `docs/DIFFS.md` before changing frames, comparison, normalization, or
   renderings.
15. `docs/PROOFS.md` before changing claims, certificates, staleness, or evidence.
16. `docs/INTERFACES.md` before changing provider, HTTP, policy, or persistence
    contracts.
17. `docs/DEVELOPMENT.md` before changing the toolchain or root tasks.
18. `docs/RELEASES.md` before changing CI, distribution, tags, or publication.
19. `docs/decisions/README.md` for the accepted current plane that constrains
    the work.
20. `plans/README.md` and the active plans before implementation work.

## Working Loop

1. **Orient** — inspect current files, accepted decisions, active plans, and the
   available environment.
2. **Bind** — identify the exact source revisions and observable claim affected
   by the work.
3. **Mine** — retrieve and project only the bounded relational or source
   evidence needed to understand the current delta.
4. **Bound** — choose the smallest end-to-end behavior and explicit resource
   limits.
5. **Decide** — record consequential or hard-to-reverse choices before they
   spread through code and formats.
6. **Change** — preserve the boundary between deterministic runtime, policy,
   providers, and presentation.
7. **Prove** — run focused checks and exercise the feature through its
   high-fidelity human `rey` CLI path; internal APIs and structured output are
   necessary but do not complete a feature slice by themselves.
8. **Record** — update documentation and plan checklists in the same change.

## Current Development Interface

Rey has a pinned Nix Rust/TypeScript toolchain, a twelve-crate Cargo workspace,
a root pnpm/Turborepo monorepo whose first package is the operator UI, and a
Git-shaped `env` CLI with process-owned `HOME`/`PWD`/`PATH` discovery seeds,
explicit agent-generated mapping resources, verified local
capability status, a `HEAD → INDEX → WORKING` admission plane, partial/full
add, resolved-application-only interactive patch selection, index-only commits,
and patch history,
lower-level proof and bounded local-only bundle contracts behind the runtime,
pure runtime-state, frontier/progress/scheduling, and
reasoning-surface contracts, plus a bounded workspace workload-package catalog,
an explicitly selected built-in conformance catalog, typed DAG executor,
scenario evaluator, qualification record, local result provider, five
workload commands, and a supervised `rey agent` process with an embedded
operator surface. Humans land on
the `/explore` context-topology canvas and normally remain in the UI; agents
use the CLI as their primary runtime interface, with humans descending to it
for exact diagnosis. `/explore` is a high-fidelity spatial game engine
specialized for evidence-bound projections of high-dimensional context. The
implemented `rey.projection-packet.v1` carries a deterministic terrain program
and bounded camera-relative working-set policy. Explorer renders a live
Three.js WebGPU-first/WebGL2-compatible surface beside the deterministic
accessible reference path; [Plan 0003](plans/0003-scene-to-explorer.md) owns
scene admission, remaining engine separation, World/Atlas/County completion,
and CLI/browser/performance proof.
The former Instrument dashboard is
Environment at `/environment`. `/cadence` keeps bounded Git reachability, Rey
admissions, and mounted browser scans on explicit partial-order clocks. `/agents` ranks
evidence-backed system recommendations and summarizes retained work results;
agent-runtime discovery remains on `/environment`. Generator provenance remains
workload evidence, not the definition of an available or assigned agent. The
foreground Rey process owns an orchestrator that supervises the operator HTTP
worker, fails closed on unexpected worker exit, and stops it cooperatively on
SIGINT or SIGTERM. It does not invoke a discovered agent runtime or schedule a
workload. The UI starts from that CLI process, passively revalidates the same workload-list
derivation, defaults to loopback, and is not a general mutation plane or public
Rey service. Channel topology remains non-navigable substrate behind Feed,
mailbox, and conversation. `rey channels poll` may use an exact Channel- and
environment-HEAD-admitted `gh` executable to retain current unread GitHub
notifications and bounded pull-request comments for the mailbox; `rey agent`
supervises that same command at the committed application cadence. Neither
path marks provider notifications read or runs from discovery alone. Feed's
Admission stream projects only retained workload commits; WORKING/INDEX review
and exact approval remain on Workloads.
Feed's compact composer admits a partial self-asserted human Observation through
the same bounded local store as `rey observations add`; it does not create a
Journal entry or grant action authority.
Feed resolves URL preview, Channel WORKING, Channel HEAD, then built-in layout;
URL edits remain detached until adoption, and
stable stream movement uses the expected-snapshot WORKING boundary without
granting INDEX, HEAD, relay, or execution authority. The separate local
observation log retains exact source/evidence bindings and Channel-admission
edges without entering topology INDEX or granting relay authority. Its fixed
footer is a two-axis communication plane: the mailbox
shows retained application-poll Channel messages beside the current
typed-attention history projection, while chevrons open an
operator/Rey/agent chat shell whose composer remains disabled until a
conversation transport is admitted. Workspace packages retain coding-harness provenance and
freeze generated scenarios at admission. The first mining workload now executes exact local
literal search, typed match comparison, ordered line comparison, bounded
frontier selection, and reasoning-surface projection through those commands.
Do not generalize that fixed provider, workload-specific derivation, or local
operator server into
regex or parser breadth, external tool execution beyond declared identity probes, recurring scheduling,
activation, browser mutation, authentication, remote service topology, remote
durability, worker restart, daemonization, multi-process fencing, or crash
durability.

Enter the environment and use:

```sh
nix develop
just setup
just rey
just check
just test
just dist-check
just build
just fmt
```

These tasks are backed by the current Cargo workspace and pinned development
environment. `rey` runs the CLI; `check`, `test`, and `build` execute real
workspace verification; `dist-check` validates the non-publishing release
plan. See `docs/DEVELOPMENT.md` and `docs/RELEASES.md` for the exact behavior.
Documentation changes should pass `just check` and manual
link/repository-truth review.

## Runtime Work

- Keep workload declarations, graph revisions, scenarios, campaigns,
  qualification records, and production runs as distinct exact identities.
- Treat workspace packages as untrusted harness/rule/human WORKING proposals.
  Bind exact producer and input revisions, freeze scenario oracles before
  staging, qualify the exact INDEX, require human commit for HEAD, and never
  present compiled conformance fixtures as product work.
- Treat `workloads create` as an immutable request handoff to an external
  coding harness. Keep drafts visible, refuse overwrite, generate no fake
  graph or oracle, and reject run until an admitted HEAD package exists.
- Validate every agent-, rule-, or human-proposed graph before execution; a
  proposal cannot introduce ambient executable authority or declare itself
  qualified.
- Direct scenario comparisons from expected to observed and retain conclusive
  failures as typed deltas.
- Keep frame construction, typed comparison, frontier selection, action
  admission, and proof evaluation deterministic and usable without an LLM.
- Keep the standalone runtime deterministic and useful from local evidence.
- Treat policy as an external decision source. Policy output is an untrusted
  proposal until the runtime validates identity, revisions, effects, and limits.
- Keep all queues, results, captures, traversals, iterations, and concurrency
  bounded.
- Make cancellation and partial failure visible at each observation and action
  boundary.
- Prefer one-way capability dependencies. Shared crates stay narrow and own
  semantics actually shared by multiple capabilities.
- Do not confuse local evidence with guarantees owned by another provider. A
  same-host integration still uses its documented contract.

## Mining Work

- Treat mining as a continuous pair of loops: each workload mines domain
  evidence, while portfolio mining derives attention from exact catalog,
  result, environment, dependency, capability, ownership, and coverage inputs.
- Keep portfolio attention provider-neutral and typed. Preserve action, reason,
  readiness, blockers/exclusions, evidence, dependency, priority, cost, and
  coverage as separate fields before scheduling or policy projection.
- Exercise portfolio behavior through `rey workloads create`, `status`,
  `diff`, `add`, `test --staged`, `commit`, `log`, `list`, and `run`; do not
  introduce a parallel top-level mining command hierarchy.

- Treat relational and source mining as peer capability families connected by
  exact projections, not as a reason to stringify data or tabularize every
  artifact.
- Bind each request and result to exact sources, operation and implementation
  revision, canonical parameters, provider/capability snapshot, effective
  limits, completeness, omissions, and dependency lineage.
- Keep native source, text, trees, graphs, and binary artifacts addressable.
  DataFrames represent matches, nodes, edges, metrics, and other genuine typed
  collections.
- Distinguish exact immutable retrieval, pure projection over frozen evidence,
  and a probe that reads mutable state or invokes an external tool.
- Freeze and admit `rg`, parsers, compiler services, language servers, and
  indexes through provider contracts before invocation; discovery alone grants
  no authority.
- Make parsing recovery, unresolved symbols, ignored/generated inputs,
  sampling, grouping, traversal bounds, and visualization elision visible.
- Preserve deep links from metrics and graphs to contributing relations, then
  to exact spans and source revisions.
- Keep visualizations semantically subordinate to authoritative artifacts;
  layout or color cannot change assessment, coverage, progress, or proof.

## Frame And Diff Work

- Preserve one logical schema across Polars, Arrow, structured output, and
  terminal rendering.
- Require explicit comparison keys for unordered relations and validate their
  uniqueness before alignment.
- Include comparison direction, labels, source revisions, lens revisions,
  normalizers, and limits in delta identity.
- Keep typed before/after values in the structured delta even when a rendering
  uses strings.
- Use Tabular Diff 0.8 as a projection for compatible tabular comparisons, not
  as the only internal representation.
- Do not stringify relational data merely to obtain a text diff, and do not
  force source text or binary content into synthetic rows without a genuine
  relational contract.
- Treat an empty frame as a typed empty relation. Preserve its declared schema
  so missing rows can be represented rather than failing key resolution.

## Proof Work

- Separate `failed`, `inconclusive`, and `stale`; they lead to different next
  actions.
- Treat similarity, progress, confidence, and coverage as distinct quantities.
- Never infer coverage from a passing diff alone.
- Hash evaluator and normalizer implementations into proof inputs so changed
  semantics invalidate previous certificates.
- Make omitted frames, ignored fields, unsupported limits, and truncated output
  reviewable evidence.
- Verify a certificate by recomputing input identity and checking referenced
  evidence, not by trusting a stored `passed` field.

## Environment Awareness

- Discover environment capabilities through bounded provider contracts rather
  than arbitrary shell startup scripts.
- Keep discovery read-only. A known version probe has a timeout and output
  bound; discovery never executes arbitrary files merely because they are on
  `PATH`.
- Keep desired application inventory distinct from search observations. A
  development dependency is not inventory intent; bind the declaration graph
  and the resulting capability snapshot as separate exact records.
- Record provider identity, resolved executable path, version,
  digest/provenance when available, trust class, supported actions, and limits.
- Revalidate the capability snapshot at action admission. Tool or provider
  drift makes a proposal stale.
- Make standalone and required-capability behavior explicit in configuration
  and evidence.
- Missing capabilities remove actions or make dependent claims inconclusive;
  they do not silently select a weaker proof contract.

## Provider Integration

- Use exact provider identities and revisions in source bindings; mutable paths
  or names alone are insufficient.
- Keep `QUERY` safe and idempotent. Effects use explicit admitted actions.
- Let each provider own its process attempts, captures, retention, and lineage.
- Let Rey own observation definitions, action rationale, frame/delta lineage,
  frontier selection, claim evaluation, and proof assembly.
- Preserve provider request and revision lineage in Rey evidence where relevant.
- Test a provider's public contract before claiming first-class integration.
- Do not make Rey's build or local runtime depend on an external provider.

## Git Polling And Activation

- Bind repository identity, object format, worktree identity, HEAD, watched
  refs, semantic index digest, and declared worktree state explicitly.
- Treat Git OIDs as opaque algorithm-qualified identities.
- Derive staged triggers from logical index entries, not the index file's mtime
  or stat-cache-only changes.
- Keep ref creation, deletion, fast-forward, rewind, rewrite/divergence, and
  unknown history distinct.
- Do not claim an ordered commit append across rebase, reset, force-push, or
  incomplete shallow history.
- Poll with no optional locks and never run hooks, aliases, filters, credential
  helpers, fsmonitor hooks, submodule commands, or mutations during discovery.
- Advance a cursor only after activation evidence commits. Expect replay after
  crashes and use deterministic activation ids plus action idempotency.
- A trigger creates an activation proposal; it never bypasses effect admission.

## Plans And Decisions

- Plans in `plans/` are current executable checklists with a top-level
  completion list.
- Mark only repository-proven facts complete and remove completed plans; Git
  retains delivery history.
- Put open choices in the active plan for that bearing. Put accepted durable
  structure in its owning foundational document and the current decision
  plane.
- Update the owning contract, decision projection, and active plan together
  when accepted structure changes.

## Hygiene

- Prefer `rg` for repository search and the documented root tasks once they
  exist.
- Keep generated Arrow, CSV, trace, certificate, and benchmark outputs out of
  source control unless deliberately maintained as small fixtures.
- Never commit credentials, service tokens, model provider keys, or private
  source snapshots.
- Avoid unresolved ambient host paths, environment variables, timestamps, or
  random values in semantic identities. Resolved environment capabilities are
  explicit inputs and must be recorded when they affect semantics.
- Do not claim deterministic, incremental, portable, or reproducible behavior
  without focused proof.
