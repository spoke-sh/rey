# Rey

> **The client oriented reasoning surface.**

Rey turns the context surrounding a project into a world that humans and
agents can inspect together. It inventories explicit local state, receives
agent-authored workloads as reviewable files, mines bounded evidence, and
projects admitted understanding into a continuous spatial interface.

![Rey's consent-first Explorer projects exact agentic workload beacons onto an unmapped context globe.](docs/assets/explore.png)

## Mining As Applied Programming

Rey is built around a practical view of programming: progress begins by
mining the environment. A programmer locates evidence, retrieves the exact
parts that matter, exposes useful structure, compares it with an expectation
or prior revision, and acts on the smallest meaningful frontier. Code
generation is one possible action near the end of that loop; it is not the
loop itself.

Rey treats **mining** as the bounded transformation of context into navigable,
addressable evidence. Relational mining works over typed records, events,
measurements, and graph relations. Source mining works over code, text,
configuration, logs, documents, and native artifacts. Exact projections let
the two meet without flattening one into the other.

Visualization is part of mining. A globe, terrain surface, table, patch, tree,
graph, or metric panel is a view over authoritative evidence. It preserves
direction, identity, completeness, omissions, limits, and links back to exact
sources. It does not become proof merely because it looks complete.

## Installation

### One-line install (macOS and Linux)

Tagged releases publish a cargo-dist shell installer:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/spoke-sh/rey/releases/latest/download/rey-installer.sh | sh
```

### Nix

Run Rey directly from the repository flake:

```sh
nix run github:spoke-sh/rey -- --help
```

### Manual download

Native archives, checksums, and platform installers are attached to
[GitHub Releases](https://github.com/spoke-sh/rey/releases). Release and
publication guarantees are documented in [Releases](docs/RELEASES.md).

Rey is pre-alpha. Until the first tagged release is published, use the pinned
development environment described in [Development](docs/DEVELOPMENT.md).

## Find Your First Bearing

From a project containing Rey workload packages, start the operator surface:

```sh
rey agent
```

`rey agent` starts the foreground Rey process. Its orchestrator supervises the
operator HTTP server and owns the lifecycle of every in-process background
worker. Startup prints one framework-style listening URL while stderr records
the exact `rey version` identity and lifecycle events as the process and
workers start and stop. Rey listens on
`127.0.0.1:5714` by default. The server root opens the Swagger-guided API at
`/api`, while the human operator enters the spatial surface at `/explore`;
exact live topology remains available through `/agents`, `/api/v1/agent`, and
`--format json`. Incoming
file-backed workload proposals appear as beacons without being treated as
admitted knowledge. Select a beacon to inspect its exact source and revision,
then use the admission surface when you are ready to consent.

The corresponding agent-side bearing is:

```sh
rey env status
rey git status
rey workloads status
rey workloads diff
```

An agent can inspect and propose through the CLI while the human remains in
the browser. Starting the Rey agent process, opening a deep link, panning, zooming, or
selecting a beacon never runs a survey or silently broadens read authority.

When developing Rey itself, use the repository wrapper:

```sh
nix develop
just setup
just rey agent
```

The development wrapper visibly builds the embedded static UI assets before
Cargo starts the agent process, so the browser always serves the just-built
application and the Vite build result remains in the terminal transcript.

## The Client-Oriented Surface

Rey organizes collaboration around two clients with one evidence plane:

- **Humans navigate and consent.** Explorer provides the spatial bearing;
  Feed carries incoming signals and retained environment/workload admission history;
  exact workload review owns consent before a commit; Journal retains
  addressable human/agent synthesis. Channel topology stays behind the scenes
  as substrate for Feed, mailbox, and conversation rather than appearing as a
  top-level browser destination. Feed resolves detached URL previews ahead of
  Channel WORKING, HEAD, and built-in layouts; adoption and stream movement
  remain explicit WORKING-only writes. Immutable observations,
  their local Channel-admission edges, partial broadcast receipts, and catch-up
  frontier remain a separate bounded state plane. Feed admits compact human
  observations through a tweet-like rich-text modal and Feed reads the
  unresolved frontier without adding unread, priority, assignment, action, or
  proof state. The mailbox does not mirror authored observations; it projects
  current messages from retained admitted-application polls beside runtime
  attention and passive-revalidation failures. The first provider-specific
  path is a bounded `gh` poll for unread GitHub notifications and comments on
  their pull requests. `rey channels poll` verifies one tick directly, while
  `rey agent` supervises the same contract at the committed application
  cadence. Following a retained GitHub mailbox evidence link requests one
  immediate exact poll; that retained receipt resets the supervisor to the
  admitted steady-state cadence. The composer creates an Observation, never a
  Journal entry.
  Selected exact unresolved observations can seed a deterministic
  unretained Journal proposal; only ordinary Journal admission retains it.
  Current action cells project as authored-only opportunities. One narrow
  retained-observation query can cross separate read-only admission and
  execution, but its bounded frame/delta enters the Journal only through an
  ordinary superseding entry. Workload inspection descends from the retained
  scenario index into content-addressed scenario-execution and directed-delta
  routes; those browser views preserve the CLI's plain, `-v`, and `-vv`
  evidence layers without reevaluating a result or granting runtime authority.
  Conversation sessions and messages are a separate bounded workspace-local
  transcript with declared writers and no delivery or execution claim. The
  browser projects that same transcript and enables append only for an exact
  session-declared human browser writer.
- **Agents inspect and propose.** The `rey` CLI exposes high-fidelity status,
  diff, add, reset, commit, log, generation, qualification, and execution
  surfaces without requiring implementation-code inspection.
- **The runtime evaluates.** Deterministic contracts bind exact inputs,
  operations, capabilities, scenarios, deltas, budgets, omissions, and proof
  lineage. An agent cannot qualify its own proposal.

Explorer is a high-dimensional projection engine, but its visual grammar is
familiar: globe, atlas, terrain, weather, waterways, regions, roads, points of
interest, and construction. These are semantic instruments rather than Earth
claims. One reversible projection keeps surface and attached features together
as the world moves from globe to map to local terrain. Camera movement changes
the lens, never source identity; atmosphere, light, material, and interaction
are correct only when they preserve that continuous bearing.

The `rey editor` CLI is the level-editor side of the same architecture. It can
generate native terrain and feature artifacts from tunable hyperparameters,
let an agent fine-tune them in WORKING, freeze exact objects in INDEX, and
commit candidate scene packages. Those packages still require a separate
qualified admission before Explorer may treat them as world fabric.

## The Runtime Loop

```text
                    explicit environment boundary
                 workspace · Git · tools · runtimes
                                  │
                       inventory capabilities
                                  │
                    ┌─────────────┴─────────────┐
                    ▼                           ▼
           relational mining              source mining
       query · group · traverse       search · parse · index
                    └─────────────┬─────────────┘
                                  ▼
                bounded projections and native evidence
                                  │
                     compare SOURCE → TARGET
                                  │
                     typed and native deltas
                                  │
               ┌──────────────────┴──────────────────┐
               ▼                                     ▼
       unresolved frontier                      scoped proof
               │                               and lineage
               ▼
       bounded work selection
               │
               ▼
      mine exact relevant evidence
               │
       delta-directed reasoning surface
               │
               ▼
     propose → admit → probe or mutate
               │
               ▼
        observe and compare again
```

The delta is not merely a report produced after work finishes. It is a runtime
control signal. Changed rows, spans, symbols, edges, metrics, claims, or
capabilities invalidate dependent observations. Unresolved differences become
a frontier. Rey mines the evidence needed to orient on selected frontier work,
then measures whether the next admitted action actually reduced the residual
delta.

```text
delta → frontier → schedule → mine → reason → propose → act → observe → delta
```

That inner loop sits inside an ongoing portfolio loop. Workloads mine their
declared domains while Rey mines workload, result, environment, dependency,
capability, ownership, and coverage facts to derive attention:

```text
catalog + results + environment + acknowledged Git + coverage
  → portfolio snapshot → workload attention
  → RETEST | REFINE | CREATE | BLOCK | POLICY_EXCLUDED
  → admitted work → test → observe portfolio again
```

Process completion alone is not progress. A command may exit successfully
while the semantic frontier is unchanged or worse. Reaching a limit, losing
evidence, or encountering an unsupported parser is not convergence; it is an
explicit incomplete or inconclusive result.

## Admission Is The Shared Grammar

Rey's environment, workload, and editor surfaces use a Git-shaped local loop:

```text
HEAD → INDEX → WORKING
```

`status` observes the two directed deltas. `diff` opens them. `add` alone
changes INDEX. `commit` records only the already reviewed and verified INDEX;
it does not re-observe ambient state. History admission records evidence. It
does not grant a tool, workload, agent, or scene permission to act. Environment
patch admission offers only currently available applications; unresolved
application searches remain unstaged, while full-snapshot admission can still
retain their explicit degradation evidence.

Repository activation uses a separate explicit evidence loop: `rey git init`
retains a baseline and any exact `--watch-ref refs/...` scope, `poll` retains
one typed HEAD/watched-ref/reachability/path/index transition and proposal set,
and `ack` advances the cursor only from that exact evidence. These commands
never mutate Git or execute a proposed workload. `rey git watch` repeats the same
observation under explicit iteration, cadence, and elapsed bounds, retaining
every successful or failed tick and its terminal receipt. Retryable failures
may recur only under the explicit retry bound; recovered runs remain partial,
and cancellation stops cooperatively at a retained boundary. A changed
transition still stops the watch and requires exact `ack`. `rey workloads
admit-activation` then applies
the ordinary workload preconditions and retains scheduling eligibility only.
`rey workloads execute-activation` revalidates those exact inputs, evaluates
only the admitted scenarios under the retained evidence budget, and records a
replay-stable result without mutating Git or replacing full-suite
qualification. Compatible proposals from the same retained Git transition can
cite that exact result without rerunning the graph; stricter budgets and
changed inputs never coalesce. `rey workloads verify-activation` separately
recomputes the complete declared suite under the same frozen capability
snapshot, compares the selected evidence exactly, and retains a bounded proof
without changing qualification.

A workload is Rey's public unit of computation: one versioned graph, scenario
suite, policy boundary, qualification contract, and total budget. Agents,
rules, and humans submit revisions through the same validated interface.
Deterministic scenarios decide qualification, and failing results remain
directed typed deltas that can select the next bounded work.

The default workspace catalog is `sys/`. A package such as
[`sys/context-anchor-survey/`](sys/context-anchor-survey/) is visible in
WORKING before it is admitted. Its file state—not a hidden service—is the
collaboration boundary.

## Evidence Before Authority

Rey remains useful from local evidence and keeps several boundaries explicit:

- discovering an executable does not grant permission to invoke it;
- locating a source does not grant permission to read or mutate it;
- admitting an observation does not admit an action;
- a renderer cannot turn interpolation into surveyed truth;
- a successful process cannot declare semantic convergence; and
- an agent cannot declare its own proof successful.

Every mined artifact binds its request, exact inputs, operation and
implementation revision, capability snapshot, effective limits,
completeness, omissions, and derivation lineage. Typed relational deltas,
native text deltas, and structural deltas keep SOURCE and TARGET explicit. A
zero delta proves agreement only inside those declared bounds.

## Foundations

Start with the [Documentation Index](docs/README.md); it is the reference map
for Rey's foundational contracts, interfaces, current decision plane, and
active implementation plans. The key bearings are:

- [Constitution](CONSTITUTION.md) — durable values and invariants.
- [Architecture](docs/ARCHITECTURE.md) — ownership, planes, data flow, and
  security boundaries.
- [Explorer](docs/EXPLORER.md) — first principles and the globe → map → terrain
  fidelity standard for the evidence-bound spatial engine.
- [CLI](docs/CLI.md) — the agent-facing interface and
  `HEAD → INDEX → WORKING` philosophy.
- [API](docs/API.md) — the Axum HTTP surface, Swagger/OpenAPI discovery,
  transport contract, routes, exposure, and authority.
- [Mining](docs/MINING.md) — bounded relational and source evidence.
- [Workloads](docs/WORKLOADS.md) — graphs, scenarios, qualification,
  admission, and execution.
- [Runtime](docs/RUNTIME.md) and [Frontier](docs/FRONTIER.md) — deterministic
  transitions, attention, progress, scheduling, and convergence.
- [Interfaces](docs/INTERFACES.md) — the high-level map between CLI, API,
  browser, evidence, provider, policy, and persistence boundaries.

Contributors should also read [Contributor Instructions](INSTRUCTIONS.md).
Accepted architectural choices are projected in the
[Current Decision Plane](docs/decisions/README.md), and executable delivery
slices live in [Current Plans](plans/README.md).

## Development

Rey uses a pinned Nix development shell, a Cargo workspace, a root pnpm and
Turborepo monorepo with the TypeScript UI as its first package, Nextest, and
cargo-dist. The normal qualification path is:

```sh
nix develop
just setup
just check
just test
```

See [Development](docs/DEVELOPMENT.md) for the complete toolchain and
[Releases](docs/RELEASES.md) for tagged publication.
