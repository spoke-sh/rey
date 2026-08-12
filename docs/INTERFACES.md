# Rey Interfaces

This document defines Rey's typed provider, policy, persistence, and HTTP/UI
boundaries. [Rey Command-Line Interface](CLI.md) is the canonical command
philosophy and command-level reference; CLI details retained here explain the
typed contracts projected by those commands.

The implemented local surface includes Git-shaped environment, workload,
editor, and Channel topology histories; file-backed workload qualification;
deterministic workload/mining/portfolio execution; immutable Channel messages
and explicit relay attempts; bounded Journal admission; and an explicitly
started embedded operator UI over the same evidence. Lower-level proof and
local-bundle contracts remain library/runtime capabilities rather than manual
commands. Automatic graph-proposal policy, recurring activation, scene
admission, general provider execution, and a conversation transport remain
incomplete. The [current decision plane](decisions/README.md) summarizes the
accepted structure; subject documents own its exact semantics.

## Interface Principles

- Machine output is stable, typed, bounded, and separate from diagnostics.
- DataFrame-shaped output preserves one logical schema across terminal, Arrow,
  and explicit JSON representations.
- Raw and native artifacts remain byte streams rather than acquiring a table
  wrapper for uniformity.
- Relational and source mining operations expose one bounded request/result
  discipline while preserving their distinct artifact semantics.
- Visualizations cite authoritative mined artifacts and expose grouping,
  elision, completeness, and deep links; they never redefine assessment.
- Every result exposes exact source revisions, format versions, completeness,
  and effective limits needed to interpret it.
- Read-only observation and effectful action are visibly different operations.
- Policy proposals carry no authority until admitted by the runtime.
- Environment discovery is bounded and returns an inspectable capability
  relation before policy selects work.
- Providers expose only their proven capabilities and never gain action
  authority through discovery alone.

## CLI Contract Projection

This document no longer owns command UX. See [Rey Command-Line
Interface](CLI.md) for the canonical implemented command map, read/mutation
posture, `HEAD → INDEX → WORKING` model, formats, streams, colors, and exit
behavior. The command and schema detail below is retained where it explains
the typed workload, environment, and provider contracts consumed by other
interfaces.

Rey's product surface is intentionally small:

```text
rey channels [--workspace PATH] [--state-dir PATH] list
rey channels [--workspace PATH] [--state-dir PATH] status
rey channels [--workspace PATH] [--state-dir PATH] diff
rey channels [--workspace PATH] [--state-dir PATH] apply <channel-graph.yaml>
rey channels [--workspace PATH] [--state-dir PATH] add
rey channels [--workspace PATH] [--state-dir PATH] diff --staged
rey channels [--workspace PATH] [--state-dir PATH] commit -m MESSAGE
rey channels [--workspace PATH] [--state-dir PATH] log [-p] [-n COUNT]
rey channels [--workspace PATH] [--state-dir PATH] message add <message.yaml>
rey channels [--workspace PATH] [--state-dir PATH] message list
rey channels [--workspace PATH] [--state-dir PATH] relay <message-id> --relay <relay-id>
rey channels [--workspace PATH] [--state-dir PATH] beacon <beacon-id>
rey env [--workspace PATH] [--state-dir PATH] status [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] add [-p] [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] diff [--staged] [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] commit -m MESSAGE
rey env [--workspace PATH] [--state-dir PATH] log [-p] [-n COUNT]
rey workloads [--workspace PATH] [--catalog-dir PATH] create <workload-id> [--title TITLE] [--intent INTENT]
rey workloads [--workspace PATH] [--catalog-dir PATH] list
rey workloads [--workspace PATH] [--catalog-dir PATH] status
rey workloads [--workspace PATH] [--catalog-dir PATH] diff [--staged]
rey workloads [--workspace PATH] [--catalog-dir PATH] add
rey workloads [--workspace PATH] [--catalog-dir PATH] test --staged [<workload-id>] [-v|-vv]
rey workloads [--workspace PATH] [--catalog-dir PATH] commit -m MESSAGE
rey workloads [--workspace PATH] [--catalog-dir PATH] log [-p] [-n COUNT]
rey workloads [--workspace PATH] [--catalog-dir PATH] run <workload-id> --input <utf8>
rey workloads --catalog conformance list|test|run|status ...
rey journal [--workspace PATH] [--state-dir PATH] add <proposal.yaml>
rey journal [--workspace PATH] [--state-dir PATH] list
rey ui [--workspace PATH] [--state-dir PATH] [--journal-state-dir PATH] [--catalog-dir PATH] [--host IP] [--port PORT]
```

`channels` exposes bounded collaboration topology through a complete local
revision loop, admits immutable file-backed messages, and gates explicit relay
and polling-beacon effects on exact Channel and environment HEAD identities. `env`
inventories and revisions the available compute boundary. `workloads` is the
public unit for composing and using runtime concepts. `journal` is the
agent-facing admission and retrieval surface for typed collaboration entries;
it does not execute their blocks. Spaces, lenses,
frames, deltas, frontiers, traces, and proofs remain typed evidence and may
gain focused diagnostic projections, but they are not peer top-level resources
that users must manually orchestrate.

Mining follows the same rule. Search, parse, index, group, traverse, diff, and
visualize are discoverable operation contracts composed inside workloads and
reasoning surfaces, not an accepted `rey mining` resource hierarchy.

`ui` is primarily a presentation command, not a peer runtime resource. It starts on
`127.0.0.1:5714` unless configured otherwise, reports exact exposure and
provenance, `/explore` human entry, and passive revalidation interval. It
serves read-only workload, environment, cadence, and Explorer projections and
admits bounded human Journal documents without authentication. An explicit
non-loopback bind exposes that narrow write to reachable clients and emits a
warning.

The implemented slice behaves as follows:

- `create` writes one immutable bounded request for an external coding harness,
  prints the exact instructions/next action, refuses overwrite, and invents no
  graph, scenario, oracle, or admission claim;
- `status` observes the complete workload HEAD, INDEX, and WORKING portfolio;
  `diff` projects INDEX-to-WORKING or HEAD-to-INDEX changes;
- `add` freezes verified package bytes as one exact INDEX; it performs no test
  or admission;
- `test --staged` executes a bounded deterministic graph/scenario pass,
  retains `EXPECTED` to `ACTUAL` typed deltas plus mining evidence, and binds
  complete all-passing qualification to the exact INDEX snapshot;
- `commit` verifies the frozen INDEX and qualification and advances HEAD
  without re-observing WORKING; `log` verifies and renders that history;
- `list` reads admitted HEAD and result indexes while carrying drafts and
  revision posture separately; it executes no graph or probe;
- `run` executes the current fresh qualified graph admitted in HEAD against one
  admitted UTF-8 input and, for the mining workload, repeatable explicit
  `--source` paths under the workspace. The portfolio workload instead binds
  retained catalog/workload/environment inputs and rejects `--input`;
- the explicit conformance catalog preserves detailed fixture inspection
  without participating in workspace admission history.

The default catalog resolves request-only drafts from
`sys/*/request.yaml` and WORKING proposals from
`sys/*/workload.yaml`. Creation requests bind a semantic request id,
bounded intent, target, requirements, and limits; they remain ineligible for
qualification/run. V1 packages support only UTF-8 ports and exact
`trim`/`uppercase` operation contracts; each package carries proposal kind,
producer revision and inputs, generated graph/suite roles, and a frozen
scenario oracle. Exact package bytes and path participate in the workload
proposal identity, INDEX qualification, and admission commit.

`--catalog conformance` instead selects compiled `rey.portfolio.attention`,
`rey.fixture.source-search`, passing `rey.fixture.text-normalize`, and failing
`rey.fixture.text-mismatch` diagnostics. The CLI labels this catalog and each
workload's origin so compiled fixtures cannot be mistaken for product work.

See [Workloads, Compute Graphs, and Scenarios](WORKLOADS.md) and
[Mining Context Into Evidence](MINING.md).

## Implemented Workload CLI

Every workload subcommand accepts `--format auto|table|json`. `auto` chooses a
table on a terminal and JSON when redirected. `--workspace` defaults to `.`;
relative `--state-dir` values resolve below the canonical workspace and an
absolute value selects an explicit separate local boundary. `--catalog`
defaults to `workspace`; `--catalog-dir` defaults to the workspace-relative
`sys` directory.

The `create` table identifies the local mutation plane, request revision,
created file, `AWAITING CODING HARNESS` admission state, absent graph/oracle,
complete harness instructions, and next action. Its structured result is
`rey.workload-create-result.v1`; the retained request is
`rey.workload-creation-request.v1`. Both are provider-neutral. Rey does not
launch an LLM or coding harness inside deterministic runtime mechanism.

The `list` table is a portfolio document rather than a flattened relation. Its
portfolio header derives qualification, scenario, run, inventory, mining,
attention, and mapped-surface coverage totals. It then renders the canonical
attention frontier;
each workload card exposes purpose, journey, passing and evaluated scenario
coverage, evaluation counts, qualification, exact graph and operation
identities, retained test/mining evidence and freshness, and last-run state.
ANSI styling is enabled only for an interactive terminal and is never the sole
carrier of meaning. Forced table output through a pipe remains ANSI-free.
Environment and workload admission status share one positional color contract:
green means a change retained in INDEX and awaiting commit, while red means
WORKING drift that has not been staged. Inserted, deleted, and modified labels
remain the authoritative change direction; color never overrides those labels.
Portfolio aggregates are derived from authoritative per-workload counts.

The `test` table is a diff-native assertion runner. It declares the selected
read-only execution boundary, `EXPECTED → ACTUAL` direction, graph path, and
workload scope before executing scenarios, then renders each result as soon as
the deterministic runtime completes it. Plain output folds passing assertions
but always opens failing or inconclusive comparisons. `-v` renders every
compact expected and actual assertion, including typed-row counts,
completeness, coverage, and structural patch summaries. `-vv` keeps that view
and additionally exposes exact workload, graph, suite, evaluator, scenario,
execution, result, delta, operation/provider/capability,
corpus/request/result, native artifact, frontier, scheduling, limit,
projection, and reasoning-surface identities. A final test summary keeps
workload qualification, required-scenario conformance, evaluation coverage,
output-delta assessment, and qualification counts separate. These verbosity
flags affect only the human projection; redirected `auto` and explicit JSON
retain the same `rey.workload-test-batch.v1` document and its existing
`observed` field names.

Portfolio-attention scenarios retain `rey.workload-attention.v1` beside the
ordered UTF-8 output delta. `-v` exposes action/reason/readiness rows; `-vv`
adds exact row, relation, source-snapshot, derivation, evidence, and dependency
identities. A qualified `run rey.portfolio.attention` emits the same typed
relation over current retained inputs. `list` and `status` derive their view
without fresh ambient discovery.

The structured schemas are `rey.workload-list.v1`,
`rey.workload-status-batch.v1`, `rey.workload-test-batch.v1`, and
`rey.workload-run-view.v1`. Their `rey.workload-catalog.v1` descriptor
separates total, admitted, and draft counts. The run view contains the unchanged verified
`rey.workload-run-result.v1` plus exact catalog and proposal provenance. Test results contain verified
`rey.scenario-output-delta.v1` documents embedding `rey.text-delta.v1`, and
mining scenarios contain `rey.source-match-delta.v1`. Topography scenarios
also retain `rey.topography-patch.v1` and its directed patch delta; `-v`
projects anchors, classified edges, regions, and frontier, while `-vv` adds
exact operation, provider, implementation, capability, limit, and lineage
bindings. Passing tests alone
contain a `rey.workload-qualification.v1` binding the exact workload, graph,
scenario suite, evaluator, and test result.

`rey.workload-list.v1` additionally carries an optional
`rey.semantic-atlas.v1` whenever verified regional topography is retained. The
atlas binds stable region identities to exact patch/revision sources and
integer synthetic semantic longitude/latitude. It declares no Earth CRS.
`workloads list` exposes its exact revision, compiler, region/cluster counts,
boundedness, admission-revision recluster rule, and the fact that zoom cannot
recluster it. The current document derives the atlas from retained admission
state; prior-revision movement deltas remain
[Plan 0003](../plans/0003-scene-to-explorer.md) work.

## Implemented Environment CLI

The executable currently exposes:

```text
rey env [--workspace <path>] [--state-dir <path>] status [--map <path>]
  [--format table|json] [--max-changes <n>]
  [--total-timeout-ms <n>] [--probe-timeout-ms <n>]
  [--max-capture-bytes <n>]

rey env [--workspace <path>] [--state-dir <path>] add [-p] [--map <path>]
  [--format table|json] [--max-changes <n>]
  [--total-timeout-ms <n>] [--probe-timeout-ms <n>]
  [--max-capture-bytes <n>]

rey env [--workspace <path>] [--state-dir <path>] diff [--staged] [--map <path>]
  [--format table|json] [--max-changes <n>]
  [--total-timeout-ms <n>] [--probe-timeout-ms <n>]
  [--max-capture-bytes <n>]

rey env [--workspace <path>] [--state-dir <path>] commit -m <message>
  [--format table|json] [--max-changes <n>]

rey env [--workspace <path>] [--state-dir <path>] log [-p]
  [-n <count>] [--format table|json] [--max-changes <n>]

```

`status` is the single environment inventory and revision view. It performs a
fresh observation, retains the complete working snapshot in
`rey.environment-status.v1`, and derives one typed variable, application,
input, and reference projection over `HEAD → INDEX → WORKING`. Human output is
a compact working-tree view: current `ENV@n`, then environment-native staged
and unstaged groups when present. Clean status contains only the coordinate and
clean result. Workspace, health, inventory, and mapping summaries remain in
the structured status evidence. Exact values, complete search records, and
topology are delegated to `diff`;
unprojected authoritative capability changes receive a human semantic label and
retain their exact capability id.

`add` replaces the admission index with the fresh working snapshot. `add -p`
prompts over canonical capability changes as confirmable `diff --rey`
environment hunks and stages only selected rows; its interactive mode requires
table output. Generic hunks omit raw structured provenance and point to the
structured diff. `diff` selects `INDEX → WORKING` by
default and `HEAD → INDEX` with `--staged`. Its table projection uses the same
three environment-native planes as `/environment`: `01 / DIRECTED TEXT`,
`02 / BOUNDED SEARCH`, and `03` `REFERENCE PLANE`. Bounded search first renders
the exact application-declaration identity as `DESIRED INVENTORY`, then the
exact target capability snapshot as `SEARCH RECORD`. The authoritative
capability assessment remains in the coordinate header; JSON is
`rey.environment-diff.v1` and does not replace the typed capability delta with
the human projection. `commit` performs no discovery and appends only the
verified retained index to the linear history at
`${workspace}/.rey/env/state.json` by default. Successful table-mode commits
are silent on stdout and stderr; explicit JSON returns the structured receipt,
`log -n 1` supplies human readback, and failures remain nonzero stderr
diagnostics. Every v1 commit binds an integer
Unix commit time as explicit retention metadata. `log` is newest-first; `-n`
bounds selection, every entry shows `ENV@n`, semantic parent, date, and
message, and `-p` expands each exact parent-to-commit transition through the
three environment-native planes. Documents without the complete v1 fields are
rejected.
The index is a separate HEAD-bound `rey.environment-admission-index.v1` at
`${workspace}/.rey/env/index.json` by default. Plain human history is a compact
revision/evidence/environment/change/mapping/message chronology; patch mode
adds directed variables, application search, inputs, and topology. Explicit
JSON uses `rey.environment-status.v1`,
`rey.environment-commit-result.v1`, and `rey.environment-log.v1`.

Discovery always records the process-owned `HOME`, `PWD`, and `PATH` seeds and
the compiled desired-adapter inventory. It loads no project configuration by
convention. `--map` explicitly selects an agent-generated workspace-relative
regular YAML resource. `rey.env-map.v1` is a closed, bounded
graph of variable, file, and desired executable nodes plus declared reference
edges. Every desired executable records why it belongs in the inventory.
Mapped file bytes are not retained. Sensitive variables are presence-only.
Non-sensitive variables may opt into presence, a value digest, or an exact
bounded UTF-8 value; files retain bounded identities; executable candidates
retain the bounded search-path count and are resolved and hashed but never
invoked by the mapping provider. Declared potential capabilities remain
unadmitted. One graph row and exact node/edge rows make the mapping and its
observed drift visible in every environment revision surface.

The desired inventory includes the `git` executable, but environment snapshots
exclude repository HEAD, ref, semantic-index, and reachability observations.
Those remain first-class through `rey-git`, cadence, and exact workload
activation evidence; Git movement alone is not an environment delta.

Admission accepts evidence into history; it does not admit executable action or
turn potential capabilities into provider contracts. There is no pathspec,
reset/restore, branch, merge, rewrite, or revision expression in this slice. An
environment commit records an observation; it is not a Git commit and does not
mutate the environment. The bounded local state
claims no `fsync`, locking, authenticated writer, multi-process transaction,
remote retention, or remote durability.

Manual `prove`, `verify`, and `verify-bundle` commands are not part of the
accepted CLI persona. Their proof, certificate, and local-retention contracts
remain usable behind workload evaluation and in focused lower-level tests.
Help must not imply that a planned provider capability is available.

## Formats

DataFrame-shaped commands support or are expected to support:

```text
--format auto|table|arrow|json
```

- `auto` renders a bounded human view on a terminal and emits Arrow IPC stream
  when redirected.
- `table` forces the complete documented terminal relation within output
  bounds.
- `arrow` writes Arrow IPC stream bytes without diagnostic text or a trailing
  newline.
- `json` emits an explicit bounded envelope retaining schema, identity,
  revisions, completeness, and cursor metadata.

Workload campaign, status, and run results are structured envelopes rather
than one relation. Their accepted `auto` behavior is a human document on a
terminal and JSON when redirected. Explicit Arrow is appropriate for catalog,
scenario, frame, or delta relations, not for forcing a graph, campaign, native
output, or mixed artifact set into a synthetic table.

Environment status, add, diff, commit, and log are mixed structured envelopes. They
default to human output, like Git commands, even when redirected; automation
must request `--format json` explicitly. `status` carries the full structured
inventory while keeping its human view navigable. Default `diff` opens the
unstaged three-plane environment projection, `diff --staged` opens the staged
projection, and `log -p` controls three-plane expansion of retained
parent-to-commit transitions.
The JSON log retains authoritative typed deltas regardless of whether the
human patch was requested.

The implemented capability change Arrow relation is
`rey.capability-changes.v1`; its frame attributes bind source and target
snapshot ids and labels, comparator identity, and delta id. Tabular Diff uses
`text/csv; charset=utf-8; profile=tabular-diff-0.8`, is portable and ANSI-free,
and is not authoritative input for proof or replay. Generic frame-delta media
types and schemas remain future diff work.

## Mining Operation Contract

The target provider-neutral mining interface has three semantic documents:

```text
mining operation contract
  id/revision/implementation · family/kind · input/output contracts
  parameters · capabilities/effects · limits · completeness · invalidation

mining request
  workload/graph/scenario/transition · frontier rationale
  exact source or artifact inputs · operation · canonical parameters
  capability snapshot/provider · requested/effective limits

mining result
  request/result identities · realized provider/tool/parser/query lineage
  native/frame/tree/graph/metric/delta/visual artifact references
  schemas/media types/lengths · completeness · omissions · consumption
  dependency and staleness edges
```

Relational and source operations share this envelope but not one artificial
payload type. Typed collections use frames and Arrow. Ordered source, patches,
trees, graphs, and binary artifacts may remain native while exposing bounded
typed index relations for navigation.

An exact immutable read can be a safe retrieval during orientation. Pure
projection over frozen evidence is deterministic graph/lens work. A mutable
read or external `rg`, parser, compiler-service, language-server, or index
invocation is a probe and requires ordinary admission and execution lineage.

Visualization is a mining result with a versioned projection contract. A
machine view exposes stable typed data or a bounded structured specification;
a human view may render a table, patch, tree, graph, timeline, or metric panel.
Both record source artifact identities, selection, grouping, ordering,
aggregation, context, elision, sampling, limits, and omissions.

### Explorer projection packet

The projection-engine boundary is not a second top-level resource. The
implemented `rey.projection-packet.v1` carries:

```text
packet identity + source evidence identities
coordinate/embedding basis + implementation revision + parameters
bounded source scene objects + scalar/vector/mask channel descriptors
typed procedural terrain program + surveyed-validity rules + world bounds
frequency bands + transient working-set sampling/cell/byte limits
scene/field/simulation/material revisions
effective object/band/working-set/resource limits
completeness + degradation + omissions + lineage
```

The packet is deterministic pure projection input. It does not contain camera
center, transient selection, measured frame time, browser graphics handles, or
pixels. An immutable scene snapshot compiled from it retains stable object and
evidence identities; camera and renderer backends consume that snapshot without
receiving authority to reinterpret source evidence.

`rey workloads test --staged context-anchor-survey -vv` exposes the implemented packet
identity, exact patch binding, synthetic anchor-orientation basis, scene
compiler, extent, terrain evaluator and seed, macro/meso/micro band parameters,
absolute-coordinate and validity rules, 255×255 / 65,025-cell / 3,576,375-byte
maximum transient working set, field descriptors, validity regions, layers,
effective limits, degradation, omissions, and lineage.
`rey.workload-list.v1` carries the same packet beside its exact patch, and
Explorer fails closed to the portfolio fallback unless both identities match.
When a semantic atlas is present, Explorer also binds the atlas and layout
compiler revisions into its immutable World scene. This adds a spherical
layout authority; it does not change the native coordinate or evidence
authority inside any regional packet.
The target `rey.explore-grammar.v1` additionally binds the renderer-independent
World-globe, semantic-Mercator, and local-isometric posture bands; hysteresis;
geometry-morph and inverse-picking policy; polar/antimeridian behavior; and
semantic/geometric LOD budgets. It is presentation mechanism and therefore
does not contain admission authority or camera instances. An admitted regional
scene must separately bind its exact editor package, native-to-semantic region
transform, county-local tangent transform, terrain and feature layers,
validity/no-data semantics, limits, omissions, and admission lineage before it
can become Atlas or County fabric. See [Explorer](EXPLORER.md) and
[Plan 0003](../plans/0003-scene-to-explorer.md).
The browser rejects a working set whose shape, channels, cells, or byte
allocation diverges from that packet. It snaps the visible envelope to
absolute scene coordinates and selects only frequency bands supported by the
current sample spacing while retaining one exact source extent. Clipmap reuse,
smooth LOD blending, retained
renderer/fallback captures, viewport evidence, and performance evidence remain
incomplete [Plan 0003](../plans/0003-scene-to-explorer.md) work. Structured
output preserves typed values rather than serializing GPU state.

The implemented schemas are `rey.source-corpus.v1`,
`rey.source-search.literal-utf8@1`, `rey.source-matches` version `1`,
`rey.source-match-delta.v1`, and `rey.text-delta.v1`. They have no peer
top-level CLI resource; `rey.fixture.source-search` composes them. Fixtures
prove canonical identity, native source binding, typed empty matches, complete
and truncated comparison, bounds, Arrow/JSON replay, source drift, and
delta-directed reasoning. Regex, case folding, directory/glob selection,
external `rg`, parser/index operations, and general visualization contracts
remain later workload slices.

The portfolio schemas are `rey.portfolio-snapshot.v1` and
`rey.workload-attention.v1`; the operation contracts are
`rey.portfolio.attention.derive@1` and
`rey.portfolio.attention.render-lines@1`. The attention relation has a Polars
frame projection keyed by semantic row id and preserves action, subject,
reason, readiness, priority, and cost.

## Standard Streams

- Selected machine data and raw artifacts go to stdout.
- Diagnostics, progress, action rationale, and remediation go to stderr.
- Interactive progress is disabled when stdout carries Arrow, CSV, JSON, or
  raw bytes.
- The human `workloads test` document streams retained scenario results to
  stdout in declaration order. Machine output emits only the final structured
  result, without transient progress; diagnostics remain on stderr.
- Policy subprocess protocols, if selected, use dedicated framed channels or
  files rather than mixing control messages with artifact stdout.

Command tests verify stdout, stderr separation, bounded input, and categorized
exit behavior. Environment inspection, status, diff, commit, and log return
`0` on successful command execution; semantic differences shown by status or
diff are normal output. Invalid input and runtime failure return `1`; Clap
retains its own argument-parsing exit behavior.

Implemented `workloads create`, `status`, `diff`, `add`, `commit`, `log`, and
`list` return `0` whenever the requested mutation or inspection succeeds.
Semantic differences and an unready INDEX are status, not command failure.
`workloads test` uses `0` for qualified, `2` for conclusive semantic failure,
`3` for inconclusive, and `1` for invalid input or runtime failure. `run` uses
`0` for passed, `3` for blocked, and `1` for invalid input or runtime failure.
Staleness remains typed state; the current executable does not assign it a
separate exit code.

## Identities

User-facing references may be stable ids, credential-free Rey URIs, or explicit
artifact paths. A mutable display name never substitutes for the exact identity
stored in evidence.

A future URI grammar may cover:

```text
rey+workload:<workload-id>@<revision>
rey+graph:<graph-id>@<revision>
rey+scenario:<scenario-id>@<revision>
rey+campaign:<campaign-id>
rey+space:<space-id>@<revision>
rey+lens:<lens-id>@<revision>
rey+frame:<frame-id>
rey+delta:<delta-id>
rey+trace:<trace-id>
rey+proof:<proof-id>
```

This grammar is illustrative, not accepted. It must be decided alongside
percent-encoding, canonicalization, and tenancy.

## Workload Declaration

A workload declaration needs stable workload identity and revision; typed
external inputs/outputs; admitted graph operations and effects; provider and
capability requirements; exact scenario suite; claim/comparator/evaluator
revisions; graph-proposal policy; graph/campaign/scenario/run limits;
qualification; and catalog/result retention requirements.

Admitted graph operations include exact mining operation contracts. A workload
declares which relational/source inputs, output artifact kinds, parser/index or
tool semantics, completeness, and mining limits its scenarios require.

Each immutable graph revision binds typed nodes, ports, dependency edges,
operation contracts, capabilities, effects, limits, and generator provenance.
Each scenario binds fixtures, test providers, selected outputs, expected
observations or claims, comparison rules, completeness, and bounds. The first
graph contract is a finite typed DAG. Exact serialization remains open. See
[Workloads, Compute Graphs, and Scenarios](WORKLOADS.md).

## Space Declaration

A space declaration needs, independent of final YAML/JSON/TOML syntax:

- id, revision, description, and owners;
- allowed environment providers;
- required capabilities and guarantee levels;
- source bindings or binding rules;
- lens, action, policy, and claim revisions;
- Git watched refs, index/worktree surfaces, poll limits, and trigger revisions
  where applicable;
- dependency and invalidation declarations;
- allowed mutation targets and effect classes;
- runtime, frame, delta, trace, and evidence limits; and
- artifact retention policy.

Configuration stores environment-variable or secret-handle names, never secret
values. Selecting a serialization format and merge/override behavior is an open
decision.

## Environment Discovery

Environment discovery produces a bounded typed capability relation before a run
admits actions. Provider configuration defines allowed workspace roots,
executable search paths, known tool probes, network endpoints, timeouts, output
bounds, and trust assumptions.

A capability row needs to expose at least:

```text
provider_id · provider_revision · capability_id · kind
resolved_location · version · digest/provenance · availability
trust_class · operations · enforcement · observed_at · error
```

`resolved_location` is provider-specific evidence. A provider URI is never
handed to a host process as a path.

Known executable discovery may resolve configured paths or `PATH`, inspect
metadata, and invoke a bounded read-only identity command such as `--version`.
It does not run unknown files, shell startup hooks, project scripts, or package
installers. An action must separately name and be admitted against the frozen
capability row.

Mining capability discovery additionally records operation revision, accepted
source/artifact kinds, output schemas/media types, encoding/language support,
determinism, completeness semantics, and enforceable file, match, node, edge,
depth, row, byte, and time limits. Discovering `rg` by version does not itself
prove an admitted source-search operation.

The runtime supports these provisional selection attitudes:

```text
--environment auto|standalone
--require-capability <capability-id>[,...]
```

`standalone` selects the implemented local providers. A required capability
fails closed if unavailable. Exact flag and configuration names remain
provisional.

## Git Polling And Activation

The Git interface inspects one explicitly selected repository/worktree and
returns typed repository, ref, commit, parent, index, and declared status
relations. It does not run repository hooks or modify refs, index, or worktree.

A poll request names:

- repository/worktree identity;
- watched refs and whether HEAD, semantic index, or bounded worktree status are
  included;
- prior cursor or initial-baseline behavior;
- commit/path traversal limits;
- trigger declarations and target workload/graph/scenario selections;
- activation concurrency and budgets; and
- cursor/evidence retention profile.

The poll result contains source and target snapshot ids, ref/index/worktree
deltas, history completeness, matched triggers, activation ids, transition
outcomes, and the next cursor. The next cursor is publishable only after the
required activation evidence reaches its declared retention boundary.

The implemented `rey git watch` is the bounded local recurrence surface over
the current HEAD and complete supported semantic-index poll. It accepts explicit maximum
iterations, interval, elapsed cadence budget, and the same bounded trigger
documents as a single poll. It retains each `rey.git-cadence-tick.v1` before
continuing, atomically retains a changed transition with its terminal tick,
and then stops. A completed invocation retains a compact
`rey.git-watch-receipt.v1` referencing its tick sequence and exact
`rey.git-watch-outcome.v1` identity. A process interruption may therefore
leave retained unreceipted ticks, which the human status reports as an
evidence gap. No watch acknowledges a transition, executes an activation, or
claims convergence.

A trigger declaration includes a stable id/revision, source event classes,
ref/path/stage predicates, required Git capabilities/completeness, target
workload revision plus scenario selection or graph entry point, coalescing
policy, budgets, and replay/idempotency behavior. Trigger output is an
activation proposal and passes normal runtime admission.

Initial event vocabulary may include:

```text
ref.created|deleted|fast_forward|rewound|rewritten
head.changed
commit.reachable_added|reachable_removed
index.changed|conflicted
worktree.changed
```

Exact configuration and output schemas remain provisional. See [Git Context
and Activation](GIT.md).

## Frontier And Scheduling Contracts

The generic contracts have no peer top-level CLI resource. The source-search
workload now projects one failure-derived frontier, scheduling decision, and
reasoning surface through `test -vv` and `status`.
`rey.frontier.v1` binds exact workload, graph, scenario-suite, campaign,
space, trace, committed-record, capability, derivation, prioritization,
coverage, and limit inputs. Its canonical `rey.frontier-rows` version `2`
relation is keyed by stable `work_id` and
retains a derived row identity, delta/claim/lens/action citations, readiness,
blockers, priority, and estimated cost.

`rey.frontier-progress.v1` compares compatible source and target frontiers in
that direction while preserving source and target graph identities. Its
`rey.frontier-progress-changes` version `2` relation
reports resolved, introduced, or updated work with source/target row ids;
unchanged work remains a summary count.

`rey.scheduling-decision.v1` rejects stale expected record, frontier, and
capability identities and selects ready work by declared priority descending,
cost ascending, then stable work id. The `rey.scheduled-work` version `2`
relation retains selection rank and exact frontier row identity. These are
deterministic selection contracts, not provider reads, action proposals, an
execution queue, or a recurring scheduler. See
[Frontier, Progress, and Scheduling](FRONTIER.md).

## Reasoning Surface Contract

Before requesting a policy proposal, the runtime constructs a bounded
delta-directed reasoning surface. The implemented
`rey.reasoning-surface.v1` envelope contains:

- surface schema, identity, and projection-contract revision;
- workload, graph, scenario-suite, campaign, space, and trace identities;
- committed and active transitions, scheduling decision, frontier frame, cited
  frontier rows, and applicable transition/residual delta identities;
- exact retrieved evidence addresses, source bindings, and provider revisions;
- exact mining request/result, operation, artifact, derivation, completeness,
  and visualization references;
- a bounded typed projection of changed and unresolved entities;
- exact versioned admissible action contract references;
- capability snapshot identity;
- effective row, delta-reference, evidence-reference, action-reference,
  omission, evidence-byte, string-byte, and retrieval-iteration bounds;
- the actual retrieval-iteration count; and
- complete, partial, or truncated status with explicit omissions.

Its canonical `rey.reasoning-surface-rows` version `3` DataFrame contains:

```text
frontier_row_id · entity_kind · entity_id
transition_delta_ids · residual_delta_ids · claim_ids
evidence_ids · admissible_action_ids
```

The semantic document retains exact versioned evidence providers, source ids
and revisions, evidence digests/media types/lengths, and action contracts.
Array-valued row fields use canonical compact JSON strings in the initial Arrow
relation.

Retrieval in this phase resolves only declared read-only evidence. A mutable
observation, tool invocation, or new lens evaluation is a probe and passes
normal proposal and admission. Surface construction does not turn a local path
into a provider resource, give a cited capability execution authority, or make the
surface the sole copy of native source content.

The reasoning-surface schema is a verified v1 library contract. It is not an
implemented top-level CLI format; workload `-vv` output projects its retained
identity and evidence. The policy-proposal schema remains a target contract.

## Policy Contract

A policy request is a bounded snapshot containing:

- reasoning-surface identity and projection-contract revision;
- workload, graph, scenario-suite, and test-campaign identities when invoked by
  the workload surface;
- space, trace, frontier, and cited delta identities;
- the bounded surface projection and its completeness/omission metadata;
- admissible graph-operation and action definitions and schemas;
- exact precondition frame and source ids;
- remaining time, iteration, action, and evidence budgets;
- prior rejection or failure facts relevant to the next choice; and
- a correlation id.

A proposal contains:

- proposal kind and exact target, including graph-revision proposal or
  admissible action;
- cited reasoning-surface, frontier row, delta, and evidence ids;
- expected information gain or residual/frontier change;
- requested sub-budgets; and
- the request correlation id and precondition identities.

An action proposal additionally supplies the selected action id/revision and
typed arguments. The runtime rejects unknown actions, stale preconditions,
malformed arguments, unauthorized effects, unsupported limits, or exhausted
budgets before an effect. Free-form rationale is optional evidence and is never
executable input.

Provider-specific chat, prompt, or tool-call envelopes stay behind policy
adapters and do not become Rey's durable action contract.

A graph-revision proposal additionally supplies the immutable typed graph,
parent graph revision when present, cited failing scenario/delta facts, and
requested graph/execution sub-budgets. Runtime graph validation occurs before
the proposal can become a campaign candidate.

## Effect Paths

Effects use the operation owned by the selected provider: an explicitly
authorized admitted action.
`QUERY` never carries a Rey mutation.

A local tool-backed action freezes:

- capability snapshot and provider identity;
- resolved executable path plus version and digest/provenance when available;
- exact argv, cwd boundary, and declared input artifacts;
- effect and trust class;
- allowed environment names;
- limits and supported/unsupported enforcement; and
- idempotency identity where the effect permits it.

The local executor is not a sandbox unless a future backend proves that claim.
It records only the process and capture lineage it can establish.

A compute-backed action freezes:

- registered tool and toolset resolution;
- exact argv and declared input artifacts;
- source/frame preconditions;
- effect and egress class;
- environment-name and secret-handle sets;
- limits and backend enforcement requirements; and
- idempotency identity.

Rey observes terminal state, validates capture completeness and media type,
materializes post-action lenses, and then decides the semantic transition
outcome.

## Persistence Paths

The workload surface introduces two abstract provider roles before selecting a
physical persistence design. A catalog provider resolves workload declarations,
immutable graph/scenario assets, and mutable selectors to exact identities. A
result provider retains graph proposals, campaigns, attempts, outputs, typed
deltas, qualification records, runs, and indexes read by `workloads list` and
`status`.

The first standalone implementation uses a bounded workspace package catalog
at `${workspace}/sys`, with the compiled catalog available only by
explicit conformance selection. It uses a bounded
`rey.local-workload-state.v1` result index at
`${workspace}/.rey/workloads/state.json`, overridable by explicit
`--state-dir`. Reads reject symlinked state files and verify every retained
semantic result. Writes use a same-directory temporary file and rename. This
single-process provider claims no `fsync`, lock, or remote durability. A graph
selected for future runs cannot exist solely in a disposable cache. A stronger
publication protocol requires a separate accepted contract; the current
workload and mining contracts do not select an engine.

## Workspace Ignore Surface

`.reyignore` is an optional regular, non-symlinked UTF-8 file at the canonical
workspace root. It is bounded to 64 KiB, 256 rules, and 4096 bytes per line.
Blank lines and `#` comments are ignored. Every other line is:

```text
<typed kind>: <case-sensitive wildcard pattern>
```

V1 kinds are `workload`, `environment variable`, `application`, `input`, and
`reference`. `*` matches zero or more bytes and `?` one byte; kinds are
literal. Unknown kinds remain parseable but have no effect on a surface that
does not own them or enter that surface's identity. Invalid UTF-8, malformed lines, unsafe file
types, and exceeded bounds fail status/add/diff/UI reads closed.

Rey validates candidate objects before applying rules. Relevant rules, the
exact `.reyignore` digest, source line, and match count are part of the filtered
WORKING identity and are exposed in structured and human status. This is an
explicit omission policy, not deletion: it does not mutate source files,
retroactively alter HEAD or INDEX, bypass validation, or grant execution
authority.

For the implemented capability claim, standalone Rey writes the local proof
bundle manifest, snapshots, typed
delta JSON and Arrow, Tabular Diff, and certificate to an explicit local
content-addressed bundle through the lower-level proof API. Publication accepts
an identical verified replay, and verification bounds and recomputes the bundle
without following symlinked evidence. Workloads and runtime composition are
the intended user-facing
consumers. The final directory name is not exposed until a same-parent staging
directory contains all objects and the manifest. The manifest, rather than the
retention-neutral certificate, states the filesystem-only guarantees and
explicit non-guarantees.

Git poll cursors are part of this publication boundary. Local mode retains a
local cursor with local-file guarantees. A cursor never advances merely
because a Git poll returned successfully. Any stronger publication boundary
must coordinate with [Proofs and Evidence](PROOFS.md) and a future accepted
persistence contract.

## Errors And Limits

Structured errors need a stable category, human detail, correlation id, and
actionable remediation. Important categories include invalid declaration,
provider unavailable, capability unavailable, capability drift, source drift,
mining operation unsupported, mining result incomplete, parser/index partial,
visualization truncated, invalid graph, graph cycle, missing graph policy,
scenario mismatch, scenario
inconclusive, unqualified graph, stale qualification, Git history incomplete,
Git ref rewritten, Git index conflicted, cursor replay, stale proposal,
incompatible frame, duplicate key, action rejected, run failed/lost,
observation incomplete, budget exhausted, evidence missing, proof failed,
proof inconclusive, and proof stale.

Errors must report which state changed and which did not. Retrying a read,
proposal, compute submission, artifact publication, or mutation follows that
operation's idempotency contract rather than one generic retry rule.

## Local Operator UI, Not A Public Rey Service

`rey ui` is the implemented exception to a CLI-only topology: a bounded HTTP
operator projection started explicitly by the operator. It serves the embedded
TanStack Router application plus `GET|HEAD /api/v1/health`,
`GET|HEAD /api/v1/workloads`, `GET|HEAD /api/v1/environment`,
`GET|HEAD /api/v1/cadence`, and `GET|HEAD /api/v1/journal`. Its explicit writes
are `POST /api/v1/journal`, which accepts bounded human JSON proposals, and
`POST /api/v1/workloads/admit`, which freezes and qualifies an exact WORKING
file snapshot before committing it with expected HEAD/WORKING preconditions.
Neither is authenticated or origin-gated on
an explicitly configured listener. Other methods are rejected. Deep browser
routes receive the embedded application shell; `GET|HEAD /` redirects to
`/explore`. The application routes are `/feed`, coordinate-bound `/explore`
query views, `/cadence`, `/agents`, `/journal/new`, `/journal/{slug}`,
`/environment`, `/workloads`, and `/workloads/$workloadId`. The workload endpoint is
derived anew from the selected workspace catalog and retained local result
index, just like `workloads list`. The environment endpoint is derived anew
from the selected workspace map and local environment history through the same
function as `env status`; it does not create UI-owned evidence.

`/workloads` renders incoming INDEX/WORKING candidates, admitted HEAD, and
request-only drafts as three native Hifi `KineticDenseTable` relations. The admitted relation keeps
revision, journey, qualification, freshness, scenario conformance, exact graph
and test identities, mining output, and attention aligned. The request relation
keeps intent, admission boundary, target package, request source, and exact
detail location aligned. Narrow viewports scroll the complete bounded relation;
they do not collapse those dimensions into cards. `/workloads/$workloadId`
continues the same grammar: admitted packages expose runtime posture, scenario
outcomes, exact workload/graph/package/test bindings, and mining output as
three relations; candidate revisions expose their plane, exact package and
snapshot identity, qualification posture, and approval boundary; creation
requests expose request posture and exact coding-harness bindings as two
relations. The Feed starts with admission rows and an exact-index approval
control. That control advances HEAD through the same local commit contract as
the CLI; it never edits WORKING or bypasses qualification.

`/explore` is the initial human bearing. With no admitted topography it renders
an abstract orientation globe rather than the legacy portfolio-card graph.
Exact request, WORKING, INDEX, and admitted-but-unrun workload revisions appear
as workload beacons sourced from `rey.workload-list.v1`. Each beacon exposes
its file, digest, producer, admission plane, and next consent step. Beacon
coordinates are stable presentation geometry only; the orientation document is
not `rey.semantic-atlas.v1`, supplies no semantic-distance claim, and cannot
execute or admit a workload. The review action enters the exact workload
record; the consent action enters `/feed?streams=admission.all`, where the
existing combined qualification and human approval gate remains authoritative.

The cadence endpoint returns `rey.ui-cadence.v1`. It retains newest-first Git
reachable history and Rey environment sequence as separate clocks, with exact
limits, parents, revisions, completeness, and omissions. Its nullable
`repository_state` separately reports working-tree counts and the exact
`HEAD`-to-local-upstream publication relation. Git ticks carry `pushed`,
`local`, or `unknown` reachability against that retained upstream revision.
The endpoint performs no network fetch; local upstream state is not a live
remote-host claim. It also describes the existing mounted-browser revalidation
schedules. That schedule description is not runtime scheduler state, and the
endpoint does not poll refs, activate a workload, or retain browser reads.
`/agents` combines two sources without conflating them. Its current
system-authored rows derive from creation requests and non-excluded attention
in the workload-list document. Its authored entries come from the ordered
`rey.journal-log.v2` returned by the Journal endpoint. `/journal/new` and exact
`/journal/{slug}` routes use one live 12-column broadsheet editor for prose,
exact Explorer, read-only query, directed diff, and proposed-action cells.
Recording a retained edit appends a human-authored superseding entry and enters
its new exact route. Journal blocks expose stable `#block-{block-id}`
permalinks. Agent YAML admitted by `rey journal add` may additionally carry
bounded frame cells. Admission is
content-identified and idempotent; it retains no arbitrary HTML and executes
no block. The same canonical-coordinate, revision-consistency, block, and byte
layout, canonical-coordinate, revision-consistency, block, and byte limits
govern both paths. Its work ledger
projects only exact current revisions, qualification/run summaries, scenario
coverage, mining and delta counts, attention, and retained evidence identities.
It does not load the environment inventory, schedule work, infer an assigned
agent, or claim live process telemetry.

`/feed` composes those existing workload, Cadence, and Journal reads into a
high-cadence inspection projection. It occupies the remaining application
viewport as independently scrolling vertical streams plus a Firehose control
rail. The default composition is Signals, Admission, and Flow, but the Firehose
can add, tune, reorder, repeat, or remove streams up to an eight-lane display
bound. Signals filters are `all|journal|git|environment`; Admission filters are
`all|now|watch|bound`; Flow filters are
`all|attention|failing|qualified`. The ordered composition uses the query
grammar `?streams={plane}.{filter}[~{percent-encoded-name}],...`, for example
`?streams=signals.journal~Review,admission.now,flow.failing`. A stream title is
an inline editor: blur or Enter normalizes and autosaves at most 48 Unicode
scalar values into the URL; Escape cancels, and an empty or derived-default
name removes the suffix. Invalid entries are ignored and an entirely invalid or
absent composition uses the three defaults. The URL is browser projection state
and a deep-link boundary, not a retained runtime configuration or new API. The
TanStack Feed route validates and owns this search state; autosave replaces the
current route location rather than writing around the router with the raw
browser History API.

Signals renders rich Git, environment, and Journal posts, including bounded
Journal block previews and exact Git lineage. Evidence bodies are collapsed by
default and expand in place. Admission ranks unresolved typed attention and
repository/request/qualification posture without writing a new attention
relation or exposing an effect control. Flow renders admitted workload
qualification, scenario, run, mining, delta, and reasoning-surface posture; it
does not claim live execution telemetry. Signal wall time is display ordering
only, and order-only records follow the timestamped window. The recent Signals
window renders at most 64 records and reports older folded source records;
Admission retains its authoritative source bound. Feed has no read cursor,
unread count, drag-to-admit behavior, pagination, durable stream retention,
causal-order claim, or additional HTTP endpoint.

The implemented Channel interface provides
`rey channels list|status|diff|apply|add|commit|log|message|relay|beacon`. It
derives one
built-in workspace-local channel, one bounded subscription, and stable Signals,
Admission, and Flow stream identities without writing local state. `apply`
accepts a workspace-contained regular non-symlinked
`rey.channel-graph.v1` YAML document, canonicalizes bounded definitions,
rejects duplicate or dangling references and semantic revision reuse, and
atomically retains a tamper-detecting `.rey/channels/working.json` proposal.
Human diff output names `added`, `removed`, `modified`, `renamed`, `retargeted`,
and `moved` operations rather than serialized state; JSON returns exact graph,
source, limit, identity, and delta envelopes. `status` and `list` remain
read-only and leave an untouched workspace untouched.

The separate `CHANNEL HEAD → CHANNEL INDEX → CHANNEL WORKING` revision loop is
complete for full-graph staging. Immutable file-backed messages can be admitted
only against Channel HEAD, and explicit `relay` or one-shot `beacon` commands
require an exact environment-HEAD application plus admitted graph declarations.
Remote inbound polling, resident scheduling, browser drag/reorder persistence,
and the richer `rey observations add|list|show` frontier remain planned.
Planned observation broadcast associates one observation identity with explicit
local channels. `rey journal seed` and
`/journal/new?observations=...` project selected exact observations into an
unretained catch-up proposal; only normal Journal admission creates an entry.
Relay declarations do not enable transport until a provider contract is
separately admitted.

The startup table and `rey.ui-server.v1` JSON expose exact address, URL,
loopback status, unauthenticated Journal-write authority,
workspace, catalog root, application,
Kinetic grammar, Precision theme, pinned grammar revision, `/explore` entry,
5000 ms passive revalidation interval, canonical Rey source repository, and
implementation Git revision. Static assets
are embedded into the binary, authored presentation is extracted from StyleX
modules into a layered atomic stylesheet, and browser responses carry
restrictive security headers.

The fixed footer is the live operator communications channel. Its mailbox
count and bottom sheet derive from typed portfolio-attention rows and passive
revalidation failures; an empty sheet states that no operator attention is
requested. It never invents heartbeat messages. The mailbox button selects the
history axis; the center chevrons select a separate traditional conversation
axis for operator ↔ Rey ↔ agent communication. Selecting the active axis
closes the plane, selecting the other switches axes, and either Escape or a
click on the background closes it.
The history axis currently identifies itself as the current mounted projection,
not a durable event store. The conversation axis exposes a transcript and
composer but explicitly has no session or transport; sending is disabled and
no UI-only messages are retained. The footer shortens the implementation revision only for
presentation, and its GitHub link uses the complete 40- or 64-hex Git object
id. The same invariant applies everywhere in the browser: a contractually Git
commit SHA is the exact GitHub commit link, never inert text. When no exact
repository binding exists, the UI exposes that boundary instead of displaying
or mislinking the SHA. BLAKE3 identities and non-Git revisions are not linked
as commits.

The Refresh control does not exist. Mounted application state passively reloads
the read-only portfolio, Feed sources, and environment delta every five seconds without
invalidating or remounting the active route. A failed background request keeps
the last good document and reports delayed revalidation; it does not reset the
viewport. `ContextCanvas` projects the portfolio document through landscape,
neighborhood, and object regimes with bounded
omission disclosures; full screen, pan, focus, and zoom do not widen the data
or action authority. Implemented local semantic coordinates have the shape
`rey+local://{kind}/{identity}?revision={revision}` with a required trailing
`role` query dimension for agents. Exact browser views use
`/explore?coordinate={percent-encoded-coordinate}&scale={canonical-number}`.
Canonical coordinates order `revision`, `role`; stale bindings remain visible.
The matrix path and parser are absent from the v1 implementation. Journal v2
retains semantic coordinate and numeric scale separately; documents outside
the complete v1 contract are rejected.

Explorer consumes admitted `rey.topography-patch.v1` evidence produced through
the workloads interface.
One continuous camera projects World, Atlas, Landscape, Neighborhood, Object,
and Evidence levels while retaining the selected provider-qualified coordinate.
Camera state never becomes resource identity. Surveyed-empty, unexplored,
omitted, stale, unsupported, truncated, and frontier regions remain distinct,
and navigation does not execute locators or workloads. The CLI must expose each patch's seed
coverage, resolution outcomes, anchors, relationships, world and atmospheric
conditions, natural-feature projection limits, excluded edge provenance,
probe prerequisites, directed delta, bounds, and lineage. [Plan
0003](../plans/0003-scene-to-explorer.md) additionally requires projection
basis, immutable scene, field channels, validity masks, material/LOD
revisions, degradation, render limits, and omissions before the high-fidelity
browser projection is considered complete. See [Context Topology
Explorer](EXPLORER.md).

The agent-facing scene authoring surface is separate:

```text
rey editor generate terrain <output.geojson> --id <source> --seed <seed> \
  [--scene-id <project>] \
  --west <lon> --south <lat> --east <lon> --north <lat> [hyperparameters]
rey editor status
rey editor add
rey editor commit -m <message>
rey editor log [-p] [-n <count>]
rey editor diff [--staged]
```

All editor commands accept `--format table|json` (and terminal-sensitive
`auto`). The selected editor state store owns the project declaration at
`project.json` (`.rey/editor/project.json` by default); the CLI never reads or
writes a workspace `rey.scene.json` and exposes no project-path override.
Declared native inputs remain bounded, regular, non-symlinked, and contained
by the workspace. `generate` creates the internal project when it is absent;
`--scene-id` sets that initial identity and otherwise defaults to the generated
source ID. Before initialization, `status` returns `rey.editor-status.v2` with
`initialized=false` and no WORKING snapshot without creating local state.
The agent then fine-tunes the generated native source directly in WORKING.
`status` and `diff` compare `HEAD → INDEX → WORKING`. `add` is the only
staging operation and freezes the exact agent-edited native bytes. `commit`
reads and validates only INDEX, advances `SCENE@n`, writes
`rey.scene-package.v1` plus
`rey.scene-admission-request.v1`, and reports `candidate_only`,
`requires_workload`, and `admitted=false`. It never changes the workload store
or UI. `log` exposes retained commit messages, parents, packages, snapshots,
and optional exact patches. Human `status` uses the concise Git-shaped
environment-status grammar: current scene commit, changes staged for commit,
changes not staged, and an actionable final state. Successful `commit` renders
the validation receipt for the frozen snapshot; validation failure prevents
HEAD from advancing. Use `log` for immutable history/package evidence and JSON
`status` for the complete typed state.

`generate terrain` writes a deterministic terrain-control GeoJSON source into
WORKING and registers it in the project. It binds the complete effective recipe
in `rey.scene-generation.v1`: seed, CRS84 bounds, feature and vertex counts,
scale interval, uplift ratio, strength, roughness, anisotropy, orientation,
edge jitter, and falloff. Same recipe means same bytes; parameter changes are
ordinary WORKING changes. The recipe reproduces the generated base; exact
post-generation agent edits are retained by the source digest and scene delta,
not folded back into an invented recipe. Generated effect values are candidate
hints and gain no admission authority. See [Context Topology
Explorer](EXPLORER.md).

`/environment` has no dashboard hero or metric strip. Its entire route body is
three full-width stacked evidence sections: directed variable text, bounded
application search, and the input/reference plane. The application plane keeps
the application-inventory identity and declared purposes distinct from the
working search snapshot identity and outcomes. Environment state, mapping, completeness, and
admission counts remain compact metadata within those sections rather than
separate visual destinations.

The coordinate rail directly beneath the application header remains sticky on
scrolling routes. Major operator sections declare exact rail coordinates; as a
section crosses the application chrome, its numbered heading replaces the
route-level coordinate in the rail's single context slot. The rail observes
navigation state only and does not mutate route or runtime state.

This listener does not establish a public API, long-running daemon contract,
multi-user scheduler, remote policy gateway, authentication system, or durable
service. Those capabilities still require explicit identity, authorization,
durability, and topology decisions.
