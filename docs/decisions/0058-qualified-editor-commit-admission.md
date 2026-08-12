# ADR 0058: Qualified Editor Commit Admission

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0046](0046-read-first-scene-editor.md) and [ADR
  0056](0056-continuous-globe-mercator-county-grammar.md)

## Context

ADR 0046 deliberately stopped the first editor slice at an immutable
candidate package and an unadmitted request. That preserved the candidate and
Explorer authority boundary, but left a completed `rey editor commit` unable
to appear in `/explore`. It also meant that commit's local checks were not a
qualified workload result: the editor could say that INDEX was internally
valid without proving that the exact frozen objects still derived that INDEX
at admission time.

Requiring a second manual command would not add authority by itself. The
durable requirement is that a fixed workload, rather than the candidate
author, qualifies the validation semantics and evaluates the exact package
before any projection becomes an Explorer input.

## Decision

`rey editor commit` is one operator command with two distinct authority
phases:

```text
INDEX + frozen native objects
             │
             │ construct candidate package and request in memory
             ▼
rey.scene-admission workload
  1. qualify the fixed validator against nine required scenarios
  2. reparse every frozen object
  3. rederive the snapshot and projected native geometry
  4. run against exact package, snapshot, object, CRS, parent, limit,
     completeness, and projection bindings
             │
             ├─ rejected → no admission and no SCENE@n advance
             ▼ admitted
SCENE@n + candidate package + separate immutable admission
             │
             ▼
rey.workload-list.v1 scene_admissions → /explore
```

The built-in `rey.scene-admission` workload is a public conformance workload.
Its graph contains `rey.scene-admission.validate@1`; its required scenarios
cover exact acceptance and rejection for coordinate, completeness, limit,
native-object, package, parent, projection, and snapshot mismatches. Every
commit obtains a fresh passing qualification for that exact workload revision
before running the real admission input. The retained `rey.scene-validation.v1`
records workload, graph, scenario-suite, evaluator, test-result,
qualification, and run identities.

The package remains `candidate_only`, and its original
`rey.scene-admission-request.v1` remains an unadmitted handoff. Authority lives
in a separate content-identified `rey.scene-admission.v1` under
`.rey/editor/admissions/`. An admission binds the request and package to one
`rey.scene-projection.v1`; it does not rewrite the package or turn editor hints
into observed facts.

Validation reparses the retained content-addressed GeoJSON rather than mutable
WORKING files. It rebuilds source counts, feature indexes, marker metadata,
bounds, coordinate counts, selected human labels/descriptions, and projection
geometry, then compares the resulting snapshot identity with INDEX. Loading
admissions for the UI repeats that derivation and workload run and requires
the retained admission to match exactly, so edited admission JSON or frozen
objects fail closed before reaching the browser.

`rey.workload-list.v1` gains an optional `scene_admissions` collection. The
Explorer adapter accepts only complete admissions whose package, validation,
snapshot, and projection identities agree. The current reference renderer
fits admitted CRS84 geometry into its planar scene, preserving polygons,
declared authored lines, POIs, labels, descriptions, roles, and exact source
revisions. This is a native scene projection, not a claim that geographic
coordinates are semantic atlas coordinates.

For the pre-existing incomplete slice, invoking `editor commit` with an empty
INDEX and an unadmitted HEAD validates and admits that exact historical
package without creating a second scene commit. Already-admitted clean HEAD
continues to report an empty index.

## Consequences

- A scene cannot advance or enter `/explore` unless the independently defined
  validator workload is freshly qualified and its exact run says `admitted`.
- The agent-authored package remains a candidate; workload evidence, not the
  package or renderer, grants admission.
- Validation is deliberately repeated on reads. The local cost is bounded by
  the editor's existing source, feature, coordinate, property, and admission
  byte limits; a mismatch makes the portfolio unavailable rather than serving
  stale geometry.
- The built-in conformance catalog grows by one workload and nine required
  scenarios.
- This closes the first vector/marker admission loop. Multiresolution raster
  terrain, native-to-semantic county transforms, validity masks, constructed
  path contracts, and richer material/LOD packets remain Plan 0029 work.
