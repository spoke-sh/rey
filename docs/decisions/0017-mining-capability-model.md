# ADR 0017: Relational And Source Mining As First-Class Capabilities

- Status: Accepted
- Date: 2026-08-08
- Builds on: [ADR 0001](0001-diff-directed-runtime.md),
  [ADR 0002](0002-dataframes-typed-deltas-and-tabular-diff.md),
  [ADR 0005](0005-environment-awareness-and-optional-spoke.md),
  [ADR 0012](0012-delta-directed-orientation.md), and
  [ADR 0015](0015-workload-centered-product.md)

## Context

Rey already inventories environment capabilities, observes spaces through
lenses, represents typed collections as DataFrames, computes directed deltas,
selects a frontier, retrieves evidence, and projects a reasoning surface. Those
contracts describe the runtime loop, but they leave the evidence-acquisition
and organization layer implicit.

Programming work depends on two equally important forms of that layer.
Traditional data work retrieves, filters, joins, groups, aggregates, traverses,
compares, and visualizes typed relations. Source work searches and slices text,
parses syntax, walks CSTs and ASTs, indexes symbols and references, derives
metrics and dependency graphs, compares revisions, and visualizes structure.
Treating those as unrelated utilities would make workload operation contracts,
provider capabilities, lineage, limits, completeness, and reasoning surfaces
fragment by tool.

Treating every source artifact as a DataFrame would be equally harmful. Ordered
text, native syntax trees, patches, and binary artifacts have semantics that a
synthetic table can erase. Conversely, stringifying typed records to obtain a
text patch loses relational types, keys, and alignment.

The missing concept is mining: the bounded transformation of context into
navigable, addressable evidence. It needs a common control contract without
becoming a universal query engine, parser bundle, durable index, or second
storage plane beside Spoke.

## Decision

### Mining Plane

Mining is a first-class architectural plane between environment capability
discovery and workload/runtime use. It binds exact sources, a versioned
operation, canonical parameters, capability/provider identity, limits,
completeness, derived artifacts, dependencies, and lineage.

Mining does not own source truth or mutation authority. Exact immutable reads
remain provider-owned retrieval. Pure projection over frozen evidence is
deterministic computation. Reading mutable state or invoking an external tool,
parser, compiler service, language server, or index is a probe that passes
ordinary admission and execution boundaries.

### Two Primary Capability Families

Rey recognizes two peer families:

1. **Relational mining** operates on typed collections through retrieve,
   select, filter, join, group, aggregate, align, order, traverse, compare,
   summarize, and visualize operations.
2. **Source mining** operates on text, code, configuration, logs, documents,
   and native artifacts through locate, retrieve, search, segment, tokenize,
   parse, index, traverse, measure, compare, and visualize operations.

The families interoperate through exact projections. Matches, syntax nodes,
symbols, references, dependencies, diagnostics, and metrics may be DataFrames.
Their rows retain exact source spans and derivation identities. Native source,
text, trees, graphs, patches, and binary artifacts remain native when that
preserves meaning.

### Common Contracts

The first common model will version:

- a mining operation contract with family/kind, input/output artifact
  contracts, parameters, effects, capabilities, limits, completeness,
  determinism, invalidation, and implementation identity;
- a mining request binding workload/graph/scenario/transition and frontier
  rationale to exact inputs, operation, parameters, provider, and limits;
- a mining result manifest indexing realized provider/tool/parser/query
  lineage, produced artifacts, schemas/media types, completeness, omissions,
  resource consumption, and dependency/staleness edges; and
- artifact references for native, relation, tree, graph, metric, delta, and
  visualization results.

The manifest is an evidence index, not a content store. Retention uses the
selected local or Spoke-backed boundary and states only its actual guarantees.

### Diff And Visualization Families

Authoritative comparison is selected by evidence shape:

- relational deltas preserve typed schema, key, row, and cell semantics;
- text deltas preserve ordered content, encoding, segmentation, spans, and
  context/elision;
- structural deltas preserve declared tree/graph identity, parent/edge
  semantics, moves, modifications, and unresolved alignment; and
- typed claim facts remain available when evidence does not reduce honestly to
  one comparison.

Visualization is a first-class mining projection, not semantic authority. A
table, patch, tree, graph, timeline, metric panel, or summary binds its source
artifact/delta identities, projection revision, selection, grouping, ordering,
layout, aggregation, context, sampling, elision, limits, omissions, and deep
links. It cannot change assessment, coverage, confidence, progress, or proof
status, and color cannot carry unique meaning.

### Workload And Runtime Placement

Workloads declare the mining operations their compute graphs may compose and
the scenarios that qualify them. A graph node cites an exact admitted operation
contract. Policy-proposed shell, query, regex, parser configuration, source, or
layout text is untrusted typed input, not executable authority.

After the scheduler selects unresolved frontier work, mining retrieves and
projects only the evidence justified by those citations and their declared
dependencies. The bounded result enters the reasoning surface. Post-action
mining produces observations from which transition and residual deltas derive
the next frontier and proof facts.

Mining remains an internal capability model under the workload-centered
product. This decision does not add a `rey mining` top-level CLI hierarchy.
Focused diagnostic projections may be added when an executable workload needs
them.

### Ownership

Concrete ownership remains distributed:

- environment/filesystem/Git providers own discovery, local source identity,
  and safe reads;
- tool and language adapters own invocation, parsing, semantic interpretation,
  and stated limitations;
- `rey-dataframe` owns bounded local typed relations and Arrow interchange;
- `rey-diff` owns relational, text, and structural comparison contracts and
  their projections;
- `rey-runtime` owns graph validation, admission, transitions, budgets, and
  mining composition;
- `rey-policy` owns the provider-neutral reasoning/proposal boundary; and
- Spoke owns durable files, objects, documents, streams, tables, composed
  query, registered tools, runs, captures, and durable lineage.

`rey-mining` is a provisional target crate for common operation, request,
result, artifact, completeness, dependency, and visualization contracts. Plan
0006 may create it only if that boundary remains narrow and avoids dependency
cycles. It will not be a query engine, parser collection, tool runner, index
database, or persistence service.

### First Implementation Bearing

The first slice must prove the shared invariants before adding broad code
intelligence. It will:

1. freeze the minimum common mining contracts;
2. adapt bounded read-only source search and exact context retrieval;
3. project match records as a typed relation while retaining native source;
4. compute one relational delta and one ordered text delta;
5. render evidence-linked table and patch projections;
6. exercise those operations through a scenario-qualified workload and a
   delta-directed reasoning-surface fixture; and
7. prove source/tool drift, truncation, unsupported input, deterministic
   replay, and zero-Spoke behavior.

AST/CST adapters, semantic resolution, code-quality metric catalogs, durable
indexes, general graph visualization, learned ranking, and recurring
scheduling remain later bearings.

## Consequences

- Rey has a coherent vocabulary for progressively richer environment
  understanding, from `rg` matches through syntax, semantic graphs, metrics,
  and visualization.
- Relational and source evidence can direct the same frontier without erasing
  their distinct native semantics.
- Workload graph generation gains discoverable, typed mining operations rather
  than ambient tools or free-form executable text.
- Reasoning surfaces can become smaller and more precise because mining is
  justified by selected deltas and dependency closure.
- Source, operation, parser/index, parameter, capability, completeness, and
  limit changes become explicit invalidation and proof inputs.
- Spoke remains the durable reasoning/query/compute plane rather than being
  duplicated by a local Rey index or storage service.
- The new common contract increases schema and fixture work before richer
  adapters can be claimed, intentionally trading early breadth for trustworthy
  composition.

## Not Decided

This decision does not select final schema names, manifest encoding, regex
engine, parser framework, language server protocol, semantic index, graph
library, metric definitions, visualization library, query language,
persistence engine, policy transport, model provider, or service topology.
