# ADR 0003: Spoke Integration And Scoped Proof Boundary

- Status: Accepted
- Date: 2026-08-07
- Superseded in part: [ADR 0005](0005-environment-awareness-and-optional-spoke.md)
  removes Spoke as a boot requirement while retaining this decision for the
  connected provider boundary.

## Context

Rey needs durable, exact source revisions; relational, graph, lexical, and
vector observations; bounded process execution; and retained lineage. Spoke
already owns those reasoning-plane concepts and exposes them through service
contracts.

Building a second Rey storage or compute plane would divide truth and weaken
lineage. Importing Spoke internals for a same-host shortcut would also make the
integration behave differently from a routed or managed deployment.

Rey additionally needs to describe when observed deltas establish a claim. A
passing diff by itself does not establish coverage, input currency, execution
controls, or broader production safety.

## Decision

Spoke is Rey's durable reasoning and compute plane:

- source content, resource revisions, documents, streams, tables, and query
  execution remain owned by Spoke;
- registered tools, compute runs, attempts, fencing, captures, and durable
  process lineage remain owned by Spoke;
- Rey uses public Spoke HTTP contracts for the reference integration and has no
  data-directory or in-process capability bypass; and
- read-only observation uses safe reads and `QUERY`, while mutation uses an
  explicit resource method or admitted compute action.

Rey owns spaces, lenses, frames, action rationale and admission policy,
deltas, frontiers, transition lineage, claims, and proof evaluation. It stores
durable Rey artifacts through ordinary Spoke resources without claiming a new
Spoke capability until that capability exists.

A proof is scoped computational evidence. Status is `pending`, `passed`,
`failed`, `inconclusive`, or `stale`. Proof identity covers exact source,
candidate, fixture, lens, schema, key, normalizer, comparator, tool, evaluator,
and limit inputs capable of changing the result. Verification recomputes that
identity and does not trust a stored status field.

The physical mapping of Rey artifacts to Spoke files, objects, streams, tables,
or documents and the atomic certificate-publication protocol remain Plan 0001
decisions.

## Consequences

- Rey development and integration tests require an explicit Spoke endpoint and
  capability discovery.
- Local and managed use exercise the same service ownership boundaries.
- A Spoke run can succeed while a Rey proof fails or remains inconclusive.
- Changed evaluator or normalizer code makes old certificates stale even when
  sources did not change.
- Missing, truncated, unsupported, or unavailable required evidence cannot be
  silently converted into a passing proof.
- Optimized future internal interfaces must preserve the same identity,
  authorization, revision, consistency, and failure semantics as the reference
  routed contract.
