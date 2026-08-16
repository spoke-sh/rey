# Plan 0004: Refine The Rey Agent Process

- Status: Active
- Owns: foreground Rey process, orchestration, supervision, background-work
  lifecycle, and admitted agent topology

## Outcome

Make `rey agent` the one explicit foreground entry to Rey's live local
surface. Every task that outlives a request or command stack must appear in a
bounded topology, have one lifecycle owner, expose its authority and limits,
stop cooperatively, and retain any evidence its semantic contract requires.
Discovering an agent runtime remains separate from assigning or invoking it.

```text
rey agent
  → Rey process (orchestrator)
    → supervised bounded background work
      → operator HTTP projection
      → exact admitted GitHub Channel polling
    → future admitted agent-runtime topology
```

## Current Boundary

The former `rey ui` command has been removed by a hard cutover. `rey agent`
starts one foreground OS process whose root role is the orchestrator. The
orchestrator registers the embedded Axum operator HTTP server and GitHub Channel
inbox poller as its two workers, owns cooperative SIGINT/SIGTERM shutdown, and
fails the process closed on an unexpected worker error, exit, or panic. The
inbox worker remains idle until Channel HEAD admits a `github_inbox`
application whose exact `gh` capability is also present in environment HEAD.
It polls immediately and then at the admitted cadence through the same bounded
`rey channels poll` path exposed to humans. One declarative API catalog owns
Axum registration and the OpenAPI 3.1 document; `/api` enters vendored Swagger
while synchronous evidence projections run outside the HTTP event loop.
`rey.agent-process.v2`,
`rey.process.v1`, and `rey.agent-topology.v1` expose the live PID, roles,
parent/child edge, placement, state, restart policy, endpoint, authority,
two-worker bound, and omissions through `--format json`,
`GET /api/v1/agent`, and the operator's `/agents` route; health returns the
same topology beside the existing operator descriptor.

This topology is runtime-only local status. GitHub response evidence, partial
failures, and current mailbox membership are retained separately as poll
receipts and immutable Channel messages. V1 has no restart, daemonization,
crash durability, multi-process fencing, autonomous workload scheduling, or
agent-runtime invocation. Browser passive revalidation remains browser-owned
work.

## Completion Checklist

### 1. Establish the process boundary

- [x] Hard-cut `rey ui` to `rey agent` across CLI behavior, examples,
  qualification tooling, and foundational contracts.
- [x] Define one foreground `rey.process.v1` orchestrator and a bounded
  `rey.agent-topology.v1` graph without conflating an agent runtime with the
  Rey process.
- [x] Keep default startup to one listening URL, emit useful process/worker
  lifecycle logs, and expose exact JSON plus HTTP/browser process status with
  topology, authority, limits, and omissions.
- [x] Project the exact health-bound topology through the browser `/agents`
  collaboration surface without inferring agent activity.

### 2. Supervise current background work

- [x] Register the operator HTTP server as orchestrator-owned background work.
- [x] Hard-cut the operator transport to Axum, generate OpenAPI from the
  registered route catalog, and make `/api` a vendored Swagger discovery root.
- [x] Keep slow synchronous evidence projections off the HTTP event loop and
  prove API discovery plus browser deep-link reachability through live-server
  tests.
- [x] Register exact admitted GitHub inbox polling as the first resident
  orchestrator-owned task.
- [x] Make SIGINT/SIGTERM cancellation cooperative and bind worker lifetime to
  the Rey process.
- [x] Fail closed on unexpected worker error, exit, or panic; keep restart and
  detached-daemon claims absent.
- [x] Prove command removal, topology output, live HTTP status, and cooperative
  worker shutdown through CLI tests.

### 3. Admit resident work deliberately

- [x] Select the first real server-side recurring task only with an explicit
  source, cadence, total bounds, cancellation boundary, retry policy,
  idempotency contract, retained outcome, and CLI/browser inspection path.
- [x] Keep poll execution state separate from semantic progress: complete and
  partial receipts retain provider results and omissions, while invalid exact
  admission fails the supervised worker closed.
- [ ] Add bounded restart only if a concrete task proves its safe replay and
  retained-attempt semantics; never restart an effect from process status
  alone.

### 4. Extend agent topology under authority

- [ ] Define assignment and invocation admission before adding a discovered
  agent runtime as a live topology node.
- [ ] Bind each admitted runtime/process node to exact application,
  environment, task, capability, budget, communication, and cancellation
  identities.
- [ ] Keep policy proposals, task assignment, process execution, conversation
  transport, and proof authority separate in CLI and browser projections.

## Deferred

Detached service installation, remote supervisors, multi-host agents,
multi-process transactionality, automatic runtime discovery-to-invocation,
unbounded queues, and autonomous general scheduling remain outside this plan.
