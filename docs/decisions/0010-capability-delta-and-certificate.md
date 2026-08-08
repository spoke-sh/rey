# ADR 0010: Capability Delta And Required-Capability Certificate

- Status: Accepted
- Date: 2026-08-07

## Context

Rey's first executable slice can inspect the local environment into the typed
`rey.capabilities.v1` relation. The next slice must prove that a diff can
direct compute without prematurely freezing a universal dataframe delta
layout. It also needs one honest proof claim that works without Spoke and can
later be retained or evaluated by Spoke.

Tabular Diff 0.8 is useful interoperability output, but modified cells combine
two values into text and CSV null conventions cannot preserve every typed
distinction. It therefore cannot be Rey's semantic delta record.

## Decision

The second slice defines a capability-specific typed delta. Inputs are JSON
capability snapshots whose schema, canonical ordering, unique composite keys,
completeness, and semantic digest are verified by recomputation before use.
The fixed key is `(provider_id, provider_revision, capability_id)`.

The comparator excludes `observed_at` and `error_detail`, matching capability
snapshot identity. All other fields are compared with exact typed equality.
Changes are ordered by key; changed field names use schema order. Insertions,
deletions, and modifications retain typed before/after records in structured
JSON. The Arrow representation is a wide typed relation with nullable
before/after columns. Frame attributes retain source and target snapshot ids,
labels, comparator identity, and delta identity even when the relation is
empty.

Tabular Diff 0.8 is a deterministic CSV projection of the typed delta. It uses
the `@@`, `+++`, `---`, and collision-safe arrow action conventions, plus the
format's `NULL` escaping rules. A summary projection contains only assessment
and counts. Neither projection is authoritative input for replay or proof.

Delta identity binds the source and target snapshot identities, bounded
evaluation inputs, comparator contract identity, counts, and ordered changes.
An incomplete source or target yields an `inconclusive` assessment. A change
limit is fail-closed: Rey emits no partial authoritative delta.

The first claim is `rey.environment.required-capabilities.v1`. It asks whether
each named capability is available in the target snapshot and binds the
source snapshot, target snapshot, capability delta, normalized requirement
set, comparator contract, and evaluator contract. A certificate has one of
`passed`, `failed`, or `inconclusive`. Verification recomputes identities and
the claim; changed snapshots, claim parameters, comparator inputs, or
evaluator inputs make the prior certificate `stale`. Structurally invalid or
self-inconsistent certificates are errors, not stale evidence.

Contract digests identify reviewed semantic definitions and revisions. They
are not claims that Rey hashed its compiled machine code. A semantic change
must bump the corresponding contract revision and digest input.

## Consequences

- Rey gains its first complete inspect-to-delta-to-certificate loop without a
  Spoke dependency.
- Capability fixtures can establish typed, Arrow, Tabular Diff, and staleness
  parity before a generic dataframe comparator is designed.
- Observation-only fields cannot create false capability changes.
- Tabular Diff remains portable review output while typed JSON and Arrow keep
  semantic authority.
- Durable retention, generic proof bundles, Git relation deltas, activation,
  and Spoke evaluation remain later vertical slices.
