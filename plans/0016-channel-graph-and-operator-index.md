# Plan 0016: Channel Graph And Operator Index

- Status: Active
- Decision: [ADR 0040](../docs/decisions/0040-workspace-channels-and-operator-index.md)
- Extends: [Plan 0011](0011-local-operator-ui.md)

## Outcome

Make Rey's collaboration plane addressable, persistent, reviewable, and ready
for agentic networking without turning Feed into a global event log. Deliver a
workspace-local channel graph with a Git-shaped `HEAD → INDEX → WORKING` loop,
then bind Feed composition and draggable headers to that same typed state.
After the local loop is proven, admit one standalone Channel observation to the
bounded default local channel set and derive the current collaboration frontier
from unresolved observations. Journal entries remain deliberate synthesis;
selected observations can seed an unretained Journal proposal for someone
catching up. Relay execution remains a separately gated provider bearing.

## Completion Checklist

- [x] Accept ADR 0040 and distinguish channels, observations, admissions,
  subscriptions, Feed streams, broadcasts, and relays.
- [x] Define bounded canonical v1 contracts for channel definitions,
  subscriptions, stable Feed streams, ordered layouts, relay declarations,
  graph snapshots, effective limits, and typed semantic deltas.
- [ ] Define the separate Channel-observation, admission, frontier, and Journal
  seed contracts.
- [x] Implement a symlink-safe workspace-local `WORKING` store with locking,
  atomic publication, tamper detection, deterministic replay, and explicit
  local-only guarantees.
- [ ] Extend the local store with verified immutable `HEAD` and staged `INDEX`
  revisions without moving observation history into the topology index.
- [x] Implement `rey channels list`, `status`, `diff`, and `apply` with compact
  human output and machine-clean structured output.
- [ ] Implement `rey channels add`, `commit`, and `log` plus staged diff with
  the complete Git-shaped admission loop.
- [ ] Implement the separate low-latency `rey observations add`, `list`, and
  `show` surface over an immutable local observation/admission log; adding an
  observation must not dirty `CHANNEL WORKING`.
- [x] Make empty/default channel state useful without writing files and keep
  layout revision identity out of environment, workload, runtime, and proof
  identities.
- [ ] Add a bounded UI read/write projection over the same store, with exact
  exposure and unauthenticated-write warnings.
- [ ] Give every Feed stream a stable logical identity and resolve layouts in
  `URL preview → WORKING → HEAD → built-in default` order without silently
  adopting a shared URL.
- [ ] Make stream headers draggable across columns, retain the existing move
  controls for keyboard access, autosave successful order changes to `WORKING`,
  and roll back/report failed writes.
- [ ] Make “Share observation” admit one standalone observation identity and
  broadcast it to the visible local `broadcast_default` set without creating a
  Journal entry.
- [ ] Implement `rey journal seed <observation-id>...` and
  `/journal/new?observations=...` as deterministic unretained proposals that
  cite exact observation identities and revisions before normal Journal
  admission.
- [ ] Project channel admissions into Feed, mailbox, and conversation surfaces
  without conflating their interaction or order semantics.
- [ ] Define relay declarations, delivery attempts, cursor/hop/idempotency
  lineage, loop prevention, provider authority, and explicit non-guarantees;
  do not implement remote transport in this plan without a separate accepted
  provider decision.
- [ ] Update foundational docs, glossary, examples, UI grammar, and plan proof
  with public behavior as each slice lands.

## Contract Sequence

### 1. Channel graph vocabulary

Freeze the smallest contracts that preserve identity through UI changes:

- `rey.channel.v1`: stable channel id, semantic revision, name, scope, accepted
  observation kinds, and `broadcast_default` posture;
- `rey.channel-observation-proposal.v1`: exact subject locator, bounded kind and
  statement, desired delta/frontier relation, evidence locators, source
  revision, author label, completeness/limits, and resolution/supersession
  relations;
- `rey.channel-observation.v1`: the validated immutable observation plus stable
  content identity and admission lineage;
- `rey.channel-admission.v1`: channel id/revision, observation locator and
  digest, channel-local sequence, author label, admission identity, and bounds;
- `rey.channel-subscription.v1`: stable subscription id, channel locators,
  kinds, filters, completeness, and limit;
- `rey.feed-stream.v1`: stable stream id, name, subscription, visual lens, and
  stream revision;
- `rey.feed-layout.v1`: ordered unique stream ids plus layout revision;
- `rey.channel-graph.v1`: canonical channel, subscription, stream, layout, and
  relay definitions with exact semantic revisions;
- `rey.channel-graph-snapshot.v1`: graph identity, exact source binding,
  effective limits, and tamper-detecting snapshot identity;
- `rey.channel-working.v1`: the exact built-in base graph identity plus one
  validated WORKING snapshot;
- `rey.channel-graph-delta.v1`: directed semantic operations between exact
  graph identities; and
- `rey.channel-frontier.v1`: a bounded deterministic projection of unresolved
  observations with exact resolution/supersession lineage and omissions.

Presentation order and names participate in channel-layout revisions but never
in workload, observation, proof, or channel semantic identity. Observation
content and channel-local admission records are not part of graph WORKING or
INDEX. Duplicate ids, dangling subscriptions, repeated order entries, invalid
channel targets, and limit overflow fail before a working proposal is accepted.

### 2. Local revision store

Implement the same human mental model as the environment loop without sharing
its types or state. `WORKING` may change frequently and survives process/page
restart. `INDEX` is bound to exact `HEAD`; stale staging fails. `commit` retains
exactly `INDEX` and never re-reads ambient browser state. Every commit carries a
monotonic local sequence, parent identity, date, message, graph identity, and
replay-verifiable directed delta.

The store claims local atomic replacement and lock coordination only. It does
not claim `fsync`, authenticated ownership, multi-user merge, remote durability,
or remote stream semantics.

### 3. Agent CLI

Close the agent verification loop before browser writes:

```text
rey channels list
rey channels status
rey channels diff
rey channels apply path/to/channel-graph.yaml
rey channels add -p
rey channels diff --staged
rey channels commit -m "Arrange local attention"
rey channels log -n 3 -p
```

`list` is the high-fidelity inventory of channels, subscriptions, streams, and
broadcast posture. `status` is compact and quiet when clean. `diff` separates
`HEAD → INDEX` from `INDEX → WORKING`. Patch admission shows semantic stream
operations such as `move`, `rename`, `retarget`, `add`, and `remove`, never raw
serialized state or opaque provenance blobs.

The observation CLI is optimized for leaving a bounded frontier marker without
writing a notebook:

```text
rey observations add path/to/observation.yaml
rey observations add path/to/handoff.yaml --channel local://operators
rey observations list --channel local://operators
rey observations show <observation-id>
rey journal seed <observation-id>... > journal-proposal.yaml
```

Observation admission is append-only and independent of `CHANNEL INDEX`.
`list` separates unresolved, resolved, and superseded rows and reports source
bounds and omissions. `journal seed` performs no write and emits a proposal
that cites every selected observation and source revision.

### 4. Feed persistence and drag

Add GET plus bounded working-state write operations to the explicitly started
operator listener. The response exposes HEAD/INDEX/WORKING identities,
effective source, dirty/staged counts, limits, omissions, and write authority.
The same core validator and store operation serve UI and CLI.

Dragging begins only from a stream header/handle, shows a clear insertion
target, preserves independent vertical scroll, and cannot accidentally trigger
post selection. Pointer drop and keyboard move produce the same complete
ordered layout proposal. The UI updates optimistically but considers the move
saved only after the server returns the exact new WORKING identity.

### 5. Broadcast one observation and seed one Journal

Admit one compact observation independently of Journal. Resolve and display the
exact bounded default local target set before submission. Retain one
observation and N channel-admission edges; a zero-target observation remains
addressable, while any partial fan-out is an explicit outcome rather than a
silent claim of broadcast success.

Channel admission never executes an action, assigns an agent, qualifies proof,
or turns a cited runtime artifact into collaboration-owned evidence. Feed and
mailbox revalidation surface the admitted observation through their
subscriptions. Then project selected observations into one unretained Journal
seed and prove that normal Journal admission alone creates the entry.

### 6. Relay boundary only

Specify, validate, and render relay declarations and retained attempt evidence.
Do not start a remote client, poll a remote cursor, or claim delivery in this
plan. A later provider slice must prove one exact local-to-provider path,
deduplication, restart, cursor replay, loop prevention, capability drift, and
honest delivery guarantees.

## Proof Matrix

| Surface | Required proof |
| --- | --- |
| Contracts | Canonical ordering, stable ids across rename/move, observation identity, immutable resolve/supersede lineage, duplicate and dangling-reference rejection, hard limits, JSON replay, tamper detection |
| Store | Missing state, HEAD/INDEX/WORKING transitions, stale index, symlinks, concurrent lock, atomic publication, corrupted parent/delta, restart |
| CLI | stdout, stderr, JSON, exit codes, clean/dirty/staged states, partial add confirmation, bounded log, no-news commit, provenance-safe patches |
| UI | reload retention, URL detached preview, deliberate adoption, pointer drag, keyboard move, insertion feedback, failed-write rollback, passive revalidation without scroll reset |
| Observations | standalone admission, source/evidence binding, unresolved frontier derivation, correction, resolution, supersession, bounded list/show, no CHANNEL-WORKING drift |
| Broadcast | zero/one/many default targets, invalid target, idempotent replay, one observation with N edges, explicit partial outcome, no action authority |
| Journal seed | one/many exact observations, deterministic proposal, source-revision citations, deep links, no implicit Journal retention or execution |
| Relay contract | source/target drift, duplicate delivery identity, hop bound, loop rejection, cursor replay, provider absence, explicit unsupported transport |

## Completed Anchor — Topology WORKING

Implement the contract and CLI-only vertical slice for one built-in local
channel, one subscription, and the current three-stream default layout:

1. freeze stable ids and canonical `rey.channel-graph.v1` plus typed layout
   delta;
2. derive the built-in default without writing local state;
3. accept one bounded YAML graph into `WORKING` through
   `rey channels apply`;
4. expose the exact change through `channels status` and `channels diff`; and
5. prove restart, invalid references, duplicate ids, tampering, symlink
   rejection, human output, structured output, and exit codes.

This anchor now gives the agent a high-fidelity CLI surface for the same facts.
The built-in graph is available without local state, while `apply` validates a
workspace-contained YAML graph and atomically retains only `CHANNEL WORKING`.

## Next Concrete Anchor

Implement one standalone `rey.channel-observation.v1` admitted through
`rey observations add`, visible in `list` and `show`, broadcast to the built-in
workspace channel, and projected into an unresolved Channel frontier. Adding
the observation must leave `CHANNEL WORKING` clean. Browser “Share observation”
and Journal seeding follow only after that same observation is inspectable
through the CLI.
