# Environment And Capabilities

This document defines Rey's target environment-discovery and capability
contracts. Environment awareness lets Rey use the best context surfaces
available without making Spoke or any host tool an invisible boot dependency.
ADR 0008 and the first Plan 0001 slice now implement the version-1 capability
relation for built-in frames, one explicit workspace, allowlisted `git` and
`rg` identity probes, and a contained Git repository observation. Optional
Spoke discovery, action admission, and other tool adapters remain Plan 0001
work. ADRs 0010 and 0011 add capability deltas, required-capability
certificates, and bounded local-only bundle retention over this relation. ADR
0017 classifies relational and source mining operations as first-class
capabilities. Plan 0006 now implements the first deterministic built-in local
source binding and literal-search capability and ADR 0018 composes it through
the workload CLI; external `rg`, parser, index, and Spoke mining adapters remain
later slices. ADR 0020 added the first explicit environment graph, ADR 0027
added bounded non-sensitive value capture and one operator delta for the CLI
and UI, and ADR 0031 hard-cuts the current mapping contract to
`rey.env-map.v3` with separate desired-application and search records. The
Git-shaped environment history revisions those observations.

## Terms

- A **context surface** is a bounded source of information or action in the
  current environment: a workspace, version-control repository, executable,
  runtime, service, or Spoke deployment.
- A **provider** owns discovery and operations for one class of context surface.
- A **capability** is one typed operation or guarantee advertised by a provider.
- A **mining capability** is a versioned relational or source operation such as
  retrieve, search, parse, index, traverse, group, compare, or visualize.
- A **capability snapshot** is the frozen typed relation of capabilities
  available to one Rey transition.
- A **profile** describes provider selection and required guarantee policy; it
  does not select a different Rey runtime.

## Principles

- Discovery is bounded observation, not ambient authority.
- Spoke is an optional provider and a first-class amplifier.
- Local fallback is useful but cannot impersonate Spoke semantics.
- Provider and tool drift are runtime deltas, not hidden machine state.
- A missing capability removes actions or makes dependent claims inconclusive;
  it never silently weakens the claim.
- Exact capability snapshots participate in action and proof identity.
- Mining discovery names semantic operations and limitations; a tool name or
  server version alone never implies search, parse, index, or query parity.

## Provider Contract

Every provider declares:

- stable id, version, and implementation digest;
- provider kind and configuration schema;
- allowed discovery roots, names, paths, or endpoints;
- discovery operations, timeouts, output limits, and error behavior;
- trust classification and authenticated identity where applicable;
- read, probe, mutation, execution, persistence, and query capabilities;
- resource controls it enforces, observes, or does not support;
- source identity and content-digest semantics;
- mining operation ids/revisions, accepted input/output artifact kinds,
  completeness semantics, and enforceable traversal/result limits;
- health and observation currency; and
- which provider changes invalidate existing actions or evidence.

Providers return structured observations. They do not write directly into a
global registry or execute policy-selected work during discovery.

## Capability Relation

The first logical relation should include:

```text
provider_id
provider_revision
provider_kind
capability_id
capability_kind
resolved_location
version
content_digest
provenance
availability
trust_class
operations
enforced_limits
unsupported_limits
observed_at
error_code
error_detail
```

ADR 0008 fixes `rey.capabilities` version `1`. Scalar fields are Polars string,
unsigned integer, or nullable string columns. Array-valued logical fields use
canonical compact JSON arrays in string columns for this schema version. A
future Arrow list/struct representation requires a schema revision. The schema
distinguishes unknown, unavailable, and unsupported values. An absent
enforcement claim is never encoded as an enforced zero or unbounded
permission.

Locations are provider-scoped. A local path, Spoke path, URL, object URI, and
logical tool name are not interchangeable strings.

## Environment Mapping Graph

The conventional `rey.env.yaml`, or an explicit workspace-relative `--map`
path, declares the local environment surfaces a programmer or agent has judged
relevant. The closed `rey.env-map.v3` schema contains:

- variable nodes with exact names, sensitivity, and `presence`, `digest`, or
  bounded UTF-8 `value` capture;
- workspace-relative regular-file nodes with a required-admission marker;
- desired executable nodes with a required purpose, resolved from the captured
  search path, and declared potential capabilities; and
- exact directed edges naming the declared relationship between nodes.

The loader bounds document bytes, strings, nodes, edges, projection rows,
individual variable values, individual file bytes, total file bytes, and
executable bytes. It rejects unknown fields, duplicate ids or edges, missing
endpoints, self-edges, path escape, symlinked mapping/input paths, invalid
UTF-8 value capture, and sensitive digest or value capture. Node and edge order
is canonicalized before graph identity is computed.

Observation never retains mapped file bytes. A sensitive variable records
presence only. A non-sensitive variable may retain presence, a
domain-separated digest, or its exact bounded UTF-8 value when the mapping
author explicitly selects `capture: value`. A file records its
workspace-relative path, regular status, length, and bounded digest. An
canonical executable-declaration subset has its own desired-application
inventory identity. An executable records its purpose, resolved path, length,
digest, and bounded search-path
count without invocation in a separate capability-snapshot search record. Its
potential capabilities remain explicitly `unadmitted` until a separate adapter
freezes operation semantics, arguments, effects, trust, and limits.

The provider projects one graph row plus exact node and edge rows into the
ordinary capability snapshot. `env status` derives a typed operator projection
over variables, applications, inputs, and references across `HEAD`, `INDEX`,
and `WORKING`. Its human view leads with the `HEAD → WORKING` env-shaped text
diff, presents the exact desired inventory, then presents the full bounded
application search record; structured output retains the complete capability
evidence and both authoritative deltas. `env diff`, `env add`,
`env commit`, and `env log -p` navigate and revision the same relation. The
YAML graph is a proposal about relevance, not execution authority or proof of a
dependency.

## Discovery Lifecycle

1. Resolve allowed provider configuration without executing project hooks.
2. Run built-in discovery under total time, result-row, and output-byte limits.
3. Resolve known local tools from explicit paths or configured search paths.
4. Run only provider-declared, read-only identity probes with individual
   timeout and capture limits.
5. Probe configured remote providers such as Spoke through their public health
   and capability contracts.
6. Normalize results into a capability frame and content identity.
7. Freeze the snapshot used for lens materialization and action admission.
8. Re-probe only at an explicit transition boundary and compute a directed
   capability delta.

Partial discovery remains visible. One failed provider does not erase healthy
providers unless the selected profile requires all of them.

## Built-In And Local Providers

The minimum standalone profile may provide:

- bounded access to one explicitly selected workspace root;
- Git repository, commit/ref/index, and bounded worktree observations when the
  workspace is a supported repository;
- file metadata and content hashing;
- built-in frame, delta, and proof operations; and
- local content-addressed evidence-bundle output.

Known-tool providers may add version control, text search, language toolchains,
compilers, formatters, linters, test runners, build tools, and language servers.
Support is adapter-specific. Rey does not infer safe semantics for an arbitrary
executable from its name.

An executable capability records its resolved path, version, digest or package
provenance when available, platform, trust class, supported operations, and
limits. A path found on `PATH` is a discovery candidate; it becomes an
admissible capability only after its provider contract validates it.

## Mining Providers

Mining capabilities use the same discovery and admission boundary as every
other provider operation. A capability row may eventually advertise operation
contracts such as:

```text
relation.retrieve · relation.group · relation.traverse
source.retrieve · source.search · source.segment
source.parse · source.index · source.measure
delta.relational · delta.text · delta.structural
visualize.table · visualize.patch · visualize.tree · visualize.graph
```

Most names remain architectural vocabulary. The implemented baseline
advertises exact capability `source.search.literal-utf8`, operation
`rey.source-search.literal-utf8`, corpus schema `rey.source-corpus.v1`, and
match relation `rey.source-matches` version `1`.

An adapter records its accepted source kinds, output artifact/schema kinds,
canonical parameters, encoding/language support, completeness behavior,
determinism, provider/tool/parser identity, and effective file, row, match,
node, edge, depth, byte, time, and memory limits. A generic `rg` identity probe
does not yet advertise an admitted `source.search` operation. A parser that can
recover a partial tree must not advertise complete syntax or semantic-index
coverage.

Provider ownership remains explicit:

- local filesystem and Git adapters establish local source identity and safe
  reads;
- tool adapters invoke and interpret allowlisted executables;
- language adapters own parser and semantic-index interpretation;
- Spoke owns its durable source, composed query, registered-tool, run, capture,
  and lineage semantics; and
- Rey binds those capabilities into mining requests, workload nodes, deltas,
  invalidation, and reasoning surfaces.

Exact immutable retrieval may be allowed as a read-only orientation operation.
Reading mutable state or invoking an external miner is a probe and requires
normal action admission. Pure projection over already frozen evidence needs no
new source authority but still binds its operation revision and limits.

### Built-In Local Source Baseline

The standalone snapshot advertises a compiled deterministic literal-search
baseline separately from the generic `rg` identity probe. Callers explicitly
select regular files beneath one canonical root. Binding rejects absolute,
parent, empty, duplicate, non-regular, escaping, and symlinked paths; applies
file, total-byte, line, path, and file-count limits; reads twice to reject drift
during binding; classifies UTF-8, binary, and invalid UTF-8 bytes; and retains
native frozen bytes beside a credential-free corpus manifest.

The search operation accepts an exact corpus artifact, non-empty
case-sensitive UTF-8 literal, and before/after context-line counts. It uses
non-overlapping byte matches in reversible path then start-byte order. Match
rows retain source, pattern, context, request, result, provider, capability,
and implementation identity. Pre- and post-search source checks prevent one
result from combining file revisions. The provider exposes complete, partial,
truncated, unsupported, and failed outcomes rather than silently omitting
binary, invalid, over-limit, malformed, or changed sources.

This provider trusts the local process and is not a filesystem sandbox. It
does not apply ignore files, generated-file policy, globs, regular expressions,
case folding, arbitrary directory traversal, or `rg` semantics. Those
capabilities require their own exact operation revision and parity fixtures.

`rey.fixture.source-search` is the first admitted consumer. Scenario tests bind
the checked-in corpus; `workloads run --source <relative-path>...` binds
caller-selected files below the canonical `--workspace`. The workload result
retains the exact corpus, request, capability snapshot, provider, result,
match, and context identities. `list`, `test -v/-vv`, `status`, and `run`
surface those facts without adding a separate mining command hierarchy.

## Spoke Provider

The Spoke provider may contribute:

- exact versioned files and objects;
- document, stream, and table relations;
- composed relational, graph, lexical, and vector query;
- registered tool identity and admitted compute;
- run, attempt, event, capture, fence, and cancellation lineage; and
- durable artifact and trace retention.

Rey uses advertised, proven capabilities rather than inferring support from a
server version string. A reachable health endpoint does not imply every Spoke
capability required by a space.

Spoke discovery and operations use public contracts. Local provider paths never
become Spoke paths, and Rey never opens the Spoke data directory.

## Profiles And Requirements

The initial semantic profiles are:

- **standalone** — disable Spoke discovery and use allowed built-in/local
  providers;
- **auto** — use built-in/local providers and add a safely configured or
  discovered Spoke provider when healthy; and
- **required capabilities** — declare exact capabilities/guarantees a space,
  lens, action, or claim needs regardless of profile.

Profiles select availability, not proof meaning. A claim requiring durable
Spoke revisions remains unavailable or inconclusive in standalone mode. A local
claim remains local even if Spoke happens to appear unless its declaration
selects Spoke evidence.

## Action Admission

An action proposal names capability ids and the snapshot against which it was
created. Admission:

- verifies the snapshot is current enough for the action;
- resolves each capability to the same provider, version, location, digest, and
  trust class;
- validates effect class, target boundary, arguments, environment names,
  limits, and required enforcement;
- rejects unsupported or changed capabilities before side effects; and
- records the selected provider and realized execution lineage.

Discovery and admission stay separate. A policy cannot transform an observed
tool into an effect merely by citing it.

## Capability Deltas

Capability snapshots use the same diff-directed model as other frames.
Meaningful changes include:

- provider or tool appeared/disappeared;
- health or authentication changed;
- version, path, digest, provenance, platform, or trust changed;
- operation support appeared/disappeared;
- an enforced or unsupported limit changed; and
- a Spoke schema or capability revision changed.

Dependency metadata maps those deltas to affected lenses, actions, and proofs.
Unrelated local frames do not become stale merely because an unused tool
changed.

A mining result becomes stale only when a capability it actually used changes:
source/provider identity, operation or implementation revision, parser/tool
version, trust, supported semantics, or an effective limit that affects the
result. Discovery of an unrelated richer miner does not invalidate existing
evidence unless the workload contract selected it as a required input.

## Local Environment Revisions

ADRs 0019 through 0021 and ADR 0027 implement the Git-shaped interaction over
capability snapshots. `rey env status` observes the explicit workspace and presents three
planes: committed `HEAD`, the admission `INDEX`, and fresh `WORKING` evidence.
Before the first commit, HEAD and the effective index are typed empty
capability relations. Without a retained index, the effective index equals
HEAD. The command reads but never creates or repairs local state. Explicit JSON
emits `rey.environment-status.v4` with the complete working snapshot, both
authoritative capability deltas, and
`rey.environment-operator-projection.v2`. Every mapped object carries exact
HEAD/index/working observations plus staged, unstaged, and overall change
classification.

`rey env add` retains the exact working snapshot as a HEAD-bound
`rey.environment-admission-index.v1`. `add -p` prompts over the canonical
`INDEX → WORKING` capability changes and applies only selected rows. File bytes
never enter the selection interface; an explicitly value-captured variable is
part of the retained capability observation. Staging a
mapped executable accepts its observation for history but grants no execution
or provider authority.

`rey env diff` repeats the fresh bounded observation and selects the shared
operator projection for `INDEX → WORKING`; `--staged` selects `HEAD → INDEX`.
Human output is one compact delta coordinate followed by exactly three
environment-native evidence planes: directed variable text, bounded
application search, and input/reference topology. The bounded-search plane
shows the exact target application-declaration identity as `DESIRED INVENTORY`
before the target capability snapshot as `SEARCH RECORD`. Unchanged mapped objects remain bounded context,
while insertions, deletions, and modifications use the selected source and
target observations. The header preserves the authoritative
capability-delta assessment and retained change count, including changes that
do not project into a mapped human object. The command accepts no loose
snapshot-file operands. Explicit JSON is `rey.environment-diff.v3` with
the complete typed capability delta.

`rey env commit -m <message>` performs no discovery. It appends the exact
verified admission-index snapshot, then clears the index after history
publication. The commit id binds a monotonic local sequence, exact parent
commit, canonical message, and snapshot id. It deliberately excludes ambient
author and wall-clock values. Incomplete snapshots can be committed as explicit
degradation evidence; they do not become complete through retention.

`rey env log` verifies the entire retained chain, selects commits newest first
under an explicit count bound, and recomputes every selected parent-to-commit
capability delta. Its human chronology keeps the semantic commit id, revision,
delta assessment, authoritative change count, environment scope, changed
dimensions, mapping, and message visible without reopening provider records.
`-p` expands each selected transition, including `EMPTY → ENV@1`, through
directed variable text, bounded application search, and input/reference
topology derived only from the retained parent and commit snapshots. It
performs no fresh observation. Explicit JSON remains
`rey.environment-log.v1` with the complete commits, snapshots, and typed
capability deltas.

The default `rey.local-environment-history.v1` state lives at
`${workspace}/.rey/env/state.json`; the separate admission index lives at
`${workspace}/.rey/env/index.json`. Both are bounded single-process local state
using same-directory temporary publication and rename. Reads verify snapshot,
commit, parent, sequence, chain/index identity, exact index base, and safe file
boundaries. History publication and index removal are not one crash-atomic
transaction; an index left after a successful commit is stale and rejected. It
is not a Git object database and claims no pathspec, reset/restore, branches,
merges, rewrite, `fsync`, locking, authenticated writer, remote durability, or
Spoke revision semantics.

Retained environment revisions are also portfolio-mining inputs. Read-only
`workloads list` and `status` consume the exact admission-index snapshot when
present, otherwise committed HEAD; they never perform a fresh environment
probe. The first `rey.portfolio.attention` slice treats admitted mapping nodes
of kind `input_file` as context surfaces. A mapped file with no declared
workload owner yields a visible `CREATE` row. Environment admission remains
evidence acceptance only: it neither creates that workload nor grants mapped
executables authority.

## Standalone Evidence

Standalone mode can write a bounded content-addressed evidence bundle to an
explicit local directory. It records source identities, capability snapshots,
tool/process observations, artifacts, and certificate digests.

The bundle does not claim:

- Spoke resource revisions or query checkpoints;
- fenced or leased compute attempts;
- multi-process transactionality;
- remote durability or retention;
- authenticated multi-user identity; or
- stronger process isolation than the local executor actually proves.

Those omissions are capability facts, not reasons to make standalone Rey
useless.

Git has additional repository snapshot, semantic index, poll cursor, and
workload-activation semantics described in [Git Context and
Activation](GIT.md). Discovering a `git` executable is not the same as
admitting a Git repository provider or enabling network and mutation commands.

## Security And Bounds

- Discovery never sources shell profiles or project environment hooks.
- Workspace traversal remains beneath explicit canonical roots and rejects
  escapes according to the provider contract.
- Version probes use direct argv, a cleared or allowlisted environment, fixed
  cwd, concurrent capture draining, and time/output limits.
- Network provider discovery uses explicit destinations and does not scan
  networks or credential stores.
- Secrets remain handles or environment references and never enter capability
  frames.
- Local execution is trusted local process supervision, not a sandbox, until a
  backend proves otherwise.

## Required Fixtures

The first environment implementation must cover:

- zero-Spoke startup;
- standalone, auto, and required-capability selection;
- explicit workspace-root bounds and path escape rejection;
- known tool present, missing, duplicated, and permission denied;
- malformed, oversized, timed-out, and non-zero version probes;
- path, version, digest/provenance, trust, and operation drift;
- mining operation appearance/disappearance, semantic-version drift,
  unsupported encoding/language behavior, result truncation, and parser/index
  completeness degradation;
- Spoke absent, unhealthy, partially capable, and healthy;
- optional-provider failure isolated from healthy local providers;
- required-provider failure before any side effect;
- deterministic capability frames and deltas for identical observations; and
- supported Git provider discovery without running repository hooks or
  changing the index.
