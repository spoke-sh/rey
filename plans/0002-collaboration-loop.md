# Plan 0002: Close The Collaboration Loop

- Status: Active
- Owns: Channel operator projection, standalone observations, Journal seeding
  and opportunities, scenario/delta routes, and conversation authority

## Outcome

Make collaboration state addressable from both CLI and browser without
collapsing Feed, Channel messages, Journal synthesis, mailbox history, and
conversation into one store or granting document admission execution
authority.

## Current Boundary

The Channel CLI implements a complete topology `HEAD → INDEX → WORKING` loop,
immutable file-backed messages, explicit direct relay through an admitted
environment application, and bounded one-shot polling beacons. Feed is a
read-only URL-composed projection. Journal v2 retains immutable broadsheet
documents and superseding revisions. The browser does not yet read or write
Channel topology, no standalone observation/frontier contract exists, Journal
action cells are inert, exact scenario/delta routes are absent, and the footer
conversation composer correctly remains disabled.

## Completion Checklist

### 1. Project current Channel state

- [ ] Add bounded UI reads and explicit WORKING writes over the same Channel
  validator/store, with exact listener exposure and unauthenticated-write
  warnings.
- [ ] Resolve Feed layouts in `URL preview → WORKING → HEAD → built-in`
  order; keep URL layouts detached until deliberately adopted.
- [ ] Support pointer drag and keyboard movement over stable stream identities,
  report the resulting semantic delta, and roll back failed writes.

### 2. Admit one collaboration observation

- [ ] Define immutable observation, channel-admission, resolution,
  supersession, and bounded frontier contracts separately from Channel graph
  INDEX and from Journal documents.
- [ ] Implement high-fidelity CLI add/list/show paths, exact evidence/source
  bindings, idempotent broadcast to an explicit bounded local target set, and
  typed partial fan-out outcomes.
- [ ] Project unresolved observations into Feed and mailbox without inventing
  a global clock, unread state, assignment, action, or proof authority.

### 3. Bridge observations and Journal deliberately

- [ ] Implement deterministic unretained Journal seeds from selected exact
  observation identities in the CLI and `/journal/new`.
- [ ] Require ordinary Journal validation and admission before retaining a
  seed; never duplicate an observation into the Journal automatically.
- [ ] Derive unsuperseded action cells as explicit authored opportunities on a
  reasoning surface, then use Plan 0001's normal workload/policy admission
  boundary if an opportunity becomes runtime work.
- [ ] Execute one admitted read-only query separately and append its bounded
  frame/delta only as a superseding Journal entry.

### 4. Complete operator evidence routes

- [ ] Add exact scenario and directed-delta browser routes that retain CLI
  plain/`-v`/`-vv` evidence layering without independent assessment.
- [ ] Keep every Git SHA link, semantic coordinate, Journal fragment, source
  revision, omission, and limit exact through those routes.

### 5. Admit conversation only with a transport contract

- [ ] Define participant, session, message, ordering, retention, read/write
  authority, transport availability, and failure contracts before enabling the
  composer.
- [ ] Keep mailbox history and the conversation transcript separate and leave
  send visibly disabled whenever the transport is unavailable.

### 6. Qualify the slice

- [ ] Prove CLI/browser parity, restart/tamper behavior, stale topology,
  observation correction, partial broadcast, Journal seed identity, inert
  blocks, failed writes, passive revalidation, accessibility, and bounds.
- [ ] Pass focused Rust/browser tests, `just check`, `just test`, the embedded
  UI build, and the packaged Nix path.

## Deferred

Remote inbox cursors, resident daemons, multi-user identity, authentication,
TLS, generalized relay providers, remote stream durability, and a public Rey
service remain outside this plan.
