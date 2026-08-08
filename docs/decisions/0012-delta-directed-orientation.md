# ADR 0012: Delta-Directed Orientation And Runtime Lifecycle

- Status: Accepted
- Date: 2026-08-07

## Context

ADR 0001 establishes that deltas direct compute, but the architecture moves
from a derived frontier directly to a policy proposal. A useful agent runtime
needs an explicit account of how it retrieves the evidence relevant to an
unresolved delta, projects that evidence into bounded policy input, and
determines whether the resulting action made semantic progress.

Without that account, a policy can receive an accidental workspace snapshot,
retrieval can become an unbounded provider-specific prompt concern, and a
successful process can be mistaken for progress. A single lifecycle status
would also conflate provider execution, semantic convergence, and proof or
evidence state.

## Decision

Rey has a bootstrap path and a recurring transition path. Bootstrap discovers
the bounded capability snapshot, materializes declared initial observations,
and establishes baselines or evaluates claims. It does not fabricate a
transition delta from an unobserved prior world.

The steady-state lifecycle is:

```text
committed delta and frontier
  -> orient (retrieve -> project)
  -> propose
  -> admit
  -> execute (probe | mutation)
  -> observe
  -> compare and evaluate
  -> commit transition
  -> next delta and frontier
```

The transition record is committed before its delta or frontier directs
another action. "Record" means adding replayable trace evidence at the
selected retention boundary; it does not make Rey a standalone log service or
give local traces Spoke durability.

Rey distinguishes two directed delta roles:

- a **transition delta** compares the relevant observations immediately before
  and after an action and states what changed; and
- a **residual delta** compares a declared expected or baseline observation to
  the current observation and states what remains unresolved for a claim.

Not every claim must be flattened into one table or one residual delta. The
frontier is the bounded typed relation that combines applicable residual
deltas, unsatisfied claim facts, invalidation, dependencies, admissible
actions, and priority inputs. Transition and residual deltas retain their own
direction, labels, identities, and comparison contracts.

Orientation constructs the policy input for a committed frontier:

```text
reasoning surface =
  project(retrieve(frontier, exact sources, capabilities, budgets))
```

Retrieve and project may repeat within one orientation phase as additional
exact evidence changes the surface. The runtime owns the iteration, byte,
time, and provider limits plus trace lineage; a versioned orientation strategy
owns evidence ordering and stopping readiness. The phase stops explicitly when
the surface is ready, no eligible evidence remains, or a bound is reached.
Expected information value guides navigation, but only later observation can
establish actual progress.

Retrieval resolves only declared, admitted, read-only evidence through the
provider that owns it. Local providers retain local guarantees; Spoke retains
its query, document, table, stream, storage, and revision guarantees. Rey owns
why evidence was requested and how it relates to the frontier, but does not
duplicate provider retrieval or storage semantics. If obtaining information
requires running a tool, observing a mutable source, or producing a new lens
result, it is a probe transition rather than hidden retrieval.

Projection is deterministic, versioned, typed, and bounded. A reasoning
surface binds its frontier and delta inputs, exact retrieved evidence, source
and capability revisions, projection contract, omissions, truncation, and
effective budgets. It may contain bounded DataFrame projections and handles to
native artifacts; it is neither a new durable source of authored content nor
permission to execute cited tools. A policy proposal cites the reasoning
surface and the frontier evidence from which it was made.

Rey assesses progress only after post-action observation. Progress compares
successive residual/frontier states and remains distinct from similarity,
confidence, coverage, proof status, and provider process status. The initial
assessment is typed and multidimensional: it can report resolved, introduced,
reopened, unchanged, or incomparable work; information or completeness gained;
changed guarantees; and resources consumed. No universal scalar score is
authoritative. A versioned policy may rank work with an explicit objective,
including expected residual reduction per bounded cost, without changing the
underlying deltas or proof result.

Runtime state is therefore a product of separate dimensions rather than one
giant lifecycle enum:

- provider execution records proposal, admission, attempts, terminal process
  state, captures, and cancellation;
- semantic transition state records unresolved, progressing, unchanged,
  regressing, converged, or inconclusive outcomes under exact observations;
  and
- evidence and proof state records retention, verification, missing evidence,
  and the existing proof statuses independently.

An empty frontier is convergence only when all required claims and
completeness rules permit that conclusion. Exhausted budgets, missing
evidence, incompatible residuals, unavailable providers, or an unprojectable
reasoning surface stop explicitly and never become convergence.

## Consequences

- The policy sees a small delta-directed surface instead of an ambient
  workspace snapshot.
- Retrieval quality can be evaluated by subsequent residual reduction and
  information gain without trusting the policy's prediction.
- Rules, humans, and agents use the same bounded surface and proposal
  contract.
- Exact provider ownership and standalone-versus-Spoke guarantees survive
  retrieval and projection.
- Trace replay can explain which committed delta caused which evidence to be
  retrieved and why the next action was selected.
- Generic reasoning-surface schemas, progress relations, scheduling, and
  multi-step runtime implementation remain future work and require focused
  limits, replay, convergence, and partial-failure tests.
