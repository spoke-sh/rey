# ADR 0016: First Executable Workload Slice

- Status: Accepted
- Date: 2026-08-07
- Implements: [ADR 0015](0015-workload-centered-product.md)
- Supersedes public identity schemas from:
  [ADR 0014](0014-frontier-progress-and-scheduling.md)

## Context

ADR 0015 makes workload the public computation unit and requires one small
zero-agent slice through `workloads list`, `test`, `run`, and `status` before
generic scheduling or a provider-specific agent loop. The repository has no
workload manifest, catalog, graph executor, scenario delta, qualification, or
result provider.

The first slice needs retained cross-command progress without selecting a
database or pretending local files provide Spoke guarantees. It also needs a
real failing scenario so the CLI proves that a test failure is an authoritative
typed delta rather than prose or process status.

## Decision

### Built-In Catalog

The first catalog is compiled into Rey and contains two bounded text workloads:

- `rey.fixture.text-normalize` executes `trim -> uppercase` and has two passing
  required scenarios; and
- `rey.fixture.text-mismatch` executes `uppercase` against the same expectation,
  producing one passing and one failing required scenario.

These are explicit fixtures, not a general declaration format. They use
`rey.workload.v1`, `rey.compute-graph.v1`, and `rey.scenario-suite.v1` semantic
documents owned by `rey-runtime`. Graphs contain one-input/one-output built-in
UTF-8 operations, typed source edges, selected outputs, exact contract
identities, and hard node/input/output/string-byte limits.

Graph validation proves unique ids, known typed inputs and outputs, admitted
operation contracts, compatible edges, acyclicity, stable topological order,
and every effective bound before execution. Inline executable text and
external tools are unsupported.

### Scenario Delta And Qualification

`rey-diff` adds the narrow `rey.scenario-output-delta.v1` specialization. It
binds workload, graph, scenario, output, comparator, `EXPECTED`, `OBSERVED`,
UTF-8 type, and equal/different assessment under a semantic digest. Both equal
and different results are retained.

`rey-runtime` executes every required scenario deterministically and emits
`rey.workload-test-result.v1`. A result passes only when every required output
delta is equal. Any different delta fails. This built-in slice has complete
inputs and no inconclusive provider path, but the status vocabulary reserves
that result.

A `rey.workload-qualification.v1` record is emitted only for a passed result
and binds the exact workload, graph, scenario suite, evaluator, and test result.
No policy participates in this slice.

### Local Result Provider

The `rey` composition crate stores a bounded `rey.local-workload-state.v1` JSON
document beneath `${workspace}/.rey/workloads/state.json` by default. An
explicit `--state-dir` selects another boundary. Publication writes a
same-directory temporary document and renames it over the state file. Reads are
bounded and verify every retained semantic artifact before use.

This is a single-process local result provider. It claims no `fsync` crash
durability, multi-process transactionality, locking, authenticated writer,
remote durability, or Spoke revision semantics. Built-in workload definitions
and graphs remain in the binary; the state file is not their sole copy.

Freshness compares retained workload, graph, scenario-suite, and evaluator
identities with the current built-in catalog. Stale scenario results do not
count as passing, and stale qualification cannot admit `run`.

### CLI

The accepted commands become:

```text
rey workloads [--workspace <path>] [--state-dir <path>] list
  [--format auto|table|json]

rey workloads [--workspace <path>] [--state-dir <path>] status
  [<workload-id>] [--format auto|table|json]

rey workloads [--workspace <path>] [--state-dir <path>] test
  [<workload-id>] [--format auto|table|json]

rey workloads [--workspace <path>] [--state-dir <path>] run
  <workload-id> --input <utf8> [--format auto|table|json]
```

`auto` selects table on a terminal and JSON when redirected. Final results go
to stdout. The deterministic built-in pass emits no transient progress or
policy rationale. `list` and `status` return `0` when inspection succeeds.
`test` returns `0` when every selected workload passes, `2` on any conclusive
failure, and `3` on inconclusive/blocked state. `run` returns `3` with a
structured blocked result for missing or stale qualification. Invalid input,
state, graph, or runtime behavior returns `1` on stderr.

### Runtime Identity Cutover

The existing runtime state reducer contains no application/component fields and
remains `rey.runtime-state.v2`. The legacy fields exist in frontier,
scheduling, progress, and reasoning-surface inputs.

This slice makes a pre-alpha hard cut:

- frontier, progress, and scheduling documents/relations become v2; and
- reasoning-surface document/relation becomes v3.

Their inputs replace application/component with exact workload, graph,
scenario-suite, and campaign identities. There is no compatibility alias or
decoder for the superseded schemas.

## Consequences

- All four workload commands operate over real deterministic state.
- `list` can show progress from a prior `test` without executing scenarios.
- A failing CLI fixture preserves the exact expected/observed delta and exits
  semantically.
- `run` demonstrates qualification admission and test/run graph parity.
- External manifests, agent proposals, frontier derivation from scenario
  deltas, Spoke retention, arbitrary operations, parallel execution, retries,
  and recurring scheduling remain later bearings.
