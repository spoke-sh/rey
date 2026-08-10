# Rey

Rey is a diff-directed mining and compute runtime for agents.

It inventories the context available where it is running, mines bounded
structure from data and source surfaces, computes directed deltas, and spends
the next unit of attention or compute on unresolved change. Users organize that
loop as workloads: versioned compute graphs qualified by exact scenarios whose
failing evidence directs the next graph revision.

Rey is built around a practical view of programming: a programmer makes
progress by mining their environment. They locate evidence, retrieve the exact
parts that matter, expose useful structure, compare it with an expectation or
prior revision, visualize the result, and act on the smallest meaningful
frontier. Code generation is one possible action near the end of that loop; it
is not the loop itself.

## Mining As Applied Programming

Rey treats **mining** as the bounded transformation of context into navigable,
addressable evidence. It brings two primary capability families together:

| Capability family | What it mines | Representative operations |
| --- | --- | --- |
| **Relational mining** | Typed records, tables, events, measurements, and graph relations | retrieve, select, filter, join, group, aggregate, align, traverse, diff, summarize, visualize |
| **Source mining** | Text, code, configuration, logs, documents, and native artifacts | search, slice, tokenize, parse, index, walk ASTs/CSTs, resolve symbols and references, calculate metrics, diff text or structure, visualize |

The families meet through exact projections. Search matches can become a typed
relation. Symbols and references can become node and edge frames. A relational
delta can lead back to a precise source span. A text or syntax delta can
invalidate grouped metrics or dependency relations. Neither family is reduced
to the other: source bytes remain source bytes, while DataFrames remain the
canonical local coordinate system for genuinely typed collections.

Visualization is part of mining, not decoration added afterward. A table,
patch, tree, graph, metric panel, or workload progress view is a bounded
projection of authoritative evidence. It preserves direction, scope,
completeness, and deep links to exact sources; color never carries unique
meaning.

## The Runtime Loop

```text
                    explicit environment boundary
          workspace · Git · tools · runtimes · optional Spoke
                                  │
                       inventory capabilities
                                  │
                    ┌─────────────┴─────────────┐
                    ▼                           ▼
           relational mining              source mining
       query · group · traverse       search · parse · index
                    └─────────────┬─────────────┘
                                  ▼
                bounded projections and native evidence
                                  │
                     compare SOURCE → TARGET
                                  │
                     typed and native deltas
                                  │
               ┌──────────────────┴──────────────────┐
               ▼                                     ▼
       unresolved frontier                      scoped proof
               │                               and lineage
               ▼
       bounded work selection
               │
               ▼
      mine exact relevant evidence
               │
       delta-directed reasoning surface
               │
               ▼
     propose → admit → probe or mutate
               │
               ▼
        observe and compare again
```

The delta is not merely a report produced after work finishes. It is a runtime
control signal. Changed rows, spans, symbols, edges, metrics, claims, or
capabilities invalidate dependent observations. Unresolved differences become
a frontier. The runtime mines only the evidence needed to orient on selected
frontier work, then measures whether the next admitted action actually reduced
the residual delta.

This creates a bounded inner loop:

```text
delta → frontier → schedule → mine → reason → propose → act → observe → delta
```

That loop is nested inside an ongoing portfolio loop. Workloads mine their
domains, while Rey mines the workload catalog, retained scenario results,
environment/dependency revisions, capabilities, and coverage to discover which
workloads need attention or which relevant surfaces have no workload yet:

```text
catalog + results + environment + coverage
  → portfolio snapshot → workload attention
  → RETEST | REFINE | CREATE | BLOCK | POLICY_EXCLUDED
  → admitted work → test → observe portfolio again
```

The resulting attention relation is evidence, not a scheduler guess. It keeps
ready, blocked, and explicitly excluded rows distinct. The generic scheduler
may later select bounded ready rows; an agent may propose a change for a
selected row; neither may declare the row resolved without re-evaluation.

Process completion alone is not progress. A command may exit successfully
while the semantic frontier is unchanged or worse. Conversely, reaching a
limit, losing evidence, or encountering an unsupported parser is not
convergence; it is an explicit incomplete or inconclusive result.

## Workloads Are The Public Unit

A workload composes the mining and compute contracts needed to pursue one
purpose. It binds:

- an immutable typed compute-graph revision;
- admitted relational and source mining operations;
- exact context surfaces and capability requirements;
- required and optional scenarios;
- expected observations, comparison rules, and claims;
- policy, effect, qualification, and staleness rules; and
- graph, traversal, evidence, time, byte, and iteration limits.

An agent, deterministic rule, or human may propose a graph revision through
the same untrusted contract. Rey validates its operations, inputs, types,
capabilities, effects, and bounds. The runtime executes the graph and computes
scenario deltas; the proposer cannot declare its own graph qualified.

The default catalog is the workspace's `workloads/` directory. Each accepted
`rey.workload-package.v1` binds a generated graph, a generated but frozen
scenario oracle, exact harness provenance, and its package source revision.
`rey workloads create` adds the missing agentic entry point: it writes a
content-addressed `request.yaml` for an external coding harness and exposes the
workload as a draft until that harness materializes an admitted package. Rey
does not fabricate a graph or scenario oracle in the request step.
Compiled fixtures are an explicit diagnostic surface selected with
`--catalog conformance`; they are not presented as the user's portfolio.

The product surface stays intentionally small:

```text
rey env status
rey env diff
rey env add [-p]
rey env commit -m <message>
rey env log [-p]
rey workloads create <workload-id> [--title <title>] [--intent <intent>]
rey workloads list
rey workloads test [<workload-id>] [-v|-vv]
rey workloads run <workload-id> --input <utf8>
rey workloads status [<workload-id>]
rey workloads --catalog conformance list|test|run|status ...
rey ui [--host <ip>] [--port <port>]
```

Mining operations are capabilities used inside workload graphs and reasoning
surfaces. They are not a second hierarchy of top-level resources that users
must manually orchestrate.

## Try The Executable Slice

Rey is pre-alpha, but the standalone foundation and first workload slice are
executable:

```sh
nix develop
just setup
just rey env status
just rey env diff
just rey env add -p
just rey env add
just rey env commit -m 'accept local toolchain'
just rey env log
just rey env log -p
just rey workloads create api-drift \
  --title 'API drift mining' \
  --intent 'Mine authoritative API behavior and formalize its graph and scenarios' \
  --format table
just rey workloads list --format table
just rey workloads test rey.portfolio.label-normalization --format table -vv
just rey workloads run rey.portfolio.label-normalization --input ' refine ' --format table
just rey workloads --catalog conformance test rey.fixture.text-mismatch --format table -vv
just rey workloads --catalog conformance test rey.fixture.source-search --format table -vv
just rey workloads --catalog conformance test rey.portfolio.attention --format table -vv
just rey workloads --catalog conformance run rey.portfolio.attention --format table
just rey workloads --catalog conformance run rey.fixture.source-search \
  --input evidence \
  --source crates/rey-environment/tests/fixtures/source-corpus/alpha.txt \
  --format table
just rey ui
# Explicit network exposure; Rey warns because this read-only server has no authentication.
just rey ui --host 0.0.0.0 --port 5714
```

`rey ui` opens the human operator's primary collaboration surface. `/`
redirects to `/explore`, a full-screen-capable context-topology canvas whose
semantic lens moves from a bounded landscape, through workload and attention
neighborhoods, into exact graph, scenario, evidence, dependency, and directed-
delta objects. Exact locations use matrix-style coordinates such as
`/explore/agent/codex;at=gpt-5;lens=objects;role=coding_harness`; stale revision
bindings remain visible instead of drifting silently. `/agents` indexes exact
generator provenance and links each identified agent into that coordinate
plane. `/cadence` presents Git reachability, Rey environment admissions, and
mounted passive scans as separate partially ordered tick lanes rather than a
fictional global event log. `/environment` projects the same bounded
`HEAD → INDEX → WORKING` environment document as the CLI through exactly three
stacked evidence sections: `01 / DIRECTED TEXT` for variables,
`02 / BOUNDED SEARCH` for applications, and `03` `REFERENCE PLANE` for inputs
and topology. `/workloads` remains the exact catalog projection. The
manual Refresh control is gone: the read-only workload and environment
documents passively revalidate every five seconds in mounted application state.
Revalidation never navigates, remounts the active route, or changes the
operator's scroll position.

The server embeds a TanStack Router application expressed with
[Hifi](https://github.com/rupurt/hifi)'s Kinetic grammar and Precision theme.
All authored application styles are StyleX modules compiled by the official
StyleX Vite integration into one deterministic atomic stylesheet; runtime
Kinetic material values remain typed custom properties.
The server defaults to `127.0.0.1:5714`, accepts explicit `--host` and `--port`
values, and reports its exact exposure before serving. Its API is read-only;
binding beyond loopback is an operator-controlled, unauthenticated boundary.
See [Context Topology Explorer](docs/EXPLORER.md) for the lens, focus,
coordinate, relationship, omission, cadence, and operator/agent interface
contracts.

The default catalog contains the checked-in, coding-harness-generated
`rey.portfolio.label-normalization` package. Its exact YAML graph and frozen
scenario suite are visible at
[`workloads/portfolio-label-normalization/workload.yaml`](workloads/portfolio-label-normalization/workload.yaml).
Rey imports this accepted package. `workloads create` now produces the strict
request side of the external harness handoff; harness response admission and
automatic campaign continuation remain the next boundary.

The explicit conformance catalog contains four executable diagnostic workloads. Two graphs cover
deterministic UTF-8 `trim` and `uppercase`. The source-search graph composes
`rey.source-search.literal-utf8` with a deterministic match renderer over an
explicit bounded corpus. Its required empty and exact scenarios qualify the
graph; optional mismatch and truncation scenarios retain a typed match
relation delta, ordered line delta, omissions, one scheduled frontier row, and
a bounded reasoning surface. `run --source` executes that same qualified graph
against caller-selected files below the workspace.

The `rey.portfolio.attention` system workload mines a frozen portfolio snapshot
into the canonical `rey.workload-attention.v1` relation. Its required scenarios
cover refinement, retesting, workload creation, blockers, explicit policy
exclusion, and a clean typed-empty result. `workloads list` and `status` derive
the current attention view without probing ambient state; after qualification,
`workloads --catalog conformance run rey.portfolio.attention` evaluates the compiled catalog,
retained workload evidence, and retained environment HEAD/admission index.
Mapped input files without a declared workload owner appear visibly as
`CREATE` candidates.

`env status` is the single inventory and revision view. It observes the working
environment, leads with an env-shaped `HEAD → WORKING` variable diff, and then
groups every declared application into found, searched-but-not-found, or
observation-error evidence. The admission summary still distinguishes
`HEAD → INDEX` changes admitted for commit from `INDEX → WORKING` changes not
yet admitted. `env diff` selects the same three environment planes for
`INDEX → WORKING`; `env diff --staged` selects them for `HEAD → INDEX`.
Its human output is directed variable text, bounded application search, then
input/reference topology—not a generic capability patch. The header still
reports the authoritative capability-delta assessment and change count;
`--format json` retains the complete `rey.environment-diff.v2` evidence.
`env add` stages the complete working snapshot, while `env add -p` selects
capability changes interactively.
`env commit` appends exactly the retained admission index beneath `.rey/env`
without re-observing ambient state. `env log` renders a compact newest-first
chronology of revision, evidence, environment scope, changed dimensions,
mapping, and message. `env log -p` expands each selected parent-to-commit
transition through the same directed text, bounded search, and reference
planes. This is bounded single-process local state, not a Git object store or
a Spoke-durable log.

The checked-in [`rey.env.yaml`](rey.env.yaml) is an agent-generatable mapping
graph describing the environment surfaces this workspace cares about:

```yaml
schema: rey.env-map.v2
nodes:
  - id: cargo-home
    kind: variable
    name: CARGO_HOME
    capture: value
  - id: workspace-manifest
    kind: file
    path: Cargo.toml
    required: true
  - id: cargo
    kind: executable
    name: cargo
    required: true
    potential_capabilities: [rust.build, rust.check, rust.test]
edges:
  - from: cargo-home
    to: cargo
    relation: configures
  - from: workspace-manifest
    to: cargo
    relation: input_to
```

Rey parses this as a closed, bounded graph and projects it into the committed
capability snapshot. Non-sensitive variables may opt into bounded UTF-8 value,
digest, or presence capture; sensitive variables are always presence-only.
Exact values selected with `capture: value`, bounded file identities, resolved
executable identities, and declared edges therefore remain reproducible across
the human and structured revision surfaces. Executables are resolved and
hashed but not invoked, and their declared potential capabilities remain
visibly unadmitted.

Environment commands default to Git-like human documents and accept explicit
JSON where no interactive patch selection is required. `status --format json`
contains the complete working capability snapshot, optional admission index,
both authoritative capability deltas, and the shared typed variable,
application, input, and reference operator projection used by `/environment`.
Workload commands support human tables and structured JSON; redirected `auto`
selects JSON. `workloads test` keeps passing scenarios compact by default and
always opens failing diffs. `-v` adds matching evidence, while `-vv` binds
evidence to exact workload, graph, suite, evaluator, scenario, execution,
result, and delta identities. List, test, status, and run also expose the
selected catalog, package path/revision, generator, and frozen-oracle state.
Mining scenarios additionally expose operation,
provider, capability snapshot, corpus, request, result, relation, native
source, match, context, frontier, scheduling-decision, and reasoning-surface
identities.

## The Mining Ladder

Source mining grows in capability without changing its evidence rules:

1. **Locate and retrieve** — bounded file, object, document, table, and source
   identity; exact bytes or rows remain addressable.
2. **Search and segment** — literal or regular-expression matches, line ranges,
   chunks, tokens, and declared context windows.
3. **Parse structure** — syntax trees, CSTs, ASTs, configuration paths, and
   language-aware spans under an exact parser contract.
4. **Index semantics** — symbols, references, definitions, types, call edges,
   ownership, and dependency relations with completeness metadata.
5. **Derive measures** — grouped diagnostics, complexity and quality metrics,
   test relationships, change impact, and other versioned analyses.
6. **Compare and visualize** — line, token, syntax, relation, tree, and graph
   deltas projected into reviewable human and machine forms.

Higher rungs never erase the lower evidence. A metric links to the relation
from which it was calculated; a relation links to exact source spans; a span
links to a source revision. Missing language support or bounded traversal is
reported explicitly rather than replaced by guessed structure.

Relational mining follows the same discipline. Queries and transforms bind
input schemas, keys, ordering, operation revisions, parameters, source
checkpoints, and limits. Grouping or visualization cannot become proof merely
because it looks complete. Typed before/after values remain authoritative even
when a human view renders them as text.

See [Mining Context Into Evidence](docs/MINING.md) for the capability contracts,
[ADR 0017](docs/decisions/0017-mining-capability-model.md) for the architectural
boundary, [ADR 0018](docs/decisions/0018-first-mining-workload.md) for the first
source slice, and [ADR 0022](docs/decisions/0022-portfolio-mining-and-workload-attention.md)
for ongoing portfolio mining.

## Exact Evidence, Diffs, And Proof

Every mining result records the strongest source identity its provider can
establish, the operation and implementation revision, parameters,
capabilities, effective limits, completeness, artifacts, and lineage. Pure
projection over frozen evidence is deterministic. Reading mutable state or
invoking a tool is an explicit probe; finding `rg`, a compiler, parser, or
language server does not grant permission to run it.

Rey preserves several authoritative comparison families:

- typed relational deltas for keyed frames;
- native text deltas for ordered text;
- structural deltas for declared trees and graphs; and
- typed claim facts when a claim does not reduce honestly to one comparison.

Each comparison is directed and names both sides. Human patches, tables,
trees, graphs, summaries, and Tabular Diff 0.8 are projections; none may erase
source identities, types, keys, schemas, comparison rules, omissions, or
limits from authoritative evidence.

A zero delta is not universal proof. It establishes agreement only for the
declared sources, operations, fixtures, keys, normalizers, parser/evaluator
revisions, completeness, and coverage. Changed source, mining implementation,
capability, or effective limit makes dependent evidence stale.

## Environment, Git, And Spoke

Rey treats its environment as an explicit runtime input. A bounded discovery
pass can inventory a selected workspace, Git repository, `rg`, language
toolchains, analyzers, test runners, and a reachable Spoke deployment. Each
provider advertises exact operations, trust, provenance, and enforceable
limits. Discovery remains read-only and separate from action admission.

The `env` CLI makes those observations revisionable. `status` computes
`HEAD → INDEX → WORKING`, `add` explicitly accepts observations into a bounded
admission index, `commit` records exactly that index in a local linear history,
and `log -p` reopens those directed deltas through the environment-native
three-plane projection. Environment commits record evidence; they neither
mutate the discovered environment nor create Git commits. Admission to history
does not grant execution authority to a mapped tool.

Git is both a source-binding provider and a natural activation surface. Commit,
ref, semantic index, and declared worktree deltas can select affected workload
scenarios or graph entry points. Ref rewrites, incomplete history, linked
worktrees, and conflicts remain explicit; Rey never fabricates an append-only
event stream from a mutable repository.

Rey remains useful with zero Spoke. In standalone mode it mines explicitly
selected local surfaces with narrower disclosed guarantees. When present,
Spoke is Rey's durable reasoning and compute plane: versioned content,
composed query, admitted compute, captures, and durable lineage amplify the
same mining contracts.

Rey does not duplicate Spoke storage, document, stream, table, query, tool,
run, or capture ownership. It uses public Spoke contracts and exact revisions.
Standalone adapters never mint Spoke identities or pretend to provide Spoke
durability.

The projects improve one another without a package or startup cycle:

```text
Rey mines Spoke ──► finds a public-contract delta ──► improves Spoke
      ▲                                                   │
      └────── discovers the new mining/compute capability ◄┘
```

## Boundaries

Rey is not:

- a model server or model-provider framework;
- a general-purpose database, search engine, code index, or visualization
  store;
- a replacement for Spoke storage, query, documents, or compute;
- an ambient shell that executes every discovered tool;
- a workflow engine whose tasks have no delta or scenario semantics;
- a claim that all text, code, graphs, or binary artifacts are naturally
  tabular;
- an authority that lets an agent mutate a target without explicit admission;
- a universal code-quality, coverage, or correctness system; or
- a proof system that hides missing, unsupported, ignored, stale, or truncated
  evidence.

## Repository Guide

- [Constitution](CONSTITUTION.md) — durable values and invariants.
- [Contributor Instructions](INSTRUCTIONS.md) — working loop and verification.
- [Architecture](docs/ARCHITECTURE.md) — ownership and end-to-end data flow.
- [Mining Context Into Evidence](docs/MINING.md) — relational/source mining,
  operation, result, visualization, and runtime contracts.
- [Workloads](docs/WORKLOADS.md) — public composition, compute graphs,
  scenarios, qualification, progress, and commands.
- [Runtime](docs/RUNTIME.md) — transition machine and reasoning surfaces.
- [Frontier](docs/FRONTIER.md) — canonical work, progress, and scheduling.
- [Environment](docs/ENVIRONMENT.md) — providers, capabilities, and profiles.
- [Git](docs/GIT.md) — source identity, polling, and activation.
- [Diffs](docs/DIFFS.md) — typed, textual, and structural comparison.
- [Proofs](docs/PROOFS.md) — claims, evidence, certificates, and staleness.
- [Interfaces](docs/INTERFACES.md) — CLI, policy, provider, and Spoke contracts.
- [Development](docs/DEVELOPMENT.md) — toolchain and repository truth.
- [Roadmap](docs/ROADMAP.md) — delivery sequence.
- [Plans](plans/README.md) — active implementation bearings.
- [Decisions](docs/decisions/README.md) — accepted architectural choices.

## Current Status And Next Bearing

The repository contains an eleven-crate Rust workspace. Implemented behavior
includes bounded standalone capability discovery, allowlisted `git` and `rg`
identity probes, a partial read-only Git observation, typed capability
snapshot deltas, Arrow and Tabular Diff projections, scoped capability
certificates, bounded local proof bundles, verified local environment commits,
an agent-generatable environment mapping graph, compact `status`, exact
staged/unstaged `diff`, full and partial `add`, index-only commits, and
patch-bearing `log`, a formal runtime reducer, canonical
frontier/progress/scheduling contracts, bounded reasoning-surface contracts, the
deterministic workload CLI, and provider-neutral mining operation/request/result
manifests with canonical replay verification. The standalone
environment advertises a built-in literal source-search provider over exact
local corpora; the source-search workload executes it through the same test and
run graph, compares native ordered text and typed match relations, and projects
complete, failing, and truncated evidence through `list`, `test`, `status`, and
`run`.

Plan 0010 has now started the outer loop. Workspace packages are the default
product catalog and compiled workloads are explicitly diagnostic.
`rey.portfolio-snapshot.v1` and the
Polars-backed `rey.workload-attention.v1` relation are executable through the
scenario-qualified `rey.portfolio.attention` workload. The five workload
commands expose current attention, coverage, exact relation evidence, and
qualification-gated retained-input evaluation. Ready work, capability/evidence
blockers, and policy exclusions remain separate facts.
`rey.workload-creation-request.v1` makes draft creation visible in that same
portfolio plane: `create` records an immutable harness request, `list` and
`status` render `HYDRATE`/`AWAITING CODING HARNESS`, and `test`/`run` reject the
draft until an admitted package exists.

[Plan 0009](plans/0009-environment-admission-index.md) makes `status` the one
environment interface and places a reviewable admission index between working
observation and commit. Variables, input files, executable candidates, and
their declared relationships are visible across both staged and unstaged
planes. Revision selectors, reset/restore, branches, and Spoke retention remain
later work.

[Plan 0010](plans/0010-portfolio-mining-and-workload-attention.md) carries the
workload-centered mining strategy. Its next concrete anchor is the bounded coding-
harness response handshake. `workloads create` now supplies the request; the
next slice binds one ready attention row and exact evidence to it, has a harness
materialize the package, validates the frozen response, and re-mines whether
the row resolved. The first real payload will admit ownership for one mapped
input, then change that retained source revision and derive `RETEST` for the
exact owner. Parser/symbol mining remains a later tool for investigating
attention, not the portfolio strategy itself.

[Plan 0011](plans/0011-local-operator-ui.md) carries the high-fidelity operator
surface. `/explore` is now the default context-topology map with semantic zoom,
pan, selection-driven focus, full screen, visible bounds, passive revalidation,
matrix-style deep links, provenance-derived agent neighborhoods, and a
partially ordered cadence view. Its next concrete anchor is to replace
aggregate context counts with exact admitted environment-mapping objects before
adding scenario and directed-delta routes with the same progressive evidence
layers as `workloads test -v/-vv`.

The longer-running [Plan 0001](plans/0001-foundation.md) still owns complete Git
activation and the first routed Spoke proof.
