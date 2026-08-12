# ADR 0054: Diff-Directed Journal Broadsheet

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0037](0037-explore-bound-collaboration-journal.md),
  [ADR 0038](0038-unauthenticated-hyperlinkable-journal.md), and the fresh
  baseline in [ADR 0048](0048-fresh-v1-contract-baseline.md)
- Supersedes: ADR 0048's `.v1` assignment for Journal documents and the
  Journal HTTP projection, the sequential Journal layout, and the separate
  retained-document/composer browser surfaces

## Context

The first Journal proved bounded typed notebook admission and stable document
addresses, but its authoring and reading grammars diverged. `/journal/new`
was a conventional form which became a materially different retained document
after admission. Retained entries could declare supersession but the browser
offered no editing path, no working delta, and no way to compose evidence and
direction beside each other without flattening blocks into one vertical stack.

Rey needs a reasoning document that can juxtapose narrative, declarations,
evidence, directed deltas, and possible next actions. It also needs the same
immutability and authority boundary as the first Journal: editing cannot
rewrite retained reasoning, and laying two cells next to each other cannot
mint a semantic relationship, execute a declaration, or admit an action.

## Decision

The Journal hard-cuts to `rey.journal-entry-proposal.v2`,
`rey.journal-entry.v2`, `rey.journal-log.v2`, and
`rey.journal-admission.v2`. The HTTP projection becomes
`rey.ui-journal.v2`. V1 documents have no dual reader or automatic migration.

Every proposal and retained entry contains one required layout:

```yaml
layout:
  kind: broadsheet
  columns: 12
  bands:
    - id: evidence
      cells:
        - block_id: query
          span: 5
        - block_id: frame
          span: 7
```

The broadsheet grammar is deliberately smaller than CSS:

- the grid has exactly 12 columns;
- a document has 1–32 ordered bands, each with a unique safe id;
- a band contains one or more ordered cells, each spanning 1–12 columns;
- occupied spans cannot exceed 12 columns; unoccupied columns remain honest
  whitespace;
- every typed block is placed exactly once, and band/cell traversal must equal
  block order;
- responsive projections may stack cells while preserving that reading order,
  exact block fragment identity, content, direction, and omissions.

Layout is a bounded evidence projection. It can improve comparison and
attention, but proximity, size, whitespace, or responsive stacking introduces
no relationship or assessment absent from the typed blocks.

`/journal/new` and `/journal/{slug}` use one live broadsheet document surface.
Title, author label, prose, query, diff, action, and cell width are edited in
place. The draft is compared with either an empty base or one exact retained
entry and exposes inserted, modified, removed, and layout change counts. An
unchanged retained entry cannot be re-recorded.

Editing a retained entry submits a new human-authored proposal whose
`supersedes` is the exact entry identity. Successful admission appends the new
content-identified entry and navigates to its canonical route. It never
changes the earlier entry, its slug, sequence, fragments, or author label.
There is no retained browser draft or autosave contract in this slice.

The browser prose shorthand is only an authoring projection over typed prose
nodes. Blank-line-separated units map as follows: `# ` to heading, `- ` to
bullet, `> ` to quote, fenced triple-backtick text to code, and all other units
to paragraph. Retention stores typed nodes, not shorthand source.

Typed query and action cells remain inert. An action cell may describe mining,
building, verification, or another bounded desired delta, but Journal
admission grants no query, workload, assignment, mutation, or proof authority.
Incremental Journal ingestion into a future reasoning policy must preserve the
entry/block identity, author kind, binding, supersession, and this authority
boundary; it is not implemented by the layout engine.

## Consequences

- New and retained documents share one visual and interaction grammar.
- Rich juxtaposition is deterministic, responsive, serializable, and
  inspectable through `rey journal list` without accepting arbitrary layout
  code.
- Revision history is append-only and exact; “edit” means supersede.
- The working delta can guide an author without becoming a retained diff or
  runtime scheduler input before admission.
- Existing Journal v1 local state must be removed and required entries
  re-authored before the v2 runtime can load them.
- Reasoning-surface ingestion and executable opportunity admission remain
  incomplete follow-on work.
