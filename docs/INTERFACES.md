# Rey Interfaces

This document sketches Rey's user, environment, policy, and Spoke interfaces.
The implemented CLI includes the standalone environment
snapshot/delta/certificate loop, local-only proof bundles, and the first
built-in workload slice fixed by ADR 0016. Generic workload declarations,
provider-backed operations, graph proposal policy, and connected Spoke
behavior remain provisional.

## Interface Principles

- Machine output is stable, typed, bounded, and separate from diagnostics.
- DataFrame-shaped output preserves one logical schema across terminal, Arrow,
  and explicit JSON representations.
- Raw and native artifacts remain byte streams rather than acquiring a table
  wrapper for uniformity.
- Every result exposes exact source revisions, format versions, completeness,
  and effective limits needed to interpret it.
- Read-only observation and effectful action are visibly different operations.
- Policy proposals carry no authority until admitted by the runtime.
- Environment discovery is bounded and returns an inspectable capability
  relation before policy selects work.
- Spoke is optional unless a space, action, or claim requires one of its
  capabilities.
- Connected Spoke integration uses explicit endpoint and identity configuration
  and never resolves a Spoke name through host paths or private storage.

## Accepted CLI Shape

Rey's product surface is intentionally small:

```text
rey environment <command>
rey workloads [--workspace PATH] [--state-dir PATH] list
rey workloads [--workspace PATH] [--state-dir PATH] test [<workload-id>] [-v|-vv]
rey workloads [--workspace PATH] [--state-dir PATH] run <workload-id> --input <utf8>
rey workloads [--workspace PATH] [--state-dir PATH] status [<workload-id>]
```

`environment` inventories the available compute boundary. `workloads` is the
public unit for composing and using runtime concepts. Spaces, lenses, frames,
deltas, frontiers, traces, and proofs remain typed evidence and may gain
focused diagnostic projections, but they are not peer top-level resources that
users must manually orchestrate.

The built-in slice implements this behavior:

- `list` reads catalog and result indexes and shows exact candidate/qualified
  graph identities plus scenario progress; it executes no work;
- `test` executes one bounded deterministic graph/scenario pass, retains
  `EXPECTED` to `OBSERVED` typed deltas, and qualifies only a graph revision
  for which every required scenario freshly passes; without an id it selects
  every catalog workload and fails closed on the workload-count bound; its
  human view renders scenario results incrementally in declaration order;
- `run` executes the current fresh qualified built-in graph against one
  admitted UTF-8 input; and
- `status` reads the exact workload, graph, suite, retained deltas,
  qualification, freshness, stop reason, and latest run without repairing it.

The catalog is compiled into Rey. It deliberately exposes only the passing
`rey.fixture.text-normalize` and failing `rey.fixture.text-mismatch`
conformance workloads; it is not an external manifest format or arbitrary
operation loader.

See [Workloads, Compute Graphs, and Scenarios](WORKLOADS.md) and
[ADR 0016](decisions/0016-first-workload-slice.md).

## Implemented Workload CLI

Every workload subcommand accepts `--format auto|table|json`. `auto` chooses a
table on a terminal and JSON when redirected. `--workspace` defaults to `.`;
relative `--state-dir` values resolve below the canonical workspace and an
absolute value selects an explicit separate local boundary.

The `list` table is a portfolio document rather than a flattened relation. Its
portfolio header derives qualification, scenario, run, and inventory totals;
each workload card exposes purpose, journey, passing and evaluated scenario
coverage, evaluation counts, qualification, exact graph identities, retained
test evidence and freshness, and last-run state. ANSI styling is enabled only
for an interactive terminal and is never the sole carrier of meaning. Forced
table output through a pipe remains ANSI-free. The JSON schema is unchanged;
portfolio aggregates are derived from its authoritative per-workload counts.

The `test` table is a diff-first runner document. It declares the selected
execution path, admission mode, comparison stage, and workload scope before
executing scenarios, then renders each scenario as soon as the deterministic
runtime completes it. Plain output keeps passing scenarios compact but always
opens a failing or inconclusive scenario's directed `EXPECTED` to `OBSERVED`
delta. `-v` also renders evidence formats and matching output evidence for
passing scenarios. `-vv` additionally exposes exact workload, graph, scenario
suite, evaluator, scenario, execution, result, and delta identities. A final
portfolio section keeps workload qualification, scenario conformance,
evaluation coverage, delta assessment, and qualification counts separate.
These verbosity flags affect only the human projection; redirected `auto` and
explicit JSON retain the same `rey.workload-test-batch.v1` document.

The structured schemas are `rey.workload-list.v1`,
`rey.workload-status-batch.v1`, `rey.workload-test-batch.v1`, and
`rey.workload-run-result.v1`. Test results contain verified
`rey.scenario-output-delta.v1` documents for equal and different outputs.
Passing tests alone contain a `rey.workload-qualification.v1` binding the
exact workload, graph, scenario suite, evaluator, and test result.

## Implemented Environment CLI

The executable currently exposes:

```text
rey environment inspect [--workspace <path>]
  [--format auto|table|arrow|json]
  [--total-timeout-ms <n>] [--probe-timeout-ms <n>]
  [--max-capture-bytes <n>]

rey environment diff <source-snapshot> <target-snapshot>
  [--source-label <label>] [--target-label <label>]
  [--max-input-bytes <n>] [--max-capabilities <n>]
  [--max-changes <n>]
  [--diff-format structured|tabular-diff|summary]
  [--format json|arrow]

rey environment prove <source-snapshot> <target-snapshot>
  --require-capability <capability-id>...
  [--source-label <label>] [--target-label <label>]
  [--max-input-bytes <n>] [--max-capabilities <n>]
  [--max-changes <n>]
  [--bundle <new-directory>]
  [--max-bundle-artifact-bytes <n>] [--max-bundle-bytes <n>]

rey environment verify <certificate> <source-snapshot> <target-snapshot>
  [--max-input-bytes <n>] [--max-capabilities <n>]

rey environment verify-bundle <bundle-directory>
  [--max-artifact-bytes <n>] [--max-bundle-bytes <n>]
  [--max-capabilities <n>]
```

These commands operate on the standalone capability schema. Help must not imply
that a planned Spoke capability is available. `--format arrow` is valid only
for the structured delta; summary is JSON and Tabular Diff is CSV.

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

Delta commands require a separate representation selector because the semantic
typed delta and its human projection are not identical:

```text
--diff-format structured|tabular-diff|summary
```

- `structured` emits the authoritative bounded delta representation.
- `tabular-diff` emits a Tabular Diff 0.8 relation for compatible frames.
- `summary` emits navigation counts and scores without claiming to contain all
  proof evidence.

The implemented capability change Arrow relation is
`rey.capability-changes.v1`; its frame attributes bind source and target
snapshot ids and labels, comparator identity, and delta id. Tabular Diff uses
`text/csv; charset=utf-8; profile=tabular-diff-0.8`, is portable and ANSI-free,
and is not authoritative input for proof or replay. Generic frame-delta media
types and schemas remain Plan 0001 work.

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

Command tests verify byte-exact stdout, stderr separation, bounded input, and
categorized exit behavior. Implemented certificate commands return `0` for
passed/verified, `2` for failed, `3` for inconclusive, and `4` for stale.
Invalid input and runtime failure return `1`; Clap retains its own argument
parsing exit behavior.

Implemented `workloads list` and `status` return `0` whenever inspection itself
succeeds, even when rows show failing or stale workloads. `workloads test` and
`run` use `0` for qualified/passed, `2` for conclusive semantic failure, `3`
for inconclusive or blocked, `4` for stale, and `1` for invalid input or runtime
failure.

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
percent-encoding, canonicalization, tenancy, and Spoke artifact mapping.

## Workload Declaration

A workload declaration needs stable workload identity and revision; typed
external inputs/outputs; admitted graph operations and effects; provider and
capability requirements; exact scenario suite; claim/comparator/evaluator
revisions; graph-proposal policy; graph/campaign/scenario/run limits;
qualification; and catalog/result retention requirements.

Each immutable graph revision binds typed nodes, ports, dependency edges,
operation contracts, capabilities, effects, limits, and generator provenance.
Each scenario binds fixtures, test providers, selected outputs, expected
observations or claims, comparison rules, completeness, and bounds. The first
graph contract is a finite typed DAG. Exact serialization remains open. See
[Workloads, Compute Graphs, and Scenarios](WORKLOADS.md).

## Space Declaration

A space declaration needs, independent of final YAML/JSON/TOML syntax:

- id, revision, description, and owners;
- allowed environment providers and optional Spoke endpoint discovery;
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

`resolved_location` is provider-specific evidence. A local executable path is
never interpreted as a Spoke Files path, and a Spoke URI is never handed to a
host process as a path.

Known executable discovery may resolve configured paths or `PATH`, inspect
metadata, and invoke a bounded read-only identity command such as `--version`.
It does not run unknown files, shell startup hooks, project scripts, or package
installers. An action must separately name and be admitted against the frozen
capability row.

The runtime supports these provisional selection attitudes:

```text
--environment auto|standalone
--require-capability <capability-id>[,...]
--spoke <url>
```

`auto` may add a configured or safely discovered Spoke provider. `standalone`
disables it. A required capability fails closed if unavailable. Exact flag and
configuration names remain provisional.

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

The implemented library contracts have no direct CLI surface.
`rey.frontier.v2` binds exact workload, graph, scenario-suite, campaign,
space, trace, committed-record, capability, derivation, prioritization,
coverage, and limit inputs. Its canonical `rey.frontier-rows` version `2`
relation is keyed by stable `work_id` and
retains a derived row identity, delta/claim/lens/action citations, readiness,
blockers, priority, and estimated cost.

`rey.frontier-progress.v2` compares compatible source and target frontiers in
that direction while preserving source and target graph identities. Its
`rey.frontier-progress-changes` version `2` relation
reports resolved, introduced, or updated work with source/target row ids;
unchanged work remains a summary count.

`rey.scheduling-decision.v2` rejects stale expected record, frontier, and
capability identities and selects ready work by declared priority descending,
cost ascending, then stable work id. The `rey.scheduled-work` version `2`
relation retains selection rank and exact frontier row identity. These are
deterministic selection contracts, not provider reads, action proposals, an
execution queue, or a recurring scheduler. See
[Frontier, Progress, and Scheduling](FRONTIER.md) and
[ADR 0014](decisions/0014-frontier-progress-and-scheduling.md) and the identity
cutover in [ADR 0016](decisions/0016-first-workload-slice.md).

## Reasoning Surface Contract

Before requesting a policy proposal, the runtime constructs a bounded
delta-directed reasoning surface. The implemented
`rey.reasoning-surface.v3` envelope contains:

- surface schema, identity, and projection-contract revision;
- workload, graph, scenario-suite, campaign, space, and trace identities;
- committed and active transitions, scheduling decision, frontier frame, cited
  frontier rows, and applicable transition/residual delta identities;
- exact retrieved evidence addresses, source bindings, and provider revisions;
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
into a Spoke source, give a cited capability execution authority, or make the
surface the sole copy of native source content.

The reasoning-surface schema is a verified library contract fixed by
[ADR 0013](decisions/0013-runtime-state-and-reasoning-surface-contracts.md) and
cut over to decision-bound v2 by
[ADR 0014](decisions/0014-frontier-progress-and-scheduling.md), then to the
workload-bound v3 envelope by
[ADR 0016](decisions/0016-first-workload-slice.md). It is not an
implemented CLI format. The policy-proposal schema remains a target contract.

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

## Spoke Configuration

The Spoke provider uses an explicit or safely discovered base URL and an
authentication context supplied through configuration and environment
references. Rey discovers and validates advertised capabilities before using
the provider. Spoke absence is a normal capability result unless the selected
space or claim requires it.

The client preserves:

- Spoke resource ids, revisions, versions, checkpoints, and ETags;
- request and problem identifiers;
- query media type and result schema metadata;
- compute run, attempt, executor, fence, event, and capture identities where
  exposed; and
- effective server limits and truncation metadata.

Endpoint changes invalidate cached capability discovery and any binding that
cannot be proven to name the same durable deployment.

## Spoke Read Paths

Read-only materialization may use:

```text
GET/HEAD  exact file, object, document, table, run, and capture resources
QUERY     safe bounded Spoke query surfaces
```

Rey freezes returned revisions before building a frame. A query result without
sufficient revision/checkpoint lineage cannot satisfy a claim that requires a
reproducible snapshot.

## Effect Paths

Effects use the operation owned by the selected provider: an explicitly
authorized local action, a Spoke resource method, or a Spoke compute submission.
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
It records process and capture lineage with explicitly weaker durability than
Spoke compute.

A compute-backed action freezes:

- registered tool and toolset resolution;
- exact argv and declared input artifacts;
- source/frame preconditions;
- effect and egress class;
- environment-name and secret-handle sets;
- limits and backend enforcement requirements; and
- idempotency identity.

Spoke owns process states and captures. Rey observes terminal state, validates
capture completeness and media type, materializes post-action lenses, and then
decides the semantic transition outcome.

## Persistence Paths

The workload surface introduces two abstract provider roles before selecting a
physical persistence design. A catalog provider resolves workload declarations,
immutable graph/scenario assets, and mutable selectors to exact identities. A
result provider retains graph proposals, campaigns, attempts, outputs, typed
deltas, qualification records, runs, and indexes read by `workloads list` and
`status`.

The first standalone implementation uses a compiled-in catalog and a bounded
`rey.local-workload-state.v1` result index at
`${workspace}/.rey/workloads/state.json`, overridable by explicit
`--state-dir`. Reads reject symlinked state files and verify every retained
semantic result. Writes use a same-directory temporary file and rename. This
single-process provider claims no `fsync`, lock, remote durability, or Spoke
semantics. A graph selected for future runs cannot exist solely in a
disposable cache. Connected mode uses
public Spoke resources for stronger durability, query, compute, and lineage
claims. A general manifest encoding, Spoke mapping, and stronger publication
protocol remain undecided; ADR 0016 does not select an engine.

For the implemented capability claim, standalone Rey writes the
[ADR 0011](decisions/0011-local-proof-bundle.md) manifest, snapshots, typed
delta JSON and Arrow, Tabular Diff, and certificate to an explicit local
content-addressed bundle. `prove --bundle` publishes a new bundle or accepts an
identical verified replay; `verify-bundle` bounds and recomputes it without
following symlinked evidence. The final directory name is not exposed until a
same-parent staging directory contains all objects and the manifest. The
manifest, rather than the retention-neutral certificate, states the
filesystem-only guarantees and explicit non-guarantees.

Connected Rey can later publish the same semantic artifacts through public
Spoke resources. Publication is idempotent by content identity and must not
make a certificate visible before its required evidence reaches the claimed
retention boundary.

Git poll cursors are part of this publication boundary. Local mode retains a
local cursor with local-file guarantees. Connected mode may retain activations
and cursors in Spoke, but a cursor never advances merely because a Git poll
returned successfully.

The Spoke resource layout and commit protocol are not yet fixed. Interface work
must coordinate with `docs/PROOFS.md` and a future persistence ADR rather than
projecting the local directory layout onto Spoke.

## Errors And Limits

Structured errors need a stable category, human detail, correlation id, and
actionable remediation. Important categories include invalid declaration,
provider unavailable, capability unavailable, capability drift, source drift,
invalid graph, graph cycle, missing graph policy, scenario mismatch, scenario
inconclusive, unqualified graph, stale qualification, Git history incomplete,
Git ref rewritten, Git index conflicted, cursor replay, stale proposal,
incompatible frame, duplicate key, action rejected, run failed/lost,
observation incomplete, budget exhausted, evidence missing, proof failed,
proof inconclusive, and proof stale.

Errors must report which state changed and which did not. Retrying a read,
proposal, compute submission, artifact publication, or mutation follows that
operation's idempotency contract rather than one generic retry rule.

## No Rey Service Yet

The initial topology is a local CLI/library with built-in and environment
providers plus an optional Spoke client. A long-running Rey service, public HTTP
API, multi-user scheduler, or remote policy gateway needs a later plan and
explicit identity, authorization, durability, and topology decisions.
