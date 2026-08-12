# Plan 0004: Workload Product And CLI Contracts

## Outcome

Make workload the public organizing concept before implementing more generic
scheduling. Define versioned compute graphs, scenario-directed test campaigns,
qualification, progress, staleness, catalog/retention boundaries, and the
`list`, `test`, `run`, and `status` command semantics. Reconcile the existing
application-centered target language without claiming that workload behavior
is implemented.

## Completion Checklist

- [x] ADR 0015 accepts the workload-centered product and CLI cutover.
- [x] `docs/WORKLOADS.md` defines workload, graph, scenario, campaign,
  qualification, progress, run, catalog, retention, and staleness contracts.
- [x] The public command surface is limited to `rey env ...` and the
  four workload commands.
- [x] Failing scenarios retain directed `EXPECTED` to `OBSERVED` typed deltas.
- [x] An agent, rule, or human proposes graphs through one validated contract
  and cannot declare qualification.
- [x] List progress separates passing, evaluated, blocked, inconclusive, stale,
  optional, and unrun scenarios for one exact graph revision.
- [x] Test and run share one graph contract while keeping fixture providers and
  real effects distinct.
- [x] Catalog and result-provider responsibilities are defined without
  selecting an encoding, database, directory layout, or remote mapping.
- [x] Architecture, runtime, frontier, Git, proof, interface, roadmap, and
  contributor guidance use the workload-centered model.
- [x] Current implementation truth and the required versioned legacy-schema
  cutover remain explicit.
- [x] Documentation links, formatting, and repository truth are verified.

## Accepted Boundaries

- The initial compute graph is a bounded typed DAG of admitted operation
  contracts. Campaign revisions provide feedback; graph cycles are deferred.
- Scenario comparison is `EXPECTED` to `OBSERVED`. Conclusive differences
  fail, comparison inability is inconclusive, and complete equality is scoped
  to the declared scenario.
- A graph qualifies only when all required scenarios freshly pass for the same
  exact graph and semantic inputs.
- A graph revision invalidates prior scenario currency. Fine-grained reuse
  requires a later dependency-closure proof.
- `workloads test` may iterate graph proposals within declared bounds, but an
  already supplied graph remains deterministically testable without an LLM.
- `workloads run` defaults to a fresh qualified graph and revalidates effects
  and capability preconditions.
- `workloads list` and `status` are read-only views over retained state and do
  not execute work to repair missing progress.
- Transient progress uses stderr; final human or structured results use stdout.

## Deferred Implementation Bearing

The next slice should select only the minimum versioned declaration and record
schemas needed to run one built-in, zero-agent fixture workload through:

1. catalog resolution and `workloads list`;
2. graph/scenario inspection through `workloads status`;
3. deterministic scenario execution and typed mismatch deltas through
   `workloads test`; and
4. execution of the same fresh qualified graph through `workloads run`.

That slice must cut the legacy application/component schema fields to explicit
workload, graph, scenario, campaign, and run identities. Agent generation,
remote retention, arbitrary external operations, incremental scenario reuse,
parallel execution, recurring scheduling, and generic persistence remain
later work.

## Verification Evidence

Contract review captured on 2026-08-07:

```text
git diff --check
just check
# Rustfmt, Clippy, flake evaluation, and repository whitespace checks passed
# all local Markdown links resolved
# all application and component references reviewed as external-provider wording,
# historical ADR context, explicit legacy schema truth, or stale text removed
```
