# Environment And Capabilities

This document defines Rey's target environment-discovery and capability
contracts. Environment awareness lets Rey use the best context surfaces
available without making any host tool an invisible boot dependency.
The implemented version-1 capability relation covers the process-owned seed
set, a grouped desired-application inventory flattened into bounded executable
search records, and explicitly supplied reasoning-map observations.
Intrinsic Rey operations and the workspace binding are not environment
entries: runtime capabilities are frozen separately, while the workspace is
request and source lineage. Git repository state likewise belongs to the Git
cadence/activation provider. External `rg`, parser, index, and other action
adapters remain later work.

The current `rey.env-map.v2` keeps desired-application declarations separate
from bounded search records and permits bounded non-sensitive value capture.
Bootstrap never loads a conventional map: the process owns only `HOME`, `PWD`,
and `PATH`, while maps are explicit agent-generatable reasoning resources. The
Git-shaped environment history revisions those observations.

## Terms

- A **context surface** is a bounded source of information or action in the
  current environment: a workspace, version-control repository, executable,
  runtime, or service.
- A **provider** owns discovery and operations for one class of context surface.
- A **capability** is one typed operation or guarantee advertised by a provider.
- A **mining capability** is a versioned relational or source operation such as
  retrieve, search, parse, index, traverse, group, compare, or visualize.
- A **capability snapshot** is a frozen typed relation of capabilities bound to
  one Rey transition. Environment and intrinsic-runtime snapshots have
  separate identities and admission semantics.
- A **profile** describes provider selection and required guarantee policy; it
  does not select a different Rey runtime.

## Principles

- Discovery is bounded observation, not ambient authority.
- The implemented profile uses explicit local providers.
- A provider cannot claim guarantees it does not implement.
- Provider and tool drift are runtime deltas, not hidden machine state.
- A missing capability removes actions or makes dependent claims inconclusive;
  it never silently weakens the claim.
- Exact capability snapshots participate in action and proof identity.
- Mining discovery names semantic operations and limitations; a tool name or
  server version alone never implies search, parse, index, or query parity.
- Bootstrap discovery observes only `HOME`, `PWD`, and `PATH`; it never infers
  project or service relevance from an ambient variable name.

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

`rey.capabilities` version `1` uses Polars string,
unsigned integer, or nullable string columns. Array-valued logical fields use
canonical compact JSON arrays in string columns for this schema version. A
future Arrow list/struct representation requires a schema revision. The schema
distinguishes unknown, unavailable, and unsupported values. An absent
enforcement claim is never encoded as an enforced zero or unbounded
permission.

Locations are provider-scoped. A local path, URL, object URI, and
logical tool name are not interchangeable strings.

## Process-Owned Discovery Seeds

The standalone process begins with exactly three environment-variable seeds:

```text
HOME · PWD · PATH
```

Their bounded UTF-8 values, availability, value digests, and capture errors
are typed `environment_seed` capability rows under
`rey.discovery-seeds.v1`. This fixed set is compiled into Rey. Discovery does
not load a project configuration file, enumerate arbitrary variables, source
shell profiles, or recursively scan any seed path.

The current compiled desired-application inventory classifies each declaration
under one or more canonical groups: `communications`, `agents`, `retrieval`,
and `code`. The groups are many-to-many declaration metadata—`gh` participates
in communications and code, while `grep` and `rg` participate in retrieval.
Every compiled application name exactly matches its searched executable.
Discovery flattens those declarations by application identity before PATH
resolution, so one application is searched and observed exactly once even when
it appears in multiple groups. Each search record carries complete
`rey.discovery-application.v2` provenance. PATH resolution records executable presence for
agent runtimes without starting them; fixed bounded identity probes remain
limited to the non-interactive `git` and `rg` adapters. Discovery does not turn
a found application into assignment or execution authority.
`/environment` is the human owner of this desired/search evidence; higher-order
views may consume exact capabilities but do not repeat the executable inventory.

The provider-specific GitHub mailbox path preserves this boundary. A committed
environment HEAD may supply the exact available
`comms.application.github.identity` executable evidence, but only a separately
committed Channel application declaration admits bounded read-only `gh api`
execution. `rey channels poll` verifies one explicit tick and the foreground
`rey agent` inbox worker repeats that exact path at the committed cadence.
Credential environment names are declared in Channel HEAD and their values are
never retained. Discovery itself does not authenticate, contact GitHub, poll,
or create mail.

Environment discovery retains the `git` executable identity but does not run
repository inspection or add `git.repository.inspect` to the environment
snapshot. HEAD, refs, semantic index entries, and commit reachability belong to
the separate Git cadence/activation provider. Moving Git HEAD or staging files
therefore leaves the environment snapshot unchanged unless a declared
environment input, variable, or application observation also changed.

Endpoint and token variables are not discovery seeds. An agent may propose
explicit non-sensitive references in a reasoning map when frozen project
evidence supports their relevance; secret values remain presence-only.

## Agent-Generated Reasoning Map

An explicit workspace-relative `--map` resource declares environment surfaces
an agent, programmer, or deterministic rule has judged relevant after
discovery. A file named `rey.env.yaml` has no conventional meaning and is not
loaded unless the caller names it. The closed `rey.env-map.v2` schema contains:

- variable nodes with exact names, sensitivity, and `presence`, `digest`, or
  bounded UTF-8 `value` capture;
- workspace-relative regular-file nodes with a required-admission marker;
- desired executable nodes with zero or more canonical group identifiers, a
  required purpose, resolution from the captured search path, and declared
  potential capabilities; and
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
workspace-relative path, regular status, length, and bounded digest. A
canonical executable-declaration subset has its own desired-application
inventory identity under `rey.environment-application-inventory.v2`. An
executable's normalized groups participate in that
identity alongside its purpose and potential capabilities. It records its
resolved path, length, digest, and bounded search-path
count without invocation in a separate capability-snapshot search record. Its
potential capabilities remain explicitly `unadmitted` until a separate adapter
freezes operation semantics, arguments, effects, trust, and limits.

The provider projects one graph row plus exact node and edge rows into the
ordinary capability snapshot. `env status` derives a typed operator projection
over variables, applications, inputs, and references across `HEAD`, `INDEX`,
and `WORKING`. Its human view summarizes staged and unstaged environment-native
objects plus application-search health; `env diff` presents the exact directed
text, desired inventory, bounded search record, inputs, and topology.
Structured output retains the complete capability evidence and both
authoritative deltas. `env diff`, `env add`, `env reset`, `env commit`, and
`env log -p` navigate and revision the same relation. The
YAML graph is a generated or authored proposal about relevance, not bootstrap
configuration, execution authority, or proof of a dependency.

## Discovery Lifecycle

1. **Discovery:** capture the process-owned `HOME`, `PWD`, and `PATH` seed set
   plus declared application and reasoning-map observations under total time,
   row, and byte limits. The workspace scopes the observation but is not
   emitted as an environment capability.
2. **Reasoning over discovery:** present the frozen record to policy. An agent
   may generate a bounded `rey.env-map.v2` resource; Rey parses it only when
   explicitly supplied and never accepts it as action authority.
3. **Survey:** resolve admitted locators to exact source anchors with explicit
   provider, revision, limit, completeness, and error evidence. See
   [Locators](LOCATORS.md).
4. **Process:** incrementally consume survey artifacts and independent cadence
   ticks, derive deltas and attention, then repeat from a transition boundary.

Partial discovery remains visible. One failed provider does not erase healthy
providers unless the selected profile requires all of them.

## Built-In And Local Providers

The minimum standalone runtime may provide:

- bounded access to one explicitly selected workspace root as request lineage;
- Git repository, commit/ref/index, and bounded worktree observations when the
  workspace is a supported repository;
- file metadata and content hashing;
- intrinsic frame, mining, delta, and proof operations in a runtime snapshot;
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

External mining capabilities use the same environment discovery and admission
boundary as other provider operations. Intrinsic deterministic mining belongs
to the separately identified runtime snapshot and requires no environment
admission. Either relation may advertise operation contracts such as:

```text
relation.retrieve · relation.group · relation.traverse
source.retrieve · source.search · source.segment
source.parse · source.index · source.measure
delta.relational · delta.text · delta.structural
visualize.table · visualize.patch · visualize.tree · visualize.graph
```

Most names remain architectural vocabulary. The implemented runtime baseline
advertises exact capability `source.search.literal-utf8`, operation
`rey.source-search.literal-utf8`, corpus schema `rey.source-corpus.v1`, and
match relation `rey.source-matches` version `1`. This record is absent from
`rey env status`, `add`, `reset`, `diff`, and `commit`.

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
- language adapters own parser and semantic-index interpretation; and
- Rey binds those capabilities into mining requests, workload nodes, deltas,
  invalidation, and reasoning surfaces.

Exact immutable retrieval may be allowed as a read-only orientation operation.
Reading mutable state or invoking an external miner is a probe and requires
normal action admission. Pure projection over already frozen evidence needs no
new source authority but still binds its operation revision and limits.

### Built-In Local Source Baseline

The standalone runtime snapshot advertises a compiled deterministic
literal-search baseline separately from the environment's generic `rg`
identity probe. Callers explicitly
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
does not apply source-search ignore files, generated-file policy, regular
expressions, case folding, arbitrary directory traversal, or `rg` semantics.
The workspace-level `.reyignore` scope contract is separate: before an
environment capability snapshot becomes WORKING, it may omit typed
`environment variable`, `application`, `input`, and `reference` observations
using bounded case-sensitive `*`/`?` patterns. Rey retains a synthetic
`rey.ignore.environment` capability containing the relevant rules, exact file
digest, and match counts, so the filtered snapshot cannot claim to be naturally
complete. HEAD and INDEX retain the exact policy that shaped their snapshot.

`rey.fixture.source-search` is the first admitted consumer. Scenario tests bind
the checked-in corpus; `workloads run --source <relative-path>...` binds
caller-selected files below the canonical `--workspace`. The workload result
retains the exact corpus, request, capability snapshot, provider, result,
match, and context identities. `list`, `test -v/-vv`, `status`, and `run`
surface those facts without adding a separate mining command hierarchy.

## Profiles And Requirements

The initial semantic profiles are:

- **standalone** — use the intrinsic runtime plus allowed local providers; and
- **required capabilities** — declare exact capabilities/guarantees a space,
  lens, action, or claim needs regardless of profile.

Profiles select availability, not proof meaning. A claim requiring an
unavailable guarantee remains unavailable or inconclusive; it never silently
weakens itself.

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
- a provider schema or capability revision changed.

Dependency metadata maps those deltas to affected lenses, actions, and proofs.
Unrelated local frames do not become stale merely because an unused tool
changed.

A mining result becomes stale only when a capability it actually used changes:
source/provider identity, operation or implementation revision, parser/tool
version, trust, supported semantics, or an effective limit that affects the
result. Discovery of an unrelated richer miner does not invalidate existing
evidence unless the workload contract selected it as a required input.

## Local Environment Revisions

The Git-shaped interaction operates over capability snapshots. `rey env
status` observes the explicit
workspace and derives three planes: committed `HEAD`, the admission `INDEX`,
and fresh `WORKING` evidence.
Before the first commit, HEAD and the effective index are typed empty
capability relations. Without a retained index, the effective index equals
HEAD. The command reads but never creates or repairs local state. Explicit JSON
emits `rey.environment-status.v2` with the complete working snapshot, both
authoritative capability deltas, and
`rey.environment-operator-projection.v2`. Every process seed and explicitly
mapped object carries exact
HEAD/index/working observations plus staged, unstaged, and overall change
classification. Its default human projection is a compact working-tree view:
current `ENV@n`, then separate environment-native “changes to be committed” and
“changes not staged” groups when either exists. A clean view contains only the
environment coordinate and clean result. Human status and diff application
entries are limited to found executables and previously found executables that
disappeared; unsuccessful searches remain in structured evidence. Workspace, working-state,
observation-health, application-search, and reasoning-map summaries remain in
the structured status evidence rather than padding the default terminal view.
The human view directs exact review to `env diff` and `env diff --staged`
instead of repeating the full directed evidence. Authoritative capability
changes with no mapped operator object remain visible as individually named
semantic entries with exact capability ids.

`rey env add` requires an explicit admission scope. `add .` and `add -A`
retain the complete working snapshot as a HEAD-bound
`rey.environment-admission-index.v1`; one or more canonical environment paths
or exact mapped-input paths retain only matching `INDEX → WORKING` capability
changes. Directory pathspecs select bounded descendants, and every operand
must match or the command leaves INDEX unchanged. `add -p` walks every
unstaged hunk interactively; optional paths narrow the canonical capability
changes it prompts over. Every
prompt renders an environment-native `diff --rey` hunk for a variable,
application, input, or reference when possible, with an exact capability
fallback. The fallback names changed semantic fields but omits raw structured
provenance and directs exact inspection to JSON. File bytes never enter the selection interface; an explicitly
value-captured variable is part of the retained capability observation. Staging a
mapped executable accepts its observation for history but grants no execution
or provider authority.

`rey env diff` repeats the fresh bounded observation and selects the shared
operator projection for `INDEX → WORKING`; `--staged` selects `HEAD → INDEX`.
Human output consists of exactly two un-numbered environment-native sections:
environment variables and applications. The application section uses the same neutral context and
red `-` / green `+` before-and-after rows as variables while preserving each
found executable's name, resolved path, and comma-separated groups. Explicit
losses render only the red prior observation; unsuccessful searches remain in
JSON. Input and reference topology likewise remain in the structured snapshots
and typed capability delta instead of appearing as a third human plane.
Unchanged mapped objects remain bounded context,
while insertions, deletions, and modifications use the selected source and
target observations. The authoritative capability-delta assessment and retained
change count, source and target coordinates, and changes that do not project into
a mapped human object remain in structured output. The command accepts no loose snapshot-file operands.
Explicit JSON is `rey.environment-diff.v1` with the complete typed capability
delta.

`rey env commit -m <message>` performs no discovery. It appends the exact
verified admission-index snapshot, then clears the index after history
publication. Successful default/table execution writes nothing to stdout or
stderr: no news is good news. `--format json` emits the structured commit
receipt when automation needs one, while `rey env log -n 1` is the human
readback surface. Failures remain nonzero diagnostics on stderr. A new
`rey.environment-commit.v1` id binds a monotonic local
sequence, exact parent commit, integer Unix commit time, canonical message, and
snapshot id. The time records when Rey retained the observation; it is not a
trusted causal clock, discovery timestamp, or author identity. Existing v1
commits remain verifiable without a date. Incomplete snapshots can be committed
as explicit degradation evidence; they do not become complete through
retention.

`rey env reset [<target>]` performs a mixed reset over Rey-owned admission
state. `HEAD`, the default, leaves the retained history at its current head and
clears the admission index. `ENV@n` or an
exact full environment commit id moves HEAD to that retained commit, removes
all later entries from the linear log, and clears the index. `EMPTY` removes
the entire retained log. The command accepts no abbreviated ids, ancestry
operators, ranges, or pathspecs. Before publishing that state transition it
performs the same bounded process-owned and optional explicit-map observation
as status, then derives the post-reset HEAD-to-WORKING delta with no retained
index. Human output follows Git: clean reset is silent; otherwise it prints
`Unstaged changes after reset:` and compact `A`, `M`, or `D` object rows. The
`rey.environment-reset-result.v1` JSON receipt retains the source and target
coordinates, cleared index, removed commit ids, and complete observed status.
Reset changes only Rey's local admission records; observation never mutates
process variables, files, or executables.

`rey env log` verifies the entire retained chain, selects commits newest first
under the `-n <count>` bound, and recomputes every selected parent-to-commit
capability delta. Its human chronology begins with Git-shaped `commit ENV@n`,
parent, date, and indented message fields, then keeps delta assessment,
authoritative change count, environment scope, changed dimensions, and mapping
visible without reopening provider records. Undated v1 commits say their date
is unknown rather than fabricating one.
`-p` expands each selected transition, including `EMPTY → ENV@1`, through
the same environment-variable and application sections derived
only from the retained parent and commit snapshots. It
performs no fresh observation. Explicit JSON is `rey.environment-log.v1` with
the complete commits, snapshots, typed capability deltas, and commit-time
metadata. Commit commands emit `rey.environment-commit-result.v1`; reset
commands emit `rey.environment-reset-result.v1`.

The default `rey.local-environment-history.v1` state lives at
`${workspace}/.rey/env/state.json`; the separate admission index lives at
`${workspace}/.rey/env/index.json`. Both are bounded single-process local state
using same-directory temporary publication and rename. Reads verify snapshot,
commit, parent, sequence, chain/index identity, exact index base, and safe file
boundaries. History publication and index removal are not one crash-atomic
transaction; an index left after a published commit or history-moving reset is
stale and rejected. It is not a Git object database. Beyond the exact mixed reset above, it claims no
pathspec, restore, branches, merges, general rewrite or revision-expression
semantics, `fsync`, locking, authenticated writer, remote durability, or
external revision semantics.

The fresh v1 history does not admit repository snapshot rows. Git repository
state belongs to cadence and workload activation; a document from an earlier
pre-alpha layout is rejected rather than interpreted as an environment
revision.

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

- remote resource revisions or query checkpoints;
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

- local-only startup;
- standalone, auto, and required-capability selection;
- explicit workspace-root bounds and path escape rejection;
- known tool present, missing, duplicated, and permission denied;
- malformed, oversized, timed-out, and non-zero version probes;
- path, version, digest/provenance, trust, and operation drift;
- mining operation appearance/disappearance, semantic-version drift,
  unsupported encoding/language behavior, result truncation, and parser/index
  completeness degradation;
- optional-provider failure isolated from healthy local providers;
- required-provider failure before any side effect;
- deterministic capability frames and deltas for identical observations; and
- supported Git provider discovery without running repository hooks or
  changing the index.
