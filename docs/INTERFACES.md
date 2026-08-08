# Rey Interfaces

This document sketches Rey's user, environment, policy, and Spoke interfaces.
The implemented surface is limited to `rey environment inspect` and the
capability snapshot/delta/certificate loop fixed by ADRs 0008 and 0010. Other
command names, flags, schemas, media types, and exit codes remain provisional
until an accepted ADR and implementation tests fix them.

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

## Provisional CLI Shape

The target top-level entities are:

```text
rey doctor
rey environment <command>
rey git <command>
rey applications <command>
rey spaces <command>
rey lenses <command>
rey frames <command>
rey diffs <command>
rey runs <command>
rey traces <command>
rey proofs <command>
```

Possible first-slice operations are:

```text
rey environment inspect
rey environment diff <source-snapshot> <target-snapshot>
rey git inspect [<workspace>]
rey git poll <space>
rey git diff <source-snapshot> <target-snapshot>
rey applications describe <application-id>
rey applications run <application-id> [--component <component-id>]
rey frames materialize <space> <lens> [--at <binding>]
rey frames describe <frame-id>
rey diffs compare <source-frame> <target-frame> --keys <column,...>
rey diffs show <delta-id>
rey runs start <space> [--claim <claim-id>] [--max-steps <count>]
rey runs resume <trace-id>
rey traces show <trace-id>
rey proofs evaluate <claim-id>
rey proofs verify <proof-id-or-path>
rey proofs show <proof-id>
```

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

rey environment verify <certificate> <source-snapshot> <target-snapshot>
  [--max-input-bytes <n>] [--max-capabilities <n>]
```

These commands operate on the standalone capability schema. Help must not imply
that a planned Spoke capability is available. `--format arrow` is valid only
for the structured delta; summary is JSON and Tabular Diff is CSV.

## Formats

DataFrame-shaped commands are expected to support:

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
- Interactive progress is disabled or redirected when stdout carries Arrow,
  CSV, JSON, or raw bytes.
- Policy subprocess protocols, if selected, use dedicated framed channels or
  files rather than mixing control messages with artifact stdout.

Command tests verify byte-exact stdout, stderr separation, bounded input, and
categorized exit behavior. Implemented certificate commands return `0` for
passed/verified, `2` for failed, `3` for inconclusive, and `4` for stale.
Invalid input and runtime failure return `1`; Clap retains its own argument
parsing exit behavior.

## Identities

User-facing references may be stable ids, credential-free Rey URIs, or explicit
artifact paths. A mutable display name never substitutes for the exact identity
stored in evidence.

A future URI grammar may cover:

```text
rey+application:<application-id>@<revision>
rey+space:<space-id>@<revision>
rey+lens:<lens-id>@<revision>
rey+frame:<frame-id>
rey+delta:<delta-id>
rey+trace:<trace-id>
rey+proof:<proof-id>
```

This grammar is illustrative, not accepted. It must be decided alongside
percent-encoding, canonicalization, tenancy, and Spoke artifact mapping.

## Application Declaration

An application declaration needs:

- stable application id and revision;
- provider/profile configuration;
- referenced spaces and lens revisions;
- independently activatable component ids/revisions and dependency edges;
- trigger declarations and manual/policy entry points;
- admissible action and claim revisions;
- policy configuration;
- application and component budgets/concurrency; and
- trace, cursor, evidence, and retention policy.

Each component names required capabilities, input lenses/frames, output
observations, evaluated claims, allowed actions, and component-local bounds.

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
- trigger declarations and target component revisions;
- activation concurrency and budgets; and
- cursor/evidence retention profile.

The poll result contains source and target snapshot ids, ref/index/worktree
deltas, history completeness, matched triggers, activation ids, transition
outcomes, and the next cursor. The next cursor is publishable only after the
required activation evidence reaches its declared retention boundary.

A trigger declaration includes a stable id/revision, source event classes,
ref/path/stage predicates, required Git capabilities/completeness, target
application components, coalescing policy, budgets, and replay/idempotency
behavior. Trigger output is an activation proposal and passes normal runtime
admission.

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

## Policy Contract

A policy request is a bounded snapshot containing:

- space and trace revision;
- frontier frame identity and a bounded projection of relevant rows;
- admissible action definitions and schemas;
- exact precondition frame and source ids;
- remaining time, iteration, action, and evidence budgets;
- prior rejection or failure facts relevant to the next choice; and
- a correlation id.

A proposal contains:

- selected action id and revision;
- typed arguments;
- cited frontier row and evidence ids;
- expected information or state change;
- requested sub-budgets; and
- the request correlation id and precondition identities.

The runtime rejects unknown actions, stale preconditions, malformed arguments,
unauthorized effects, unsupported limits, or exhausted budgets before an
effect. Free-form rationale is optional evidence and is never executable input.

Provider-specific chat, prompt, or tool-call envelopes stay behind policy
adapters and do not become Rey's durable action contract.

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

Standalone Rey writes manifests, frames, deltas, traces, and proofs to an
explicit local content-addressed bundle. Connected Rey can publish them through
public Spoke resources. Publication is idempotent by content identity and must
not make a certificate visible before its required evidence reaches the
retention boundary claimed by the certificate.

Git poll cursors are part of this publication boundary. Local mode retains a
local cursor with local-file guarantees. Connected mode may retain activations
and cursors in Spoke, but a cursor never advances merely because a Git poll
returned successfully.

The initial local bundle layout, Spoke resource layout, and commit protocols are
not yet fixed. Interface work must coordinate with `docs/PROOFS.md` and an
accepted persistence ADR.

## Errors And Limits

Structured errors need a stable category, human detail, correlation id, and
actionable remediation. Important categories include invalid declaration,
provider unavailable, capability unavailable, capability drift, source drift,
Git history incomplete, Git ref rewritten, Git index conflicted, cursor replay,
stale proposal, incompatible frame, duplicate key, action rejected, run
failed/lost, observation incomplete, budget exhausted, evidence missing, proof
failed, proof inconclusive, and proof stale.

Errors must report which state changed and which did not. Retrying a read,
proposal, compute submission, artifact publication, or mutation follows that
operation's idempotency contract rather than one generic retry rule.

## No Rey Service Yet

The initial topology is a local CLI/library with built-in and environment
providers plus an optional Spoke client. A long-running Rey service, public HTTP
API, multi-user scheduler, or remote policy gateway needs a later plan and
explicit identity, authorization, durability, and topology decisions.
