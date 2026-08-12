# Locators

Locators are Rey's exact anchor protocol for the survey phase. A locator names
an object or bounded region in context. It does not retrieve the object, grant
authority to read it, admit an executable, or prove that the object exists.

A locator is not itself an Explorer camera location or necessarily a resolved
semantic coordinate. Resolution may bind a candidate locator to a typed,
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
rey-workload://context-anchor-survey?revision=<revision>
rey+local://agent/codex?revision=gpt-5&role=coding_harness
rey+local://<provider-owned-identity>
```

These examples establish families, not an implemented universal parser. The
implemented `rey-locator` slice parses canonical workspace references and
HTTP/HTTPS candidates and emits `rey+local://...` bindings. Rey does not claim
global uniqueness or federation for this local carrier.

## Resolution

Resolution is an explicit operation over a frozen capability snapshot. The
resolver checks scheme ownership, canonical syntax, source preconditions,
authority, and bounds before it retrieves anything. A successful result keeps
the original locator and returns an exact resolved coordinate and identity. A missing,
stale, unsupported, truncated, or unauthorized locator remains distinct.

Relative filesystem strings are not portable locators until they are bound to
an exact workspace or Git identity. Display shortcuts may shorten revisions,
but semantic identity always retains the complete revision.

## Implemented Library Slice

The dependency-light `rey-locator` crate now implements:

- canonical local coordinate parse/format with view-state dimensions rejected;
- lossless provider-qualified local coordinate carriers;
- canonical workspace-reference and HTTP/HTTPS locator parse/format;
- resolved, missing, stale, unsupported, unauthorized, malformed, and
  truncated outcomes with exact capability snapshots and hard limits; and
- fixtures for encoding, duplicate dimensions, opaque provider payloads,
  distinct outcomes, path escape, and deterministic replay.

Generic scheme registration and typed line/JSON/table/graph/source-span
fragments remain later breadth. The current carrier is deliberately local:
Rey claims no remote resolution, durability, global identity, or federation
semantics.

The first CLI proof is the admitted `context-anchor-survey` workload. It begins
with a bounded process-owned seed-name inventory containing `AGENTS.md` and
README variants, locates URI and reference candidates, records typed resolution
outcomes, and emits `rey.topography-patch.v1` plus a directed patch delta. Those
seed names are workload inputs rather than implicit configuration, and neither
locator parsing nor Explorer navigation initiates recursive retrieval.

Explorer projects an unresolved locator outcome as a frontier station and
local weather condition and labels the required prerequisite. It draws no line
back to the source anchor: the front is neither a crossing, a resolved
relationship, a path, nor permission to invoke a resolver.
