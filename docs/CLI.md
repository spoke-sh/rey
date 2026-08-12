# Rey Command-Line Interface

The `rey` CLI is the agent's primary runtime interface and the human
operator's exact diagnostic surface beneath `rey ui`. It is diff-directed,
read-first, bounded, and explicit about mutation. A command should let an
operator verify inputs, revisions, deltas, evidence, omissions, limits, and
lineage without reading implementation code.

This document is the canonical philosophy and command-level reference for the
implemented CLI. Run `rey <command> --help` for the exact accepted flags of the
installed binary. [Interfaces](INTERFACES.md) defines the underlying typed,
provider, HTTP, and policy contracts; subject semantics live in
[Environment](ENVIRONMENT.md), [Workloads](WORKLOADS.md),
[Explorer](EXPLORER.md), and [Journal](JOURNAL.md).

## Interface Philosophy

- **Read first.** Status, list, diff, and log operations expose current typed
  evidence before an agent proposes a mutation. Reads never gain action
  authority merely because they discover a tool, file, workload, or locator.
- **Diff directed.** Comparisons have named source and target revisions. A
  delta is scheduler and review input, not decorative output.
- **Stage exact evidence.** `add` freezes a verified snapshot in an admission
  index. It does not mean “accept whatever exists later.”
- **Commit the index.** `commit` advances history from the frozen INDEX and
  does not re-observe mutable WORKING state. A later WORKING change remains a
  separate unstaged delta.
- **Separate authorities.** Environment admission records an observation;
  workload admission makes a qualified graph current; scene commits package
  authored candidates; Journal admission retains a document. None silently
  grants another authority.
- **Keep humans and automation equivalent.** Human tables and structured JSON
  are projections of the same typed state. Color and layout may aid reading but
  are never the sole carriers of meaning.
- **Fail closed and bounded.** Unsafe paths, malformed inputs, stale
  preconditions, unsupported operations, exceeded limits, and incomplete
  evidence remain explicit rather than being guessed around.

## The Revision Model

Environment, workload, and scene-editor admission use three planes:

```text
retained history             frozen proposal              current observation
     HEAD          ───────────── INDEX          ───────────── WORKING
       │                  HEAD → INDEX                 INDEX → WORKING
       │                  staged delta                 unstaged delta
       └──────── commit consumes INDEX        add freezes exact WORKING
```

The ordering in `HEAD → INDEX → WORKING` names comparison and review flow; it
does not imply that HEAD is copied into WORKING or that mutable bytes are
allowed to leak into a commit.

### HEAD

HEAD is the newest committed revision on one Rey-owned local history:
`ENV@n`, `WORKLOAD@n`, or `SCENE@n`. It is immutable retained evidence. These
are Rey semantic revisions, not Git commits.

### INDEX

INDEX is the exact verified snapshot selected for the next commit. It is bound
to its HEAD base. If no index exists, the effective INDEX for comparison is
HEAD. Staging admits evidence for review; it does not necessarily admit action
or make a workload runnable.

### WORKING

WORKING is the fresh observation or authored file state at the explicit
workspace boundary. Environment WORKING is discovered capability evidence;
workload WORKING is the verified `sys/*/workload.yaml` catalog; editor WORKING
is the internal `.rey/editor/project.json` declaration plus its workspace-native
sources. Before generation initializes that declaration, editor WORKING is
absent and `status` reports that boundary without creating `.rey`. WORKING can
continue to change while INDEX is reviewed.

### Standard Loop

```text
rey <surface> status
rey <surface> diff
rey <surface> add
rey <surface> diff --staged
rey <surface> commit -m "why this exact revision is retained"
rey <surface> log -p
```

`status`, `diff`, and `log` are read surfaces. `add` changes only INDEX.
`commit` verifies and records only INDEX. Environment supports interactive
partial staging with `env add -p`; workload and editor staging are complete
snapshot operations.

Workloads add one mandatory gate between staging and commit:

```text
rey workloads add
rey workloads test --staged
rey workloads commit -m "approve qualified package set"
```

Qualification binds the complete passing scenario result to the exact INDEX.
An agent may author, stage, and test a package, but only explicit approval
advances workload HEAD. `workloads run` resolves only a fresh qualified package
already admitted in HEAD.

## Surfaces And State Machines

| Surface | Implemented state model | Important boundary |
| --- | --- | --- |
| `env` | `ENV HEAD → INDEX → WORKING` | Commits capability observations, never execution authority. |
| `git` | Retained cursor → pending transition → acknowledged cursor | Reads the repository only; activation outputs are proposals and acknowledgement executes nothing. |
| `workloads` | `WORKLOAD HEAD → INDEX → WORKING` plus staged qualification | Only qualified HEAD packages are runnable. |
| `editor` | `SCENE HEAD → INDEX → WORKING` | Commits candidate packages; it does not admit `/explore` evidence. |
| `channels` | `CHANNEL HEAD → INDEX → WORKING` plus immutable messages and relay attempts | Graph commits admit topology only; relay separately requires admitted message, application, environment, and relay identities. |
| `journal` | Proposal → validated retained entry | Direct document admission; blocks are inert and gain no query or action authority. |
| `ui` | Explicit server process over the same typed state | Human projection with narrow Journal and workload-admission writes, not a second runtime. |

Channel message admission is append-only and independent of the topology
INDEX. Journal sequence is not HEAD/INDEX state.

## Command Map

The implemented top-level surface is:

```text
rey channels   list | status | diff | apply | add | commit | log | message | relay | beacon
rey env        status | add | diff | commit | log
rey git        status | init | poll | ack
rey editor     generate | status | add | diff | commit | log
rey workloads  create | list | status | add | diff | test | commit | log | admit-activation | execute-activation | verify-activation | run
rey journal    add | list
rey ui
```

Global options belong to their surface rather than to the root command.
`--workspace` defaults to `.`. A relative `--state-dir` resolves beneath the
canonical workspace; an absolute state directory deliberately selects another
local retention boundary. Workspace-contained source inputs reject path escape
and unsafe file types according to their contract.

### `rey channels`

```text
rey channels [--workspace PATH] [--state-dir PATH] list
rey channels [--workspace PATH] [--state-dir PATH] status
rey channels [--workspace PATH] [--state-dir PATH] diff
rey channels [--workspace PATH] [--state-dir PATH] apply GRAPH.yaml
rey channels [--workspace PATH] [--state-dir PATH] add
rey channels [--workspace PATH] [--state-dir PATH] diff --staged
rey channels [--workspace PATH] [--state-dir PATH] commit -m MESSAGE
rey channels [--workspace PATH] [--state-dir PATH] log [-p] [-n COUNT]
rey channels [--workspace PATH] [--state-dir PATH] message add MESSAGE.yaml
rey channels [--workspace PATH] [--state-dir PATH] message list
rey channels [--workspace PATH] [--state-dir PATH] relay MESSAGE_ID --relay RELAY_ID
rey channels [--workspace PATH] [--state-dir PATH] beacon BEACON_ID
```

`apply` validates a workspace-contained `rey.channel-graph.v1` YAML document
and atomically writes Channel WORKING. `add` freezes the exact WORKING graph in
INDEX. `commit` validates and retains exactly INDEX without rereading a graph
file, while `diff` and `diff --staged` preserve the INDEX-to-WORKING and
HEAD-to-INDEX directions.

`message add` admits an immutable `rey.channel-message.v1` file against an
exact Channel HEAD; it does not send it. `relay` is an explicit effect boundary
that requires the exact message, relay declaration, communications-application
declaration, Channel HEAD, and environment HEAD to agree. The application
declaration freezes an absolute executable, executable digest, optional
version, separate direct argv placeholders, timeout, and output limit. Rey clears the child environment,
does not invoke a shell, and retains only bounded process outcome and output
digests. The CLI never infers send syntax or authority merely because a binary
was discovered.

`beacon` performs one explicit bounded polling tick over retained messages. It
deduplicates already delivered message/relay pairs and invokes at most the
beacon's admitted batch bound. It is not a resident daemon and it does not poll
remote inboxes; scheduling and inbound cursors require a later runtime slice.

### `rey env`

```text
rey env [--workspace PATH] [--state-dir PATH] status [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] add [-p] [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] diff [--staged] [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] commit -m MESSAGE
rey env [--workspace PATH] [--state-dir PATH] log [-p] [-n COUNT]
```

`status`, default `diff`, and `add` perform bounded environment discovery.
Discovery begins only from process-owned `HOME`, `PWD`, and `PATH`, compiled
adapters, and an explicitly supplied reasoning map. It may execute only fixed,
bounded read-only identity probes for known adapters. `diff` shows
INDEX-to-WORKING; `diff --staged` shows HEAD-to-INDEX. `commit` performs no
discovery and successful human/table output is intentionally silent. Use
`log -n 1` for readback or `--format json` for a commit receipt.

The compiled identity-only application inventory includes the major agent
runtimes plus Slack (`slack`), GitHub CLI (`gh`), Telegram CLI
(`telegram-cli`), iMessage (`imsg`), Microsoft 365/Teams (`m365`), Signal
(`signal-cli`), and Discord (`discord`) candidates. These names are bounded
discovery candidates, not claims that each project offers an official or
compatible messaging CLI. Their discovery records explicitly leave transport,
message admission, polling-beacon, and relay authority unsupported.

### `rey editor`

```text
rey editor [--workspace PATH] [--state-dir PATH]
  generate terrain OUTPUT.geojson --id SOURCE --west N --south N --east N --north N [PARAMETERS]
rey editor ... status
rey editor ... add
rey editor ... diff [--staged]
rey editor ... commit -m MESSAGE
rey editor ... log [-p] [-n COUNT]
```

`generate terrain` deterministically creates or updates an owned native
GeoJSON terrain-control source and bootstraps `.rey/editor/project.json` when
absent. The selected `--state-dir` owns that project declaration; there is no
workspace `rey.scene.json` input or output and no `--project` override. Native
source paths remain workspace-relative, bounded, regular, and non-symlinked.
Its seed and hyperparameters are retained as generation lineage; use
`rey editor generate terrain --help` for the complete tunable set. Agents may
then fine-tune the native WORKING files. `add` freezes exact objects, `commit`
validates only the frozen INDEX, and the resulting `SCENE@n` package remains an
unadmitted candidate. There is intentionally no separate `init`, `import`, or
`validate` command.

### `rey git`

```text
rey git [--workspace PATH] [--state-dir PATH] status
rey git ... init
rey git ... poll [--trigger TRIGGER.yaml]...
rey git ... watch [--trigger TRIGGER.yaml]... [--max-iterations N]
  [--interval-ms N] [--max-elapsed-ms N]
rey git ... ack TRANSITION_ID
```

`status` performs a bounded read-only repository observation and compares its
exact snapshot identity with the retained cursor without creating `.rey`
state. `init` explicitly retains that snapshot as the first cursor. `poll`
revalidates the repository/worktree identity, classifies HEAD movement,
compares the complete supported semantic index, and retains one changed transition plus
its exact triggers and deterministic proposal-only activations. Repeating the
same poll is identity-stable and does not duplicate pending evidence; a
different transition cannot replace an unacknowledged one.

`watch` repeats that exact poll observation only under explicit iteration,
cadence, and elapsed scheduling bounds. Each observation is retained first as
a content-identified cadence tick. A changed tick atomically retains the same
pending transition and proposal evidence as `poll`, then stops. A normal stop
also retains a compact receipt that cites its exact tick range, measured
elapsed time, stop reason (`pending_transition`, `iteration_limit`, or
`time_limit`), omissions, and authority. If interruption occurs after a tick
but before the receipt, `git status` exposes the unreceipted count. `watch`
never acknowledges a transition, advances the cursor, executes a workload, or
silently starts another watch.

`ack` requires the exact pending transition identity, retains it in local
history, and advances the cursor from that evidence. A snapshot id, stale
transition, or tampered state fails closed. It does not execute a workload,
mutate Git, contact a remote, or turn a trigger into authority. Trigger inputs
are bounded workspace-contained regular YAML or JSON documents under
`rey.git-activation-trigger.v1`; they bind repository/worktree, event and
completeness selection, exact workload/graph/scenarios, and activation budget.
The human view exposes source and target snapshots, movement completeness,
events, semantic-index posture, omissions, proposals, authority, and next
acknowledgement. JSON retains the same typed documents.

After acknowledgement, `rey workloads admit-activation` is the separate
ordinary workload gate. It never changes Git and does not run as part of
`poll` or `ack`.

### `rey workloads`

```text
rey workloads [--workspace PATH] [--state-dir PATH]
  [--catalog workspace|conformance] [--catalog-dir sys] create ID [--title TITLE] [--intent INTENT] [--attention-row ROW_ID]
rey workloads ... list
rey workloads ... status
rey workloads ... add
rey workloads ... diff [--staged]
rey workloads ... test --staged [ID] [-v|-vv]
rey workloads ... commit -m MESSAGE
rey workloads ... log [-p] [-n COUNT]
rey workloads ... admit-activation ACTIVATION_ID
rey workloads ... execute-activation ADMISSION_ID
rey workloads ... verify-activation EXECUTION_ID [--max-evidence-bytes BYTES]
rey workloads ... run ID [INPUTS AND LIMITS]
```

The default workspace catalog reads request drafts and package proposals from
`sys/<workload>/`. `create` writes a bounded coding-harness request; it does
not invent a graph or invoke an agent. With `--attention-row`, it recomputes
the current portfolio runtime and accepts only the exact ready `CREATE` row
selected into its frontier and reasoning surface. The human and JSON results
retain the portfolio/environment, attention, frontier, scheduling, surface,
permitted-action, current-package, delta, and limit bindings. The eventual
`workload.yaml` must cite the exact retained request path and content digest.
`list` reads admitted HEAD and retained results while carrying draft/revision
posture separately. `status` labels the admission planes `AWAITING HARNESS`,
`WORKING`, `INDEX UNQUALIFIED`, `INDEX QUALIFIED`, and `HEAD`; it observes
the complete HEAD/INDEX/WORKING portfolio without executing it. `test
--staged` runs the frozen scenario suite and retains directed expected-to-
actual evidence. The human runner is diff-native: plain output opens only
unresolved assertions, `-v` shows every compact `EXPECTED → ACTUAL` assertion,
and `-vv` adds exact evidence objects, identities, limits, and lineage.
Human `list` and `status` output identifies every bounded owned-surface
declaration with its exact source revision and required capability ids; JSON
retains the same typed declarations. It also identifies exact Git dependency
kind, repository/worktree, symbolic ref, and expected revision. Portfolio
attention separately reports the live owner binding, revision drift, missing
capability, unowned mapped-surface, and acknowledged Git dependency facts.
Attention rows expose their exact evidence and dependency identities. Fresh
ambient Git state and pending polls do not affect this projection; only the
retained cursor established by `git init` or advanced by `git ack` does. With an exact
retained environment snapshot, the same document includes a distinct runtime
frontier identity, its source attention trace and portfolio/environment
bindings, and each admitted ready row. Without that snapshot the human view
states that the runtime frontier is unavailable; it does not mint an identity
for absent capability evidence. The runtime section also renders the distinct
scheduler decision and selected cost budget, then the reasoning-surface
identity, completeness, evidence/action counts, and surface limits when one
row is selected. Progress and proof remain explicitly absent until a prior
frontier and evaluated transition exist. JSON retains the full verified
frontier/scheduling/surface envelope rather than the human summary.
`admit-activation` resolves only a proposal retained in acknowledged Git
history and requires it to match the current cursor, admitted workload HEAD,
graph, scenario selection, capability snapshot, and effective bounds. It
retains a content-identified, idempotent scheduling admission. The human
receipt and `list` runtime-admissions section expose every binding and state
plainly that no execution occurred; JSON carries the complete typed contract.
`execute-activation` revalidates that admission against current acknowledged
Git, workload HEAD, exact scenario contracts, retained capabilities, and its
action/evidence budget. It evaluates only the selected scenarios, retains
their exact deltas and evidence separately from `last_test`, and returns exit
`0`, `2`, or `3` for passed, failed, or inconclusive evidence. Repeating an
executed admission returns the retained receipt without rerunning the graph.
The human receipt exposes source/target Git evidence, scenario execution and
delta identities, evidence consumption, omissions, authority, and the explicit
boundary that full-suite qualification is unchanged. A compatible admission
from the same Git transition may reuse a directly evaluated retained result;
JSON carries its `source_execution_id`, while human execution and list views
label the result `COALESCED` and name the source. Exact inputs must match and
the retained evidence must fit the new admission's budget.

`verify-activation` revalidates the same Git, workload HEAD, contract, and
capability preconditions, then evaluates every declared scenario under an
explicit local evidence bound. It compares each selected scenario result with
the corresponding full recomputation result exactly and retains the typed
comparison independently from `last_test`. Repeating the command replays the
proof without executing scenarios again. Exit `0` means equivalent, `2` means
different, and qualification remains unchanged in either case.

`commit` requires fresh complete qualification for the
exact INDEX. `run` executes only the exact qualified graph in HEAD through its
declared providers.

`--catalog conformance` selects immutable compiled diagnostic fixtures. It is
not the product catalog and does not participate in workspace admission
history. Workload-specific input, source, context, and mining bounds vary by
operation; `rey workloads run --help` is the exact flag reference.

### `rey journal`

```text
rey journal [--workspace PATH] [--state-dir PATH] add PROPOSAL.yaml
rey journal [--workspace PATH] [--state-dir PATH] list
```

`add` validates and idempotently retains one workspace-contained
`rey.journal-entry-proposal.v2`. Agent-authored entries must place every typed
block in the bounded 12-column broadsheet, but admission executes none of them.
Humans author or supersede entries through the same live document surface at
`/journal/new` and `/journal/{slug}`. `list` exposes retained entries in
admission order, including exact revision, band, cell-kind, and span structure.

### `rey ui`

```text
rey ui [--workspace PATH] [--state-dir PATH]
  [--journal-state-dir PATH] [--catalog-dir sys]
  [--host 127.0.0.1] [--port 5714]
```

`ui` starts the browser operator projection over the same workload,
environment, cadence, Journal, and Explorer evidence. It defaults to loopback.
Its human entry route is `/explore`. A fresh workload state opens on an
unmapped orientation globe whose beacons are exact file-backed workload
candidates; inspection and consent descend into the existing workload and
Feed admission surfaces. The globe does not execute a survey or imply that the
project has already been mapped.
An explicit non-loopback listener exposes unauthenticated Journal admission
and exact workload approval to reachable clients; Rey reports that boundary
and the surrounding deployment must protect it when required.

Browser workload approval is a combined human action over visible file state:
it checks expected HEAD and WORKING identities, freezes the reviewed files in
INDEX, runs the full suite, and commits only the exact qualified result. It
does not bypass the three-plane contract.

## Read And Mutation Posture

| Command family | Posture |
| --- | --- |
| `status`, `list`, `diff`, `log` | Read-only with respect to Rey admission state. A status/diff may perform its documented bounded fresh observation. |
| `git status` | Read-only Git and Rey-state observation; it creates no cursor. |
| `git init`, `git poll`, `git watch`, `git ack` | Retain a baseline, one pending transition, bounded cadence evidence, or exact cursor advancement respectively; none mutates Git or executes a workload. |
| `env add`, `editor add`, `workloads add` | Mutate only the corresponding INDEX. |
| `env commit`, `editor commit`, `workloads commit` | Verify and advance only from INDEX; never absorb later WORKING state. |
| `workloads test --staged` | Executes bounded scenario probes and retains qualification evidence; never advances HEAD. |
| `workloads admit-activation` | Retains exact scheduling eligibility for one acknowledged Git proposal after ordinary workload precondition checks; executes nothing. |
| `workloads execute-activation` | Revalidates one retained admission, evaluates its selected scenarios under the exact evidence budget, and retains a replay-stable non-qualifying result; never mutates Git. |
| `workloads verify-activation` | Fully recomputes the declared suite for one retained activation execution, compares selected evidence exactly, and retains a bounded non-qualifying equivalence proof; never mutates Git. |
| `workloads run` | Executes an admitted graph and retains results under declared provider/effect contracts. |
| `editor generate`, `workloads create` | Explicitly author workspace files; neither admits its output. |
| `channels apply` | Writes only the Channel WORKING proposal. |
| `journal add` | Retains a document only; notebook blocks remain inert. |
| `ui` | Starts a server; its two narrow write paths are Journal admission and qualified workload approval. |

Process success and semantic convergence remain separate. A successful status
may report differences, an unready INDEX, omissions, or unresolved work.

## Ignore Policy

An optional workspace-root `.reyignore` narrows fresh WORKING observations:

```text
workload: context-anchor-survey
environment variable:*
application: code?
```

Rules are typed, case-sensitive, and support `*` and `?`. Rey validates
candidates before applying them. Relevant rules, exact file digest, source
line, match count, and omitted count enter the affected WORKING identity and
appear in status and structured output. Ignore policy does not delete files,
rewrite HEAD or INDEX, bypass validation, or grant authority. See
[Interfaces](INTERFACES.md#workspace-ignore-surface).

## Human And Structured Output

Most command groups accept `--format auto|table|json`:

- `auto` renders the human document on a terminal and emits JSON when
  redirected;
- `table` forces the bounded human projection;
- `json` emits the typed envelope with exact identities, limits, completeness,
  and omissions.

Environment history commands intentionally default to `table` even when
redirected; automation must request `--format json`. The current public CLI
does not advertise Arrow as a generic envelope format. Arrow remains typed
interchange for genuinely relational artifacts inside the runtime.

Human verbosity changes projection detail, not semantic results. In particular,
plain `workloads test` folds passing assertions and opens unresolved patches;
`-v` exposes compact expected and actual assertions; and `-vv` additionally
opens exact evidence, identity, limit, and lineage bindings. JSON retains the
complete document independently of those flags.

ANSI styling is enabled only on an interactive terminal when `NO_COLOR` is
absent and `TERM` is not `dumb`. Redirected and structured output is ANSI-free.
Environment and workload status share one positional color contract:

- green means a change already staged in INDEX and awaiting commit;
- red means WORKING drift not staged in INDEX.

The explicit `new:`, `deleted:`, and `modified:` labels remain authoritative;
color never supplies direction by itself.

## Standard Streams

- Human and machine results go to stdout.
- Diagnostics, warnings, and actionable failures go to stderr.
- Structured stdout contains no progress chatter or trailing diagnostic text.
- Human workload tests may stream retained scenario results in declaration
  order; machine output emits only the final structured document.
- Successful human `env commit` is silent on both streams.

## Exit Behavior

Inspection, staging, commit, history, creation, and listing commands return `0`
when the requested operation succeeds. Reported semantic differences or an
unready INDEX are state, not command failure. Invalid input, unsafe state, or a
runtime failure returns `1`; command-line parsing uses Clap's own failure
behavior.

`workloads test` additionally distinguishes semantic outcome:

| Code | Meaning |
| --- | --- |
| `0` | Qualified or passed. |
| `2` | Conclusive semantic failure. |
| `3` | Inconclusive. |
| `1` | Invalid input or runtime failure. |

`workloads run` returns `0` for passed, `3` for blocked, and `1` for invalid
input or runtime failure. Staleness remains explicit typed state, but the
current executable does not emit a separate stale exit code. Callers must not
interpret a zero process exit as a universal proof claim.

## Local State And Guarantees

By default Rey keeps local state under the selected workspace:

```text
.rey/env/         environment history and admission index
.rey/git/         Git cursor, pending activation evidence, cadence receipts, and transition history
.rey/workloads/   workload objects, qualification results, and history
.rey/editor/      scene INDEX objects, packages, and history
.rey/channels/    Channel WORKING proposal
.rey/journal/     retained Journal entries
```

These stores are bounded, verified local single-process evidence boundaries.
They do not claim authenticated writers, remote durability, external-service semantics,
or Git object-database behavior. Source-owned workload, scene, Channel, and
Journal proposal files remain outside the local object caches. Clearing
`.rey` clears retained local Rey state, not authored workspace files.
