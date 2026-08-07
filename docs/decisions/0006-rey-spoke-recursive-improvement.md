# ADR 0006: Rey–Spoke Recursive Improvement Loop

- Status: Accepted
- Date: 2026-08-07
- Extends: [ADR 0003](0003-spoke-and-proof-boundary.md) and
  [ADR 0005](0005-environment-awareness-and-optional-spoke.md)

## Context

Rey is intended to be Spoke's first external application on the runtime. A
first-party internal shortcut would avoid the real integration friction that an
external agent application needs Spoke to solve. A hard circular dependency,
however, would make Rey unable to diagnose a broken Spoke and make Spoke unable
to start without Rey.

The projects need a recursive improvement loop in which each makes the other
more capable while their package, storage, and startup graphs remain acyclic.

## Decision

Rey exercises Spoke strictly as an external client of its public runtime
contracts. Its real codebase exploration workloads become conformance spaces
for Spoke query, compute, persistence, and lineage.

The feedback loop is:

1. Rey inventories the current environment and Spoke capability surface.
2. Standalone or connected Rey explores the Rey and Spoke repositories.
3. Missing capabilities, schema mismatches, failed runs, excessive friction,
   and proof gaps become typed deltas and frontier evidence.
4. The relevant project changes behind its own ownership boundary.
5. Rey re-probes the environment and observes the new capability or behavior.
6. The same scoped fixture establishes whether the gap closed and identifies
   the next frontier.

Cross-project evidence names exact Rey and Spoke revisions plus the public
contract under test. A conformance fixture may be mirrored or generated in both
repositories, but each repository can execute its side independently. Neither
imports the other's private crates or data formats merely to make the test pass.

Bootstrapping remains acyclic:

- Rey's core, environment discovery, local frames, deltas, and proofs require
  no Spoke process.
- Spoke's core build, startup, and capability tests require no Rey process.
- Connected integration is an additional acceptance layer for both projects.

Rey may use Spoke-backed codebase spaces to improve Rey itself. That recursive
use is recorded like any other action and is subject to exact source bindings,
capability snapshots, mutation admission, and fresh proofs.

## Consequences

- Spoke public interfaces are evaluated against a real external application
  from the first Rey integration slice.
- Rey needs a queryable capability/conformance frame, not only ad hoc client
  error logs.
- Spoke gaps should yield reproducible fixtures and scoped claims that can be
  cited in Spoke plans.
- New Spoke capabilities are not considered useful to Rey until discovery and
  an end-to-end external-client path prove them.
- Neither repository may introduce a private integration shortcut solely to
  make the feedback loop green.
- Cross-project compatibility state can be stale independently when either Rey,
  Spoke, the fixture, or the evaluator changes.
