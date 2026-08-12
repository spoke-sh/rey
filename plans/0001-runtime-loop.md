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

Rey already derives typed portfolio attention, canonical frontier/progress,
deterministic scheduling, and reasoning surfaces. Workspace creation requests
are visible and content-identified, but Rey does not invoke a harness or bind a
harness response to one selected attention row. Workload-owned surfaces are
not declared, retained environment/Git changes do not generally invalidate
their owners, and no recurring scheduler or Git cursor drives the loop.

## Completion Checklist

### 1. Bind ownership and invalidation

- [ ] Define bounded workload-owned surface declarations with exact workload,
  graph, environment, and source revisions.
- [ ] Derive changed-dependency and missing-capability facts from retained
  environment and Git evidence instead of fixture-only or empty live fields.
- [ ] Prove ownership collision, missing owner, changed owner, stale revision,
  policy exclusion, incomplete coverage, and typed-empty attention behavior.

### 2. Hand attention to runtime work

- [ ] Project admitted ready attention rows into the generic frontier without
  copying the portfolio or allowing the scheduler to invent reasons.
- [ ] Build one bounded reasoning surface that cites the exact attention row,
  dependent deltas, evidence, omissions, allowed graph operations, and total
  budget.
- [ ] Preserve distinct attention, frontier, scheduling, progress, and proof
  identities through CLI table and structured output.

### 3. Complete one harness request/response cycle

- [ ] Bind one `CREATE` or `REFINE` row, its reasoning surface, current package
  state, failing deltas, permitted operations, and limits into a creation
  request.
- [ ] Accept one immutable external harness response as a verified WORKING
  package; do not launch an ambient executable or fabricate graph/scenario
  content in Rey.
- [ ] Stage exact bytes, execute the frozen scenarios, require human admission,
  and re-mine whether the original row resolved, changed, or remained open.
- [ ] Show `AWAITING HARNESS`, `WORKING`, `INDEX UNQUALIFIED`, `INDEX
  QUALIFIED`, and `HEAD` distinctly through the human CLI.

### 4. Add activation and recurrence

- [ ] Complete bounded Git commit/ref/index observation, movement
  classification, retained cursors, and idempotent activation proposals.
- [ ] Coalesce replay safely and advance a cursor only after required evidence
  reaches its claimed retention boundary.
- [ ] Run the loop under explicit iteration, time, action, evidence, retry,
  cancellation, and partial-failure bounds; stop without claiming convergence
  when a bound or evidence gap is reached.

### 5. Qualify the slice

- [ ] Exercise the complete path through `rey workloads ... -vv`, including
  stdout, stderr, JSON, exit behavior, stale preconditions, and re-observation.
- [ ] Prove deterministic replay and equivalence with full bounded
  recomputation for the selected invalidation fixture.
- [ ] Pass focused tests, `just check`, `just test`, the packaged Nix path, and
  repository link/truth review.

## Deferred

Learned ranking, automatic retirement, arbitrary provider tools, parser/index
breadth, provider-specific agent loops, remote queues, distributed scheduling,
and autonomous production mutation require later plans.
