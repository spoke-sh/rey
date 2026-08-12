# ADR 0018: First Executable Mining Workload

- Status: Accepted
- Date: 2026-08-08
- Implements: [ADR 0017](0017-mining-capability-model.md)
- Extends: [ADR 0016](0016-first-workload-slice.md)

## Context

ADR 0017 accepted provider-neutral relational and source mining and Plan 0006
implemented the common manifests plus one exact local literal-search provider.
Those contracts were useful library groundwork, but a user could not verify
them through Rey's workload-centered product surface. Leaving source binding,
matches, completeness, and lineage visible only to Rust tests would violate the
project invariant that a completed feature needs a high-fidelity human CLI
path.

The first executable slice needs to prove the entire handoff without choosing
regex semantics, an external `rg` process, parser/index breadth, an agent
provider, or recurring scheduling.

## Decision

### Built-In Workload And Graph

Add `rey.fixture.source-search` as the third compiled conformance workload. Its
finite typed graph contains two nodes in dependency order:

```text
UTF-8 pattern
  -> rey.source-search.literal-utf8@1
  -> rey.builtin.source-matches.render-lines@1
  -> UTF-8 canonical match lines
```

The search node receives a separately admitted source-run input binding a
canonical root, explicit relative files, context window, source-binding limits,
mining limits, and capability snapshot. Test mode selects the checked-in source
corpus. Run mode requires one or more repeatable `--source` paths beneath the
selected workspace. Both modes execute the same graph and operation contracts.

### Scenario Semantics

The suite contains four reviewed scenarios:

- required `empty` proves a complete typed empty relation;
- required `exact` proves two exact matches and native contexts;
- optional `mismatch` deliberately changes one reviewed expected match and
  proves complete relational/text failure evidence; and
- optional `truncated` lowers the match/row limit to one and proves an explicit
  inconclusive relation with a `match_limit` omission.

Only required scenarios enter qualification. Optional failure and truncation
remain visible retained evidence and never broaden the qualification claim.

### Directed Deltas And Views

Add `rey.text-delta.v1` for bounded deterministic ordered UTF-8 line alignment
and `rey.source-match-delta.v1` for typed expected-to-observed match alignment
by reversible path identity plus byte span. The text delta preserves direction,
line coordinates, final-newline state, limits, and replay. The relation delta
preserves typed inserted/deleted/modified rows, changed fields, native source/
match/context identities, completeness, limits, and replay. Incomplete mining
always makes the relation assessment inconclusive.

The ANSI-independent human projections have exact contracts:

- `rey.text-delta.terminal-patch@1`; and
- `rey.source-matches.terminal-table@1`.

They expose authoritative delta ids and do not participate in comparison
assessment. Structured JSON retains the underlying verified artifacts.

### Delta-Directed Reasoning Fixture

For the complete failing mismatch only, a versioned workload-specific
derivation creates one ready `rey.frontier.v2` row citing the relation and text
deltas. The existing generic scheduler selects exactly one row under a
one-unit work/cost bound. One `rey.reasoning-surface.v3` then cites the mining
result, match relation, native matches/contexts, deltas, provider, limits, and
the admissible `rey.action.propose-graph-revision@1` action.

This fixture proves the transition between evidence and bounded policy input.
It does not execute the proposed revision or create a recurring loop.

### CLI Contract

All four workload commands expose the slice:

- `list` shows portfolio mining totals plus operation order, complete/
  incomplete result counts, relation-delta counts, and reasoning-surface
  counts;
- `test` declares graph-plus-probe mode and renders scenario results in
  declaration order; plain failures open diffs, `-v` adds matches/context/
  completeness/omissions/limits, and `-vv` adds exact projection and evidence
  bindings;
- `status` reopens all retained mining and reasoning evidence read-only; and
- `run` requires explicit sources and renders operation/provider, consumption,
  output size, exact bindings, matches, contexts, and omissions.

Machine output remains clean verified JSON, diagnostics remain on stderr, and
semantic exit behavior is unchanged.

### Pre-Alpha Schema Cut

The graph now has a `SourceMatches` value and operation signatures are part of
validation. Make a hard cut to v2 for workload, compute graph, scenario suite,
scenario-output delta, test result, qualification, run result, local workload
state, list, status, status batch, and test batch schemas. No compatibility
decoder silently reinterprets v1 local state under the new mining semantics.

## Consequences

- Mining is verifiable through the workload product rather than an internal
  API or a new top-level resource hierarchy.
- The first slice proves exact source binding, complete/empty/different/
  truncated evidence, typed and ordered comparison, human/machine projection,
  qualification, run parity, frontier selection, and reasoning projection with
  local-only execution.
- Exact platform path bytes and all effective source/mining limits participate
  in run-context identity; lossy display paths do not.
- The optional failing and inconclusive scenarios intentionally make portfolio
  evidence richer while preserving a narrowly scoped required qualification.
- `rg`, regex/case folding, glob/directory traversal, AST/CST parsing, semantic
  indexing, general visualization, agent proposals, actions, and recurring
  scheduling remain outside this decision.
