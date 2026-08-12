# ADR 0040: Workspace Channels And Operator Index

- Status: Accepted
- Date: 2026-08-10
- Extends: [ADR 0032](0032-seed-discovery-survey-and-live-communications.md), [ADR 0037](0037-explore-bound-collaboration-journal.md), and [ADR 0039](0039-bounded-operator-feed.md)
- Supersedes: ADR 0039's URL-only Feed-layout retention boundary and ADR 0037's
  use of Journal entries as the low-latency agent-finding surface

## Context

Feed now composes bounded Signals, Admission, and Flow lenses, but its stream
order, names, and filters live only in the browser URL. That makes one layout
deep-linkable without making the operator's work durable or reviewable. It also
leaves Feed streams, the runtime mailbox, standalone frontier observations,
Journal documents, conversation, and future agent communication adjacent in
the UI without a common addressing and routing concept.

Persisting a visual layout in the environment admission index would be wrong.
Operator collaboration state is not environment evidence and must not stale a
workload, proof, or capability snapshot. Treating Feed as a universal event log
would also be wrong: Git time, Journal admission order, environment sequence,
and workload results retain different clocks and owners.

## Decision

Rey introduces **Channels** as the workspace-local collaboration substrate.
The concepts remain distinct:

- a **Channel observation** is one compact immutable frontier statement,
  retained independently from the Journal. It binds an exact subject locator,
  kind, statement, desired delta or frontier relation, evidence locators,
  source revision, self-asserted author label, completeness/limits, and any
  exact supersedes or resolves relations;
- a **Journal entry** is a deliberate rich notebook synthesis that may cite and
  compose exact Channel observations but does not own or replace them;
- a **channel** is a stable addressable collaboration boundary with an exact
  semantic revision and declared local broadcast posture;
- a **channel admission** associates one Channel-observation identity with one
  channel at a channel-local sequence position without copying the observation
  or granting action authority;
- a **subscription** is a bounded selection over one or more channels and
  observation kinds;
- a **Feed stream** is an ordered human projection of one subscription plus a
  visual lens; it is not itself a channel;
- a **broadcast** admits one observation identity to an explicit set of
  channels; and
- a **relay** is an explicit provider-backed edge that forwards admitted
  observations while retaining origin, hop, cursor, attempt, and delivery
  lineage.

Channel definitions, subscriptions, Feed streams, ordered layouts, and relay
declarations form a bounded **channel graph**. Observation content and
channel-admission history are separate append-only collaboration records; they
are not configuration nodes in that graph. Broadcast admission to several
channels does not establish
causal or total order between their channel-local sequences. A channel message
does not become a workload action, assignment, proof, authenticated statement,
or provider invocation merely by being admitted or relayed.

The current **Channel frontier** is a deterministic bounded projection of
retained Channel observations that have not been resolved or superseded by an
exact later relation. Observations never acquire a mutable `resolved` flag.
Progress, resolution, correction, and handoff remain new immutable
observations so a catching-up human or agent can reconstruct how the frontier
moved. A Channel observation may cite a runtime observation, frame, delta,
workload result, Journal block, or Explorer coordinate as evidence; it is a
collaboration statement over those artifacts, not a second owner for them.

### Workspace-local revision loop

Channel topology and Feed composition use a separate Git-shaped local revision
loop:

```text
CHANNEL HEAD → CHANNEL INDEX → CHANNEL WORKING
```

- `WORKING` is the immediately autosaved operator/agent proposal. Dragging,
  renaming, tuning, adding, or removing a Feed stream changes this plane so a
  page reload preserves the work.
- `INDEX` is the reviewed selection staged for admission.
- `HEAD` is the last immutable committed channel-graph revision.

Every Feed stream has a stable logical identity independent of its name,
position, filters, subscription, and content revision. Reordering changes an
ordered layout relation; it does not replace stream identity. Channel layout is
retained collaboration state but remains presentation-only with respect to
runtime, workload, environment, and proof semantic identities.

Channel observations and channel-local admission sequences do not pass through
`CHANNEL INDEX`. They are high-cadence immutable records admitted through their
own validator and local log. This keeps a new finding from dirtying topology
configuration and keeps a stream rename from changing observation identity.

The implemented `streams` URL remains a portable deep-link projection. An
explicit URL composition is a detached preview and does not silently overwrite
workspace `WORKING`. The first deliberate edit may adopt the complete preview
as `WORKING`. Without an explicit URL composition, Feed resolves `WORKING`,
then `HEAD`, then the built-in default.

Rey currently has no authenticated user identity. The first index is therefore
one workspace-local operator index, not a verified personal preference store or
multi-user profile. Any later profile namespace must remain self-asserted until
an identity and authorization contract exists.

### CLI and UI boundary

The agent-facing topology command family is `rey channels`:

```text
rey channels list
rey channels status
rey channels diff [--staged]
rey channels apply <channel-graph.yaml>
rey channels add [-p]
rey channels commit -m <message>
rey channels log [-p] [-n <count>]
```

`apply` writes a validated `WORKING` proposal; `add` stages all or selected
typed changes; `commit` admits exactly `INDEX`; and `log -p` reopens retained
directed deltas. No-news output and human/structured/exit-code behavior follow
the established Git-shaped environment loop without sharing its store.

The low-latency frontier surface is separate:

```text
rey observations add <observation.yaml> [--channel <locator>]... [--no-broadcast]
rey observations list [--channel <locator>]
rey observations show <observation-id>
rey journal seed <observation-id>...
```

`observations add` validates and retains one standalone immutable observation.
Unless targets or `--no-broadcast` are explicit, it resolves the same visible,
bounded local `broadcast_default` set used by Feed. `list` projects the current
frontier and its exact resolution/supersession lineage; `show` exposes the full
bounded statement and evidence links. `journal seed` deterministically projects
selected exact observations into a Journal-entry proposal. It does not retain
a Journal entry; normal `rey journal add` or `/journal/new` submission remains
the admission boundary.

The CLI and typed local store must exist before browser persistence is called
complete. The UI may then expose the same working-state mutation through a
bounded endpoint started by `rey ui`. Stream headers become pointer-draggable,
while existing move controls remain the keyboard-accessible equivalent. Every
successful drop writes the complete validated order to `WORKING`; failed writes
restore the last retained order and report through the communication plane.

Because the local listener has no authentication, a channel-working write is
self-asserted local operator input. Non-loopback exposure must report that
reachable clients can replace workspace-local channel `WORKING`. It never gains
commit, relay, compute, or action authority.

### Observation broadcast, Journal seeds, and relay

Feed's “Share observation” authors one standalone Channel observation, not a
Journal entry. The proposal exposes the exact target channel locators. The
default target set is the visible, bounded set of local channels declaring
`broadcast_default: true`, not an implicit remote audience. Admission retains
one observation plus channel-local edges rather than duplicating content per
recipient. An observation remains valid and addressable without a channel
admission; any failed or partial fan-out must be reported explicitly and may
not be presented as complete broadcast.

An agent that cannot justify a full Journal entry can still leave a compact
finding, question, progress note, blocker, or handoff. A later operator or agent
may select those observations and open
`/journal/new?observations=<id>,<id>` or run `rey journal seed`. The seed cites
the exact observation identities and source revisions and may arrange them
into notebook blocks, but it is an unretained proposal until ordinary Journal
admission. Rey therefore helps someone catch up without automatically stacking
up authored Journal documents or pretending generated synthesis was reviewed.

Relay is a later provider contract, not part of the first local index slice. A
relay must freeze exact source and target channel locators, accepted kinds,
filters, provider capability and revision, authority, cursor, hop limit,
idempotency key, payload digest, attempt outcome, and omissions. Loop prevention
uses observation origin plus destination and a hard hop bound. Remote
documents and streams remain provider-owned; Rey records public bindings and realized
lineage rather than implementing competing durable transport.

## Implementation Status

The first topology anchor implements `rey.channel-graph.v1`, canonical graph
snapshots and semantic deltas, the no-write built-in workspace graph, and a
symlink-safe, locked, atomically published `CHANNEL WORKING` proposal. Agents
can inspect it through `rey channels list`, `status`, and `diff`, then apply a
bounded workspace-contained YAML graph through `rey channels apply`. Human
patches name semantic stream operations; structured output retains exact
source, graph, limit, and delta identities.

`CHANNEL HEAD`, `CHANNEL INDEX`, `add`, `commit`, `log`, staged diff, browser
layout persistence, standalone observations, channel-local admissions, Journal
seeds, and relay execution are not implemented by that anchor.

## Consequences

- Feed layout work survives reload immediately while admission remains an
  explicit `status → diff → add → commit → log -p` loop.
- Feed, mailbox, conversation, standalone observations, and Journal can share
  channel addressing without becoming the same interface or inventing a global
  event clock.
- Compact observations retain the collaboration frontier; Journals synthesize
  and cite them when deeper reasoning is worth preserving.
- The channel graph owns topology and composition, the observation log owns
  content, channel-local logs own routing/admission edges, and subscriptions
  own selection intent.
- URL layouts stay shareable without silently changing workspace state.
- Draggable columns require stable stream identities, typed reorder deltas,
  rollback on failed persistence, and an accessible non-pointer control.
- Broadcast is local and explicit by default; relay transport, remote identity,
  delivery guarantees, authentication, and multi-user profiles remain later
  contracts.
