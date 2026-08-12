# Plan 0001: Close The Runtime Loop

- Status: Active
- Owns: workload ownership, live invalidation, attention-to-frontier handoff,
  bounded harness response, Git activation, and recurring execution

## Outcome

Turn the implemented deterministic pieces into one bounded recurring loop
without letting portfolio attention become policy, letting a coding harness
admit its own response, or treating Git/environment change as execution
authority.

```text
retained catalog + results + environment + Git + coverage
  → typed attention → generic frontier → bounded reasoning surface
  → immutable harness response in WORKING
  → INDEX qualification → human HEAD admission
  → re-observation and attention/frontier delta
```

## Current Boundary

Rey derives typed portfolio attention, a canonical frontier, deterministic
scheduling, and bounded reasoning surfaces. Workloads declare bounded owned
surfaces; retained mapped-file and capability changes derive owner attention.
One selected ready `CREATE` row now crosses an immutable external-harness
request/response and human admission cycle without Rey invoking a harness.
A manual bounded Git poll now retains cursor, pending transition, and
proposal-only activation evidence. Workloads can declare exact Git HEAD or
semantic-index dependencies; portfolio attention compares them only with the
acknowledged cursor snapshot, so ambient repository movement cannot silently
invalidate work. An acknowledged proposal can now cross a separate exact
workload admission gate and execute only its retained scenario selection after
the Git cursor, workload HEAD, capabilities, and budget are revalidated. The
result is replay-stable and cannot replace full-suite qualification. Activation
executions from compatible proposals in the same transition can coalesce onto
that exact retained result without erasing either activation identity. A
bounded recurring scheduler and cross-poll debounce remain open.

## Completion Checklist

### 1. Bind ownership and invalidation

- [x] Define bounded workload-owned surface declarations with exact workload,
  graph, environment, and source revisions.
- [x] Derive changed-dependency and missing-capability facts from retained
  environment and Git evidence instead of fixture-only or empty live fields.
- [ ] Prove ownership collision, missing owner, changed owner, stale revision,
  policy exclusion, incomplete coverage, and typed-empty attention behavior.

### 2. Hand attention to runtime work

- [x] Project admitted ready attention rows into the generic frontier without
  copying the portfolio or allowing the scheduler to invent reasons.
- [x] Build one bounded reasoning surface that cites the exact attention row,
  dependent deltas, evidence, omissions, allowed graph operations, and total
  budget.
- [x] Preserve distinct attention, frontier, scheduling, progress, and proof
  identities through CLI table and structured output.

### 3. Complete one harness request/response cycle

- [x] Bind one `CREATE` or `REFINE` row, its reasoning surface, current package
  state, failing deltas, permitted operations, and limits into a creation
  request.
- [x] Accept one immutable external harness response as a verified WORKING
  package; do not launch an ambient executable or fabricate graph/scenario
  content in Rey.
- [x] Stage exact bytes, execute the frozen scenarios, require human admission,
  and re-mine whether the original row resolved, changed, or remained open.
- [x] Show `AWAITING HARNESS`, `WORKING`, `INDEX UNQUALIFIED`, `INDEX
  QUALIFIED`, and `HEAD` distinctly through the human CLI.

### 4. Add activation and recurrence

- [x] Poll bounded Git HEAD and partial semantic-index observations into
  classified movement, a retained cursor, and idempotent proposal-only
  activations with an exact evidence acknowledgement.
- [x] Admit one acknowledged activation against the current Git cursor,
  workload HEAD, graph, scenario selection, capability snapshot, and effective
  budget without executing it.
- [x] Execute one admitted scenario selection under its exact evidence budget,
  retain its typed deltas separately from qualification, and replay the result
  without rerunning the graph.
- [ ] Extend activation evidence through watched refs, reachable/path deltas,
  and complete supported index semantics.
- [x] Coalesce compatible same-transition activations onto one exact retained
  scenario result only when all frozen inputs match and the result fits the
  stricter admission budget.
- [ ] Advance recurrence state only after required evidence reaches its claimed
  retention boundary; retain skipped/coalesced poll evidence explicitly.
- [ ] Run the loop under explicit iteration, time, action, evidence, retry,
  cancellation, and partial-failure bounds; stop without claiming convergence
  when a bound or evidence gap is reached.

### 5. Qualify the slice

- [x] Exercise the complete path through `rey workloads ... -vv`, including
  stdout, stderr, JSON, exit behavior, stale preconditions, and re-observation.
- [ ] Prove deterministic replay and equivalence with full bounded
  recomputation for the selected invalidation fixture.
- [ ] Pass focused tests, `just check`, `just test`, the packaged Nix path, and
  repository link/truth review.

## Deferred

Learned ranking, automatic retirement, arbitrary provider tools, parser/index
breadth, provider-specific agent loops, remote queues, distributed scheduling,
and autonomous production mutation require later plans.
