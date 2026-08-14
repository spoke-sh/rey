# Rey Command-Line Interface

The `rey` CLI is the agent's primary runtime interface and the human
operator's exact diagnostic surface beneath `rey agent`. It is diff-directed,
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
`commit` verifies and records only INDEX. Environment add requires an explicit
scope: `env add .` or `env add -A` stages every unstaged change, while one or
more environment or mapped-input paths stage only matching changes.
Interactive partial staging uses `env add -p [<path>...]`; without a path it
walks every stageable unstaged hunk. Application search outcomes whose current
WORKING availability is `unavailable` or `error` are excluded from interactive
selection and remain unstaged. Workload and editor staging are complete
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
| `channels` | `CHANNEL HEAD → INDEX → WORKING` plus immutable messages, application polls, and relay attempts | Graph commits admit topology only; polling and relay separately require exact application, environment, and Channel identities. |
| `conversations` | Immutable sessions plus append-only per-session transcript sequence | Admission retains local dialogue only; it does not deliver, invoke an agent, relay, schedule work, or grant proof authority. |
| `journal` | Proposal → validated retained entry | Direct document admission; blocks are inert and gain no query or action authority. |
| `agent` | Foreground Rey process with a bounded supervised topology | Orchestrator-owned operator projection with narrow Observation/Journal/conversation admission, Channel WORKING, and workload-admission writes; no autonomous workload or agent-runtime invocation. |

Channel message admission is append-only and independent of the topology
INDEX. Journal sequence is not HEAD/INDEX state.

## Command Map

The implemented top-level surface is:

```text
rey env        status | add | reset | diff | commit | log
rey workloads  create | list | status | add | diff | test | commit | log | admit-activation | execute-activation | verify-activation | run
rey journal    add | list | seed | opportunities | query
rey agent
rey editor     generate | status | add | diff | commit | log
rey version    [--format table|json]
rey channels   list | status | diff | apply | add | commit | log | message | relay | beacon | poll
rey conversations status | session | message
rey observations add | list | show | resolve
rey git        status | init | poll | watch | ack
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
rey channels [--workspace PATH] [--state-dir PATH] poll APPLICATION_ID
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
remote inboxes.

`poll` is the first provider-specific inbound path. The selected Channel-HEAD
application must declare a GitHub inbox and bind the exact
`comms.application.github.identity` capability, absolute `gh` path, executable
digest, optional version, target Channel, `github.com` host, credential
environment names, poll cadence, timeout, capture bound, and
notification/PR/comment limits.
The same exact capability must exist as available in environment HEAD before
the command invokes it. Rey executes fixed `gh api` GET requests for the
authenticated user's current unread notifications and, for bounded
`PullRequest` subjects, the issue-level and review-thread comment endpoints.
Comment requests select newest updated rows first and use the notification's
provider `last_read_at` value as `since` when present.
It never takes provider-supplied argv, invokes a shell, or marks notifications
read. The current provider snapshot, exact source revisions and links, response
digests, omissions, and idempotent Channel-message admissions are retained in
`rey.github-channel-poll-receipt.v1`. Repeating the same provider snapshot
reuses its immutable messages. The latest poll for the exact current Channel
HEAD defines the mailbox frontier, so a later complete empty poll removes old
notifications from the current projection without immediately deleting
retained evidence.
The bounded local store rolls its oldest poll receipts and then evicts only
GitHub messages no longer referenced by a retained receipt; it does not evict
locally authored Channel messages to make provider room.
This command is the explicit one-shot verification surface. `rey agent`
registers a separate supervised inbox worker that invokes the same command
immediately when an admitted GitHub application appears and thereafter at its
committed cadence. It performs no immediate retry; a complete or partial tick
is retained before the next cadence, while an exact-admission failure fails the
worker and foreground process closed.

### `rey conversations`

```text
rey conversations [--workspace PATH] [--state-dir PATH] status [--session ID] [-n COUNT]
rey conversations [--workspace PATH] [--state-dir PATH] session add SESSION.yaml
rey conversations [--workspace PATH] [--state-dir PATH] session list
rey conversations [--workspace PATH] [--state-dir PATH] message add MESSAGE.yaml
```

`session add` admits an immutable exact `rey.local-transcript/v1` session with
declared participants, writers, and optional human browser writer. `message
add` appends only when its exact session, self-asserted author, write authority,
and optional prior same-session reply all verify. Identical proposals are
idempotent. `status` exposes provider availability, exact session/log/source
identities, per-session ordering, retention, read/write authority, effect
boundary, completeness, omissions, bounds, and failure behavior.

Every message has `delivery: not_attempted`. The command never invokes an
agent, contacts a remote provider, uses Channel relay, creates an observation
or Journal entry, schedules work, mutates runtime state, or proves a claim.
The default store is `.rey/conversations`; missing state is an explicit
read-only unavailable transcript and creates nothing. See
[Conversations](CONVERSATIONS.md).

### `rey env`

```text
rey env [--workspace PATH] [--state-dir PATH] status [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] add (-A|PATH...|-p [PATH...]) [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] reset [HEAD|EMPTY|ENV@n|COMMIT_ID] [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] diff [--staged] [--map PATH]
rey env [--workspace PATH] [--state-dir PATH] commit -m MESSAGE
rey env [--workspace PATH] [--state-dir PATH] log [-p] [-n COUNT]
```

`status`, default `diff`, and `add` perform bounded environment discovery.
Discovery begins only from process-owned `HOME`, `PWD`, and `PATH`, compiled
adapters, and an explicitly supplied reasoning map. It may execute only fixed,
bounded read-only identity probes for known adapters. `diff` shows
INDEX-to-WORKING; `diff --staged` shows HEAD-to-INDEX. `reset` is a narrow
mixed reset: it moves environment HEAD to an exact retained target, removes
later entries from the linear commit log, and clears the separate admission
index so its effective snapshot again equals HEAD. Its default target is
`HEAD`, which clears only the index; `EMPTY` returns to the typed empty
history. Targets do not accept abbreviations or revision expressions. After
the reset, the command performs the same bounded WORKING observation and typed
projection as `status`; table output is silent when clean or prints Git-shaped
`A`, `M`, and `D` rows under `Unstaged changes after reset:`. Reset never
mutates the ambient environment. `status`, default `diff`, `add`, and `reset`
perform discovery; `commit` and `log` do not. Successful human/table commit
output is intentionally silent. Use `log -n 1` for readback, or request JSON
for a commit or reset receipt with the complete post-reset status.

Bare `rey env add` is rejected before discovery or mutation. `.` and `-A`
select the complete `INDEX → WORKING` delta. Other operands match canonical
environment paths such as `environment/application/git` or exact mapped input
paths such as `docs/toolchain.toml`; directory prefixes select bounded
descendants. Every pathspec must match an unstaged change. `-p` without a path
walks all stageable unstaged hunks; supplied paths narrow that interactive set.
An unmatched path leaves INDEX unchanged. Patch selection offers only
applications whose current WORKING observation is `available`; unavailable and
errored application hunks are counted as excluded and cannot be selected by
`y` or `a`. A scope containing only unresolved application hunks fails without
creating or changing INDEX. Full `add .` and `add -A` retain the complete
snapshot, including explicit missing/error degradation evidence.

The compiled identity-only application inventory includes the major agent
runtimes plus Slack (`slack-cli`), GitHub CLI (`gh`), Telegram CLI
(`telegram-cli`), iMessage (`imsg`), Teams (`teams`), and Signal
(`signal-cli`) candidates. Application names exactly match the executable
searched on `PATH`. These names are bounded
discovery candidates, not claims that each project offers an official or
compatible messaging CLI. Their discovery records explicitly leave transport,
message admission, polling-beacon, and relay authority unsupported.
The GitHub polling path does not weaken that boundary: only a separately
committed Channel application plus an exact matching environment HEAD and an
explicit `rey channels poll` command admit the read-only `gh api` probe.
Declarations carry a normalized many-to-many group set. The initial groups are
`communications`, `agents`, `retrieval`, and `code`; `env diff` groups the
typed desired inventory in that order. `grep` and `rg` are retrieval
applications; `rg` is not classified as a semantic code-analysis tool. Human
`env diff` and compact `env
status` render only flattened found matches, one per application, while keeping
a previously found application visible if it disappears. Unsuccessful searches
and the complete desired inventory remain available in structured evidence.

### `rey editor`

```text
rey editor [--workspace PATH] [--state-dir PATH]
  source add INPUT.geojson --id SOURCE --role ROLE [--scene-id PROJECT]
  generate terrain OUTPUT.geojson --id SOURCE --west N --south N --east N --north N [PARAMETERS]
rey editor ... status
rey editor ... add
rey editor ... diff [--staged]
rey editor ... commit -m MESSAGE
rey editor ... log [-p] [-n COUNT]
```

`source add` verifies one existing workspace-relative RFC 7946 GeoJSON file,
its stable feature identities, coordinate bounds, and role-specific geometry,
then registers its explicit semantic role in editor WORKING. It creates the
internal project when absent, using `--scene-id` or the source ID. Repeating the
same registration is idempotent; changing an existing source ID's path/role or
reusing its path under another ID is rejected rather than silently rebound.
The human receipt exposes the exact content revision, bytes, feature/coordinate
coverage, native bounds, and candidate-only authority. It does not copy,
rewrite, stage, commit, admit, or fetch the source.
For `--role terrain`, each feature must be a Point with longitude, latitude,
and elevation in meters plus a bounded `material` property. The receipt and
later `-vv` admission evidence retain exact content/object revisions; an
accepted scene prints `TERRAIN` with its program, evaluator, micrometer samples,
materials, and no-interpolation authority. `terrain_control` does not qualify.

`generate terrain` deterministically creates or updates an owned native
GeoJSON terrain-control source and bootstraps `.rey/editor/project.json` when
absent. The selected `--state-dir` owns that project declaration; there is no
workspace `rey.scene.json` input or output and no `--project` override. Native
source paths remain workspace-relative, bounded, regular, and non-symlinked.
Its seed and hyperparameters are retained as generation lineage; use
`rey editor generate terrain --help` for the complete tunable set. Agents may
then add or fine-tune native WORKING files. `add` freezes exact objects, `commit`
validates only the frozen INDEX, and the resulting `SCENE@n` package remains an
unadmitted candidate. There is intentionally no separate `init` or `validate`
command; `source add` is registration, not a copy-style import.

Admission crosses the normal workload plane:

```text
rey workloads add
rey workloads test --staged scene-admission -vv
rey workloads commit -m "Admit scene validation workload"
rey workloads run scene-admission --scene SCENE@n [--editor-state-dir PATH]
```

The run requires a fresh qualified `scene-admission` graph in workload HEAD and
one exact committed editor revision. Human output distinguishes native CRS84,
synthetic semantic, semantic-Mercator, County-local, and camera coordinates and
prints the Mercator `360000000µ°` wrap, `±85051129µ°` polar cutoff/disclosure,
and analytic inverse boundary plus the envelope-bound County-local inverse and
its explicit non-footprint disclosure beside the admitted/absent County
footprint state, exact package, packet, validity, limits, omissions, and
lineage. Each explicitly declared native source role remains a separately
named layer in this output; terrain controls retain candidate-only authority.
`-vv` prints every layer's exact kind/object membership/source revision and the
footprint identity, source bindings, native rings, coordinate count, and authority. JSON
retains the complete `rey.scene-admission-result.v1`. Rejected validation
scenarios are conclusive typed results; no run mutates editor state or admits a
browser scene.

### `rey git`

```text
rey git [--workspace PATH] [--state-dir PATH]
  [--max-reachable-commits-per-direction N]
  [--max-path-changes-per-ref N] status
rey git ... init [--watch-ref refs/...]...
rey git ... poll [--trigger TRIGGER.yaml]...
rey git ... watch [--trigger TRIGGER.yaml]... [--max-iterations N]
  [--max-retries N] [--interval-ms N] [--max-elapsed-ms N]
rey git ... ack TRANSITION_ID
```

`status` performs a bounded read-only repository observation and compares its
exact snapshot identity with the retained cursor without creating `.rey`
state. `init` explicitly retains that snapshot as the first cursor. Repeatable
`--watch-ref` values must be canonical full `refs/...` names; initialization
sorts and freezes that exact scope and records a missing ref as `ABSENT` so a
later creation is observable. `status`, `poll`, and `watch` reuse the retained
scope without discovering more refs. `poll` revalidates repository/worktree
identity, classifies HEAD and each changed watched ref independently, compares
bounded added/removed reachable-commit sets and tree-to-tree path changes,
compares the complete supported semantic index, and retains one changed
transition plus its exact triggers and deterministic proposal-only
activations. The global
`--max-reachable-commits-per-direction` limit bounds each side of each changed
ref. `--max-path-changes-per-ref` bounds each changed ref's canonical raw-byte
path sequence; every change retains direction, reversible identity, modes, and
object OIDs without rename inference. Shallow, unavailable, and truncated
evidence remains explicit partial evidence. Repeating the same poll is
identity-stable and does not duplicate pending evidence; a different
transition cannot replace an unacknowledged one.

`watch` repeats that exact poll observation only under explicit iteration,
cadence, elapsed, and retry bounds. Each successful or failed attempt is
retained first as a content-identified cadence tick. Failed ticks retain
bounded typed provider evidence but no counterfeit observed snapshot. A
changed tick atomically retains the same pending transition and proposal
evidence as `poll`, then stops. Every terminal invocation retains a compact
receipt with exact tick range, bounds, measured elapsed time, completeness,
omissions, and stop reason (`pending_transition`, `iteration_limit`,
`time_limit`, `retry_limit`, `cancelled`, or `failure`). Recovered failures
remain partial. SIGINT/SIGTERM cancellation is cooperative at the bounded
command/tick boundary. A hard interruption after a tick but before the receipt
remains visible through the unreceipted count. `watch` never acknowledges a
transition, advances the cursor, executes a workload, silently starts another
watch, or claims convergence.

`ack` requires the exact pending transition identity, retains it in local
history, and advances the cursor from that evidence. A snapshot id, stale
transition, or tampered state fails closed. It does not execute a workload,
mutate Git, contact a remote, or turn a trigger into authority. Trigger inputs
are bounded workspace-contained regular YAML or JSON documents under
`rey.git-activation-trigger.v1`; they bind repository/worktree, event and
optional exact `HEAD`/watched-ref selection, completeness, exact
workload/graph/scenarios, activation budget, and optional reversible byte
prefixes for path events. Proposals retain their exact matched ref names and
matched path identities/directions. The human view exposes source and target
snapshots, HEAD and watched-ref movement completeness, events, semantic-index
posture, added/removed commit OIDs, exact path changes and limits, omissions,
proposals, matched refs/paths, authority, and next acknowledgement. JSON
retains the same typed documents.

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
graph, scenario selection, admitted environment snapshot, automatic intrinsic
runtime snapshot, and effective bounds. It
retains a content-identified, idempotent scheduling admission. The human
receipt and `list` runtime-admissions section expose every binding and state
plainly that no execution occurred; JSON carries the complete typed contract.
`execute-activation` revalidates that admission against current acknowledged
Git, workload HEAD, exact scenario contracts, both frozen snapshots, and its
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
rey journal [--workspace PATH] [--state-dir PATH]
  [--observation-state-dir PATH] seed OBSERVATION_ID...
  --author AGENT_ID [--format table|json]
rey journal [--workspace PATH] [--state-dir PATH] opportunities
  [-n COUNT] [--format table|json]
rey journal [--workspace PATH] [--state-dir PATH]
  [--observation-state-dir PATH] query admit ENTRY_ID BLOCK_ID
  [--format table|json]
rey journal [--workspace PATH] [--state-dir PATH]
  [--observation-state-dir PATH] query execute ADMISSION_ID
  --author AGENT_ID --proposal-out RESULT.json [--format table|json]
rey journal [--workspace PATH] [--state-dir PATH] query list
  [--format table|json]
```

`add` validates and idempotently retains one workspace-contained
`rey.journal-entry-proposal.v2`. Agent-authored entries must place every typed
block in the bounded 12-column broadsheet, but admission executes none of them.
Humans author or supersede entries through the same live document surface at
`/journal/new` and `/journal/{slug}`. `list` exposes retained entries in
admission order, including exact revision, band, cell-kind, and span structure.
`seed` selects 1–16 unique exact unresolved observations, canonicalizes them by
observation sequence, and emits a content-identified valid broadsheet proposal.
It is read-only and unretained; normal `journal add` validation and admission
are still required to create a Journal entry.

`opportunities` derives action cells only from unsuperseded Journal leaves and
renders their exact author, document fragment, semantic binding, desired
delta, citations, completeness, omissions, and effective row limit. Each row
is authored-only with no readiness, assignment, execution, or proof authority;
runtime work still requires the verified workload/policy admission boundary.
`query admit` retains exact read-only authority for one current
`rey.observations/rey frontier` cell and exact Journal/observation inputs but
does not execute it. `query execute` revalidates those inputs, retains bounded
frame/delta evidence, and writes a create-new unretained superseding proposal;
it does not append a Journal entry. `query list` is read-only. Only a later
ordinary `journal add RESULT.json` validates and retains the superseding entry.

### `rey observations`

```text
rey observations [--workspace PATH] [--state-dir PATH] add OBSERVATION.yaml
  [--channel ID ...] [--no-broadcast] [--format table|json]
rey observations [--workspace PATH] [--state-dir PATH] list
  [-n COUNT] [--format table|json]
rey observations [--workspace PATH] [--state-dir PATH] show OBSERVATION_ID
  [--format table|json]
rey observations [--workspace PATH] [--state-dir PATH] resolve RESOLUTION.yaml
  [--format table|json]
```

`add` admits one content-identified immutable collaboration observation from a
workspace-contained regular file. Explicit `--channel` targets, or the
effective graph's bounded default set, yield a retained per-target partial
broadcast receipt; `--no-broadcast` admits locally only. `list` projects the
bounded unresolved frontier and its completeness, omissions, and state counts.
`show` exposes exact source, evidence, self-asserted author, Channel admissions,
and closure. `resolve` appends one idempotent resolution. None changes Channel
INDEX/HEAD, relays, schedules, assigns, executes, or proves work.

The browser counterpart begins at Feed's `Share an observation` control. Its
tweet-like rich-text modal submits one Markdown body of at most 500 characters
to the same local store with the compact surface's fixed `finding` kind, fixes
the self-asserted human author to `operator`, scopes the subject to the workspace
root, marks coverage partial with the missing-evidence omission, and uses the
effective graph's default broadcast set. It creates no Journal entry; the CLI
remains the high-fidelity path for exact kind, subject, evidence, completeness,
supersession, and explicit Channel selection.

### `rey version`

```text
rey version [--format table|json]
```

`version` prints the semantic package version and the exact Git commit captured
when the binary was built. Human output is `rey VERSION (commit SHA)`;
`--format json` emits the equivalent `rey.version.v1` document with `version`
and `commit_sha` fields. The command reads no workspace or mutable repository
state. A source archive built without an admitted build revision reports the
existing explicit `unknown` boundary instead of guessing a commit.

### `rey agent`

```text
rey agent [--workspace PATH] [--state-dir PATH]
  [--journal-state-dir PATH] [--channel-state-dir PATH]
  [--conversation-state-dir PATH] [--catalog-dir sys]
  [--host 127.0.0.1] [--port 5714] [--format auto|table|json]
```

`agent` starts the foreground Rey process. Its orchestrator registers the
embedded operator HTTP server and exact admitted GitHub inbox poller as two
bounded background workers and defaults the listener to loopback. The inbox
worker remains idle without a committed `github_inbox` application. The
default human startup output is one
line—`INFO:     Listening on http://127.0.0.1:5714 (Press CTRL+C to quit)`—and
stderr first identifies the exact Rey version and build commit, then logs
process, agent-startup, worker, shutdown, and failure lifecycle events.
`--format json` exposes the complete `rey.agent-process.v1` document
with its nested `rey.process.v1` and `rey.agent-topology.v1`; the same exact
topology remains human-visible on `/agents`. SIGINT and SIGTERM stop both
workers cooperatively; an unexpected exit fails the process closed. V1 never restarts
or detaches a worker and does not invoke discovered agent runtimes or
autonomously schedule workloads.

The operator worker projects the same workload, environment, cadence, Journal,
and Explorer evidence. The separate inbox worker polls only exact Channel- and
environment-HEAD GitHub applications through `rey channels poll`; it neither
invokes a discovered application directly nor marks notifications read.
Its human entry route is `/explore`. A fresh workload state opens on an
unmapped orientation globe whose beacons are exact file-backed workload
candidates; inspection and consent descend into the exact workload review and
approval surface. Feed projects the resulting retained commit only after that
admission succeeds. The globe does not execute a survey or imply that the
project has already been mapped.
An explicit non-loopback listener exposes unauthenticated Journal and
conversation admission, Channel WORKING replacement, and exact workload
approval to reachable clients; Rey reports that boundary and the surrounding deployment must protect
it when required. Channel topology is non-navigable browser substrate for Feed,
mailbox, and conversation; `rey agent` exposes no `/channels` route or primary
navigation item. `/feed` uses the Channel projection in
`URL preview → WORKING → HEAD → built-in`
order. URL tuning stays detached until the operator adopts it; stable drag,
move-button, and `Alt+Arrow` layout movement conditionally replaces WORKING,
shows the returned semantic delta, and restores the prior layout after a stale
or rejected write. These controls grant no additional Channel authority.

The footer conversation axis reads the same default bounded transcript as
`rey conversations status`. It enables the composer only when the exact
session declares a human browser writer. Append binds the displayed log and
session identities, derives that writer rather than accepting an arbitrary
author, and retains `delivery: not_attempted` through the same validator/store.
Stale state, missing transport/writer, invalid content, or persistence failure
rejects the write and keeps the composer or failure boundary visible. Mailbox
history remains a separate read projection over current retained Channel poll
messages, typed attention, and revalidation failures. Authored Observations do
not enter it.

Browser workload approval is a combined human action over visible file state:
it checks expected HEAD and WORKING identities, freezes the reviewed files in
INDEX, runs the full suite, and commits only the exact qualified result. It
does not bypass the three-plane contract.

Admitted workload detail pages also read the bounded
`rey.ui-workload-evidence-catalog.v1` index. Each retained scenario links by
its exact execution identity to
`/workloads/{workload-id}/scenarios/{execution-id}`; every output-text,
source-match, and topography directed delta links by its exact delta identity
to `/workloads/{workload-id}/deltas/{delta-id}`. Scenario and delta pages keep
the CLI projection order as plain outcome, `-v` assertions or source-to-target
changes, and `-vv` identity/source/limit/omission/lineage evidence. They read
and reverify retained results only. They do not execute a scenario, recompute
an assessment, qualify a graph, admit a workload, or grant action or proof
authority. A retained stale result remains exact but labels the current source
as unbound; an unknown identity returns not found rather than selecting the
latest result.

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
| `channels poll` | Executes bounded read-only GitHub API probes only through exact Channel/environment HEAD application admission, then atomically retains the poll receipt and immutable Channel messages; it never marks provider notifications read. |
| `journal add` | Retains a document only; notebook blocks remain inert. |
| `journal seed`, `journal opportunities` | Read-only deterministic projections; neither retains a document, schedules work, or executes a block. |
| `journal query admit` | Retains one exact read-only query admission; executes nothing and leaves the Journal unchanged. |
| `journal query execute` | Revalidates exact admitted inputs, retains bounded query evidence, and authors a create-new superseding proposal; leaves the Journal unchanged. |
| `agent` | Starts the supervised foreground Rey process; its operator worker's narrow writes are Journal admission, expected-log/session conversation append, expected-snapshot Channel WORKING replacement, and qualified workload approval. |

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

`git watch` returns `0` for a complete bounded stop or retained pending
transition, `3` when retained failure evidence makes the watch inconclusive,
`130` after a cooperatively retained cancellation, and `1` for invalid input
or unsafe state. Structured failure/cancellation receipts remain on stdout.

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
