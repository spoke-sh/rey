# ADR 0013: Runtime State And Reasoning Surface Contracts

- Status: Accepted; v1 schema selections superseded by ADR 0014
- Date: 2026-08-07

## Context

ADR 0012 fixes Rey's bootstrap and recurring orientation/transition lifecycle,
distinguishes transition from residual deltas, and defines the reasoning
surface as a bounded delta-directed policy input. Those architecture rules are
not yet precise enough to test replay, reject an illegal phase change, or prove
that a surface retains exact frontier, evidence, capability, and action
identity.

Generic scheduling would be premature without those contracts. Otherwise the
first scheduler or agent adapter would implicitly select lifecycle states,
surface bounds, completeness behavior, and identity semantics that belong to
the deterministic runtime.

## Decision

Rey adds two contract-only crates with no scheduler or provider executor:

- `rey-runtime` owns `rey.runtime-state.v1`, a pure deterministic reducer over
  typed lifecycle events; and
- `rey-policy` owns `rey.reasoning-surface.v1` plus its canonical
  `rey.reasoning-surface-rows` version `1` DataFrame projection.

### Runtime State

The v1 Rey phases are:

```text
bootstrapping
ready
orienting
awaiting_proposal
admitting
executing
observing
evaluating
committing
stopped
```

The normal recurring path is:

```text
ready
  -> begin_orientation
orienting
  -> reasoning_surface_ready
awaiting_proposal
  -> proposal_received
admitting
  -> proposal_admitted
executing
  -> execution_finished
observing
  -> observation_completed
evaluating
  -> evaluation_completed
committing
  -> transition_committed
ready | stopped
```

`orientation_failed` and `proposal_rejected` also enter `committing`; they do
not disappear into a retry. Cancellation requested during execution remains
in `executing` until the provider reports a terminal execution outcome, after
which Rey still observes and evaluates. A successful execution enters
`observing`; it cannot jump to convergence or commit.

Every active event must cite the same transition identity established by
`begin_orientation`. Evidence is `pending` during an active transition. Rey can
continue from a committed transition only when evidence is `retained` or
`verified` at the selected local or Spoke-backed retention profile. Missing,
stale, or pending evidence cannot admit the next action.

Semantic outcome remains independent of execution and evidence state. V1
semantic outcomes are `unresolved`, `progressing`, `unchanged`, `regressing`,
`converged`, and `inconclusive`. A converged outcome has no next frontier and
must follow a complete observation and commit retained or verified evidence
with a `converged` stop reason. Unavailable or failed observation is
inconclusive. Nonterminal outcomes retain a next frontier. Inconclusive
evaluation stops explicitly. Bootstrap can be unresolved, converged, or
inconclusive, but it cannot claim progress without a predecessor residual.
Budget, timeout, cancellation, capability, evidence, and failure stops do not
become convergence.

The runtime state is content-identified with domain-separated, length-framed
BLAKE3 over its trace, phase, sequence, retention/evidence state, committed and
active identities, observations, transition and residual deltas, semantic
outcome, stop state, and effective limits. V1 bounds accepted event count plus
retained observation, transition-delta, and residual-delta references. State
verification recomputes identity and checks limits, phase shape, and canonical
order. This is replayable state, not a durable event-log implementation.

### Reasoning Surface

`rey.reasoning-surface.v1` binds:

- exact application, component, space, trace, committed transition, active
  transition, frontier frame, capability snapshot, and projection identities;
- effective row, delta-reference, evidence-reference, action-reference,
  omission, evidence-byte, string-byte, and retrieval-iteration limits;
- actual retrieval iteration count and complete, partial, or truncated state;
- canonical projected frontier rows;
- exact provider/source/revision/media-type/content identities for selected
  evidence;
- exact versioned admissible action contracts; and
- explicit typed omissions.

Each projected row has a unique `frontier_row_id`, entity kind and identity,
separate transition- and residual-delta identity arrays, claim identities,
evidence identities, and admissible action identities. A row must cite at least
one delta or claim. One delta cannot silently hold both roles in the same row,
and every evidence or action citation must resolve within the surface.

The constructor canonicalizes list and row order, rejects duplicate top-level
identities, enforces every declared limit, derives the semantic surface
identity, and exposes the rows as a typed Polars DataFrame. Arrow metadata
retains the surface, application/component/space, trace/transition, frontier,
capability, projection, completeness, retrieval, and count identities. Native
evidence remains provider-owned content referenced by the surface rather than
being flattened into cells.

A complete surface has no omissions. Partial or truncated surfaces must name
at least one omission. Unavailable or failed orientation does not manufacture
an empty reasoning surface; it follows `orientation_failed` and commits an
explicit stop.

### Boundary

These crates do not:

- derive, rank, or schedule a frontier;
- retrieve from local or Spoke providers;
- decide when iterative orientation has enough information;
- define or invoke a policy proposal adapter;
- admit or execute an action;
- compare arbitrary frames or calculate generic progress; or
- persist a trace or claim Spoke durability.

Those behaviors must consume these contracts and remain independently bounded
and testable.

## Consequences

ADR 0014 later preserves these lifecycle and surface invariants while cutting
the runtime and reasoning surface to v2 so a scheduling decision is recorded
before orientation. This document remains the rationale and historical v1
contract.

- Illegal lifecycle jumps and mismatched transition identities fail before a
  scheduler or executor exists.
- Process success, semantic convergence, and evidence retention are proven as
  separate dimensions in executable fixtures.
- A future scheduler receives one canonical, bounded policy surface rather
  than inventing its own context envelope.
- Surface identity changes with frontier, evidence, capability, action,
  projection, omission, or effective-limit semantics.
- The next implementation bearing can address deterministic frontier and
  progress relations without coupling them to provider execution or an LLM.
