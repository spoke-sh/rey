# Plan 0011: Local Operator UI

- Status: Active
- Decision: [ADR 0025](../docs/decisions/0025-local-operator-ui.md)

## Outcome

Raise Rey's collaboration language from sequential command documents to a
persistent, high-fidelity workload instrument while preserving the workload
CLI and typed runtime evidence as authoritative:

```text
rey ui
rey ui --host 0.0.0.0 --port 5714
```

The first slice is an embedded TanStack Router application using Hifi's Kinetic
grammar with the Precision theme. It reads the same bounded workload portfolio
projection as `rey workloads list`; it does not mutate state or create a second
scheduler.

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
the instrument rendered live qualification, scenario, run, coverage, attention,
catalog, graph, and evidence data without console/server failure.

After Hifi's StyleX migration on the same date, the upstream checkout was
verified at `5874cdf`, every vendored core/Kinetic source was diffed against
that head, and the UI was rebuilt without a handwritten stylesheet. The server
test opens the embedded `app.css` and proves extracted StyleX priority layers.
Desktop portfolio/workload captures and a 700×1200 responsive capture preserved
the Precision instrument. `just check`, `just test`, `just build`, and the clean
Nix package build passed again after the migration.

## Next Concrete Anchor

Project the retained scenario results and authoritative `EXPECTED → OBSERVED`
deltas already visible in `rey workloads test -v/-vv` into exact workload and
scenario routes. Keep list compact, make failing diffs open by default, reveal
matching evidence progressively, and preserve deep links and revision lineage.
The API should reuse the workload status/test representations rather than
inventing a UI-only evidence model.

## Deferred

Mutation controls, campaign execution from the browser, automatic refresh,
WebSockets, multi-user identity, authentication, TLS, remote deployment,
Spoke-backed streams, and a general Rey service topology are not part of this
slice.
