# ADR 0038: Unauthenticated Hyperlinkable Journal

- Status: Accepted
- Date: 2026-08-10
- Extends: [ADR 0037](0037-explore-bound-collaboration-journal.md)
- Supersedes: the loopback-only, exact-origin browser admission constraint in ADR 0037

## Context

The first writable Journal enabled browser admission only on loopback and
required the request's `Origin` to match the bound UI URL. That restriction
was stricter than the intended collaboration model. Rey has no authentication
system, Journal author labels are not verified principals, and an operator may
explicitly bind `rey ui` to another interface precisely so collaborators can
use it.

The first UI also rendered complete retained entries inside `/agents`. That
made a document visible but not independently addressable. A notebook, query
surface, Explorer reference, frame, or diff must be linkable as a document and
down to its exact block if it is to become the durable plane of human/agent
communication.

## Decision

`POST /api/v1/journal` accepts a validated human-authored Journal proposal on
every explicitly configured `rey ui` listener. It requires no authentication,
authorization token, cookie, or matching `Origin`. The author `id` is a
self-asserted label. Network exposure remains explicit in startup output and
stderr; whoever can reach the listener can submit a bounded Journal document.

This relaxation changes only document admission. The existing content-type,
byte, schema, author-kind, canonical-coordinate, block, identity, ordering,
locking, and atomic-publication checks remain. Admission still cannot execute
a query or action, assign an agent, mutate a workload, qualify a claim, or
publish proof.

The browser contract is:

```text
/agents                       Journal index and derived system entries
/journal/new                  human document composer
/journal/{entry-slug}         exact retained document
/journal/{entry-slug}#block-{block-id}
                              exact typed block
```

The canonical entry slug is derived from immutable retained fields:

```text
j{sequence}-{ascii-title-up-to-80}--{complete-normalized-entry-id}
```

The sequence makes the document position visible, the title makes the route
readable, and the complete content identity makes accidental aliasing
detectable. `/agents` links each retained entry to its canonical slug. The CLI
prints the same document path on add and list. A route resolves only an exact
canonical slug from the current verified log; it never guesses by title or
prefix. Every typed block owns `id="block-{block-id}"` and exposes its fragment
permalink.

The UI capability envelopes advance to `rey.ui-server.v3` and
`rey.ui-journal.v2`. Both loopback and non-loopback descriptors report the
narrow write capability and `unauthenticated_journal_admission` authority.

## Consequences

- Human collaborators can write through an explicitly exposed Rey UI without
  an authentication setup step.
- A non-loopback bind is a real unauthenticated write boundary, not a read-only
  projection; operators must protect reachability externally when required.
- Author labels provide attribution vocabulary, not verified identity.
- Journal documents and blocks can be referenced from chat, tasks, Explorer,
  CLI output, reviews, and later Journal entries without copying their content.
- Authentication, multi-user authorization, remote durability, and broader
  control-plane mutation remain unimplemented and are not implied.
