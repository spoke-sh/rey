# ADR 0028: Environment Three-Plane Diff

- Status: Accepted
- Date: 2026-08-09
- Extends: [ADR 0027](0027-environment-operator-delta.md)
- Supersedes the default human `env diff` patch projection in:
  [ADR 0020](0020-environment-mapping-graph.md) and
  [ADR 0021](0021-environment-admission-index.md)
- Generic human `env log -p` projection superseded by:
  [ADR 0029](0029-environment-history-projection.md)

## Context

ADR 0027 made `rey env status` and `/environment` project the environment a
programmer recognizes: directed variable text, bounded executable search,
inputs, and declared references. `rey env diff` still opened the generic
capability patch. That wall exposed internal provider records before it exposed
the selected environment direction, leaving the related commands with
different human grammars.

The capability delta remains authoritative. It may also contain changes, such
as repository observation drift, that do not correspond to a mapped variable,
application, input, or reference. Refining the human interface must preserve
that boundary rather than silently equating the operator projection with the
complete delta.

## Decision

The default human `rey env diff` projects the shared
`rey.environment-operator-projection.v1` through exactly three sequential
evidence planes:

```text
01 / DIRECTED TEXT
  selected environment variables

02 / BOUNDED SEARCH
  applications found, searched but not found, errored, or no longer searched

03
REFERENCE PLANE
  inputs and declared topology
```

Without `--staged`, every object selects its exact `INDEX` source and
`WORKING` target plus `index_to_working` classification. With `--staged`, it
selects `HEAD`, `INDEX`, and `head_to_index`. Unchanged objects in the
selected planes remain bounded context. Modified literal variables and input
identities render before and after lines; secrets retain their declared
presence-only or non-literal capture semantics.

One compact coordinate header reports the selected direction, view, workspace,
authoritative capability assessment, and retained capability-change count.
Consequently a nonzero authoritative count may coexist honestly with zero
mapped-object changes.

Structured output does not adopt the human projection. `--format json`
continues to emit `rey.environment-diff.v2` with the complete typed capability
delta, frozen working snapshot, admission index, and revision coordinates.
At the time of this decision, `env log -p` and interactive `env add -p`
continued to use the generic capability patch where exact capability-level
review was the operation being performed. ADR 0029 later moves retained log
history onto the environment-native projection; interactive admission remains
capability-level.

## Consequences

- `status`, `diff`, and `/environment` now share one recognizable
  environment grammar without creating another observation model.
- Unstaged and staged output cannot accidentally read from the wrong plane;
  object selection is explicit and tested in both directions.
- Found and not-found applications remain visible as the complete bounded
  target search, not merely as changed capability rows.
- Lower-level provider fields no longer dominate ordinary human diff output,
  but automation and diagnostic consumers retain the accepted v2 JSON
  contract.
- A future generic human diagnostic view must be an explicit interface; it
  must not displace these three default evidence planes.
