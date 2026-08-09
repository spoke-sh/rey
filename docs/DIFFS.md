# Relational, Text, And Structural Diffs

This document defines Rey's target semantic contracts for frames, typed deltas,
native deltas, and diff renderings. Physical schemas and serialized formats
remain provisional until accepted by an ADR and proved by fixtures. ADR 0017
places relational, text, and structural comparison inside the common mining
model. Capability deltas, ordered UTF-8 line deltas, scenario-output deltas,
and the typed source-match relation delta are implemented; generic frame and
structural delta families remain target contracts.

## Direction

Every comparison is directed from `SOURCE` to `TARGET`. Callers may supply more
specific labels such as `BASELINE` and `CANDIDATE` or `EXPECTED` and `OBSERVED`.

- A deletion exists in `SOURCE` and not in `TARGET`.
- An insertion exists in `TARGET` and not in `SOURCE`.
- A modification has the same aligned entity in both frames and at least one
  unequal comparable value.

Direction and labels are semantic metadata. They must remain visible in
structured artifacts and cannot depend on color or command prose.

## Frame Contract

A frame is a bounded typed relation with enough metadata to reproduce or audit
its observation:

| Field | Meaning |
| --- | --- |
| identity | Content-derived frame id and semantic format version |
| space | Owning space id and revision |
| lens | Lens id, revision, and implementation digest |
| schema | Ordered column names, logical types, nullability, and schema revision |
| keys | Unique comparison key columns and ordering semantics |
| capabilities | Frozen provider/tool snapshot and guarantees used to create the frame |
| sources | Exact Spoke bindings or strongest immutable local identities available |
| mining | Operation/request/result identities and derivation dependencies when the frame is mined |
| normalizers | Ordered versioned transformations applied before comparison |
| limits | Requested and effective evaluation bounds |
| completeness | Complete, truncated, partial, unavailable, or failed observation state |
| data | Bounded Polars DataFrame, normally interchanged as Arrow |
| lineage | Relevant queries, checkpoints, runs, attempts, captures, and request ids |

Wall-clock observation time may be useful lineage, but it is not part of
semantic identity unless a lens explicitly declares time as an input.

An empty frame retains its complete declared schema and keys. It is not an
untyped absence.

## Compatibility

Two frames are directly comparable only when:

- their relation semantics are compatible;
- their schema versions can be aligned under an explicit rule;
- required key columns exist with compatible types;
- keys are unique within each unordered relation;
- ordering expectations agree when row or column order is meaningful;
- normalizers and equality rules are known; and
- both completeness states permit the requested claim.

Schema insertion, deletion, and an explicitly declared rename can be valid
changes. An inferred rename is unsafe because two unrelated columns may happen
to contain similar values.

Incompatible types, duplicate keys, missing keys, unknown completeness, or
unapproved schema evolution return an explicit incompatible or inconclusive
assessment. They never fall back to positional guessing.

## Keys And Ordering

Relational rows are unordered by default and require one or more unique key
columns. Composite keys compare their typed tuple values. Null keys are rejected
unless a relation contract explicitly defines stable null-key identity.

Ordered comparison is opt-in for relations where position carries meaning. The
frame records the order expression or stable ordinal source. Reordering must not
be inferred from nondeterministic collection order.

Column order is part of schema identity but not necessarily equality. A lens
declares whether a column move is semantically meaningful.

## Normalization And Equality

Normalization exists to remove declared representational variance, not to hide
behavioral differences. Each normalizer has a stable id, revision,
implementation digest, input/output type contract, and parameters.

Examples include canonical path separators, stable timestamp precision, or
sorting a set-valued list. Removing ownership, status, causality, security, or
error information merely to obtain a passing diff is invalid.

Equality is type-aware. Initial semantics should cover nulls, booleans,
integers, floating-point policy, strings, binary values, temporal values,
lists, structs, and categoricals explicitly. Approximate numeric or vector
comparison is a distinct declared predicate with tolerances; it is not default
cell equality.

## Typed Delta

The authoritative delta retains:

- format version and content identity;
- source and target frame ids, labels, schemas, and source bindings;
- key and ordering definitions;
- normalizer and equality definitions;
- schema changes;
- inserted and deleted keyed rows;
- modified rows with typed before/after cells;
- reordering where meaningful;
- unchanged/context counts without requiring all unchanged rows to be stored;
- requested and effective limits;
- completeness and assessment status; and
- deterministic summaries intended for navigation.

A delta assessment is `equal`, `different`, `incompatible`, or `inconclusive`.
Runtime or transport errors are recorded separately and may cause an
inconclusive observation. Similarity is a diagnostic summary and never changes
the assessment.

The exact Arrow representation for heterogeneous before/after values remains an
open design item. Until selected, implementations must not stringify values and
then claim typed round-trip behavior.

## Mining Comparison Families

Portfolio mining consumes comparison evidence rather than inventing another
untyped task queue. `rey.workload-attention.v1` derives actions and reasons
from exact qualification/result state, changed dependencies, missing
capabilities, and ownership coverage. `REFINE`, `RETEST`, `CREATE`, `BLOCK`,
and `POLICY_EXCLUDED` remain typed facts with citations. A later scheduler may
select ready rows, but cannot erase the source deltas or reinterpret a blocked
row as equal or converged.

The common delta invariants apply to several evidence shapes. Rey chooses a
declared comparison family rather than coercing every input into a table or
string.

### Relational Delta

A relational delta is the typed delta described above: compatible frames align
under exact schema, key, ordering, normalizer, and equality contracts. It is
authoritative for collections such as search matches, syntax nodes, symbols,
references, dependency edges, diagnostics, metrics, and grouped observations.

### Text Delta

A text delta compares ordered text while retaining:

- exact source and target artifact identities and labels;
- encoding, newline, segmentation, and normalization rules;
- ordered hunks with source/target byte and line spans;
- inserted, deleted, changed, and unchanged-context evidence;
- context/elision policy and requested/effective line, hunk, and byte limits;
- binary, invalid-encoding, oversized-line, and unavailable-input behavior;
- completeness and assessment; and
- deterministic native patch and structured projections.

A line patch is a useful human projection, but the authoritative result cannot
depend only on terminal text. It retains spans and source addresses needed to
connect a hunk with match, syntax, symbol, metric, workload, and proof evidence.

### Structural Delta

A structural delta compares declared trees or graphs such as configuration
paths, CSTs, ASTs, symbol graphs, reference graphs, or dependencies. Its
contract fixes entity identity, parent/edge semantics, ordering, alignment,
move classification, parser/index revisions, and completeness. Insertions,
deletions, modifications, moves, edge changes, and unresolved alignment remain
distinct.

Similarity does not authorize guessed alignment. Parse recovery, unsupported
language features, ambiguous symbol resolution, bounded traversal, or
incomplete graph closure can make the comparison partial or inconclusive.

### Claim Facts

Evidence that does not reduce honestly to a relational, text, or structural
comparison remains a typed claim fact. The runtime may place all four families
in one frontier, but it does not flatten them into an artificial mega-delta.

### Implemented Capability Specialization

ADR 0010 fixes a narrow `rey.capability-delta.v1` specialization rather than
claiming the generic representation above is solved. It compares verified
`rey.capabilities.v1` snapshots by the composite key `(provider_id,
provider_revision, capability_id)`. Exact typed equality covers every snapshot
identity field; `observed_at` and `error_detail` are deliberately excluded.

Structured JSON retains typed semantic records on both sides. The wide
`rey.capability-changes.v1` Arrow relation retains a typed `UInt64` provider
revision, nullable before/after columns, changed-field names, and lineage in
frame attributes. Change ordering and field ordering are deterministic. Empty
deltas retain their schema and lineage. A bounded summary and Tabular Diff 0.8
CSV are non-authoritative projections of that delta.

### Implemented Source-Mining Specializations

ADR 0018 adds two bounded authoritative forms:

- `rey.text-delta.v1` binds source/target UTF-8 artifact identities and labels,
  comparator, line segmentation, final-newline state, input/line/alignment/
  change/string limits, and deterministic LCS-aligned context/delete/insert
  rows grouped in an ordered hunk. Replay requires the exact source and target
  text. `rey.scenario-output-delta.v2` embeds this result while retaining its
  expected/observed strings for workload evaluation.
- `rey.source-match-delta.v1` aligns expected and observed rows by reversible
  path encoding/identity plus start/end byte span. It preserves typed
  insertions, deletions, modifications and changed fields, exact source/match/
  context ids, reviewed expectations, mining completeness, relation counts,
  and explicit limits. Any incomplete mining result makes the comparison
  inconclusive even when retained rows happen to agree.

The workload terminal renderer projects the text delta as an ANSI-independent
line patch and the relation as counts plus typed changed rows, matches, native
context, omissions, and deep bindings. These projections do not replace the
structured artifacts or participate in their identities. General text context
elision, token deltas, arbitrary frame comparison, and structural alignment
remain later contracts.

## Tabular Diff 0.8 Projection

For compatible tables Rey projects a typed delta into the
[Frictionless Data Tabular Diff Format 0.8](https://specs.frictionlessdata.io/tabular-diff/).
The projection uses the standard action and schema markers:

| Marker | Meaning |
| --- | --- |
| `@@` | Header row containing column names |
| `!` | Schema row describing column changes |
| `+++` | Row or column inserted in `TARGET` |
| `---` | Row or column present in `SOURCE` and deleted from `TARGET` |
| `->`, `-->`, ... | Modified row and collision-safe before/after separator |
| blank | Unchanged context |
| `...` | Omitted unchanged context |
| `:` | Reordered row or column when ordering matters |

Tabular Diff converts modified cells to text and has conventions for preserving
null versus the string `NULL`. Rey follows those conventions in the portable
projection while retaining original typed values in its structured delta.

The terminal table and CSV artifact are renderings of the same Tabular Diff
relation. ANSI color may enhance a terminal but never appears in stored
evidence or carries unique meaning.

## Other Evidence Shapes

DataFrames are central to typed collections, but one artificial mega-table is
not the goal:

- ordered source text and logs may use a line-oriented patch;
- nested configuration may use a typed path/value comparison;
- binary content may compare identity and metadata while retaining raw
  artifacts separately;
- vector observations may use explicit distance and tolerance relations; and
- a transition spanning several frames retains a set of individual deltas with
  an aggregate index rather than flattening unrelated schemas.

Every non-tabular delta still has direction, exact source identities, bounded
evidence, and a structured summary that can participate in a frontier frame.
Visualization contracts from [Mining Context Into Evidence](MINING.md) may
project these results as patches, trees, graphs, timelines, or metric panels,
but layout, grouping, sampling, or color never alters their assessment.

Workload scenarios use the same contract with `EXPECTED` as source and
`OBSERVED` as target. A conclusive non-empty scenario delta is retained as the
failure that directs the next graph revision; an incompatible or incomplete
comparison is inconclusive. A passing empty delta remains scoped to the exact
scenario fixtures, output selection, comparator, completeness, and graph
revision. See [Workloads, Compute Graphs, and Scenarios](WORKLOADS.md).

Git comparisons retain typed repository semantics in addition to optional text
patches. Commit/ref deltas preserve OIDs, parent/reachability facts, path modes,
and movement classification. Index deltas compare logical entries and stages so
a stat-cache-only raw index rewrite does not appear as staged-content change.
See [Git Context and Activation](GIT.md).

## Invalidation

A delta identifies changed source entities and fields. Capability-snapshot
deltas also identify provider, tool, version, trust, or guarantee drift. Git
deltas identify ref movement, commit reachability, semantic index, and declared
worktree changes. Lens, trigger, and action dependency metadata maps those
changes onto potentially affected observations. Invalidation is conservative:
false-positive re-evaluation is acceptable, but suppressing a semantically
affected lens is incorrect.

Initial implementations may recompute a full bounded lens. Later incremental
execution must prove parity against full recomputation for insertions,
deletions, modifications, schema changes, and dependency changes.

## Required Proof Fixtures

The first diff engine must include fixtures for:

- equal frames;
- inserted and deleted rows;
- modified typed cells;
- inserted, deleted, and declared-renamed columns;
- typed empty source and target frames;
- compound keys and duplicate-key rejection;
- nulls versus empty strings and literal `NULL` strings;
- deterministic object/list normalization;
- meaningful and meaningless ordering;
- incompatible schemas and types;
- truncated or failed observations;
- zero-Spoke and Spoke-connected frames with explicit capability metadata;
- provider/tool appearance, disappearance, and version or digest drift;
- Git fast-forward, rewind, rewrite, merge, incomplete-history, and semantic
  index deltas;
- bounded context omission;
- byte-for-byte deterministic structured and Tabular Diff artifacts;
- ordered text insertion, deletion, replacement, context elision, encoding,
  long-line, binary, and byte/hunk-limit behavior;
- tree insertion, deletion, modification, move, ambiguous alignment, parse
  recovery, and incomplete traversal;
- exact deep links between relational rows, text spans, structural entities,
  and their mined source artifacts; and
- deterministic ANSI-free patch, tree, and graph projections whose omissions
  remain explicit.
