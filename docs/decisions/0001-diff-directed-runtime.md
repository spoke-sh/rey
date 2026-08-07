# ADR 0001: Diff-Directed Runtime And Policy Boundary

- Status: Accepted
- Date: 2026-08-07

## Context

Agent systems often treat a workspace snapshot as context, execute a tool, and
produce a diff only for human review at the end. That leaves scheduling and
convergence dependent on an agent repeatedly interpreting a large, loosely
structured state.

Rey needs a runtime model suitable for high-dimensional spaces such as
codebases. It must be useful with or without an LLM, explain why each unit of
compute ran, and distinguish tool execution from observed semantic progress.

## Decision

Rey is a diff-directed compute runtime. A directed delta between typed
observations is a first-class control value. Deltas invalidate dependent lenses,
form a bounded frontier of unresolved work, and supply the input from which the
next action is selected.

The deterministic runtime owns:

- exact source and frame preconditions;
- lens materialization and dependency invalidation;
- action schemas, effect classes, admission, and budgets;
- delta computation and frontier derivation;
- transition, retry, cancellation, and failure lineage;
- convergence and stopping semantics; and
- proof evaluation and artifact assembly.

Policy is separate. An agent, deterministic rule, or human receives a bounded
frontier and set of admissible actions and returns a structured proposal. The
runtime validates that proposal and does not trust policy to declare its own
success, authority, evidence, or convergence.

The first topology is a local Rey CLI/library process. A long-running service or
distributed scheduler is not implied.

## Consequences

- The runtime can be tested deterministically without a model provider.
- Agent quality can be compared independently from comparison and proof
  correctness.
- Every action must cite frozen inputs and frontier evidence.
- A successful process is followed by observation and delta evaluation; exit
  status alone cannot establish semantic success.
- Dependency and prioritization semantics become versioned runtime contracts.
- The first implementation may recompute bounded frames fully; incremental
  execution needs parity proof against that semantic baseline.
