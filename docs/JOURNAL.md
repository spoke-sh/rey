# Collaboration Journal

The Rey Journal is the shared, retained reasoning surface between a human
operator, an agent, and Rey's deterministic frontier projection. It sits
between transient conversation and executable workload admission:

```text
conversation → entry proposal → validate → retain → project → supersede
                                      │
                                      └─ no execution authority
```

A Journal entry can explain context, carry a read-only query declaration,
show a bounded frame or directed diff, point into `/explore`, and propose a
next action. Admission means only that Rey accepted and ordered the document.
It does not execute a query, assign an agent, mutate a workload, prove an
observation, or let an author qualify its own claim.

## Entry Contract

`rey.journal-entry-proposal.v1` is the authored contract. Admission derives a
content identity and produces `rey.journal-entry.v1`; the ordered retained
collection is `rey.journal-log.v1`.

| Field | Meaning |
| --- | --- |
| `schema` | Exact proposal schema |
| `title` | Human-readable entry identity, 1–240 characters |
| `author` | Typed `human`, `agent`, or `system` identity |
| `binding` | Canonical Explorer coordinate plus the exact source revision encoded by its `at` dimension |
| `supersedes` | Optional identity of an earlier retained entry; history is never rewritten |
| `blocks` | Ordered bounded document of 1–32 typed blocks |

The retained entry adds `entry_id`, one-based `sequence`, and `admitted_at`.
The entry identity is derived from the canonical proposal rather than its
admission time. Re-admitting identical content is idempotent and returns the
existing entry. A supersession must point backward within the same log.

Every entry coordinate uses the canonical Explorer grammar:

```text
/explore/{kind}/{identity};at={revision};lens={regime}[;role={agent-role}]
```

Matrix dimensions are lexically ordered. The decoded `at` value must equal
`binding.source_revision`. The binding may intentionally name historical
context; Explorer resolution, rather than journal admission, determines
whether that exact context is current, stale, or missing.

## Document Addresses

Every retained entry and typed block has a stable browser address:

```text
/agents                                      Journal index
/journal/new                                 new human entry
/journal/{slug}                              exact retained entry
/journal/{slug}#block-{block-id}             exact typed block
```

The canonical slug is
`j{sequence}-{bounded-ascii-title}--{normalized-full-entry-id}`. The readable
title aids recognition; the complete content identity prevents two documents
from collapsing onto one route. Entry routes resolve exactly and never fall
back to a title, prefix, sequence, or current document. Selecting an entry in
the index enters its canonical route, and successful creation replaces
`/journal/new` with the admitted entry route.

Block ids are unique within an entry and become fragment permalinks. A block
heading links to its own fragment so prose, query, frame, diff, Explore, and
action context can be cited without copying the document. These addresses are
presentation coordinates over retained Journal state; they do not turn a URL
into admission or execution authority.

## Notebook Blocks

The ordered block document is the high-fidelity communication grammar. It is
typed data, never arbitrary HTML or executable browser content.

| Block | Purpose | Important bounds |
| --- | --- | --- |
| `prose` | WYSIWYG-style notebook nodes: heading, paragraph, bullet, quote, or code | 1–128 nodes; 64 KiB total text |
| `explore` | Embedded map/lens reference with its own exact source revision | Canonical coordinate; optional caption |
| `query` | SQL or another provider query declaration | `mode` must be `read_only`; 32 KiB statement; admission never executes it |
| `frame` | Bounded tabular result preview tied to an earlier query block and immutable snapshot | 64 typed columns; 100 preview rows; explicit nulls, row count, and truncation |
| `diff` | Directed comparison between exact source and target locators | Explicit direction and `equal`, `different`, or `inconclusive` assessment |
| `action` | Proposed operation and desired delta | Bounded evidence and dependency locator sets |

Frame, diff, and action blocks are communication surfaces, not proof merely
because an author submitted them. Exact evidence locators and the runtime's
normal validators remain authoritative.

This is the first implemented authoring envelope. The `/journal/new` composer currently
creates prose, exact Explore, and optional read-only SQL cells. Agent proposals
may use every block type, so richer retained artifacts render immediately in
the same Journal document interface.

## Authoring Surfaces

Agents write a workspace-contained YAML proposal through the CLI:

```sh
rey journal add path/to/entry.yaml
rey journal list
```

`rey journal add` admits only `author.kind: agent`. Proposal files must be
regular, non-symlinked, contained beneath the workspace, and at most 1 MiB.
The command does not execute any block.

Humans enter through `/journal/new`. `POST /api/v1/journal` accepts a bounded
JSON proposal on every address the operator explicitly binds. It requires no
session, credential, token, cookie, or matching `Origin`, and it accepts only
`author.kind: human`. Author ids are therefore self-asserted labels, not
authenticated principals. Whoever can reach a non-loopback listener can
submit a Journal document; Rey warns about that write boundary at startup.

Rey's system entries remain deterministic projections from workload requests
and attention. They are not copied into the authored log, so retained human or
agent statements stay distinguishable from current derived frontier state.

Both authoring paths use the same validator, content identity, ordered local
log, entry/block limits, and atomic locked publication beneath
`.rey/journal/journal.json`. This local log is standalone runtime state, not a
claim of Spoke durability, multi-user consistency, authenticated identity, or
remote retention. Unauthenticated admission does not weaken validation,
content identity, limits, atomic publication, or the rule that Journal blocks
carry no execution authority.

## Example Agent Proposal

```yaml
schema: rey.journal-entry-proposal.v1
title: Source coverage moved after survey
author:
  kind: agent
  id: codex
binding:
  coordinate: /explore/workload/source-mining;at=blake3%3Aabc;lens=objects
  source_revision: blake3:abc
blocks:
  - kind: prose
    id: context
    document:
      - kind: paragraph
        text: Two unowned source surfaces remain after the latest survey.
  - kind: query
    id: coverage-query
    language: sql
    provider: spoke
    mode: read_only
    statement: select * from coverage where owner is null
    parameters: {}
  - kind: action
    id: next-bearing
    operation: refine
    desired_delta: Reduce unowned source surfaces from two to zero.
    evidence_ids:
      - spoke+local://coverage/latest
    dependency_ids: []
```

See [Context Topology Explorer](EXPLORER.md) for coordinate resolution and
[ADR 0037](decisions/0037-explore-bound-collaboration-journal.md) plus [ADR
0038](decisions/0038-unauthenticated-hyperlinkable-journal.md) for the decision
boundary.
