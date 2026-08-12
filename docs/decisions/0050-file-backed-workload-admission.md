# ADR 0050: File-Backed Workload Admission

Status: Accepted

## Context

ADR 0049 introduced the correct HEAD → INDEX → WORKING workload loop, but the
browser could approve only an INDEX that an agent had already staged and
qualified in hidden local state. After `.rey` was cleared, the authored
`context-anchor-survey` files still existed yet the human admission control was
disabled. The default `workloads/` root also obscured the intended system
namespace.

Incoming workloads are authored artifacts. The admission surface must begin
from their visible, bounded file state and must not require an invisible CLI
ritual before a human can review them.

## Decision

- The default workspace workload catalog is `sys/`. Each immediate child is a
  package directory, for example `sys/context-anchor-survey/`, containing
  `request.yaml`, `workload.yaml`, or both.
- `sys/*/workload.yaml` is authoritative WORKING input. `.rey/workloads`
  retains content-addressed INDEX objects, qualification evidence, and HEAD
  history; it is not the source of incoming proposals.
- The CLI keeps the explicit Git-shaped diagnostic loop: `add` freezes
  WORKING, `test --staged` qualifies INDEX, and `commit` advances HEAD without
  rereading WORKING.
- Browser admission accepts a message plus exact expected HEAD and WORKING
  snapshot identities. In one operator action Rey stages the reviewed files,
  executes every frozen scenario suite in that complete snapshot, and commits
  only the resulting exact qualified INDEX.
- A stale HEAD or changed WORKING snapshot rejects admission. Qualification
  failure never advances HEAD; the frozen INDEX and test evidence remain
  available to `status`, `diff`, and `test --staged` diagnostics.
- The browser write route is `/api/v1/workloads/admit`. It does not imply that
  mutable WORKING bytes can bypass INDEX or qualification.

This supersedes ADR 0049 only for the browser's requirement that INDEX already
exist and qualify before the approval request. ADR 0049's three planes,
content-addressed INDEX, exact qualification, HEAD-only execution, and CLI
commit semantics remain in force. The default root portions of ADRs 0023 and
0024 are superseded from `workloads/` to `sys/`.

## Consequences

A fresh clone or cleared `.rey` directory still presents
`context-anchor-survey` as an incoming file package in `rey ui`. The human can
admit exactly what is visible without manually running `add` and `test`, while
the resulting retained history preserves the same frozen evidence and runtime
boundary as the CLI path.

Admission runs bounded read-only scenario probes and therefore can take longer
than a pure commit. The button and error rendering must expose that combined
qualification/admission operation honestly.
