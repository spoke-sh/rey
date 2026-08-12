# Plan 0028: Diff-Directed Journal Broadsheet

- Status: Active
- Decision: [ADR 0054](../docs/decisions/0054-diff-directed-journal-broadsheet.md)

## Outcome

Give human and agent-authored Journal entries one immutable, typed broadsheet
grammar and make `/journal/new` and `/journal/{slug}` the same live document
experience. A human can compose and revise rich layouts while seeing the
working delta against an exact base. The CLI remains the high-fidelity agent
inspection/admission surface, and no document cell gains execution authority.

## Milestones

- [x] Define the 12-column ordered band/cell grammar, bounds, responsive
  semantics, and v2 hard cut.
- [x] Validate layout completeness, uniqueness, span bounds, and reading order
  in the canonical Rust admission path.
- [x] Render retained entries through the broadsheet engine while preserving
  exact entry and block links.
- [x] Replace the separate new-entry form and retained viewer with one live
  editing surface.
- [x] Record retained edits as immutable human-authored supersessions and
  navigate to the new exact route.
- [x] Link the exact predecessor and bounded direct supersession branches on
  retained routes.
- [x] Expose live inserted, modified, removed, and layout delta counts.
- [x] Support live prose, read-only query, directed diff, and proposed-action
  cells plus deterministic 1–12-column composition.
- [x] Render broadsheet/revision structure through `rey journal add|list`.
- [x] Add Rust validator, UI grammar, route-parity, CLI stdout, HTTP, structured
  output, and embedded build proof.
- [ ] Derive exact authored opportunities from unsuperseded action blocks into
  a separate reasoning-surface projection with explicit source and authority.
- [ ] Define workload/policy admission from an authored opportunity without
  letting Journal retention execute or qualify it.
- [ ] Add retained browser draft recovery only after its identity, ownership,
  bounds, and loss semantics are decided.

## Acceptance

- [x] A malformed, overflowing, incomplete, duplicated, or reordered layout is
  rejected before content identity or retention.
- [x] New and retained routes carry the same `data-journal-surface=broadsheet`
  surface and controls.
- [x] Recording an edit sets `supersedes` and appends rather than overwrites.
- [x] Responsive layout preserves semantic reading order.
- [x] Query and action cells remain visibly inert.
- [x] `rey journal list --format table` shows band, cell kind, width, revision,
  route, and exact identities; JSON retains the full v2 grammar.
- [x] Focused Rust and browser suites pass and the embedded UI builds.

This plan remains active because incremental reasoning-surface opportunity
derivation and its separate workload admission handshake are intentionally not
implemented by the layout slice.
