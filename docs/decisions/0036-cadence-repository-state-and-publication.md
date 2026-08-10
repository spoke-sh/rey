# ADR 0036: Cadence Repository State And Publication

- Status: Accepted
- Date: 2026-08-10
- Extends: the cadence projection in [ADR 0030](0030-operator-cadence-agents-and-explorer-coordinates.md)

## Context

The first cadence projection retained reachable commits but did not answer two
immediate operator questions: what is changing in the current working tree,
and which visible revisions have reached the branch's upstream? A commit lane
without those dimensions shows chronology while hiding the repository state
that should direct attention.

Git's working tree, index, `HEAD`, and local tracking refs are distinct mutable
surfaces. A clean tree does not imply a pushed branch, and a synchronized
branch does not imply a clean tree. A local tracking ref also does not prove the
current state of a remote host unless a separate admitted network operation has
updated it.

## Decision

`rey-git` adds `rey.git-repository-status.v1`, derived through bounded direct
Git argv. Its working-tree axis records independent staged, unstaged,
untracked, and conflicted entry counts from porcelain-v2 status. Its
publication axis records branch, exact `HEAD` OID, configured upstream, exact
locally retained upstream OID, ahead/behind counts, and one of `pushed`,
`unpushed`, `behind`, `diverged`, `no_upstream`, `detached`, `unborn`, or
`unknown`.

Ahead/behind counts are computed between the retained exact OIDs. Each bounded
Git cadence commit is classified `pushed` when it is reachable from that exact
upstream OID, `local` when it is not, and `unknown` when no exact upstream is
available. These reads never fetch, push, contact GitHub, or advance a ref.
"Pushed" therefore means reachable from the local tracking-ref snapshot, not
remotely verified at request time.

`GET /api/v1/cadence` hard-cuts to `rey.ui-cadence.v2`. It adds a nullable
`repository_state` and a nullable publication classification on each tick.
`/cadence` leads with two adjacent instruments:

- working-tree state and its four attention counts; and
- push relation, branch-to-upstream binding, ahead/behind counts, and exact
  linked revisions.

The UI labels the comparison `LOCAL REF` and `NO NETWORK FETCH`. Repository
state is section 01; retained independent clocks, mounted scan descriptions,
and the reference plane follow it. This remains a live read-only projection,
not a Git poll cursor, remote synchronization operation, activation stream, or
generic scheduler.

## Consequences

- Operators can distinguish uncommitted work from unpushed work without
  leaving the cadence plane.
- Commit chronology exposes publication as a bounded reachability fact rather
  than visual inference.
- Exact OIDs bind ahead/behind and per-commit classification against ref
  movement during a request.
- Remote freshness remains explicit future work; Rey must not describe a local
  tracking ref as a live GitHub observation.
- Ref movement classification, cursor replay, workload activation, and remote
  transport retain their existing owners.
