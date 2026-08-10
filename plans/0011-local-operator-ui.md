# Plan 0011: Local Operator UI

- Status: Active
- Decisions: [ADR 0025](../docs/decisions/0025-local-operator-ui.md),
  [ADR 0026](../docs/decisions/0026-context-topology-explorer.md),
  [ADR 0030](../docs/decisions/0030-operator-cadence-agents-and-explorer-coordinates.md),
  [ADR 0032](../docs/decisions/0032-seed-discovery-survey-and-live-communications.md),
  [ADR 0034](../docs/decisions/0034-agent-runtime-inventory-and-derived-task-plane.md),
  [ADR 0035](../docs/decisions/0035-agent-recommendations-and-observed-work.md),
  [ADR 0036](../docs/decisions/0036-cadence-repository-state-and-publication.md)

## Outcome

Raise Rey's collaboration language from sequential command documents to a
persistent, high-fidelity operator environment. Humans spend their normal Rey
time in the UI; agents use the workload CLI as their primary interface, and
humans descend to it for exact diagnosis. Typed runtime evidence remains
authoritative:

```text
rey ui
rey ui --host 0.0.0.0 --port 5714
```

The application is an embedded TanStack Router application using Hifi's
Kinetic grammar with the Precision theme. `/explore` is the default context-
topology canvas. It reads the same bounded workload portfolio projection as
`rey workloads list`; it does not mutate state or create a second scheduler.
`/cadence` keeps observable clocks partially ordered. `/agents` ranks current
system recommendations and summarizes observed work from retained evidence;
agent application discovery remains in Environment. Matrix-style Explorer
coordinates make exact topology records shareable.

## Completion Checklist

- [x] Accept ADR 0025 and bound the UI as a local read-only operator projection,
  not a public or durable Rey service.
- [x] Add `rey ui` with loopback defaults, configurable IP/port, ephemeral-port
  support, machine output, exact startup evidence, and a non-loopback warning.
- [x] Serve embedded SPA assets, deep-link fallback, health, and live workload
  portfolio endpoints through a bounded synchronous server.
- [x] Build portfolio, workload-list, and workload-detail routes with TanStack
  Router and direct Kinetic/Precision grammar imports.
- [x] Pin exact Hifi Core and Kinetic Git packages without relying on an
  ambient sibling checkout, copied source, or an unbounded dependency script.
- [x] Track exact Hifi head `058c650`, including its typed dense-table
  alignment surface, material color properties, directional Kinetic control
  travel, and layered lighting, and keep every authored application rule in
  StyleX modules using the official extraction plugin and CSS layers.
- [x] Cover frontend derivation, HTTP routing/method safety, human startup
  output, structured output, stderr, and exit behavior with focused tests.
- [x] Complete full UI, workspace, Nix package, and manual browser verification.
- [x] Make `/explore` the default human entry and hard-cut Instrument to
  Environment at `/environment`.
- [x] Formalize context topology, canvas, semantic lens, regime, neighborhood,
  focus, relationship, and omission as explicit read-model/React concepts.
- [x] Implement bounded landscape, neighborhood, and object projections with
  typed identity retention and classified edges.
- [x] Add pointer/keyboard semantic zoom, pan, selection traversal, fit, and
  native full-screen canvas behavior.
- [x] Fit Explore within `100dvh` and remove route-level document scrolling so
  wheel input has only the semantic-lens meaning.
- [x] Remove manual Refresh and passively revalidate the read-only portfolio at
  a reported 5000 ms interval.
- [x] Keep passive portfolio and environment revalidation in mounted projection
  state so polling cannot remount the active route or reset viewport position.
- [x] Reduce `/environment` to three full-width stacked evidence sections:
  directed text, bounded search, and the reference plane.
- [x] Bind the footer to the exact Rey implementation revision and link the
  complete Git object id without confusing semantic BLAKE3 identities for
  source commits.
- [x] Enforce one browser invariant for every known Git SHA: the displayed SHA
  itself links to the exact GitHub commit, while unbound repositories and
  non-Git identities remain explicit.
- [x] Cover root redirect, both top-level routes, lens ordering, bounds,
  identity preservation, and folded-evidence disclosure.
- [x] Add `/cadence` with bounded reachable Git commits, verified Rey
  environment admissions, explicit scan contracts, partial ordering, and
  visible omissions.
- [x] Lead `/cadence` with independent working-tree and local-upstream
  publication instruments, and classify bounded commit ticks against the exact
  retained upstream revision.
- [x] Replace the provenance-derived `/agents` registry with current bounded
  tasks, derived workflow operations, and the process-owned major agent-runtime
  desired/found/missing inventory.
- [x] Move runtime inventory back to Environment and restore `/agents` as dense
  recommendation and work-ledger tables over authoritative portfolio evidence.
- [x] Formalize canonical matrix-style Explorer coordinates with unique
  unordered dimensions, exact `at` bindings, agent roles, route tests, and
  visible stale/missing resolution.
- [x] Keep the coordinate rail fixed beneath the header and advance its exact
  numbered section coordinate as the operator scrolls through a route.
- [x] Make the global footer a fixed live communications channel with a typed
  attention mailbox, subtle count, quiet state, revalidation failures, sliding
  bottom sheet, and exact implementation revision.
- [x] Separate the footer communication controls: mailbox selects history,
  chevrons select operator/Rey/agent conversation, and Escape closes either.
- [x] Dismiss either communication axis by clicking the background while
  preserving interaction inside the bottom sheet.
- [x] Render the conversation axis with a traditional transcript and composer
  while explicitly disabling sends when no transport is admitted.
- [ ] Define and implement the agent-visible conversation CLI/API contract,
  participant and session identities, message admission, bounded retention,
  and exact read/write authority.
- [ ] Add exact scenario/delta routes and preserve CLI `-v`/`-vv` evidence
  layering in the visual projection.

## Current Proof

The frontend derives portfolio totals, workload journey, scenario progress,
attention, provenance, and mining evidence from `rey.workload-list.v5`. Rust
integration tests start a real ephemeral listener and verify the embedded app,
SPA fallback, health, live catalog response, browser headers, rejected writes,
loopback startup document, structured descriptor, and non-loopback warning.
Its authored presentation is fully StyleX-extracted: no handwritten
application CSS remains, while Kinetic material values stay typed and dynamic.

Captured on 2026-08-09:

```text
nix develop path:$PWD --command just check
# frontend formatting/typecheck, 12/12 UI tests, Vite build, Rustfmt, Clippy,
# and flake evaluation passed
nix develop path:$PWD --command just test
# 136/136 Rust tests, 12/12 UI tests, and all documentation tests passed
nix develop path:$PWD --command just build
# deterministic UI assets and the complete Rust workspace built
nix build path:$PWD#rey --no-link
# the filtered, locked package containing embedded UI assets built successfully
```

A real `rey ui --host 127.0.0.1 --port 0 --format json` listener reported its
exact ephemeral URL, Kinetic grammar, Precision theme, pinned grammar revision,
loopback exposure, and read-only authority. Live health and workload requests
returned `rey.ui-health.v1` and this workspace's `rey.workload-list.v5`.
Isolated Chromium captures verified both `/` and `/workloads` at 1600×1200;
the initial environment rendered live qualification, scenario, run, coverage, attention,
catalog, graph, and evidence data without console/server failure.

After Hifi's StyleX migration on the same date, the upstream checkout was
initially verified at `5874cdf`, every vendored core/Kinetic source was diffed
against that head, and the UI was rebuilt without a handwritten stylesheet.
The pin was later advanced to `0440cfe`; the two changed Kinetic component
sources are exact upstream copies, while the remaining vendored sources and MIT
license are byte-identical. The server test opens the embedded `app.css` and
proves extracted StyleX priority layers.
Desktop portfolio/workload captures and a 700×1200 responsive capture preserved
the Precision operator surface. `just check`, `just test`, `just build`, and the clean
Nix package build passed again after the migration.

The context-topology extension on 2026-08-09 added four deterministic lens
tests (seven UI tests total), extended the real HTTP proof across the root
redirect plus `/explore` and `/environment`, and retained all 135 Rust tests.
`just check`, `just test`, `just build`, and `nix build path:$PWD#rey --no-link`
all passed. The packaged output resolved to
`/nix/store/wx6cr2xzv68ixxg058yf62ym46bd9pwn-rey`.

The Hifi refresh on 2026-08-09 advanced the exact grammar pin to `0440cfe`.
The embedded bundle now contains upstream's directional press coordinates and
layered Kinetic lighting properties; the CLI reports the full revision.
`just check`, `just test`, and the clean Nix package build passed, with the
package resolving to `/nix/store/hns4rkgg0p3k6s064vjfakwc0mi64j2r-rey`.

The Hifi refresh on 2026-08-10 ended source vendoring and pinned pnpm's
Git-hosted Core and Kinetic package paths to remote `origin/main` revision
`058c650`. Upstream package preparation now builds Core before Kinetic,
declares Core as Kinetic's peer contract, and serializes clean package smoke
builds. Rey's lockfile resolves both packages to the same GitHub codeload SHA,
and its build allowlist names only those exact artifacts. The CLI and health
descriptor report the same full revision.

The environment scroll-stability correction removes router invalidation from
the polling path and adds two focused scheduler tests. All 12 frontend tests,
TypeScript validation, formatting, and the production asset build pass; failed
or overlapping refreshes cannot replace the last good mounted document. The
complete 136-test Rust suite and documentation tests also pass, and the updated
package resolves to `/nix/store/fjaxdy5w9k8ycicr03l310rdm1ajk51q-rey`.

The cadence/agent/coordinate extension on 2026-08-09 adds a bounded
`rey.git-commit-sequence.v1`, the read-only `rey.ui-cadence.v1` endpoint,
partial-order tick lanes, an exact generator registry, agent object scenes,
and canonical matrix-style coordinate parsing/resolution. Focused proof now
covers Git bounds and parents, agent aggregation, URI ordering and rejection,
stale bindings, topology projection, route matching, embedded routes, and API
method safety. `just check` passed frontend formatting/typecheck, 19/19 UI
tests, the production build, Rustfmt, Clippy with warnings denied, and flake
evaluation. `just test` passed 138/138 Rust tests, 19/19 UI tests, and every
documentation test. `just build` and the clean Nix package build passed; the
package resolved to `/nix/store/m9dvhkwfkh2ykdvsad357gxdn29prscf-rey`.

The cadence repository-state extension on 2026-08-10 adds
`rey.git-repository-status.v1` and hard-cuts the endpoint to
`rey.ui-cadence.v2`. Working-tree counts and local-upstream publication remain
independent, exact OIDs bind divergence and per-commit reachability, and the UI
states `NO NETWORK FETCH` rather than implying remote freshness. Provider
fixtures cover clean, dirty, no-upstream, pushed, unpushed, per-commit, and
conflicted states; component proof covers the paired instruments, publication
labels, and exact GitHub revision links. `just check` passes formatting,
TypeScript, 31/31 UI tests, the production build, Clippy, and flake evaluation;
`just test` passes 148/148 Rust tests, 31/31 UI tests, and documentation tests.

The Git-SHA presentation invariant on 2026-08-10 routes the footer and every
Git cadence revision through one `GitCommitLink` boundary. Component proof
requires the visible compact SHA itself to carry the complete GitHub commit
URL, and cadence rendering proves the same contract end to end. An unbound
repository renders an explicit boundary without exposing an inert SHA. `just
check`, `just test`, and `just build` pass with 31/31 UI tests, 144/144 Rust
tests, and every documentation test.

The communication-plane refinement gives mailbox history and conversation
separate tested axes. Selecting the active axis closes the plane and selecting
the other switches it. Escape and the background close either axis while the
sheet remains interactive. The chat axis uses a conventional transcript/composer
grammar but proves the present authority boundary with no session, no
transport, no retention, and a disabled send action.

The agent-plane refinement separates available collaboration applications from
past generator provenance. Six major agent runtimes enter the same bounded
environment inventory/search evidence as other applications. That inventory
stays on `/environment`. `/agents` collapses matching creation-request and
attention evidence into ranked recommendations and adds an observed-work
ledger over retained revision, test, run, mining, delta, and attention facts.
It explicitly labels those rows as retained results rather than live agent
telemetry. Tasks remain bounded coordination envelopes and journeys remain
derived rather than retained objects.

## Next Concrete Anchor

Plan 0012 delivered the shared `HEAD → INDEX → WORKING` environment operator
delta, the high-fidelity `rey env status` document, and the exact read-only
`/environment` workbench without independent browser probing. `/explore` still
uses aggregate environment coverage and should eventually consume these exact
nodes and relationships.

The nearer operator-UI anchor is one task-assignment handshake: define the task
CLI/API contract, bind one discovered runtime and exact locator to one ready
task, admit invocation separately from discovery, and return artifact/delta
evidence to the frontier. That same boundary supplies the participant and
session identity needed before chat becomes writable. Scenario/delta routes
remain the next evidence projection after that contract.

## Deferred

Workload and campaign mutation controls, WebSockets as an assumed conversation
transport, URL-addressable canvas focus, high-cardinality search, multi-user
identity, authentication, TLS, remote deployment,
Spoke-backed streams, and a general Rey service topology are not part of this
slice.
