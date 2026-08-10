# ADR 0029: Environment History Projection

- Status: Accepted
- Date: 2026-08-09
- Extends: [ADR 0028](0028-environment-three-plane-diff.md)
- Supersedes the default human `env log` and `env log -p` projections in:
  [ADR 0019](0019-git-shaped-environment-history.md) and
  [ADR 0020](0020-environment-mapping-graph.md)

## Context

`rey env status`, `rey env diff`, and `/environment` now share a bounded
environment-native grammar. History remains the exception. Plain `env log`
renders a dense block of snapshot, provider, comparator, digest, and retention
metadata for every commit; `env log -p` adds the generic capability wall.
The operator must translate that implementation vocabulary back into variables,
applications, inputs, and references for every historical transition.

History has a useful Git-shaped distinction that should remain: plain log is a
compact chronology, while `-p` expands patches. Changing the human projection
must not change retained commits, recomputed authoritative deltas, selection
bounds, or structured output.

## Decision

Plain `rey env log` renders selected commits newest first. Each entry exposes:

- the full semantic commit id and HEAD marker;
- local `ENV@n` revision and parent revision;
- authoritative delta direction, assessment, and retained change count;
- target environment scope across variables, applications, inputs, and
  references;
- changed counts for those same dimensions;
- retained mapping coordinate and commit message.

Snapshot ids, comparator ids, provider records, storage paths, and detailed
capability fields do not dominate the default chronology. They remain in the
structured document.

`rey env log -p` appends the same three evidence planes used by environment
diff beneath every selected commit:

```text
01 / DIRECTED TEXT
02 / BOUNDED SEARCH
03 / REFERENCE PLANE
```

Every expansion is derived from the exact retained parent and commit snapshots.
The root transition uses the typed empty source and remains
`EMPTY → ENV@1`. Log performs no discovery, does not reload the current map,
and does not consult the admission index or working environment. Unchanged
mapped objects remain bounded context; modifications render selected historical
before and after observations.

The `-n` and change limits are applied by the authoritative log derivation
before presentation. Explicit JSON remains `rey.environment-log.v1`, including
complete commits, snapshots, typed capability deltas, and the existing `patch`
request flag. Human projection data is derived, not added as a second retained
history model.

## Consequences

- Plain history becomes scannable without losing semantic revision identity or
  the distinction between authoritative and mapped-object changes.
- `-p` means environment-native patch expansion consistently across current
  and historical deltas.
- Historical rendering is deterministic and immune to current environment or
  mapping drift because it uses retained snapshots only.
- A nonzero capability change count may coexist with zero mapped changes and is
  reported honestly in the entry coordinate.
- Automation retains the accepted v1 log schema; capability-level diagnostics
  can inspect its typed deltas without forcing those records into the default
  human interface.
