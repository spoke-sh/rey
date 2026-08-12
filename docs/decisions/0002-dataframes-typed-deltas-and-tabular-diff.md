# ADR 0002: DataFrames, Typed Deltas, And Tabular Diff

- Status: Accepted
- Date: 2026-08-07

## Context

Rey needs one local representation for typed observations across codebase
inventories, symbol graphs, diagnostics, tests, runtime facts, and proof checks.
The representation must join and filter efficiently while preserving types and
source lineage.

The Frictionless Data Tabular Diff Format expresses table-to-table change as a
table and provides a portable, color-independent human format. It converts
modified cells to text, however, and therefore cannot be Rey's only semantic
representation.

## Decision

Polars DataFrames are the canonical bounded in-process representation for typed
collections, frames, frontiers, and query results. Apache Arrow is the preferred
typed columnar interchange and sequential transport family.

Each frame retains schema, keys, source bindings, lens revision, normalizers,
limits, completeness, and content identity in addition to its DataFrame.
DataFrames remain derived working state and do not replace source bytes or
durable resource identity.

Rey defines an authoritative typed delta between compatible frames. It retains
directed source/target identity, schema changes, keyed row changes, typed
before/after values, comparison rules, completeness, limits, and summaries.

For compatible tabular frames Rey projects the typed delta into Frictionless
Data Tabular Diff Format 0.8. CSV and terminal tables are renderings of that
projection. Color is optional presentation and carries no unique semantics.

Native ordered text, nested values, vectors, and binary artifacts may use
appropriate bounded comparison representations. Rey does not force all content
into one synthetic table, although structured summaries participate in frames
and frontiers.

## Consequences

- Frame and delta code pays the Rust compile-time and memory cost of Polars and
  Arrow; feature selection must remain deliberate.
- Empty frames retain their declared schemas and keys.
- Unordered relational comparison requires unique explicit keys.
- Tabular Diff string conversion cannot be used to claim typed round-trip
  behavior.
- Terminal, Arrow, JSON, and Tabular Diff forms must derive from the same
  semantic observation rather than independent implementations.
- Exact Arrow layouts, media types, and serialization schemas require a narrow
  follow-up decision before durable artifacts stabilize.

## References

- [Frictionless Data Tabular Diff Format 0.8](https://specs.frictionlessdata.io/tabular-diff/)
