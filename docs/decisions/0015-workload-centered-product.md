# ADR 0015: Workload-Centered Product And Scenario-Qualified Graphs

- Status: Accepted
- Date: 2026-08-07
- Narrows: [ADR 0007](0007-git-polling-and-delta-activation.md)
- Requires a versioned identity cutover from:
  [ADR 0014](0014-frontier-progress-and-scheduling.md)

## Context

Rey's architecture had made applications, spaces, lenses, frames, diffs, runs,
traces, and proofs separate provisional top-level CLI resources. The runtime
contracts then advanced through a formal transition machine, frontier
scheduling, and reasoning surfaces without a simple public object that
explained what a user was asking Rey to accomplish.

That surface is inverted. Users want to inventory their environment, then work
with named workloads. A workload executes a compute graph generated or revised
by an agent, rule, or human. Scenarios test that graph, and the authoritative
failure is the directed difference between expected and observed results. The
same scenario evidence should show progress and direct the next bounded graph
revision.

Without fixing this product model first, generic scheduling or an agent adapter
would likely expose internal runtime resources directly and choose graph,
scenario, progress, qualification, and retention semantics accidentally.

## Decision

### Public Surface

Workload is Rey's public unit of computation. The accepted top-level product
surface is:

```text
rey environment ...
rey workloads list
rey workloads test [<workload-id>]
rey workloads run <workload-id>
rey workloads status [<workload-id>]
```

Spaces, lenses, frames, deltas, frontiers, actions, traces, and proofs remain
typed runtime and evidence concepts. They may have focused diagnostic
projections later, but they are not a set of peer resources a user must
manually compose to run Rey.

### Workload And Graph

A versioned workload binds typed external inputs and outputs, allowed operation
contracts and effects, environment requirements, graph-proposal policy,
scenario suite, claims and evaluators, limits, qualification, and retention
requirements.

A graph revision is immutable and content-identified. It binds typed nodes,
ports and edges; exact operation contracts; capability and effect
requirements; limits; and generator provenance. The initial graph contract is
a finite typed DAG. Campaign-level graph revision provides the bounded feedback
loop. Implicit graph cycles and arbitrary executable text are rejected.

An agent, deterministic rule, or human proposes a graph through one untrusted
proposal contract. The deterministic runtime validates graph structure,
operations, capabilities, effects, preconditions, and limits before any node
executes.

### Scenarios And Qualification

A scenario binds exact fixtures and test providers, selected graph outputs,
expected typed observations or claims, comparison semantics, completeness,
and limits. Comparisons are directed from `EXPECTED` to `OBSERVED`.

A scenario passes only when every required comparison is equal and required
claim passes with complete evidence. A conclusive mismatch fails and retains
the authoritative typed delta or failed claim fact. Missing, incompatible,
truncated, unsupported, timed-out, or lost evidence is inconclusive rather
than a guessed pass or failure.

A graph revision qualifies only when every required scenario has a fresh,
complete passing result for that exact workload, graph, scenario-suite,
capability, comparator, evaluator, and limit identity. Optional scenarios are
visible but gate only when declared. A policy cannot declare qualification.

### Test Campaign

`workloads test` is a bounded graph-improvement campaign. It validates or
requests a graph, executes scenarios, computes expected-to-observed deltas,
derives progress and a frontier, and either qualifies the exact graph or
projects failing evidence for the next graph proposal. It repeats only within
explicit graph-revision, scenario-attempt, time, byte, evidence, and compute
bounds.

The campaign remains useful without an LLM. An existing graph can always be
tested deterministically. If no graph exists and no proposal policy is
available, the result is explicitly blocked and inconclusive.

With no workload id, the command selects every workload resolved by the chosen
catalog and fails closed rather than truncating when the workload-count bound
is exceeded.

The first implementation conservatively invalidates all scenario results when
the graph revision changes. Reusing a result later requires an explicit
dependency-closure proof.

### Progress And Commands

`workloads list` reads catalog and retained result state without executing
work. Its progress bar is a human projection of exact required, passed, failed,
inconclusive, blocked, running, queued, stale, and optional scenario counts for
one graph revision. It exposes passing and evaluated coverage separately and
does not turn a percentage into proof.

`workloads status` is also read-only and exposes exact revisions, candidates,
qualification, per-scenario results and deltas, campaign stop reason, frontier,
capabilities, evidence, staleness derivation, and latest run.

`workloads run` resolves a fresh qualified graph, binds admitted real inputs
and providers, and executes the same graph contract used by scenarios. Process
success does not replace output observation or claim evaluation.

List and status return success when inspection succeeds regardless of the
workloads shown. Test and run use `0` for passed/qualified, `2` for conclusive
failure, `3` for inconclusive or blocked, `4` for stale, and `1` for invalid
input or runtime failure. Machine results go to stdout; transient progress and
policy rationale go to stderr.

Batch test aggregation orders invalid/runtime failure, stale input, conclusive
failure, inconclusive/blocked, then all-qualified success. A graph-revision
budget stop does not erase a complete failing scenario delta; it is
inconclusive only when required evaluation could not complete.

### Catalog And Retention

Rey requires a catalog provider for declarations and immutable graph/scenario
assets, and a result provider for campaigns, proposals, attempts, outputs,
deltas, qualifications, runs, and queryable indexes. This decision does not
select their encoding, directory layout, database, or Spoke mapping.

Standalone providers disclose local filesystem-only guarantees at an explicit
artifact boundary. Connected providers use public Spoke contracts for any
durability, query, compute, or lineage guarantees claimed. A selected graph
cannot exist only in a disposable cache or DataFrame.

### Pre-Alpha Cutover

The previous application-centered provisional CLI and target declaration are
superseded. Historical ADRs remain unchanged, but current architecture and
plans use workload, graph, scenario, campaign, and run language.

The implemented `rey.frontier.v1`, `rey.runtime-state.v2`, and
`rey.reasoning-surface.v2` family still contains application/component fields.
Those fields are not silently redefined. A versioned schema cutover must bind
workload/graph/scenario/campaign identities before the workload CLI is
implemented.

## Consequences

- Users see environment inventory and workloads rather than an internal
  resource graph masquerading as a product interface.
- Scenario failures are typed scheduling evidence and a direct input to graph
  revision, not terminal prose.
- Test and run share one graph semantics while keeping fixture and real effects
  distinct.
- Qualification, evaluated coverage, progress, process state, and proof status
  remain separate.
- Agent graph generation can be added without making an LLM part of runtime
  correctness or granting generated code ambient authority.
- The next implementation bearing is a small zero-agent workload through all
  four commands and a versioned schema cutover, before generic recurring
  scheduling or a provider-specific agent loop.

## Not Decided

This decision does not select a manifest encoding, catalog discovery layout,
persistence engine, Spoke artifact mapping, policy transport, model provider,
parallel graph executor, cyclic graph semantics, recurring service, or
multi-user scheduler.
