# Locators

Locators are Rey's exact anchor protocol for the survey phase. A locator names
an object or bounded region in context. It does not retrieve the object, grant
authority to read it, admit an executable, or prove that the object exists.

A locator is not itself an Explorer camera location or necessarily a resolved
Spoke coordinate. Resolution may bind a candidate locator to a typed,
provider-qualified coordinate, or retain a missing, stale, unsupported,
unauthorized, malformed, or truncated outcome. The coordinate identifies the
semantic object or region; camera center, scale, lens, and selection are
separate presentation state.

## Contract

A locator has:

- a scheme with one owning resolver;
- a canonical payload identifying a resource or region;
- optional unique canonical query dimensions such as exact revision or agent
  role;
- an explicit mutable or immutable identity class;
- a bounded display form and a lossless machine form; and
- resolution evidence binding the locator, provider, source revision, limits,
  completeness, and result or error.

Representative families are:

```text
env://PATH
worktree:///src/lib.rs?revision=<worktree-id>#L40-L62
git://<commit>/src/lib.rs#L40-L62
rey-workload://portfolio-label-normalization?revision=<revision>
rey+local://agent/codex?revision=gpt-5&role=coding_harness
spoke+local://<provider-owned-identity>
```

These examples establish families, not an implemented universal parser. Rey
must not reinterpret a provider-owned Spoke locator or claim global uniqueness
without that provider's public contract.

## Resolution

Resolution is an explicit operation over a frozen capability snapshot. The
resolver checks scheme ownership, canonical syntax, source preconditions,
authority, and bounds before it retrieves anything. A successful result keeps
the original locator and returns an exact resolved coordinate and identity. A missing,
stale, unsupported, truncated, or unauthorized locator remains distinct.

Relative filesystem strings are not portable locators until they are bound to
an exact workspace or Git identity. Display shortcuts may shorten revisions,
but semantic identity always retains the complete revision.

## Library Bearing

The next implementation slice should add a dependency-light `rey-locator`
crate with:

- closed locator and canonical query-dimension types;
- canonical parse/format round trips;
- scheme registration without provider execution;
- fragment types for lines, JSON pointers, table keys, graph nodes, and source
  spans;
- stale/missing/unsupported resolution outcomes; and
- fixtures for encoding, duplicate dimensions, path escape, revision drift,
  opaque provider payloads, and deterministic ordering.

The first CLI proof is the admitted `context-anchor-survey` workload in [Plan
0017](../plans/0017-incremental-context-topography.md). It begins with a bounded
process-owned seed-name inventory containing `AGENTS.md` and README variants,
locates URI and reference candidates, records typed resolution outcomes, and
emits a topography patch. Those seed names are workload inputs rather than
implicit configuration, and neither locator parsing nor Explorer navigation
initiates recursive retrieval.
