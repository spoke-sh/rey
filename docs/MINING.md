# Mining Context Into Evidence

This document defines Rey's target mining model. [ADR 0017](decisions/0017-mining-capability-model.md)
accepts mining as the capability layer that joins environment surfaces to
workload graphs, deltas, frontiers, and reasoning surfaces. Plan 0006 owns the
first executable slice. Generic mining requests, text deltas, structural
indexes, and visual specifications are not implemented yet.

## Purpose

Mining is the bounded process of turning context into navigable, addressable
evidence. It answers four questions before policy proposes work:

1. What exact sources and operations are available?
2. What bounded structure can be extracted from them?
3. What changed or remains unresolved?
4. Which representation makes that evidence useful without overstating it?

Mining is how Rey makes high-dimensional environments tractable. It does not
mean scraping everything, building a second durable index beside Spoke, or
giving an agent arbitrary query or execution authority. Every mining operation
has explicit sources, semantics, limits, completeness, and lineage.

## Capability Families

### Relational Mining

Relational mining operates on typed collections such as tables, events,
measurements, diagnostics, symbols, references, dependency edges, tests, and
claims. Its operation vocabulary can include:

```text
retrieve · select · filter · join · group · aggregate
align · order · traverse · compare · summarize · visualize
```

Polars DataFrames are Rey's canonical bounded in-process representation for
these collections, and Arrow is the preferred typed interchange family.
Providers still own source query semantics: Spoke owns Spoke query and durable
tables; a database provider owns its snapshot and query contract; Rey owns the
versioned mining request, bounded projection, delta use, and lineage that joins
the result to a workload.

Relational operations bind logical schemas, key and ordering rules, operation
revision, parameters, input identities, provider checkpoints, and effective
row/column/cell/byte/time limits. Grouping and aggregation retain enough
lineage to identify their contributing scope. A summary or visualization is
not allowed to become the only authoritative copy of typed values.

### Source Mining

Source mining operates on ordered text, code, configuration, logs, documents,
and native artifacts. Its capability ladder includes:

```text
locate and retrieve
  -> search and segment
  -> tokenize and parse
  -> index symbols and relationships
  -> traverse syntax and semantic graphs
  -> derive metrics and grouped views
  -> compare and visualize
```

The ladder is cumulative. A syntax node links to its exact parser revision and
source span. A symbol or reference links to the syntax/source evidence from
which it was derived. A metric links to the contributing relation and declared
formula. Unsupported language features, parse recovery, ambiguous resolution,
partial traversal, generated code, binary input, and invalid encoding remain
visible completeness facts.

`rg` awareness is a useful low-rung capability: an admitted adapter can return
bounded exact match records and context spans. It does not imply AST support,
semantic resolution, or permission to run arbitrary commands. Language
parsers, compiler services, and semantic indexes are richer providers that
must advertise their own contracts and limitations.

## Common Mining Lifecycle

```text
inventory capability
  -> bind exact source or snapshot
  -> admit operation and effective limits
  -> retrieve or probe
  -> extract/project/organize
  -> compare and assess completeness
  -> retain artifact references and lineage
  -> render bounded machine and human projections
```

The lifecycle distinguishes three execution cases:

- **exact retrieval** reads already identified immutable evidence through the
  provider that owns it and can occur during bounded orientation;
- **pure projection** transforms frozen evidence deterministically in process
  or as an admitted graph node; and
- **probe mining** observes mutable state or invokes a tool and therefore
  passes ordinary proposal, admission, execution, observation, and budget
  boundaries.

Mutation is not mining. A mining result may justify a later mutation proposal,
but no read, parse, index, metric, diff, or visualization grants that effect.

## Operation Contract

A versioned mining operation contract needs at least:

| Field | Meaning |
| --- | --- |
| identity | Stable operation id, revision, implementation digest, and semantic version |
| family | Relational or source mining |
| kind | Retrieve, search, transform, parse, index, traverse, measure, compare, or visualize |
| input contract | Accepted native artifact, relation, tree, graph, or prior mining-result types |
| output contract | Produced artifact kinds, schemas, media types, and identity rules |
| source requirements | Provider operations, snapshot guarantees, encodings, languages, and trust |
| determinism | Pure/frozen semantics or explicitly variable/tool-observed semantics |
| effects | Read-only retrieval, pure projection, or probe; never an implicit mutation |
| parameters | Typed, canonical, bounded arguments and defaults |
| limits | Rows, bytes, matches, files, depth, nodes, edges, time, memory, and output bounds |
| completeness | Conditions for complete, partial, truncated, unsupported, unavailable, or failed results |
| invalidation | Source, provider, parser, operation, parameter, and limit changes that make evidence stale |

Operation discovery and operation admission are distinct. The capability
snapshot freezes provider identity, path or endpoint, version,
digest/provenance, trust, supported operation revision, enforceable limits, and
availability before a graph can select it.

## Request And Result

A mining request binds:

- workload, graph, scenario/campaign, space, and active transition when
  applicable;
- exact source bindings or exact input mining artifacts;
- the selected operation contract and canonical parameters;
- capability snapshot and provider selection;
- requested and effective limits;
- expected output kinds, schemas, keys, and completeness; and
- the frontier/delta/claim that justified the work.

A mining result manifest binds:

- request and result identities;
- realized provider, tool, parser, query, run, or capture lineage;
- exact source identities and any post-read drift check;
- native artifact, frame, tree, graph, metric, delta, and visualization
  references produced;
- schemas, media types, logical lengths, keys, and ordering where applicable;
- completeness, omissions, unsupported semantics, warnings, and errors;
- effective resource consumption and limits; and
- dependency edges needed for invalidation and staleness.

The manifest is an evidence index, not a content store. Native source remains
owned by its source provider. Working frames are bounded state. Retained
artifacts use the selected local or Spoke-backed evidence boundary and claim
only its actual guarantees.

## Artifact Shapes

Mining may produce several peer evidence shapes:

- **native artifacts** — exact or content-addressed bytes, ordered text,
  documents, captures, patches, or provider resource references;
- **relations** — bounded typed frames for matches, symbols, references,
  diagnostics, metrics, nodes, edges, or grouped results;
- **trees and graphs** — native structured artifacts plus typed node/edge/span
  relations when tabular navigation is useful;
- **deltas** — authoritative relational, text, tree, graph, or claim-specific
  comparisons with explicit direction; and
- **visual projections** — tables, patches, trees, graphs, timelines, metric
  panels, or summaries linked to authoritative evidence.

An artifact is not forced into a DataFrame when doing so loses native meaning.
Conversely, typed relational values are not stringified merely to reuse a text
patch. Cross-artifact references use exact credential-free evidence addresses.

## Diff Families

Mining results participate in comparison according to their declared shape:

1. **Relational delta** aligns typed keyed or explicitly ordered relations and
   preserves schema, row, and cell changes.
2. **Text delta** compares ordered text under exact encoding, segmentation,
   normalization, context, and byte/line limits while retaining source
   identities and native content addresses.
3. **Structural delta** aligns declared tree or graph entities under versioned
   identity rules and preserves insertions, deletions, moves, modifications,
   and unresolved alignment.
4. **Claim fact** records a typed predicate result when forcing the evidence
   into one of those comparisons would be dishonest.

Every family uses explicit `SOURCE` to `TARGET` direction. Workload scenarios
normally label that direction `EXPECTED` to `OBSERVED`. A human patch or graph
view is a projection of its authoritative delta, not a substitute for it.

## Visualization Contract

Visualization exists to improve orientation for humans and policies while
remaining evidence-honest. A visualization projection declares:

- source artifact and delta identities;
- projection contract and implementation revision;
- selected fields, grouping, ordering, layout, context, and elision;
- requested and effective display bounds;
- omissions, aggregation, sampling, and truncation;
- semantic labels and non-color encodings; and
- deep links back to exact evidence.

Tables are appropriate for repeated fields; patches for ordered local change;
trees for nesting; graphs for dependencies; timelines for transitions; and
metric panels for several distinct measurements. The choice is semantic, not
cosmetic. A visualization never changes comparison assessment, proof status,
coverage, confidence, or progress.

Machine projections expose stable typed documents or relations. Terminal
renderings may add ANSI styling only when interactive, and meaning remains
legible without it.

## Workload And Runtime Placement

Workloads declare which mining operations a graph may compose, the context
surfaces they may read, and the scenarios that qualify their behavior. A graph
node cites an exact operation contract; generated shell, query, regex, parser
configuration, or source text does not become executable merely because a
policy proposed it.

Within the runtime:

```text
frontier work
  -> schedule
  -> mine exact evidence
  -> project reasoning surface
  -> policy proposes graph revision or action
  -> runtime admits and executes
  -> mine post-action observations
  -> compute transition and residual deltas
  -> derive next frontier and proof facts
```

Mining during orientation is delta-directed: it begins with selected frontier
citations and expands only through declared dependencies and remaining bounds.
It does not sweep the ambient workspace to create a generic prompt. The
reasoning surface cites mining-result artifacts and omissions rather than
copying an unbounded repository into policy input.

## Provider And Spoke Boundary

Rey composes mining but does not seize provider ownership:

- filesystem and Git providers own local source identity and safe reads;
- tool adapters own executable invocation, parsing, capture, and limitation
  semantics for tools such as `rg`;
- language adapters own parser, syntax, semantic, and index interpretation;
- `rey-dataframe` owns bounded local relational representation and Arrow
  interchange;
- `rey-diff` owns authoritative comparison contracts and projections;
- Spoke owns durable files, objects, documents, streams, tables, composed
  query, registered tools, runs, captures, and durable lineage; and
- the Rey runtime owns workload admission, delta/frontier rationale, limits,
  mining composition, invalidation, and policy-surface projection.

An adapter may use a Spoke query rather than duplicate it locally. A local
fallback exposes narrower guarantees and never mints Spoke revisions or claims
Spoke query semantics.

## Current Truth And First Slice

Current Rey discovers the allowlisted `rg` and `git` executables by bounded
identity probes, observes part of one Git repository, operates on typed
capability frames, and implements narrow UTF-8 scenario deltas. It does not yet
execute `rg` as a mining provider, expose a generic mining request/result,
compare arbitrary source artifacts, parse ASTs/CSTs, build a semantic index,
or render general tree/graph visualizations.

Plan 0006 starts with one end-to-end standalone slice:

1. freeze common operation, request, result, artifact, limit, completeness, and
   lineage contracts;
2. adapt bounded read-only source search and exact context retrieval;
3. project matches as a typed relation while retaining source text natively;
4. compute one relational delta and one ordered text delta;
5. render bounded table and patch projections with exact deep links;
6. exercise the same contracts through a scenario-qualified workload and a
   delta-directed reasoning-surface fixture; and
7. prove deterministic behavior, provider drift, failure, truncation, and
   zero-Spoke operation.

AST/CST adapters, semantic resolution, broad code-quality metrics, durable
indexes, generic graph visualization, learned ranking, and recurring
scheduling follow only after that slice proves the common invariants.

## Required Fixtures

Mining implementation work needs fixtures for:

- exact source identity and source drift during retrieval;
- empty, single, multiple, overlapping, and Unicode text matches;
- binary or invalidly encoded content under explicit policies;
- long lines, deep paths, symlink escapes, ignored/generated files, and bounded
  file/match/context overflow;
- missing, changed, timed-out, malformed, and non-zero tool providers;
- deterministic built-in/tool parity where both claim the same semantics;
- typed empty match relations and unique match identity;
- insertion, deletion, modification, reorder, parse recovery, unsupported
  structure, and incomplete traversal;
- aggregation/grouping provenance and limit behavior;
- text, relation, tree, graph, and visualization direction without reliance on
  color;
- complete, partial, truncated, unsupported, unavailable, failed, and stale
  results; and
- identical semantic artifacts with zero Spoke and equivalent Spoke-backed
  source bindings where that provider is available.

## Non-Goals

The mining model does not select a universal query language, parser framework,
language server protocol, index database, metric catalog, visualization
library, persistence engine, or new top-level CLI group. Those choices require
an end-to-end workload need, bounded fixtures, and an ownership decision.
