# ADR 0022: Portfolio Mining And Workload Attention

- Status: Accepted
- Date: 2026-08-08
- Extends: [ADR 0017](0017-mining-capability-model.md), [ADR 0018](0018-first-mining-workload.md)

## Context

Rey's first mining slice treats mining as an operation inside one workload
graph: retrieve exact source evidence, compare it, and use a residual delta to
orient a graph revision. That is necessary but incomplete. In normal use, the
runtime must also mine its workload portfolio to discover which existing
workloads need attention and which admitted context surfaces have no workload
at all.

Mining is therefore ongoing, not a one-time graph stage. Workloads are both
instruments that mine a domain and objects that Rey mines for qualification,
staleness, dependency, capability, coverage, and policy evidence. A generic
scheduler must not invent those domain facts, and an agent must not receive an
unbounded catalog dump or declare its own work resolved.

## Decision

Rey has two nested, bounded mining loops:

```text
inner workload campaign
  execute graph → observe scenarios → compute deltas → refine graph

outer portfolio campaign
  mine catalog/results/environment/coverage → derive attention
  → test, refine, create, or block work → observe portfolio again
```

The outer loop emits `rey.workload-attention.v1`, a canonical typed relation
whose rows preserve an action, subject kind and identity, reason, readiness,
evidence and dependency references, priority, and estimated cost. Its first
action vocabulary is `REFINE`, `RETEST`, `CREATE`, `BLOCK`, and
`POLICY_EXCLUDED`. Blocked and excluded rows remain visible; they are not
silently removed from counts.

The deterministic `rey.portfolio.attention.derive@1` operation consumes one
bounded `rey.portfolio-snapshot.v1`. The snapshot binds exact workload and
graph revisions, retained qualification/result evidence, explicit policy,
changed dependencies, missing capabilities, mapped context surfaces,
ownership, environment snapshot identity, and effective limits. Polars is the
canonical in-process relation for attention rows; structured JSON and terminal
text are projections of the same semantic identity.

Attention is scheduler input, not a scheduling verdict. A later scheduler may
select only admitted ready rows and must preserve their evidence. It does not
create attention facts, hide blockers, or reinterpret policy exclusions. An
agent may receive a bounded reasoning surface for selected attention and
propose a workload or graph change through ordinary admission. Scenario and
portfolio re-evaluation, not the proposer, decide whether attention resolved.

Portfolio mining remains workload-centered. The system workload
`rey.portfolio.attention` composes derivation and rendering operations and is
visible through the existing commands:

```text
rey workloads list
rey workloads test rey.portfolio.attention -vv
rey workloads run rey.portfolio.attention
rey workloads status rey.portfolio.attention
```

`list` and `status` derive a read-only view from the compiled catalog, retained
workload results, and retained environment HEAD or admission index; they do not
probe ambient state. `run` uses the same retained inputs and requires a fresh
qualification. `test` uses reviewed snapshots covering refine, retest,
create, block, policy exclusion, and a clean portfolio with no attention.

The first live surface source is an admitted environment-mapping input file.
Until workload ownership declarations exist, such a surface is truthfully
unowned and produces `CREATE`. Deliberately failing fixtures and the portfolio
miner itself have explicit exclusion policies; exclusion affects attention,
not whether their graphs may be tested directly.

## Consequences

- Mining has a continuous portfolio role in addition to per-workload evidence
  acquisition.
- Workload attention, coverage, blockers, and exclusions become directly
  human-verifiable in the primary CLI.
- The generic scheduler remains provider- and domain-neutral.
- A clean portfolio emits a typed empty attention relation rather than an
  invented task or convergence proof.
- Parser/symbol mining, generated external workloads, automatic graph edits,
  ownership declaration syntax, dependency invalidation, recurring execution,
  and agent proposal/admission remain later Plan 0010 milestones.
