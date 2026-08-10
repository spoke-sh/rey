# ADR 0037: Explore-Bound Collaboration Journal

- Status: Accepted
- Date: 2026-08-10
- Extends: the recommendation plane in [ADR 0035](0035-agent-recommendations-and-observed-work.md)
- Extended: browser admission and document addressing in [ADR 0038](0038-unauthenticated-hyperlinkable-journal.md)

## Context

The first high-level `/agents` surface called its leading section System
Recommendations. That accurately described Rey's derived rows, but it made the
surface one-way: Rey reported what should happen and humans or agents had no
shared place to add bounded context, redirect attention, or propose a richer
reasoning surface.

Chat is too transient and tasks are too operational for this role. The mailbox
is history, the conversation plane is dialogue, and the workload frontier owns
validated execution attention. Rey needs a collaboration journal between those
planes: a place to state what should happen next while binding the statement to
the exact topology it concerns.

## Decision

Section 01 of `/agents` becomes the **Journal**, titled "What should happen
next." Current request and attention evidence continues to produce
deterministic system entries authored by `rey`; the observed-work ledger
remains a separate second section.

The target journal admits entries from three author kinds through one validated
contract:

- `human` entries originate from the operator UI;
- `agent` entries originate from the Rey CLI or an equivalent admitted agent
  interface; and
- `system` entries are deterministic projections from retained Rey evidence.

Every retained authored entry must bind an exact canonical Explorer coordinate
and source revision. It carries a content identity, author identity and kind,
creation order, desired operation or delta, evidence and dependency locators,
readiness, limits, and resolution/supersession state. High-fidelity content is
a bounded typed block document—text, relation, delta, evidence, or
visualization references—not arbitrary HTML, script, executable code, or an
unbounded browser payload. `/explore` owns topology and lens semantics; a
journal entry points into that plane rather than copying its state.

Human and agent writes must pass the same admission, validation, identity,
limit, and stale-coordinate checks. Neither author kind receives implicit
execution authority, assignment authority, or permission to qualify its own
proposal. Journal admission and workload/action admission remain distinct.

The implemented Journal uses `rey.journal-entry-proposal.v1`,
`rey.journal-entry.v1`, and the ordered `rey.journal-log.v1`. Its document
grammar contains prose, exact Explorer, read-only query, bounded frame,
directed diff, and proposed-action blocks. Blocks are inert on admission: in
particular, retaining a query does not execute it and retaining an action does
not grant effect authority.

Agents admit workspace-contained YAML proposals through `rey journal add` and
inspect the shared sequence through `rey journal list`. Humans use the
`/agents` block composer. Both paths share validation, content identity,
idempotency, order, supersession, limits, locking, and atomic local retention.
The CLI accepts agent authors; the browser endpoint accepts human authors only
from the exact same-origin loopback listener. Non-loopback `rey ui` remains a
read-only network projection and disables the composer. System entries remain
deterministic projections from current request and attention evidence rather
than copies in the authored log.

Each exact coordinate is canonical and internally revision-consistent. It may
name historical context; Explorer resolution determines whether that binding
is current, stale, or missing. See [Collaboration Journal](../JOURNAL.md) for
the complete format and implemented authority boundary.

## Consequences

- `/agents` becomes a shared collaboration surface rather than a one-way agent
  recommendation dashboard.
- System recommendations, human direction, and agent findings meet in one
  vocabulary without becoming assignments or chat logs; current system rows
  remain visibly derived while authored entries are retained and ordered.
- Exact Explorer coordinates keep rich entries inspectable, stale-detectable,
  and safe to traverse.
- A loopback-only human write exception keeps the browser boundary narrow;
  explicit network binds retain read-only authority.
- Query execution, action admission, assignment, proof, remote durability,
  multi-user identity, and Spoke retention remain separate contracts.
