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
is `rey.scene.json` plus its declared native sources. WORKING can continue to
change while INDEX is reviewed.

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
| `workloads` | `WORKLOAD HEAD → INDEX → WORKING` plus staged qualification | Only qualified HEAD packages are runnable. |
| `editor` | `SCENE HEAD → INDEX → WORKING` | Commits candidate packages; it does not admit `/explore` evidence. |
| `channels` | `BUILT-IN → CHANNEL WORKING` | `apply` writes a proposal; Channel HEAD, INDEX, `add`, `commit`, and `log` remain planned. |
| `journal` | Proposal → validated retained entry | Direct document admission; blocks are inert and gain no query or action authority. |
| `ui` | Explicit server process over the same typed state | Human projection with narrow Journal and workload-admission writes, not a second runtime. |

Do not infer a missing revision loop. In particular, Channel proposals are not
committed history, and Journal sequence is not HEAD/INDEX state.

## Command Map

The implemented top-level surface is:

```text
rey channels   list | status | diff | apply
rey env        status | add | diff | commit | log
rey editor     generate | status | add | diff | commit | log
rey workloads  create | list | status | add | diff | test | commit | log | run
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
```

`list`, `status`, and `diff` compare the canonical built-in collaboration graph
with an optional bounded WORKING proposal. `apply` validates a
workspace-contained `rey.channel-graph.v1` YAML document and atomically writes
only Channel WORKING. It does not relay messages, admit observations, or create
Channel history.

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

### `rey editor`

```text
rey editor [--workspace PATH] [--state-dir PATH] [--project rey.scene.json]
  generate terrain OUTPUT.geojson --id SOURCE --west N --south N --east N --north N [PARAMETERS]
rey editor ... status
rey editor ... add
rey editor ... diff [--staged]
rey editor ... commit -m MESSAGE
rey editor ... log [-p] [-n COUNT]
```

`generate terrain` deterministically creates or updates an owned native
GeoJSON terrain-control source and bootstraps the project when absent. Its
seed and hyperparameters are retained as generation lineage; use
`rey editor generate terrain --help` for the complete tunable set. Agents may
then fine-tune the native WORKING files. `add` freezes exact objects, `commit`
validates only the frozen INDEX, and the resulting `SCENE@n` package remains an
unadmitted candidate. There is intentionally no separate `init`, `import`, or
`validate` command.

### `rey workloads`

```text
rey workloads [--workspace PATH] [--state-dir PATH]
  [--catalog workspace|conformance] [--catalog-dir sys] create ID [--title TITLE] [--intent INTENT]
rey workloads ... list
rey workloads ... status
rey workloads ... add
rey workloads ... diff [--staged]
rey workloads ... test --staged [ID] [-v|-vv]
rey workloads ... commit -m MESSAGE
rey workloads ... log [-p] [-n COUNT]
rey workloads ... run ID [INPUTS AND LIMITS]
```

The default workspace catalog reads request drafts and package proposals from
`sys/<workload>/`. `create` writes a bounded coding-harness request; it does
not invent a graph or invoke an agent. `list` reads admitted HEAD and retained
results while carrying draft/revision posture separately. `status` observes
the complete HEAD/INDEX/WORKING portfolio without executing it. `test
--staged` runs the frozen scenario suite and retains directed expected-to-
observed evidence. `commit` requires fresh complete qualification for the
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
| `env add`, `editor add`, `workloads add` | Mutate only the corresponding INDEX. |
| `env commit`, `editor commit`, `workloads commit` | Verify and advance only from INDEX; never absorb later WORKING state. |
| `workloads test --staged` | Executes bounded scenario probes and retains qualification evidence; never advances HEAD. |
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
`workloads test -v` opens evidence and `-vv` opens exact identity and lineage
bindings; JSON retains the complete document independently of those flags.

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
