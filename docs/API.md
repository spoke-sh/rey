# Agent HTTP API

This document defines the HTTP interface hosted by `rey agent`. It is the
formal contract for API discovery, versioned routes, transport behavior,
authority, and errors. [Interfaces](INTERFACES.md) is the cross-surface map;
[CLI](CLI.md) defines the primary agent-facing interface; subject documents
define the semantics of the typed documents carried over HTTP.

## First Principles

1. **The API is a projection, not a second runtime.** HTTP exposes the same
   typed evidence and bounded admissions as the CLI. A route does not acquire
   scheduling, execution, provider, or proof authority from being reachable.
2. **Discovery starts at the API root.** `/` redirects to `/api`, and `/api`
   redirects to the embedded Swagger interface. The exact OpenAPI document is
   available without a browser.
3. **One catalog owns routing and documentation.** Registered API methods,
   OpenAPI paths, operation identifiers, authority descriptions, and request
   schemas derive from one declarative Rust route catalog. A test fails if
   the generated OpenAPI document omits a registered operation.
4. **Reads remain safe and writes remain explicit.** `GET` and `HEAD` project
   bounded state. The small set of `POST` routes admits only the authority
   stated for that route and uses exact preconditions where mutable state is
   involved.
5. **Exposure is part of the contract.** The server is unauthenticated and
   loopback-only by default. Binding a non-loopback address makes every
   enabled write reachable to that network; Rey reports this boundary but
   does not provide TLS, authentication, or origin policy.
6. **Evidence boundaries survive transport.** Schema identity, revisions,
   completeness, omissions, limits, and lineage stay in response documents.
   Swagger examples cannot broaden those guarantees.
7. **Route readiness is demand-scoped.** The browser's root shell loads only
   process identity, Channels, Observations, conversation, and the exact
   revalidation cursor. It does not request the workload portfolio. A route
   waits only for the projections it consumes; Cadence, Environment, Journal
   entries, and exact scenario/delta evidence remain independent of a cold
   workload projection.

## Discovery And Versioning

Run the foreground process:

```sh
rey agent
```

The default origin is `http://127.0.0.1:5714`.

| Surface | Path | Behavior |
| --- | --- | --- |
| Server root | `/` | `307 Temporary Redirect` to `/api`. |
| API root | `/api` | `307 Temporary Redirect` to `/api/docs/`. |
| Swagger | `/api/docs/` | Embedded interactive documentation. Assets are served below `/api/docs/`. |
| OpenAPI | `/api/openapi.json` | OpenAPI 3.1 JSON generated from the registered route catalog. |
| Versioned API | `/api/v1/...` | Current pre-alpha HTTP operations. |
| Browser application | `/explore` and application routes below | Embedded same-origin operator application. |

`rey.ui-server.v2` reports `http_framework`, `api_root`, `swagger_ui`, and
`openapi_document` alongside the listener, exposure, enabled writes, roots,
application identity, and implementation revision. `rey.agent-process.v2`
nests that descriptor in the exact supervised process topology.

The `/api/v1` prefix versions the route family. Each response also carries its
own Rey document schema. Those versions are independent: a route can remain
in `v1` while a response document makes an intentional hard cut. Rey is
pre-alpha and does not provide implicit aliases or migration readers.

## Server Model

The operator worker is an [Axum](https://github.com/tokio-rs/axum) router
inside the foreground Rey process. It is one of the orchestrator's bounded
supervised workers, not a detached service. SIGINT or SIGTERM requests
cooperative shutdown; an unexpected worker error, exit, or panic fails the
process closed.

Axum owns routing, extraction, body limits, method fallbacks, and graceful
listener shutdown. Existing synchronous evidence projections run on Tokio's
blocking pool. A cold or expensive projection such as `/api/v1/workloads`
therefore does not occupy the HTTP event loop or prevent Swagger, static
assets, health, or other requests from being accepted. This is request
concurrency, not background workload scheduling.

The browser preserves the same separation above HTTP. Its lightweight root
loader never fetches `/api/v1/workloads`; portfolio-dependent routes opt in to
that endpoint. Opening the mailbox makes runtime attention demand-visible:
retained Channel history is usable immediately while the workload attention
projection identifies itself as loading. In-flight portfolio requests are
deduplicated.

Swagger assets are compiled into the binary. Opening the API documentation
does not require a CDN or widen network authority.

## Transport Contract

### Methods

- `GET` returns a bounded representation.
- `HEAD` is available for every registered read and returns the same status
  and headers without a body.
- `POST` is accepted only by the explicit write routes in this document.
- Unsupported methods return `405` with a typed error and an `Allow` header.

### Media, caching, and compression

- JSON responses use `application/json; charset=utf-8`.
- JSON projections and errors use `Cache-Control: no-store`.
- Browser HTML and embedded assets use `Cache-Control: no-cache`.
- Large workload and workload-evidence projections support gzip when
  `Accept-Encoding` admits it and return `Vary: Accept-Encoding`.
- JSON writes require `Content-Type: application/json`; unsupported media
  returns `415`.
- The router rejects bodies larger than 1 MiB before endpoint decoding.
  Endpoint-specific limits are usually smaller and still apply.
- Request targets longer than 4,096 bytes return `414`.

The browser application uses a same-origin content security policy. The API
does not advertise cross-origin access. There is no cookie, session, token,
authenticated identity, TLS, or CSRF guarantee.

## Read Operations

Every row is available through `GET` and `HEAD`.

| Path | Response schema | Contract and authority |
| --- | --- | --- |
| `/api/v1/health` | `rey.agent-health.v2` | Readiness plus exact process, worker topology, and operator descriptor. |
| `/api/v1/agent` | `rey.agent-process.v2` | Foreground process, supervision, lifecycle, authority, limits, and omissions. |
| `/api/v1/revalidation` | `rey.ui-revalidation.v1` | Exact bounded source-change cursor; no assessment or scheduling. |
| `/api/v1/cadence` | `rey.ui-cadence.v1` | Partially ordered retained Git, environment, and mounted-browser cadence. |
| `/api/v1/environment` | `rey.environment-status.v2` | The same bounded environment-status derivation available through the CLI. |
| `/api/v1/channels` | `rey.ui-channels.v1` | Channel status and current retained provider mailbox frontier. |
| `/api/v1/conversations` | `rey.conversation-transcript.v1` | One bounded workspace-local transcript and its exact writer/delivery boundary. |
| `/api/v1/feed/admissions` | `rey.ui-feed-admissions.v1` | Verified retained environment and workload commits; no fabricated activity. |
| `/api/v1/journal` | `rey.ui-journal.v2` | Verified bounded Journal log and browser admission boundary. |
| `/api/v1/journal/opportunities` | `rey.journal-opportunity-surface.v1` | Authored action cells only; no readiness, assignment, execution, or proof claim. |
| `/api/v1/journal/queries` | `rey.journal-query-state.v1` | Retained query admission and execution evidence; no browser query mutation. |
| `/api/v1/journal/seed?observations={id[,id]}` | `rey.journal-seed.v1` | Deterministic unretained proposal from exact unresolved observations. |
| `/api/v1/observations` | `rey.observation-frontier.v1` | Bounded unresolved collaboration frontier and exact Channel admissions. |
| `/api/v1/workloads` | `rey.workload-list.v1` | Portfolio, retained results, attention, atlas, and compact terrain transport. May be expensive on a cold revision. |
| `/api/v1/workloads/admissions` | `rey.workload-log.v1` | Newest verified workload commits under the retained history bound. |
| `/api/v1/workloads/evidence` | `rey.ui-workload-evidence-catalog.v1` | Index of exact retained scenario and directed-delta references. |
| `/api/v1/workloads/{workload_id}/scenarios/{execution_id}` | `rey.ui-workload-scenario-evidence.v1` | One exact content-addressed scenario execution; never falls back to latest. |
| `/api/v1/workloads/{workload_id}/deltas/{delta_id}` | `rey.ui-workload-delta-evidence.v1` | One exact retained directed delta in its original direction. |

Path identities must be non-empty UTF-8 after percent decoding and cannot
contain `/` or NUL. An unknown exact identity returns `404`; it never selects a
newer retained result.

## Write Operations

Swagger can issue these requests for local inspection, but that convenience
does not alter their authority or safety boundary.

| Method and path | Request | Result | Exact authority |
| --- | --- | --- | --- |
| `POST /api/v1/journal` | `rey.journal-entry-proposal.v2` | `rey.journal-admission.v2`; `201` when newly retained, `200` when idempotently present. | Validate and retain one bounded self-asserted human document. No typed block executes. |
| `POST /api/v1/observations` | `rey.ui-observation-write.v1` | Observation admission/broadcast receipt; `201`. | Admit one partial self-asserted finding and attempt bounded broadcast to default local Channels. No relay, action, or proof authority. |
| `POST /api/v1/conversations/messages` | `rey.ui-conversation-message-write.v1` | Conditional append receipt; `201`. | Append as the declared browser writer only when log and session identities match. Delivery remains `not_attempted`. |
| `POST /api/v1/channels/working` | `rey.ui-channel-working-write.v1` | Conditional replacement receipt; `201` when changed, `200` when unchanged. | Validate a complete graph and replace only Channel WORKING when expected HEAD and WORKING snapshots still match. |
| `POST /api/v1/workloads/admit` | Workload approval with `message`, `expected_head`, and `expected_working` | Exact qualification/admission receipt; `201`. | Freeze reviewed files, require fresh HEAD/WORKING preconditions, run the complete suite, and commit only that qualified INDEX. |

Write requests reject unknown fields where their typed decoder requires a
closed document. Endpoint-specific size and semantic bounds include:

- Journal proposals: at most 1 MiB before the deeper Journal structural and
  character limits apply.
- Observation writes: at most 32 KiB; the finding body is at most 500
  characters.
- Workload approvals: at most 16 KiB.
- Channel graphs and conversation messages: their subject-contract limits
  remain authoritative even though the shared router has the 1 MiB ceiling.

See [Journal](JOURNAL.md), [Observations](OBSERVATIONS.md),
[Conversations](CONVERSATIONS.md), [Workloads](WORKLOADS.md), and
[Interfaces](INTERFACES.md) for the semantic planes those writes affect.

## Error Contract

API errors emitted by Rey use:

```json
{
  "schema": "rey.api-error.v1",
  "category": "api_route_not_found",
  "detail": "no read-only Rey UI API route matches this target"
}
```

`category` is a stable machine-facing classification within this schema;
`detail` is bounded human diagnostic context. The current status families are:

| Status | Meaning |
| --- | --- |
| `400` | Malformed path, query, JSON, or request contract. |
| `403` | The current retained contract does not authorize the requested writer. |
| `404` | Unknown API route or unknown exact retained identity. |
| `405` | Registered route, unsupported method. |
| `409` | Exact precondition changed or the conditional write conflicts. |
| `413` | Shared or endpoint-specific request body limit exceeded. |
| `414` | Request target limit exceeded. |
| `415` | JSON media type required. |
| `422` | Well-formed request rejected by semantic validation. |
| `500` | A bounded projection, retained artifact, encoding, or handler failed. |

Errors do not imply retry safety. A caller must use the endpoint's exact
preconditions and subject contract to decide whether a new attempt is valid.

## Browser Routes

The embedded single-page application is a projection of the same typed
evidence. Known application routes and deep links return the embedded shell;
the client router selects the view.

| Route | Bearing |
| --- | --- |
| `/explore` | Continuous evidence-bound spatial view and default human bearing. |
| `/feed` | Verified admissions, observations, and collaboration attention. |
| `/environment` | Environment evidence and exact application inventory. |
| `/workloads` and workload deep links | Portfolio, review, results, and exact evidence. |
| `/journal`, `/journal/new`, `/journal/{slug}` | Journal index, authoring, and stable content-addressed entries. |
| `/cadence` | Partially ordered retained clocks and repository posture. |
| `/agents` | Exact Rey process and supervised topology. |

Panning, zooming, opening a deep link, selecting an object, or loading a page
does not execute a locator, run a survey, schedule a workload, or widen read
authority.

## Verification

The route catalog and OpenAPI document are covered by Rust invariants. Server
tests start the real Axum listener and verify health, Swagger HTML and assets,
the OpenAPI document, application deep links, typed method errors, writes,
gzip, and exact evidence routes. The CLI integration path starts `rey agent`,
discovers its printed origin, and verifies `/`, `/api`, Swagger, OpenAPI,
`/explore`, health, process, and evidence projections.

Vitest also invokes the actual root and Cadence route loaders together and
asserts that `/api/v1/cadence` and lightweight shell endpoints are requested
while `/api/v1/workloads` is not.

The source of truth for registered operations is
`crates/rey/src/api.rs`. The OpenAPI document is a runtime projection of that
catalog; do not hand-maintain a second static specification.

## Current Boundary

This interface is a local pre-alpha operator surface. It has no public-service
availability target, horizontal scaling contract, authentication, TLS,
cross-origin policy, rate limiter, durable request queue, server-side browser
session, or compatibility guarantee beyond its declared schemas. The
foreground CLI remains the high-fidelity agent verification path. A future
remote service must make those boundaries explicit rather than treating this
listener as one by deployment convention.
