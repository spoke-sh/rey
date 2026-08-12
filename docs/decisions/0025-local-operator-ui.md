# ADR 0025: Local Operator UI

- Status: Accepted; workload write boundary superseded by ADR 0049
- Date: 2026-08-09
- Extends: [ADR 0022](0022-portfolio-mining-and-workload-attention.md)
- Extended: scoped loopback Journal admission in [ADR 0037](0037-explore-bound-collaboration-journal.md)

## Context

The workload CLI is Rey's authoritative human verification surface, but a
portfolio also has simultaneous dimensions that are easier to navigate as a
persistent visual instrument: qualification, scenario coverage, mining
completeness, current attention, graph identity, and retained evidence. This
projection must not introduce a second runtime, store, scheduler, or semantic
assessment path.

The requested interface is a TanStack Router TypeScript application expressed
with Hifi's Kinetic grammar and Precision theme. The packages are not published
in the package registry at the accepted revision, so the build uses exact
Git-hosted package paths rather than an ambient sibling checkout or an unpinned
branch.

An HTTP listener also changes Rey's dependency and exposure boundary. The
existing prohibition on a public, authenticated, multi-user Rey service still
stands.

## Decision

`rey ui` starts a bounded, read-only local operator server. It binds to
`127.0.0.1:5714` by default. `--host <IP>` and `--port <PORT>` configure the
listener; port `0` asks the operating system for an ephemeral port. The startup
document reports the exact bound address, URL, exposure, workspace, catalog,
application, grammar, theme, grammar revision, API routes, and read-only data
plane. JSON emits `rey.ui-server.v1`.

Binding a non-loopback address is an explicit operator choice and always emits
a stderr warning that the listener has no authentication. The server provides
no TLS, identity, authorization, mutation, execution, scheduling, remote
retention, or durability claim. It accepts only `GET` and `HEAD`; other methods
fail with `405 Method Not Allowed`.

The embedded single-page application uses code-based TanStack Router routes
for the portfolio, workload list, and exact workload detail. It obtains live
read-only projections from:

- `GET /api/v1/health`, a `rey.ui-health.v1` description of the exact server;
- `GET /api/v1/workloads`, the same `rey.workload-list.v5` projection used by
  `rey workloads list`; and
- embedded, content-stable application assets with SPA route fallback.

API responses are uncached. Static responses carry a restrictive content
security policy and related browser hardening headers. Request targets are
bounded. ADR 0026 later admits bounded passive browser revalidation; no
WebSocket or write endpoint is implied.

The application imports `@hifi/core` and `@hifi/kinetic` as pnpm Git-hosted
monorepo packages pinned to revision
`058c6504fc10740360717e97e687fd77bef6a5c5`. The lockfile resolves both package
paths to that exact GitHub codeload artifact, and `pnpm-workspace.yaml` admits
build scripts only for those two exact URLs. Hifi declares Core as Kinetic's
peer contract, while Rey installs both explicitly. The initial vendored
integration used `5874cdfe0c237ddd35bb121824a166ebb5b5654e`; source vendoring
ended at this revision. The current package adds Hifi's typed dense-table
alignment surface and material color properties while retaining directional
control travel and layered lighting. Rey uses the Kinetic grammar and the
`precision` material theme directly.

Rey follows Hifi's StyleX application architecture introduced by upstream
commit `9a981c5`: every authored structural, stateful, responsive, motion, and
accessibility rule is declared in `*.stylex.ts`, and the official
`@stylexjs/unplugin` Vite integration extracts one layered atomic stylesheet.
There is no parallel handwritten application stylesheet. Typed Kinetic
material values remain programmable runtime custom properties, matching the
upstream separation between material data and compiled structural rules.

The Rust composition binary uses `tiny_http` 0.12 as a deliberately narrow
synchronous HTTP dependency. This does not select an async runtime or general
service topology. Vite and StyleX build deterministic embedded assets before the Rust
workspace build, and Crane includes those assets in the filtered source.

## Consequences

- A user can move from `rey workloads list` to an exact, live portfolio
  instrument without changing the underlying runtime or retained state.
- CLI and browser surfaces share one workload-list derivation, preventing an
  independent UI assessment model.
- Loopback is safe by default; broader exposure is visible and intentionally
  unauthenticated, so it is suitable only inside an operator-controlled
  network boundary.
- Browser writes, scenario execution, server-side scheduling, authentication,
  TLS, remote deployment, and remote live streams remain future decisions.
- ADR 0026 extends this surface with the default context-topology Explorer and
  passive read-only revalidation.
