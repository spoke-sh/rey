# Workloads, Compute Graphs, And Scenarios

This document defines Rey's workload-centered product model. ADR 0015 fixes
the broad semantics and ADR 0016 fixes the implemented first slice. Rey now
ships a bounded built-in catalog, typed UTF-8 DAG executor, scenario evaluator,
local result provider, and the four `rey workloads` commands. External
manifests, generated graph revision, and the recurring improvement campaign
remain target contracts. ADR 0017 makes relational and source mining
first-class graph capabilities; ADR 0018 fixes their first executable workload
slice.

## Public Unit

A **workload** is Rey's public unit of computation. It binds a versioned
compute-graph contract, scenario suite, environment requirements, policy
boundary, admitted mining operations, claims, effects, limits, and retention
requirements under one stable identity.

Spaces, lenses, frames, deltas, frontiers, actions, traces, and proofs remain
important runtime concepts, but users should not have to orchestrate them as
unrelated top-level resources. A workload composes those contracts into one
thing that can be listed, tested, run, and inspected.

The accepted command surface is:

```text
rey environment ...
rey workloads [--workspace PATH] [--state-dir PATH] list
rey workloads [--workspace PATH] [--state-dir PATH] test [<workload-id>] [-v|-vv]
rey workloads [--workspace PATH] [--state-dir PATH] run <workload-id> --input <utf8> [--source <path>...]
rey workloads [--workspace PATH] [--state-dir PATH] status [<workload-id>]
```

The environment group inventories available compute. The workloads group
declares what that compute is for, measures whether a generated graph behaves
as expected, and runs a qualified graph against admitted inputs.

The built-in catalog contains three conformance workloads:

- `rey.fixture.source-search` executes literal source search followed by a
  canonical match renderer. Required empty/exact scenarios pass; optional
  mismatch/truncation scenarios retain review evidence without blocking
  qualification.
- `rey.fixture.text-normalize` executes `trim -> uppercase` and passes two
  required scenarios.
- `rey.fixture.text-mismatch` executes `uppercase` and preserves one passing
  and one failing required scenario.

These are not a general workload declaration format.

## Workload Contract

A workload declaration binds at least:

- stable workload id and immutable revision;
- description and ownership metadata;
- typed external input and output contracts;
- allowed graph-node operation contracts and effect classes;
- allowed relational/source mining operations, input artifact kinds,
  completeness requirements, and traversal/result limits;
- environment profile, required capabilities, and trust requirements;
- graph-proposal policy contract, which may be an agent, rule, or human;
- exact scenario-suite revision and required/optional scenario membership;
- claim, comparator, evaluator, and normalizer revisions;
- graph, campaign, scenario, execution, evidence, and output limits;
- qualification and staleness rules; and
- catalog and retention requirements.

The declaration syntax and file layout are not selected yet. Whatever syntax
is chosen must resolve mutable names to exact workload, graph, scenario,
evaluator, capability, and policy revisions before execution. Secret values do
not belong in the declaration.

Changing any semantic field creates a new workload revision. A display name or
catalog head may move, but every campaign and run records the exact revision it
resolved.

## Compute Graph Revision

A **compute graph revision** is an immutable, content-identified proposal for
how one workload transforms its declared inputs into outputs. It contains:

- workload id and revision;
- graph id, schema version, revision, and semantic digest;
- generator kind and exact policy/rule/human provenance;
- cited reasoning surface and failing delta identities when generated from a
  test campaign;
- stable node ids and versioned operation-contract references;
- typed input and output ports;
- directed edges connecting compatible ports;
- declared workload inputs, observable outputs, and claim scopes;
- node capability, effect, precondition, and limit requirements; and
- total node, edge, depth, byte, execution, and output bounds.

The initial graph contract is a finite typed directed acyclic graph. Feedback
belongs to the bounded campaign that proposes a new immutable graph revision,
not to an implicit cycle inside one execution. A future bounded loop node or
cyclic graph contract requires an explicit schema decision and termination
fixtures.

Graph validation rejects duplicate or unresolved ids, missing ports,
incompatible edge types, cycles, unavailable operation contracts, undeclared
capabilities or effects, unbounded nodes, and any graph exceeding effective
limits. A policy may compose only admitted operation contracts. Inline shell,
source, or tool text does not become executable merely because an agent put it
in a graph proposal.

Dependency order comes from graph edges. A stable topological order provides a
deterministic serial baseline; a later parallel executor must prove the same
declared output semantics. This is graph execution order, not Rey's generic
frontier scheduler. The frontier scheduler selects unresolved work that may
justify a graph revision; it does not invent node dependencies.

### Mining Operations In Graphs

A mining graph node cites one exact provider-neutral operation contract and
binds typed inputs, output artifact kinds, canonical parameters, capability
requirements, effects, completeness, and limits. Two primary families are
available to the target model:

- relational operations retrieve, select, filter, join, group, aggregate,
  traverse, compare, summarize, and visualize typed collections; and
- source operations retrieve, search, segment, tokenize, parse, index,
  traverse, measure, compare, and visualize text, code, configuration, logs,
  documents, and native artifacts.

Search matches, syntax nodes, symbols, references, dependencies, diagnostics,
and metrics may become typed frames, but they retain exact links to native
source evidence. A graph may pass those frames into relational grouping or
comparison without turning source bytes into a giant cell.

Discovery is not graph admission. Generated shell, regex, query, parser
configuration, source text, or visualization layout is untrusted typed input
and must satisfy the selected operation schema and bounds. Invoking an external
miner or reading mutable state is a probe; pure projection over frozen evidence
is deterministic compute. Neither grants mutation authority.

The implemented source-search graph is the first concrete instance. Its
external UTF-8 pattern flows into `rey.source-search.literal-utf8@1`; that node
also requires an explicit source-run input that freezes the canonical root,
relative file paths, context window, source-binding limits, mining limits, and
capability snapshot. The resulting `SourceMatches` value flows into
`rey.builtin.source-matches.render-lines@1` and the workload's UTF-8 output.
Scenario tests and admitted runs use this identical graph contract. Test mode
binds a checked-in corpus and reviewed expected relations; run mode requires at
least one repeatable `--source` path beneath `--workspace`.

## Scenarios

A **scenario** is a versioned test case for one workload contract. It binds:

- stable scenario id and revision;
- required or optional qualification role;
- exact fixture and external-input bindings;
- exact capability profile and test-provider substitutions;
- selected graph outputs, observations, and claims to evaluate;
- expected typed frames, native artifacts, predicates, or failure facts;
- comparison direction, keys, ordering, normalizers, tolerances, and
  evaluator revisions;
- required completeness and coverage; and
- per-scenario time, step, row, byte, output, and evidence limits.

Scenario execution is read-only by default. An operation that would mutate a
real target must be replaced by an explicit fixture provider or admitted to a
declared isolated test target. Test mode never silently grants production
authority.

Each comparison is directed from `EXPECTED` to `OBSERVED`. A scenario may
produce several typed deltas when it evaluates several outputs. Its semantic
evaluation is:

- `passed` when every required comparison is equal and every required claim
  passes with complete evidence;
- `failed` when at least one required comparison conclusively differs or one
  required claim conclusively fails;
- `inconclusive` when missing, incompatible, truncated, unsupported, timed-out,
  or lost evidence prevents a decision; or
- `pending` while required evaluation has not completed.

A failed scenario is therefore not just a process exit or message. It retains
the authoritative typed delta or failed claim fact that did not match. An
incompatible comparison is inconclusive rather than a guessed failure, and an
empty delta passes only the exact scope declared by that scenario.

Execution, evaluation, and freshness remain separate dimensions. Provider
execution can be queued, running, succeeded, failed, cancelled, timed out, or
lost. Evaluation can be pending, passed, failed, or inconclusive. Freshness is
derived by recomputing the scenario-result input identity; it is never trusted
as a stored label.

## Test Campaign

`rey workloads test` starts or resumes a bounded **test campaign**. A campaign
binds the exact workload and scenario-suite revisions, capability snapshots,
policy contract, graph lineage, effective limits, attempts, deltas, and
retention profile.

The campaign loop is:

```text
resolve workload and freeze capabilities
  -> validate or request candidate graph revision
  -> execute selected scenarios through that exact graph
  -> compare EXPECTED to OBSERVED and retain typed deltas
  -> derive scenario progress and unresolved frontier
  -> if required scenarios pass: qualify graph revision
  -> otherwise mine/project bounded failing evidence
  -> policy proposes next graph revision
  -> validate proposal and repeat, or stop at an explicit bound
```

An agent is one graph-proposal policy. A deterministic rule or human can
propose through the same contract. The runtime, not the proposer, validates
the graph, runs scenarios, computes deltas, derives progress, and decides
qualification. An agent cannot declare its graph successful.

If an admitted graph already exists and no proposal policy is available, Rey
can still run one deterministic scenario pass. If no graph exists and no
policy can propose one, the campaign is blocked and inconclusive; Rey does not
invent a graph. Policy unavailability never prevents deterministic execution
or verification of a graph that is already present.

Each proposed graph revision is immutable and retained with its parent,
reasoning-surface, cited-delta, and policy identities. A revision change makes
prior scenario results non-current. The first implementation must
conservatively rerun every required scenario for the new graph. Reuse is
allowed later only when an explicit dependency closure proves that the
scenario's inputs and reachable graph semantics are unchanged.

A graph revision is **qualified** only when all required scenarios have fresh,
complete passing results for that same exact graph and workload revision. An
optional scenario remains visible but does not gate qualification unless the
workload says it does. A test campaign stops distinctly on qualification,
conclusive failure with no admitted next proposal, policy or capability
unavailability, cancellation, timeout, budget exhaustion, invalid graph,
stale input, or runtime failure.

Publishing a qualification record may advance the workload's qualified-graph
selector at the selected catalog boundary. That mutable selector is not graph
identity and never rewrites earlier graph revisions or campaign results. A
failing candidate remains evidence and does not displace the last fresh
qualified graph.

### Human Test Runner

The table projection is a running conformance document, not a stream of
unstructured logs. It first fixes the execution path, read-only graph mode,
`EXPECTED` to `OBSERVED` comparison direction, and workload scope. Scenario
results then appear as soon as they complete, while preserving declaration
order. Each line keeps scenario passing, evaluation, and required/optional
role explicit.

Verbosity expands evidence without changing evaluation:

- plain output omits evidence for passing scenarios and always opens the
  directed delta for failing or inconclusive scenarios;
- `-v` adds the evidence format and matching output evidence for passing
  scenarios, plus source matches, context, completeness, omissions, and
  effective limits for mining scenarios; and
- `-vv` also binds that evidence to exact workload, graph, suite, evaluator,
  scenario, execution, result, delta, mining operation/provider/capability,
  corpus/request/result, native source/match/context, frontier, scheduling, and
  reasoning-surface identities.

For example, a failing plain scenario remains immediately actionable:

```text
FAIL rey.fixture.text-mismatch · 02/02 surrounded · 0/1 outputs equal · required
     Evidence deltas:
         Delta (output text):
         @@ text · utf8 @@
         - REY
         +  REY
```

The final portfolio summary reports workload qualification, scenario
conformance, scenario evaluation, delta assessments, and issued
qualifications as separate dimensions. Verbosity is a human projection only:
JSON always emits the same verified `rey.workload-test-batch.v2` envelope.

## Running A Workload

`rey workloads run` resolves one exact workload revision and its current fresh
qualified graph, freezes real input and capability bindings, validates effects
and limits, and executes the same graph contract used by scenarios.

Test and run are not separate graph languages:

- test mode binds fixtures, expected outputs, comparators, and safe test
  providers;
- run mode binds caller-admitted inputs and real providers; and
- both retain exact graph, node, capability, output, claim, attempt, and trace
  lineage.

Run refuses an absent, unqualified, or stale graph by default. A successful
process is not by itself a successful workload run. Declared output claims and
post-execution observations determine the semantic result. Effectful graph
nodes still pass ordinary action admission immediately before execution.

For `rey.fixture.source-search`, `--source` is repeatable and identifies exact
relative files below the canonical workspace. `--context-before`,
`--context-after`, and `--max-matches` bind the effective search projection;
zero or missing required bounds fail before execution. Other current workloads
reject source-specific options. The human run view reports graph order, output
size, completeness, consumption, exact corpus/request/capability bindings,
each match and context deep link, and every omission.

## Scenario Progress

Progress is always scoped to one exact workload, graph, and scenario-suite
revision. The denominator is the number of selected required scenarios. Rey
keeps at least these counts separate:

```text
required · passed · failed · inconclusive · blocked
running · queued · stale · optional
```

The human `rey workloads list` view first aggregates qualification, scenario,
run, and inventory dimensions across the portfolio. It then renders one card
per workload with a derived journey, passing and evaluated coverage, explicit
evaluation counts, qualification state, exact candidate and qualified graph
identities, retained test evidence, freshness, and latest run state. For
example:

```text
WORKLOAD PORTFOLIO
  Qualification          2/3 qualified · 1 failing · 0 inconclusive · 0 stale
  Scenarios              5/6 passing · 6/6 evaluated · 0 stale · 2 optional
  Runs                   1 passed · 0 blocked · 2 not run
  Inventory              3 total · 3 tested · 0 untested
  Mining                 1 workloads · 4 retained results · 1 incomplete

rey.fixture.source-search
  Journey                RUN READY
  Scenario conformance   ████████████████████  100%  2/2 passing · 2/2 evaluated
  Evaluation             2 passed · 0 failed · 0 inconclusive · 0 stale · 2 optional
  Qualification          QUALIFIED
  Graph                  rey.fixture.source-search.graph@1
  Operations             rey.source-search.literal-utf8@1 → rey.builtin.source-matches.render-lines@1
  Mining evidence        4 results · 3 complete · 1 incomplete · 4 relation deltas · 1 reasoning surfaces
  Candidate              blake3:...
  Qualified              blake3:...
  Test evidence          blake3:... · fresh
  Last run               not run
```

The bar cannot rely on color for meaning: labels, counts, percentages, and
glyphs remain legible with color disabled or redirected. Structured output
continues to carry the underlying per-workload counts rather than portfolio
rendering glyphs. A failed or inconclusive scenario may count as evaluated but
never as passed. A stale result is shown as stale evidence and counts as
untested for the current graph. No percentage or bar is a proof of semantic
progress across graph revisions.

Qualification is convergence only for the workload's declared required
scenario scope. It is not universal code coverage, production safety, or proof
that optional behavior was exercised.

The list card uses the catalog's exact campaign-head graph revision. If no test
campaign exists, it uses the current qualified graph; if neither exists, the
workload is untested. Candidate and qualified graph identities are separate
fields, so a failing candidate never makes an older qualified graph appear to
have failed.

## Catalog, Results, And Staleness

The workload surface needs two provider contracts:

1. a **catalog provider** resolves workload declarations, immutable graph and
   scenario revisions, and mutable selectors to exact identities; and
2. a **result provider** retains campaigns, proposals, attempts, outputs,
   deltas, qualification records, runs, and indexes needed by `list` and
   `status`.

The first standalone result provider stores a bounded
`rey.local-workload-state.v2` JSON index at
`${workspace}/.rey/workloads/state.json`, or below explicit `--state-dir`.
It verifies retained test and run artifacts on every read and publishes a
same-directory temporary document with rename. It claims no `fsync` crash
durability, locking, multi-process transactionality, authenticated writer, or
Spoke semantics. This does not select a general database or manifest encoding.
Connected mode uses
public Spoke contracts for any durability, query, run, or lineage guarantees
it claims. Rey does not project a local layout onto Spoke or create a second
durable service.

User-authored workload and scenario declarations, and any graph selected for
future runs, cannot exist only in a disposable cache or DataFrame. Generated
indexes and terminal renderings are projections. `list` and `status` are
read-only and do not execute scenarios merely to fill missing state.

A scenario result or qualification becomes stale when any semantic input used
to produce it changes, including workload, graph, scenario, fixture, expected
output, provider capability, mining operation/implementation, parser/index,
canonical parameters, comparator, evaluator, normalizer, policy scope,
completeness semantics, or effective limit. A production run additionally
binds its real inputs and admission snapshot. Staleness is derived from exact
identities and dependency facts, never toggled manually.

## Command Semantics

| Command | Contract |
| --- | --- |
| `workloads list` | Read the selected catalog and retained result index; show every resolved workload, candidate and qualified graph identities, operations, fresh scenario counts, progress bar, mining-result completeness, relation-delta/reasoning counts, and last campaign/run summary. It performs no graph execution. |
| `workloads test [id]` | Test one workload, or every workload resolved by the selected catalog when no id is supplied. Execute bounded graphs and probes, retain actual-versus-expected text/relation deltas and mining evidence, and qualify only exact all-passing required-scenario graph revisions. The workload-count bound fails closed rather than silently truncating. |
| `workloads run id` | Execute the exact current fresh qualified graph against admitted real inputs and explicit source paths through the declared providers; retain outputs and exact mining lineage. |
| `workloads status [id]` | Read detailed current state for one workload, or a catalog summary when omitted: resolved revisions, candidate/qualified graph, campaign stop reason, per-scenario deltas, mining results, omissions, frontier selection, reasoning evidence, qualification, staleness, and latest run. It performs no repair or execution. |

`list` and `status` return success when inspection succeeds even if workloads
are failing. `test` and `run` use semantic exit categories: `0` for qualified
or passed, `2` for conclusive failure, `3` for inconclusive or blocked, `4` for
stale, and `1` for invalid input or runtime failure. Argument-parser errors
retain the CLI framework's behavior.

For a multi-workload test, invalid/runtime failure takes precedence, then stale
input, then any conclusive failure, then any inconclusive or blocked result;
only an all-qualified selection returns `0`. A complete failing scenario stays
failed when the graph-revision budget is exhausted; the budget is an additional
stop fact, not a reason to erase its delta. Exhaustion is inconclusive only when
it prevents required evaluation.

The human table streams retained scenario results to stdout in declaration
order and ends with the portfolio result. Structured JSON emits only the final
verified result envelope; it never mixes progress into stdout, and diagnostics
remain on stderr. `auto` renders the human document on a terminal and
structured JSON when redirected. Tabular catalog and scenario relations may
explicitly emit Arrow; a workload result envelope or native output is not
forced into a DataFrame merely for format uniformity.

## Relationship To Existing Runtime Contracts

The formal transition machine, frontier scheduler, and reasoning surface are
mechanisms inside a workload campaign; they are not competing public resource
models. Scenario deltas produce workload-specific frontier rows. Scheduling
selects bounded unresolved work. Mining retrieves and projects the relevant
relational and source evidence. A policy then proposes a graph revision or
another admitted action.

The workload slice made the required pre-alpha hard cut. `rey.frontier.v2`,
`rey.frontier-progress.v2`, `rey.scheduling-decision.v2`, and
`rey.reasoning-surface.v3` bind exact workload, graph, scenario-suite, and
campaign identities. The runtime reducer remains `rey.runtime-state.v2`
because its state never contained the legacy application/component envelope.
No compatibility alias or decoder silently relabels the superseded schemas.

## Initial Implementation Boundary

The implemented slice uses three small fixture workloads, finite graphs
composed from built-in deterministic operations, and reviewed scenarios. It
exercises all four commands without an agent or Spoke. The source-search
workload closes the first mining loop from explicit corpus through operation,
typed relation/native context, ordered and relational deltas, one scheduled
frontier row, reasoning surface, qualification, and admitted real-input run.
Generic distributed or recurring scheduling, arbitrary code execution,
external workload manifests, a persistence engine, parser/index breadth, and
provider-specific policy loops remain outside this boundary.
