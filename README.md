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

The product surface stays intentionally small:

```text
rey environment ...
rey workloads list
rey workloads test [<workload-id>] [-v|-vv]
rey workloads run <workload-id>
rey workloads status [<workload-id>]
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
just rey environment inspect --format table
just rey workloads list --format table
just rey workloads test rey.fixture.text-normalize --format table
just rey workloads test rey.fixture.text-mismatch --format table -vv
just rey workloads run rey.fixture.text-normalize --input ' rey ' --format table
```

The current built-in workload graphs provide deterministic UTF-8 `trim` and
`uppercase` operations. They prove graph validation, expected-to-observed
scenario deltas, qualification, local result state, and test/run graph parity.
They are not yet the general mining operation model described above.

The implemented environment loop can also be exercised entirely through files
selected by the caller:

```sh
just rey environment inspect --format json > baseline.json
just rey environment inspect --format json > candidate.json
just rey environment diff baseline.json candidate.json
just rey environment diff baseline.json candidate.json \
  --diff-format tabular-diff
just rey environment prove baseline.json candidate.json \
  --require-capability frame.arrow-stream > certificate.json
just rey environment verify certificate.json baseline.json candidate.json
just rey environment prove baseline.json candidate.json \
  --require-capability frame.arrow-stream \
  --bundle proof.bundle > bundled-certificate.json
just rey environment verify-bundle proof.bundle
```

Environment frame commands support bounded JSON and Arrow IPC. Workload
commands support human tables and structured JSON; redirected `auto` selects
JSON. `workloads test` keeps passing scenarios compact by default and always
opens failing diffs. `-v` adds matching evidence, while `-vv` binds evidence to
exact workload, graph, suite, evaluator, scenario, execution, result, and delta
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

See [Mining Context Into Evidence](docs/MINING.md) for the target capability
contracts and [ADR 0017](docs/decisions/0017-mining-capability-model.md) for the
accepted architectural boundary.

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

The repository contains a ten-crate Rust workspace. Implemented behavior
includes bounded standalone capability discovery, allowlisted `git` and `rg`
identity probes, a partial read-only Git observation, typed capability
snapshot deltas, Arrow and Tabular Diff projections, scoped capability
certificates, bounded local proof bundles, a formal runtime reducer, canonical
frontier/progress/scheduling contracts, bounded reasoning-surface contracts,
and the first deterministic workload CLI slice.

Mining is now accepted target architecture, not yet an implemented generic
operation family. The next bearing is [Plan 0006](plans/0006-mining-strategy.md):
freeze provider-neutral mining contracts, implement one bounded relational and
source mining path, produce directed typed and text evidence, and exercise it
through a scenario-qualified workload before adding AST/CST adapters, semantic
indexes, broad metrics, or generic scheduling.

The longer-running [Plan 0001](plans/0001-foundation.md) still owns complete Git
activation and the first routed Spoke proof.
