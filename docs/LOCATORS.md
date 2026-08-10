# Locators

Locators are Rey's exact anchor protocol for the survey phase. A locator names
an object or bounded region in context. It does not retrieve the object, grant
authority to read it, admit an executable, or prove that the object exists.

## Contract

A locator has:

- a scheme with one owning resolver;
- a canonical payload identifying a resource or region;
- optional unordered, unique matrix dimensions such as exact revision,
  semantic lens, or agent role;
- an explicit mutable or immutable identity class;
- a bounded display form and a lossless machine form; and
- resolution evidence binding the locator, provider, source revision, limits,
  completeness, and result or error.

Representative families are:

```text
env://PATH
worktree:///src/lib.rs;at=<worktree-id>#L40-L62
git://<commit>/src/lib.rs#L40-L62
rey-workload://portfolio-label-normalization;at=<revision>
/explore/agent/codex;at=gpt-5;lens=objects;role=coding_harness
spoke+local://<provider-owned-identity>
```

These examples establish families, not an implemented universal parser. Rey
must not reinterpret a provider-owned Spoke locator or claim global uniqueness
without that provider's public contract.

## Resolution

Resolution is an explicit operation over a frozen capability snapshot. The
resolver checks scheme ownership, canonical syntax, source preconditions,
authority, and bounds before it retrieves anything. A successful result keeps
the original locator and returns an exact resolved identity. A missing,
stale, unsupported, truncated, or unauthorized locator remains distinct.

Relative filesystem strings are not portable locators until they are bound to
an exact workspace or Git identity. Display shortcuts may shorten revisions,
but semantic identity always retains the complete revision.

## Library Bearing

The next implementation slice should add a dependency-light `rey-locator`
crate with:

- closed locator and matrix-dimension types;
- canonical parse/format round trips;
- scheme registration without provider execution;
- fragment types for lines, JSON pointers, table keys, graph nodes, and source
  spans;
- stale/missing/unsupported resolution outcomes; and
- fixtures for encoding, duplicate dimensions, path escape, revision drift,
  opaque provider payloads, and deterministic ordering.

The first CLI proof should let an agent generate or validate a mapping
resource and show its locator anchors before any generic background scheduler
is introduced.
