# Workloads, Compute Graphs, And Scenarios

This document defines Rey's workload-centered product model. Rey loads bounded
coding-harness-authored graph and scenario contracts from
`sys/*/workload.yaml`, validates them, stages exact bytes, qualifies the frozen
INDEX, and admits only an explicit human action. Request-only packages retain a
strict external-harness handoff and visible draft state; Rey does not invoke a
harness. Relational and source mining are first-class graph capabilities, and
the implemented portfolio-mining conformance workload makes typed workload
attention an outer-loop input. Owned mapped surfaces and exact Git HEAD or
semantic-index dependencies derive live invalidation from retained evidence;
acknowledged Git activations can enter schedule-only workload admission, while
selected-scenario activation execution is retained separately from full-suite
qualification. Compatible same-transition executions can reuse exact retained
evidence, and a separate bounded full recomputation proves whether selected
execution evidence is exactly equivalent. Bounded Git cadence observation
retains every successful or failed tick plus retry, cancellation,
partial-failure, and terminal receipts. Cross-poll debounce and autonomous
activation scheduling remain future work.

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
rey env ...
rey workloads [--workspace PATH] [--catalog-dir PATH] create <workload-id> [--title TITLE] [--intent INTENT]
rey workloads [--workspace PATH] [--catalog-dir PATH] status
rey workloads [--workspace PATH] [--catalog-dir PATH] diff [--staged]
rey workloads [--workspace PATH] [--catalog-dir PATH] add
rey workloads [--workspace PATH] [--catalog-dir PATH] test --staged [<workload-id>] [-v|-vv]
rey workloads [--workspace PATH] [--catalog-dir PATH] commit -m <message>
rey workloads [--workspace PATH] [--catalog-dir PATH] log [-p] [-n COUNT]
rey workloads [--workspace PATH] [--catalog-dir PATH] list
rey workloads [--workspace PATH] [--catalog-dir PATH] admit-activation <activation-id>
rey workloads [--workspace PATH] [--catalog-dir PATH] execute-activation <admission-id>
rey workloads [--workspace PATH] [--catalog-dir PATH] verify-activation <execution-id> [--max-evidence-bytes <bytes>]
rey workloads [--workspace PATH] [--catalog-dir PATH] run <workload-id> --input <utf8>
rey workloads [--workspace PATH] [--catalog-dir PATH] run context-anchor-survey --source <path> [--source <path>...]
rey workloads [--workspace PATH] [--catalog-dir PATH] run scene-admission --scene SCENE@n [--editor-state-dir PATH]
rey workloads --catalog conformance list|test|run|status ...
rey agent [--workspace PATH] [--catalog-dir PATH] [--host IP] [--port PORT]
```

The environment group inventories available compute. The workloads group
declares what that compute is for, hands creation requests to a coding harness,
measures whether a staged generated graph behaves as expected, admits an exact
qualified INDEX through human approval, and runs HEAD against admitted inputs.

The operator worker supervised by `rey agent` is the human admission surface
for this same workload plane. It opens
on `/explore`; before survey evidence exists, exact workload beacons link to
inspection and the exact workload review surface. The UI derives incoming candidates and
admitted HEAD from the same typed state as the CLI and advances HEAD by
freezing, qualifying, and committing the exact reviewed WORKING file snapshot
with explicit HEAD and WORKING preconditions. It defines no second catalog,
qualification rule, attention relation, or package mutation path. Journal
writes and workload approval remain separate authorities. `/feed` projects a
bounded verified workload commit log only after this admission succeeds; it
does not render candidates, attention, or an approval control as feed items.

The default catalog is the workspace package catalog. Checked-in packages are
WORKING proposals, not admitted workloads. The initial product proposal is
`context-anchor-survey`; an operator must stage, qualify, and approve it before
it appears in HEAD or can run.

The explicitly selected built-in conformance catalog contains four diagnostic
workloads:

- `rey.portfolio.attention` derives and renders the canonical typed attention
  relation from one frozen portfolio snapshot. Six required scenarios cover
  blocked, clean, create, excluded, refine, and retest behavior.

- `rey.fixture.source-search` executes literal source search followed by a
  canonical match renderer. Required empty/exact scenarios pass; optional
  mismatch/truncation scenarios retain review evidence without blocking
  qualification.
- `rey.fixture.text-normalize` executes `trim -> uppercase` and passes two
  required scenarios.
- `rey.fixture.text-mismatch` executes `uppercase` and preserves one passing
  and one failing required scenario.

These fixtures are runner/system conformance, not the user's product
portfolio. They are available only with `--catalog conformance`.

The default workspace catalog contains the coding-harness-generated
`context-anchor-survey` and `scene-admission` packages. The survey graph accepts declared seed names, runs
the deterministic bounded survey operation, passes a typed topography patch to
its renderer, and compares frozen human-readable evidence. `test -v` renders
each typed assertion as `EXPECTED → ACTUAL`; `test -vv` additionally exposes
the exact seed, resolution, anchor, edge, region, frontier, omission, limit,
and lineage evidence behind those assertions. After explicit admission, `run` requires
repeatable `--source` paths and retains one exact patch; `list` and
`GET /api/v1/workloads` project HEAD and retained state without surveying again.

`scene-admission` accepts a canonical `SCENE@n` label, validates an exact
editor transfer envelope and all bounded native GeoJSON bytes, and renders a
typed accepted/rejected result. Its frozen required suite covers package and
object tampering, stale parent state, unsupported format, coordinate mismatch,
duplicate identity, missing objects, byte bounds, polar coordinates,
antimeridian crossing, and replay. An accepted run retains
`rey.admitted-regional-scene.v1` with its embedded
`rey.regional-projection-packet.v1`; it does not copy candidate-only terrain
hints into observed height and does not change `/explore`.

## Workload Creation Request

`rey workloads create <id>` is the unbound agentic entry point.
`--attention-row <row-id>` instead requires an exact currently selected,
ready `CREATE` attention row and binds the verified portfolio snapshot,
environment snapshot, attention, frontier row, scheduling decision, and
reasoning surface into the request. Stale, unknown, ineligible, or unscheduled
rows fail before the package directory is created. Neither form makes the
deterministic runtime call an LLM or generates placeholder tests. The command
creates `sys/<id>/request.yaml` with schema
`rey.workload-creation-request.v1`, a semantic request identity, bounded intent,
target package path, generation/admission requirements, and effective limits.
The returned `rey.workload-create-result.v1` includes the exact created path,
instructions for the external coding harness, and the required next action.

The optional `rey.workload-creation-attention.v1` binding carries the selected
action and reason, subject, evidence and dependency identities, typed failing
delta references, permitted operation, and complete surface limits. A `CREATE`
request explicitly records that the current package revision is absent; its
failing-delta set may be typed empty because the gap is an unowned surface,
not a failed package scenario. The request digest covers the complete binding.

Creation is explicit local mutation. Rey confines the catalog root to a
workspace-relative non-symlinked path, validates the id as a safe package name,
uses create-new semantics, and refuses to overwrite any existing workload
directory. It creates no `workload.yaml`; a harness must mine authoritative
revisioned inputs and materialize that file.

Authored Journal action cells remain outside this boundary. The bounded
Journal opportunity surface labels them `authored_only` and grants no
scheduler readiness. An author, UI, or Journal projection cannot pass one as
`--attention-row`; only an exact current row derived, scheduled, and verified
by the portfolio reasoning surface is accepted. If an authored opportunity is
later supported by runtime evidence, the normal attention derivation and this
existing creation/admission path remain authoritative.

A request-only directory is a draft catalog entry. `workloads list`, `status`,
and `/workloads` render its `HYDRATE` journey, missing graph,
non-admitted oracle, exact request/source revisions, and `AWAITING HARNESS`
state. `test` and `run` reject it. Once `workload.yaml` appears, Rey verifies
that its workload id matches the retained request and that generation inputs
cite the exact request path and content digest, then exposes the package in
`WORKING`. A response copied from another or mutated request fails closed. The
request remains beside the package as creation lineage; materialization is not
admission. The explicit path remains `WORKING → INDEX UNQUALIFIED → INDEX
QUALIFIED → HEAD`, and re-observation determines whether the source attention
row resolved, changed identity, or remained open.

## Git Activation Admission

`rey git poll` can retain deterministic proposal-only activations, and `rey git
ack` admits the exact transition evidence into the cursor. Neither command
admits workload execution. `rey workloads admit-activation <activation-id>` is
the separate ordinary workload gate. It accepts only a proposal from
acknowledged history whose target is still the current cursor and whose
transition is the cursor's exact retained evidence.

Admission revalidates the current workload HEAD, exact workload and graph
contracts, declared scenario selection, retained environment snapshot,
automatic intrinsic runtime snapshot, proposal completeness, and
scenario/action/evidence budgets. An empty trigger scenario
selection resolves to the complete admitted suite; named selections must be
canonical, known, and within both proposal and workload limits. The effective
admission narrows actions to one and evidence to the local four-megabyte bound.
Pending proposals, stale cursors, unknown scenarios, graph drift, empty HEAD,
or missing capability evidence fail before state changes.

The retained `rey.workload-activation-admission.v2` is content identified and
idempotent. It carries the complete Git proposal, Git target and transition,
workload HEAD commit/snapshot, workload/graph/suite contracts, resolved
scenario ids, separate environment and runtime snapshot identities, effective
budget, and explicit
schedule-only authority. `workloads list` exposes the same typed admissions in
JSON and a `RUNTIME ADMISSIONS` human section. Admission does not run a
scenario, mutate Git, claim progress, or consume the activation. Admission is
not a permanent freshness label.

`workloads execute-activation <admission-id>` revalidates the frozen Git
cursor, workload HEAD, exact workload/graph/suite/scenario contracts, retained
environment, intrinsic runtime capabilities, and effective action/evidence
budget. It executes only the
selected scenarios and retains their directed deltas and native evidence as
`rey.workload-scenario-execution-result.v1`, wrapped by the exact admission in
`rey.workload-activation-execution.v1`. The measured serialized evidence must
fit the admission cap before retention. The result is deliberately separate
from `last_test`: even an all-passing subset cannot issue or replace exact
full-suite qualification. Repeating the command returns the retained execution
without rerunning the graph. Failure and inconclusive evidence remain visible
through semantic exit codes and the human receipt. No path mutates Git.

`workloads verify-activation <execution-id>` revalidates the execution's exact
acknowledged Git cursor, workload HEAD, workload/graph/suite/scenario
contracts, retained environment snapshot, and current intrinsic runtime
snapshot. It then executes the complete
declared scenario suite, retains the bounded full result, and compares each
originally selected scenario result exactly with its counterpart. The
content-identified `rey.workload-activation-recomputation.v1` records
`EQUIVALENT` or `DIFFERENT`, both result identities, per-scenario execution
identities, evidence consumption, and comparison-only authority. It never
updates `last_test` or qualification, never mutates Git, and replays an
existing proof without executing scenarios again.

Before running a new graph, execution checks retained results from other
admissions in the same Git transition. Reuse is permitted only when source and
target Git snapshots, workload HEAD, workload/graph/suite/evaluator contracts,
declared and selected scenarios, and both snapshot identities are identical,
and
the retained evidence fits the new admission's possibly stricter byte budget.
The new execution receipt preserves its own admission and activation ids and
records the exact `source_execution_id`; the original result is not relabeled.
Only a directly evaluated result may be a coalescing source, which prevents
opaque reuse chains. Changed inputs or insufficient budget fall through to an
independent execution and normal validation rather than silently widening the
reuse boundary. Bounded `git watch` cadence retains successful and failed
observation ticks under explicit retry/cancellation/partial-failure bounds but
does not admit or execute them; cross-poll debounce and autonomous activation
scheduling remain planned.

## Workspace Package Admission

`rey.workload-package.v1` is the first deliberately narrow workload package
DSL. Every immediate child of the configured catalog root must contain either
a regular, non-symlinked `request.yaml` draft or a regular, non-symlinked
`workload.yaml` proposal. A package supplies workload ports, a typed graph, a
frozen scenario suite, and generation provenance:

```yaml
schema: rey.workload-package.v1
generation:
  kind: coding_harness
  producer: codex
  producer_revision: gpt-5
  generated: [compute_graph, scenario_suite]
  inputs:
    - source: docs/RUNTIME.md
      revision: blake3:<exact-content-digest>
ownership:
  surfaces:
    - surface_id: docs/RUNTIME.md
      source_revision: blake3:<exact-observed-content-digest>
      required_capabilities: [parser.rust]
  git_dependencies:
    - dependency_id: repository-head
      repository_id: blake3:<exact-repository-identity>
      worktree_id: blake3:<exact-worktree-identity>
      kind: head
      symbolic_ref: refs/heads/main
      source_revision: sha1:<exact-object-id>
```

The complete product proposal is
[`sys/context-anchor-survey/workload.yaml`](../sys/context-anchor-survey/workload.yaml).
V1 public workload ports remain UTF-8. Its supported internal node value types
include UTF-8 and `topography_patch`, and its closed operation set includes the
text conformance operations plus the exact survey and patch-render operations.
This is an admission surface, not arbitrary code loading.
Discovery is workspace-confined and count/byte bounded; unknown fields,
unknown operations, duplicate workload ids, unsafe paths, and incomplete
generation provenance fail closed.

The optional ownership block is semantic workload input. A surface declaration
names one mapped surface, the exact source revision the graph was built
against, and a canonical set of required capability ids. Workload limits bound
surface, Git-dependency, and capability counts. Portfolio composition binds
surface declarations to the exact workload, graph, retained environment
snapshot, and admitted mapped-file revision. A changed or unobserved revision
derives a dependency-change fact; an unavailable required capability derives a
blocked attention fact. Two workloads cannot own the same surface in one
portfolio snapshot.

A Git dependency instead binds a stable dependency id, exact repository and
optional worktree semantic identities, one `head` or `semantic_index` kind,
and the exact source revision. HEAD dependencies may additionally bind the
symbolic ref and use an algorithm-qualified `sha1:` or `sha256:` object id, or
`unborn`; semantic-index dependencies use an exact `blake3:` entry digest or
`absent`. Declarations are canonical, bounded, fail closed on malformed
identity/revision combinations, and participate in workload identity.

Portfolio composition compares Git declarations only with the cursor snapshot
retained by `rey git init` or advanced by exact `rey git ack` evidence. A fresh
ambient repository observation, including `rey git status` or an unacknowledged
pending poll, cannot change workload attention. A missing cursor is
`unobserved`; an acknowledged mismatch derives `dependency_changed` and cites
the exact Git snapshot plus the actual dependency revision. Neither kind of
declaration grants file-read, Git-mutation, activation, or action authority.

The root `.reyignore` file may contain `workload: <pattern>` rules. Rey loads
and validates the complete catalog before filtering matching package or draft
identities, preventing ignored malformed content from bypassing validation.
The relevant rules, source digest, source lines, and match counts participate
in the WORKING snapshot identity and remain visible as omissions. Changing the
file therefore creates WORKING drift even when the resulting package set is
otherwise unchanged. Existing HEAD and INDEX are immutable; an ignored HEAD
workload remains admitted and runnable until a later explicit admission changes
HEAD.

The package path and exact source-byte digest form the WORKING proposal
identity. `add` copies those exact bytes into INDEX and qualification records
the complete INDEX snapshot identity. Changing generator provenance, graph, or
suite after qualification creates a different WORKING snapshot; restaging it
clears exact-index qualification. The graph and scenario suite may both be
generated by a coding harness, but neither the package nor its proposer can
declare admission. A failing graph cannot modify its oracle during the
campaign.

CLI `commit` reads no WORKING bytes. It verifies content-addressed INDEX objects,
requires fresh passing qualification for every staged package and the complete
snapshot, and appends one parent-linked `WORKLOAD@n` commit. `run` resolves only
the newest commit. The browser admission endpoint begins from visible file
state: it requires exact expected HEAD and WORKING identities, stages those
WORKING bytes, runs the complete frozen scenario suite, and commits only if the
resulting INDEX qualifies. Changed files or HEAD reject the action; failed
qualification leaves HEAD unchanged and retains the INDEX evidence for CLI
diagnosis.

## Workload Contract

A workload declaration binds at least:

- stable workload id and immutable revision;
- description and ownership metadata;
- typed external input and output contracts;
- allowed graph-node operation contracts and effect classes;
- allowed relational/source mining operations, input artifact kinds,
  completeness requirements, and traversal/result limits;
- environment profile, required capabilities, and trust requirements;
- exact acknowledged Git HEAD or semantic-index dependency revisions;
- graph-proposal policy contract, which may be an agent, rule, or human;
- exact scenario-suite revision and required/optional scenario membership;
- claim, comparator, evaluator, and normalizer revisions;
- graph, campaign, scenario, execution, evidence, and output limits;
- qualification and staleness rules; and
- catalog and retention requirements.

The V1 syntax covers only the executable UTF-8 slice. Future package revisions
must resolve mutable names to exact workload, graph, scenario, evaluator,
capability, and policy revisions before execution. Secret values do not belong
in the declaration.

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

Each comparison is directed from `EXPECTED` to `ACTUAL`. `ACTUAL` is the human
label for the retained observed artifact; structured schemas retain their
existing `observed` field names. A scenario may
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
  -> compare EXPECTED to ACTUAL and retain typed deltas
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

The table projection is a diff-native assertion runner, not a stream of
unstructured evidence logs. It fixes the read-only execution boundary and the
comparison direction as `EXPECTED → ACTUAL`, names the graph path being
executed, and renders scenario results as soon as they complete in declaration
order. Each scenario line keeps passing, evaluation, and required/optional
role explicit.

Every evaluated claim is projected as an assertion. UTF-8 output assertions
compare expected and actual artifacts; source-mining assertions separately
compare typed rows and required completeness; topography assertions expose
required completeness, actual coverage, and the directed structural patch.
Large equal values collapse to line, byte, and content identity. Different
values open their authoritative patch immediately. Incomplete evidence is an
inconclusive assertion rather than a passing value accompanied by a prose
warning.

Verbosity changes only this human projection:

- plain output folds passing assertions and opens only failing or inconclusive
  `EXPECTED → ACTUAL` comparisons;
- `-v` shows every compact assertion, including actual values, counts,
  completeness, coverage, and patch summaries; and
- `-vv` keeps the assertion view and additionally opens exact workload, graph,
  suite, evaluator, scenario, execution, result, delta, mining
  operation/provider/capability, corpus/request/result, native
  source/match/context, frontier, scheduling, limits, projection, and lineage
  evidence.

For example, a failing assertion remains immediately actionable:

```text
  FAIL 02/02 surrounded · 0/1 assertions satisfied · required
    Assertions (EXPECTED → ACTUAL)
      ! output.text · DIFFERENT
        EXPECTED "REY"
        ACTUAL   " REY "
        @@ -1,1 +1,1 @@
        - REY
        +  REY␠
```

The final test summary reports workload qualification, required-scenario
conformance, evaluation coverage, output-delta assessments, and issued
qualifications as separate dimensions. Verbosity is a human projection only:
JSON always emits the same verified `rey.workload-test-batch.v1` envelope,
including catalog and proposal provenance.

The browser preserves this layering through exact retained-evidence routes.
`GET|HEAD /api/v1/workloads/evidence` indexes current admitted workload
results without executing them. Scenario routes are keyed by the retained
scenario execution digest; directed-delta routes are keyed by the exact
`rey.scenario-output-delta.v1`, `rey.source-match-delta.v1`, or
`rey.topography-patch-delta.v1` identity. Their human pages render the same
plain → `-v` → `-vv` progression and retain exact native source-context links,
semantic coordinates, revisions, omissions, limits, and lineage. The server
verifies the retained result before projection and never substitutes a mutable
latest name for an unknown content identity. Freshness remains derived: stale
evidence is inspectable, but the current package source is explicitly not
claimed as that result's source binding.

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

For `context-anchor-survey`, `--source` instead supplies the complete declared
seed set. Run confines every regular-file read to the canonical workspace,
rejects symlinks and escapes, binds the current environment capability
snapshot, compares against the last retained patch when present, and retains
one new directed patch. Candidate/resolution rows remain complete in JSON;
the human table folds long row sets at a declared projection limit while
reporting how many rows remain in the authoritative structured result.

## Scenario Progress

Progress is always scoped to one exact workload, graph, and scenario-suite
revision. The denominator is the number of selected required scenarios. Rey
keeps at least these counts separate:

```text
required · passed · failed · inconclusive · blocked
running · queued · stale · optional
```

The human `rey workloads list` view first aggregates qualification, scenario,
run, inventory, mining, attention, runtime-admission, and mapped-surface coverage dimensions
across the portfolio. It then renders a canonical attention frontier and one card
per workload with a derived journey, passing and evaluated coverage, explicit
evaluation counts, qualification state, exact candidate and qualified graph
identities, retained test evidence, freshness, and latest run state. For
example:

```text
WORKLOAD PORTFOLIO
  Qualification          3/4 qualified · 1 failing · 0 inconclusive · 0 stale
  Scenarios              11/12 passing · 12/12 evaluated · 0 stale · 2 optional
  Runs                   1 passed · 0 blocked · 3 not run
  Inventory              4 total · 4 tested · 0 untested
  Mining                 2 workloads · 10 retained results · 1 incomplete
  Attention              0 refine · 0 retest · 1 create · 0 blocked · 1 policy excluded
  Coverage               1 mapped surfaces · 0 owned · 1 unowned

ATTENTION FRONTIER
  CREATE           Cargo.toml · unowned_surface · ready · priority 80 · cost 5
  POLICY_EXCLUDED  rey.fixture.text-mismatch · policy_excluded · excluded · priority 0 · cost 0

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
`rey.local-workload-state.v1` JSON index at
`${workspace}/.rey/workloads/state.json`, or below explicit `--state-dir`.
It verifies retained test and run artifacts on every read and publishes a
same-directory temporary document with rename. It claims no `fsync` crash
durability, locking, multi-process transactionality, authenticated writer, or
external-service semantics. Workspace package YAML is a catalog input contract;
it does not select a result database or remote representation. A future result
provider must state the durability, query, run, and lineage guarantees it
actually implements; Rey does not project the local layout onto it.

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
| `workloads create id` | Create one immutable bounded request for an external coding harness at `<catalog>/<id>/request.yaml`; print exact instructions and next action. It refuses overwrite and creates no graph, scenario, or admission claim. |
| `workloads status` | Observe compact workload HEAD, INDEX, and WORKING state, drafts, staged and unstaged changes, qualification omissions, and approval readiness. It performs no mutation or graph execution. |
| `workloads diff [--staged]` | Compare INDEX to WORKING, or HEAD to INDEX with `--staged`, retaining exact snapshot identities and inserted/deleted/modified workload ids. |
| `workloads add` | Verify and stage the complete WORKING package catalog as one exact content-addressed INDEX snapshot. It neither tests nor admits packages. |
| `workloads test --staged [id]` | Test one staged workload, or every workload in INDEX when no id is supplied. Execute bounded graphs and probes, retain actual-versus-expected deltas and evidence, and bind complete all-passing qualification to the exact INDEX snapshot. |
| `workloads commit -m message` | Human approval gate: verify exact INDEX objects and qualification, append one parent-linked workload commit, advance HEAD, and clear INDEX without observing WORKING. |
| `workloads log [-p] [-n count]` | Verify and render workload admission history newest first, optionally deriving exact parent-to-commit package patches. |
| `workloads list` | Project admitted HEAD and retained result state while separately carrying drafts and HEAD/INDEX/WORKING revision posture. It performs no graph execution. |
| `workloads admit-activation id` | Revalidate one proposal from acknowledged Git history against the current cursor, workload HEAD, graph, scenarios, capabilities, and effective budget; retain schedule-only admission without executing it. |
| `workloads execute-activation id` | Revalidate one retained admission, execute only its selected scenarios under the exact evidence cap, and retain replay-stable non-qualifying deltas and evidence without mutating Git. |
| `workloads verify-activation id` | Revalidate one retained execution, fully recompute its declared suite under an explicit evidence cap, compare selected scenario evidence exactly, and retain a replay-stable non-qualifying proof. |
| `workloads run id` | Execute only an exact current fresh qualified graph admitted in HEAD against explicit inputs through declared providers; retain outputs and exact mining lineage. `scene-admission` additionally requires `--scene SCENE@n` and may select an explicit editor state directory. |

`list` and `status` return success when inspection succeeds even if workloads
are failing. `verify-activation` returns `0` for exact equivalence and `2` for
a conclusive difference. `test` and `run` use semantic exit categories: `0`
for qualified
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

There is also an outer campaign. `rey.portfolio.attention` mines workloads and
mapped surfaces into a bounded attention relation. Its ready rows can later
become generic frontier rows; blocked and policy-excluded rows remain visible
but ineligible. This derivation directs scheduling and does not replace it.

The fresh pre-alpha baseline uses `rey.frontier.v1`,
`rey.frontier-progress.v1`, `rey.scheduling-decision.v1`, and
`rey.reasoning-surface.v1` bind exact workload, graph, scenario-suite, and
campaign identities. The runtime reducer is `rey.runtime-state.v1`. Documents
outside those complete v1 contracts are rejected.

## Initial Implementation Boundary

The implemented product slice creates bounded harness requests and loads
bounded accepted workspace packages. The
checked-in package proves coding-harness provenance, frozen scenarios,
qualification, run gating, and provenance-driven staleness through the
evaluation and inspection commands. The creation command proves the request
half of the handshake and exposes drafts, but Rey does not launch the harness.

Four small compiled workloads remain in the explicit conformance catalog. The
source-search workload closes the first mining loop from explicit corpus through operation,
typed relation/native context, ordered and relational deltas, one scheduled
frontier row, reasoning surface, qualification, and admitted real-input run.
The portfolio workload closes a second deterministic path from catalog,
retained results, and environment mapping through typed attention, scenario
qualification, list/status inspection, and retained-input run. Workspace
packages may now bind mapped files to bounded ownership declarations; live
portfolio composition derives owners, changed source dependencies, and missing
required capabilities from the retained environment snapshot. Packages may
also bind exact Git HEAD or semantic-index dependencies; live portfolio
composition derives their changes only from the acknowledged Git cursor
snapshot. Ready attention reaches the generic frontier and one bounded
reasoning surface, and a selected `CREATE` row can complete the immutable
harness-response and human-admission cycle. Acknowledged Git proposals can
cross exact admission and selected-scenario execution without changing
qualification. Generic distributed or recurring scheduling, arbitrary code
execution, a persistence engine, parser/index breadth, and provider-specific
policy loops remain outside this boundary.
