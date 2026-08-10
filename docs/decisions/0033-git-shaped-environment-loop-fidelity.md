# ADR 0033: Git-Shaped Environment Loop Fidelity

- Status: Accepted
- Date: 2026-08-10
- Extends: [ADR 0021](0021-environment-admission-index.md)
- Supersedes the default human `env status`, `env add -p`, and `env log`
  projections in: [ADR 0027](0027-environment-operator-delta.md),
  [ADR 0029](0029-environment-history-projection.md), and
  [ADR 0031](0031-desired-application-inventory-and-search-records.md)
- Advances environment commits, commit results, and logs to v2

## Context

The environment revision mechanism already has Git's useful three-plane
structure: committed HEAD, an admission index, and a freshly observed working
environment. Its human interface does not consistently express that structure.
`env status` repeats the full environment diff and becomes a wall of text;
`env add -p` prompts over implementation-level capability records; and
`env log` separates the local commit number from the commit header and has no
date. Operators must mentally reconstruct the familiar status, patch, commit,
and history loop.

The typed capability delta must remain authoritative. A higher-fidelity human
projection may summarize it, but must disclose any authoritative changes that
do not map to a variable, application, input, or reference.

## Decision

`rey env status` is a compact working-tree view. It identifies the current
`ENV@n` HEAD (or an unborn environment), working-tree state, observation
completeness, desired-application search summary, and reasoning-map coordinate.
It then renders separate Git-shaped groups:

```text
Changes to be committed:
Changes not staged for environment commit:
```

Rows name environment-native objects and classify them as new, modified, or
deleted. The view does not repeat exact values, binary identities, or topology
details; `rey env diff --staged`, `rey env diff`, and structured status remain
the drill-down surfaces. Capability changes without an operator object are
reported individually with a stable human semantic label and exact capability
id rather than hidden behind an aggregate count.

`rey env add -p` remains a deterministic selector over canonical
`INDEX → WORKING` capability changes, but each prompt is an environment-native
hunk. It renders a stable `diff --rey` object path, direction and change kind,
then variable, application, input, or reference before/after evidence when a
projection exists. The prompt is:

```text
Stage this hunk [y,n,q,a,d,?]?
```

`y` stages one hunk, `n` skips it, `a` stages it and all remaining hunks, `q`
quits while preserving prior selections, and `d` leaves it and all remaining
hunks unstaged. An unmapped capability falls back to its exact provider,
revision, and capability record. Interactive patch mode remains table-only.
Raw `provenance` and `error_detail` values are never printed by the fallback;
it reports that structured evidence changed and directs exact inspection to
`rey env diff --format json`.

The environment snapshot retains the desired `git` application and its bounded
identity observation, but excludes `git.repository.inspect`. HEAD, refs,
semantic index entries, and reachability are first-class Git cadence and
workload-activation inputs rather than environment admissions. Git movement by
itself therefore cannot dirty `rey env status` or create an `env add -p` hunk.
Existing histories remain immutable and verifiable; a retained legacy Git row
appears once as a deletion from environment scope and its compact hunk explains
the ownership move.

`rey env log -n <count>` selects at most that many commits, newest first, under
the existing bound. Every human entry begins with one Git-shaped coordinate:

```text
commit ENV@2 <semantic-commit-id> (HEAD)
Parent: ENV@1 <semantic-parent-id>
Date:   Mon Aug 10 17:12:37 2026 +0000

    commit message
```

The evidence, environment, changed-dimension, mapping, and optional `-p`
planes follow that header.

New `rey.environment-commit.v2` commits retain an integer Unix commit time.
The timestamp is explicit commit metadata and participates in the semantic
commit digest alongside sequence, parent, message, and snapshot. This is a
commit-time observation, not discovery evidence, author identity, proof time,
or a claim of trusted clock ordering. Existing `rey.environment-commit.v1`
records remain readable and verifiable without migration; their human date is
`unknown (legacy environment commit)`. A v1 record cannot acquire a timestamp,
and a v2 record without one is invalid.

Structured commit results and logs advance to
`rey.environment-commit-result.v2` and `rey.environment-log.v2`. The local
history container stays `rey.local-environment-history.v1` because it is a
bounded chain envelope that can safely contain verified v1 and v2 commit
records. Status, admission-index, add-result, and diff schemas do not change.

## Consequences

- The default CLI follows the same scan → patch → commit → history rhythm as
  Git without pretending environment records are Git objects.
- Status stays compact while exact current and staged evidence remains one
  command away.
- Patch selection is human-verifiable in environment vocabulary while staging
  remains keyed by authoritative capability identities.
- Git application discovery stays visible without copying repository snapshots
  or semantic index entries into every environment revision.
- Dates are available for new history without fabricating chronology for
  legacy commits or conflating wall time with causal ordering.
- Structured consumers must accept the v2 commit-result and log envelopes and
  the optional timestamp needed to decode mixed legacy histories.
