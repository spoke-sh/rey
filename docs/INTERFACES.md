# Interfaces

Rey's interfaces are different views and admission boundaries over one typed
runtime. This document is the high-level map: it explains which surface owns
which job, how identities and authority survive transitions between them, and
where each detailed contract lives.

It intentionally does not repeat command grammars, HTTP routes, workload
schemas, or renderer internals. Follow the linked contract for those details.

## First Principles

1. **One semantic state, several projections.** CLI, HTTP, browser, frames,
   deltas, and visualizations may render the same evidence differently. They
   cannot disagree about identity, revision, direction, completeness,
   omissions, limits, or authority.
2. **The CLI is the agent's primary runtime interface.** It is the
   high-fidelity path for inspecting and invoking implemented behavior. A
   feature is not complete when only an internal API or browser control can
   exercise it.
3. **The browser is the human's primary collaboration surface.** It begins at
   the foreground `rey agent` process and projects the same typed evidence.
   Polish, spatial continuity, and live updates cannot mint truth.
4. **HTTP is a transport boundary, not an alternate assessment.** The API
   exposes bounded reads and a small explicit set of local admissions. Route
   reachability never grants general query, mutation, scheduling, execution,
   assignment, provider, or proof authority.
5. **Provider ownership remains visible.** Rey freezes what it observed and
   the guarantees attached to that observation. It does not counterfeit a
   database transaction, remote cursor, authenticated identity, or delivery
   guarantee that the provider did not supply.
6. **Presentation is downstream of evidence.** Explorer, tables, diffs, Feed,
   Journal, and conversation views retain source links and boundaries. A
   lens, renderer, or layout can improve perception only.

## Contract Map

| Surface or domain | Detailed contract | Owns |
| --- | --- | --- |
| Agent CLI | [CLI](CLI.md) | Command grammar, human and structured output, exit behavior, `HEAD → INDEX → WORKING`, and the agent verification path. |
| Agent HTTP | [API](API.md) | Axum server model, API root, Swagger/OpenAPI, routes, transport, exposure, errors, and HTTP authority. |
| Browser application | [API](API.md#browser-routes), [Architecture](ARCHITECTURE.md#operator-projection) | Same-origin human routes and their relationship to typed API evidence. |
| Explorer | [Explorer](EXPLORER.md), [`@rey/explorer` guide](../packages/explorer/README.md) | Spatial semantics, coordinate/view separation, validity-safe terrain, projection transitions, renderer lifecycle, and fidelity. |
| Environment | [Environment](ENVIRONMENT.md) | Bootstrap seeds, bounded discovery, capability snapshots, application admission, trust, and provider guarantees. |
| Locators | [Locators](LOCATORS.md) | Canonical resource addressing, dimensions, exact resolution, and unsupported or ambiguous outcomes. |
| Mining | [Mining](MINING.md) | Relational and source operation families, frozen requests/results, lineage, limits, and native artifacts. |
| Workloads | [Workloads](WORKLOADS.md) | Versioned graph, scenarios, policy, qualification, total budget, results, and admission. |
| Runtime | [Runtime](RUNTIME.md) | Deterministic transitions, probes and effects, cancellation, retries, budgets, and process versus semantic outcomes. |
| Frontier | [Frontier](FRONTIER.md) | Typed attention, dependencies, invalidation, prioritization inputs, progress, and convergence. |
| Diff | [Diff](DIFF.md) | Directed typed comparison, alignment, schema change, text/structural direction, and renderings. |
| Proof | [Proof](PROOF.md) | Claims, evidence manifests, assessment, certificates, staleness, and missing evidence. |
| Git | [Git](GIT.md) | Repository snapshots, semantic index state, ref movement, polling, activation, and exact revision links. |
| Observations | [Observations](OBSERVATIONS.md) | Immutable human/agent statements, evidence bindings, Channel admissions, partial outcomes, and resolution. |
| Journal | [Journal](JOURNAL.md) | Retained authored synthesis, stable routes, typed blocks, seeds, queries, opportunities, and non-execution boundary. |
| Conversations | [Conversations](CONVERSATIONS.md) | Local sessions, transcript ordering, writers, browser composer availability, retention, and delivery boundary. |
| Architecture | [Architecture](ARCHITECTURE.md) | Ownership map, process topology, planes, data flow, and security boundaries. |
| Accepted choices | [Decision Plane](decisions/README.md) | Current cross-contract decisions and implemented posture. |
| Delivery slices | [Plans](../plans/README.md) | Active executable plans, sequencing, completion checks, and verification paths. |

## Surface Responsibilities

### CLI

The `rey` CLI is the public unit of agent interaction. Human output must show
the relevant inputs, progress, results, directed deltas, evidence, omissions,
limits, and revision lineage without requiring implementation knowledge.
Structured output preserves the same contract for automation.

The CLI distinguishes observation from admission and admission from action.
Status commands observe; `add` freezes an exact INDEX; `commit` consumes the
verified INDEX without rereading ambient WORKING; explicit run or action
commands invoke only their named authority.

### API

`rey agent` hosts the local HTTP projection. `/api` is the discovery root,
`/api/docs/` is embedded Swagger, and `/api/openapi.json` is the generated
OpenAPI 3.1 document. Read operations are `GET|HEAD`; the documented `POST`
operations are explicit bounded local admissions.

The API and registered server routes derive from one route catalog. The exact
transport and endpoint contract lives only in [API](API.md).

### Browser

The browser is a high-fidelity projection of the API's typed evidence, not a
parallel store or assessment engine. It remains live through passive
revalidation and future retained event transport. A quiet footer means no
operator attention is requested; the UI must not invent activity.

Browser controls are allowed only when their write authority is separately
declared. Loading a route, panning Explorer, opening a detail, or selecting an
object is read-only navigation. Following a current retained GitHub mailbox
evidence link is the narrow exception that also requests the separately
declared exact bounded inbox poll; the probe retains a receipt and current
messages but does not mutate provider read state.

### Evidence projections

Frames, deltas, tables, graphs, maps, and terrain are projections. Each must
retain exact source and implementation revisions, direction, scope,
completeness, omissions, and limits. Derived visual continuity is never a
substitute for observed coverage.

## Identity Across Interfaces

Identity is semantic and typed. Display labels, browser routes, camera state,
table positions, and timestamps are not substitutes for content or provider
identity.

- A Git commit identity is an exact commit SHA on a bound repository. The
  browser links a known SHA to that exact commit; it does not guess the
  repository or label semantic digests as commits. Cadence derives supported
  GitHub, GitLab, and Bitbucket web bindings only from the configured upstream
  remote read through the admitted `git` executable; this performs no remote
  transport and never retains embedded credentials.
- A Journal entry has a stable human-readable route carrying exact content
  identity. Typed blocks expose fragment permalinks.
- An Explorer resource coordinate remains separate from camera center, scale,
  viewport, selection, and level of detail. Zoom may replace a visual
  aggregate but cannot change source truth.
- An exact workload scenario or delta route resolves only that retained
  content identity. It never falls back to the newest result.
- Environment applications bind provider, path, version, digest/provenance,
  trust, supported operations, and effective limits before action.

Semantic digests, provider revisions, Git commits, implementation revisions,
and schema versions remain distinct even when a UI presents them together.

## Authority Across Interfaces

The following axes do not imply one another:

| Axis | What it permits | What it does not imply |
| --- | --- | --- |
| Discovery | Observe a potential provider or application. | Invocation, trust, assignment, or persistence. |
| Read admission | Query an exact bounded source. | Mutation, scheduling, or proof. |
| Document admission | Retain a validated proposal or authored statement. | Execution of its blocks or recommendations. |
| Workload admission | Commit an exact fully qualified INDEX to workload HEAD. | Autonomous future execution outside declared activation. |
| Action admission | Perform one explicit provider effect with fresh preconditions. | General provider authority or semantic success. |
| Assignment | Bind an admitted task to an eligible runtime. | Process execution or proof authority. |
| Execution | Run bounded compute or an admitted effect. | Convergence, correctness, or proof. |
| Assessment | Evaluate a claim against exact evidence. | Provider truth beyond that evidence. |

An agent, deterministic rule, and human may propose through the same validated
contract. None may declare its own proof successful.

## Shared Document Conventions

Rey-owned documents carry a schema discriminator such as
`rey.workload-list.v1`. A schema version defines the document contract, not
the HTTP route, binary package, provider protocol, or underlying source
revision. During pre-alpha, incompatible changes are hard cutovers unless an
active plan explicitly defines migration behavior.

Typed documents should expose, where relevant:

- exact input and source identities;
- operation and implementation revisions;
- effective limits and the capability snapshot;
- completeness and explicit omissions;
- derivation lineage;
- authority and supported operations;
- directed source and target labels for comparisons;
- staleness derived from changed inputs.

Human renderings may abbreviate but must provide a path to the exact evidence.

## Errors, Limits, And Partial Outcomes

Errors are typed by layer. CLI commands use documented stdout, stderr, and
exit-code behavior. HTTP uses `rey.api-error.v1`. Provider, mining, workload,
runtime, and proof documents retain their own outcome classifications rather
than collapsing all failure into a transport status.

Bounds apply before optimization or presentation work is accepted. A partial
result names what completed, what failed, what was omitted, and whether replay
is deterministic. Process success and semantic convergence are separate.

Unsupported, unexplored, missing, ignored, stale, and truncated are distinct
states. An interface must not present any of them as empty observed evidence.

## Provider And Policy Boundary

Providers retain ownership of storage, query, document, stream, table, tool,
run, capture, authentication, and transaction guarantees. Rey records the
guarantees actually admitted for an exact capability snapshot.

Policy selects or proposes within validated options. It cannot:

- add provider guarantees;
- widen a locator or evidence scope;
- suppress blockers or policy exclusions;
- let a proposer mark its own frontier row resolved;
- transform similarity or confidence into coverage or proof;
- bypass total workload budgets or action preconditions.

## Persistence Boundary

Authored content and provider-owned source artifacts cannot exist only as a
cache, DataFrame, queue, visual projection, or delta rendering. Retained state
uses the subject contract's exact content identity and lineage. A projection
cache may accelerate replay but never becomes the authority it projects.

Workspace-local histories for environment, workloads, editor packages,
Channels, Observations, Journal, conversations, Git polling, and qualified
results remain separate because their admission and mutation authorities are
different.

## Workspace Ignore Surface

An optional workspace-root `.reyignore` narrows fresh WORKING observations
using typed, case-sensitive patterns. It does not delete files, rewrite HEAD
or INDEX, bypass validation, or grant authority. Relevant rules, the exact
ignore-file digest, source lines, match counts, and omitted counts enter the
affected WORKING identity and structured output. The command grammar and
examples live in [CLI](CLI.md#ignore-policy).

## Security And Exposure

Local does not mean trusted. Rey defaults the agent listener to loopback and
reports whether the effective bind is loopback-only. The current HTTP surface
is unauthenticated. An explicit non-loopback bind exposes its documented
writes to reachable clients; the surrounding deployment must supply any
required network isolation, authentication, and TLS.

Finding an executable, agent application, credential-shaped value, or remote
locator grants no permission to use it. Bootstrap begins only from the
process-owned `HOME`, `PWD`, and `PATH` seed set, compiled adapters, and
explicitly supplied maps under the Environment contract.

Secrets, private provider state, and private source snapshots do not enter
versioned project artifacts or generated proof merely because an interface
could serialize them.

## Adding Or Changing An Interface

A change is complete only when it:

1. has one semantic owner and does not duplicate another plane's authority;
2. defines typed inputs, outputs, revisions, limits, omissions, and error or
   partial-outcome behavior;
3. preserves exact identity across CLI, API, browser, and evidence links;
4. exposes a high-fidelity CLI verification path for implemented behavior;
5. updates the subject contract, API or CLI contract when applicable, current
   decision plane, and active plan in the same logical change;
6. includes focused tests for malformed input, exact preconditions, limits,
   deterministic replay, and human/structured rendering as appropriate.

Do not add a route, flag, browser control, or stored field as a drive-by
shortcut around an unresolved ownership decision.

## Current Boundary

Rey currently implements a local foreground agent process, bounded CLI,
embedded Axum API/browser worker, admitted GitHub Channel inbox poller, local
revision stores, workload qualification/runtime slices, evidence projections,
and the Explorer rendering engine described by the linked contracts.

This map does not imply a general remote service, authenticated multi-user
collaboration, autonomous agent invocation, universal provider adapters,
durable distributed scheduling, or broader proof than the retained evidence
supports.
