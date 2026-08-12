# ADR 0008: First-Slice Contracts And Dependency Baseline

- Status: Accepted
- Date: 2026-08-07

## Context

The foundational runtime needs concrete types and dependencies before it can
honestly expose an executable. The first slice must define typed tabular
transport, produce stable semantic identities, and observe a
local environment without turning ambient executables or Git configuration
into authority.

The complete frame, delta, Git polling, activation, and proof schemas remain
larger than one safe implementation step. Freezing all of them before fixtures
exist would give speculative layouts accidental durability.

## Decision

The initial workspace uses Rust 2024 with a minimum Rust version of 1.96. It
pins Polars 0.55.2 with default
features disabled and only `fmt` and `ipc_streaming` enabled. Arrow IPC stream
is the typed frame representation and
`application/vnd.apache.arrow.stream` is its media type.

The first crates have one-way ownership:

```text
rey-core
  ├── rey-dataframe
  └── rey-environment
         └── rey-git

rey (composition and CLI) depends on all four
```

`rey-core` owns length-framed BLAKE3 semantic identity and small shared
observation contracts. `rey-dataframe` owns Polars frames and Arrow encoding.
`rey-environment` owns capability records, snapshots, executable resolution,
and bounded direct-process observation. `rey-git` owns Git-specific read-only
interpretation. The `rey` crate composes providers but owns no provider logic.

The capability relation schema is `rey.capabilities` version `1`. Array-valued
logical fields use canonical compact JSON arrays in string columns for this
version; changing them to Arrow list or struct columns requires a schema
revision. JSON documents carry the same ordered records plus frame metadata.
Arrow streams carry relation, schema version, semantic digest, row count, and
completeness as custom schema metadata.

Capability snapshot identity sorts records by provider, provider revision, and
capability id, then hashes each semantic field with explicit byte lengths.
Display formatting and observation time do not participate. This local slice
does not have an authenticated clock, so `observed_at` remains null rather than
inventing time evidence.

Known tools are an adapter allowlist, initially `git` and `rg`. Resolution
searches only configured paths, invokes only the adapter's fixed identity
arguments, uses direct argv and a cleared environment, concurrently drains
bounded output, and terminates on deadline or overflow. Discovery does not
grant execution authority.

Git inspection initially records repository/worktree path identities, object
format, bare and shallow state, symbolic or detached/unborn HEAD, and an index
entry digest over ordered `(mode, oid, stage, raw path bytes)` tuples returned
by bounded read-only Git commands. Raw paths are retained with reversible
base64 identity and a separate lossy display form. The index digest is labeled
incomplete for flag-sensitive semantics until intent-to-add,
assume-unchanged, skip-worktree, sparse, and split-index fixtures are covered.
It therefore cannot yet drive a proof or application activation that requires
a complete semantic index.

## Consequences

- The first CLI can emit a useful typed local-only capability snapshot without
  implying that polling, activation, mutation, or proof exists.
- Stable semantic ids do not depend on JSON map ordering, terminal rendering,
  timestamps, or raw Git index stat-cache bytes.
- Polars compile cost is paid once through Crane dependency artifacts.
- A new schema version is required for nested Arrow capability fields or new
  identity-bearing fields.
- Git index flag completeness, watched refs, commit traversal, cursoring, and
  replay-safe activation remain future Git work.
