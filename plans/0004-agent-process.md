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
      → future explicitly admitted resident tasks
    → future admitted agent-runtime topology
```

## Current Boundary

The former `rey ui` command has been removed by a hard cutover. `rey agent`
starts one foreground OS process whose root role is the orchestrator. The
orchestrator registers the embedded operator HTTP server as its only worker,
owns cooperative SIGINT/SIGTERM shutdown, and fails the process closed on an
unexpected worker error, exit, or panic. `rey.agent-process.v1`,
`rey.process.v1`, and `rey.agent-topology.v1` expose the live PID, roles,
parent/child edge, placement, state, restart policy, endpoint, authority,
one-worker bound, and omissions through CLI startup, `GET /api/v1/agent`, and
the operator's `/agents` route; health returns the same topology beside the
existing operator descriptor.

This topology is runtime-only local status. It is not a retained proof or
process history. V1 has no restart, daemonization, crash durability,
multi-process fencing, autonomous workload scheduling, or agent-runtime
invocation. Browser passive revalidation remains browser-owned work.

## Completion Checklist

### 1. Establish the process boundary

- [x] Hard-cut `rey ui` to `rey agent` across CLI behavior, examples,
  qualification tooling, and foundational contracts.
- [x] Define one foreground `rey.process.v1` orchestrator and a bounded
  `rey.agent-topology.v1` graph without conflating an agent runtime with the
  Rey process.
- [x] Expose table and JSON startup evidence plus an exact HTTP process/status
  route with process, topology, authority, limits, and omissions.
- [x] Project the exact health-bound topology through the browser `/agents`
  collaboration surface without inferring agent activity.

### 2. Supervise current background work

- [x] Register the operator HTTP server as orchestrator-owned background work.
- [x] Make SIGINT/SIGTERM cancellation cooperative and bind worker lifetime to
  the Rey process.
- [x] Fail closed on unexpected worker error, exit, or panic; keep restart and
  detached-daemon claims absent.
- [x] Prove command removal, topology output, live HTTP status, and cooperative
  worker shutdown through CLI tests.

### 3. Admit resident work deliberately

- [ ] Select the first real server-side recurring task only with an explicit
  source, cadence, total bounds, cancellation boundary, retry policy,
  idempotency contract, retained outcome, and CLI/browser inspection path.
- [ ] Distinguish queued, running, succeeded, failed, cancelled, timed out, and
  lost process state from semantic progress and convergence.
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
