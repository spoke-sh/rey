# Plan 0011: Local Operator UI

- Status: Active
- Decisions: [ADR 0025](../docs/decisions/0025-local-operator-ui.md),
  [ADR 0026](../docs/decisions/0026-context-topology-explorer.md)

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

## Completion Checklist

- [x] Accept ADR 0025 and bound the UI as a local read-only operator projection,
  not a public or durable Rey service.
- [x] Add `rey ui` with loopback defaults, configurable IP/port, ephemeral-port
  support, machine output, exact startup evidence, and a non-loopback warning.
- [x] Serve embedded SPA assets, deep-link fallback, health, and live workload
  portfolio endpoints through a bounded synchronous server.
- [x] Build portfolio, workload-list, and workload-detail routes with TanStack
  Router and direct Kinetic/Precision grammar imports.
- [x] Pin and record exact Hifi upstream sources and license without relying on
  an ambient sibling checkout.
- [x] Track Hifi head `5874cdf` and migrate every authored application rule to
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
- [x] Bind the footer to the exact Rey implementation revision and link the
  complete Git object id without confusing semantic BLAKE3 identities for
  source commits.
- [x] Cover root redirect, both top-level routes, lens ordering, bounds,
  identity preservation, and folded-evidence disclosure.
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
# frontend typecheck, 3/3 UI tests, Vite build, Rustfmt, Clippy, and flake evaluation passed
nix develop path:$PWD --command just test
# 135/135 Rust tests, 3/3 UI tests, and all documentation tests passed
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
verified at `5874cdf`, every vendored core/Kinetic source was diffed against
that head, and the UI was rebuilt without a handwritten stylesheet. The server
test opens the embedded `app.css` and proves extracted StyleX priority layers.
Desktop portfolio/workload captures and a 700×1200 responsive capture preserved
the Precision operator surface. `just check`, `just test`, `just build`, and the clean
Nix package build passed again after the migration.

The context-topology extension on 2026-08-09 added four deterministic lens
tests (seven UI tests total), extended the real HTTP proof across the root
redirect plus `/explore` and `/environment`, and retained all 135 Rust tests.
`just check`, `just test`, `just build`, and `nix build path:$PWD#rey --no-link`
all passed. The packaged output resolved to
`/nix/store/wx6cr2xzv68ixxg058yf62ym46bd9pwn-rey`.

The environment scroll-stability correction removes router invalidation from
the polling path and adds two focused scheduler tests. All 12 frontend tests,
TypeScript validation, formatting, and the production asset build pass; failed
or overlapping refreshes cannot replace the last good mounted document. The
complete 136-test Rust suite and documentation tests also pass, and the updated
package resolves to `/nix/store/fjaxdy5w9k8ycicr03l310rdm1ajk51q-rey`.

## Next Concrete Anchor

Plan 0012 delivered the shared `HEAD → INDEX → WORKING` environment operator
delta, the high-fidelity `rey env status` document, and the exact read-only
`/environment` workbench without independent browser probing. `/explore` still
uses aggregate environment coverage and should eventually consume these exact
nodes and relationships.

The nearer operator-UI anchor is to project retained scenario results and
authoritative `EXPECTED → OBSERVED` deltas from
`rey workloads test -v/-vv` into exact workload/scenario routes without
inventing a UI-only evidence model.

## Deferred

Mutation controls, campaign execution from the browser, WebSockets, URL-
addressable canvas focus, high-cardinality search, multi-user identity,
authentication, TLS, remote deployment,
Spoke-backed streams, and a general Rey service topology are not part of this
slice.
