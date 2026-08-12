# ADR 0049: Git-Shaped Workload Admission History

Status: Accepted

## Context

Workspace workload packages previously declared their own accepted admission
state. That collapsed agent proposal, deterministic qualification, and human
approval into one YAML document. It also made the operator UI a projection of
already-admitted workloads instead of the place where incoming agentic work is
reviewed.

The environment and scene editor already establish the more useful human/agent
loop: mutable authored bytes are WORKING, `add` freezes exact bytes in INDEX,
and a human commit advances HEAD. Workloads need the same separation, with one
additional gate: every package in INDEX must pass its frozen scenario suite
against that exact INDEX snapshot before approval.

## Decision

Workspace workload admission has three planes:

```text
agent-authored workload.yaml   rey workloads add   rey workloads commit / UI approval
WORKING                     ───────────────────▶ INDEX ───────────────────────────────▶ HEAD
proposal                                         frozen + qualified                    admitted
```

- A `rey.workload-package.v1` owns the workload contract, generated graph,
  frozen scenario suite, and generation lineage. It contains no admission
  declaration and cannot admit itself.
- `rey workloads status` is the compact portfolio status over HEAD, INDEX, and
  WORKING. `diff` compares INDEX to WORKING; `diff --staged` compares HEAD to
  INDEX.
- `rey workloads add` stages the complete verified workspace catalog and
  writes exact package bytes into a content-addressed local object store.
- `rey workloads test --staged [id]` is the only workspace qualification
  surface. A commit is ready only when every package in the complete INDEX has
  fresh passing qualification bound to that INDEX snapshot identity.
- `rey workloads commit -m <message>` advances HEAD from the already-frozen
  INDEX. It does not observe WORKING. The commit records its parent, exact
  snapshot, qualification identities, message, timestamp, and semantic
  identity. `rey workloads log [-p]` renders that verified linear history.
- `rey workloads list` projects admitted HEAD plus separate draft and revision
  posture. `rey workloads run` resolves only HEAD. The explicit conformance
  catalog remains an immutable diagnostic surface and does not participate in
  workspace admission history.
- `rey ui` enters the admission Feed, shows incoming INDEX and WORKING
  candidates, and can approve only an exact qualified INDEX using expected
  HEAD and INDEX preconditions. Approval remains enabled on every explicitly
  configured bind, including non-loopback binds. Because this surface has no
  authentication, non-loopback startup must warn that reachable clients can
  admit workloads.
- The product catalog starts with `context-anchor-survey` as an unadmitted
  agentic proposal. `rey.portfolio.label-normalization` is removed; text
  normalization remains only in the explicitly selected conformance fixtures.

The v1 local state reader rejects the prior self-admission shape. There is no
compatibility or migration reader during pre-alpha development.

## Consequences

An agent can author, stage, and qualify a proposal, but it cannot make that
proposal runnable. A human sees the exact pending revision in the browser and
creates the admission event explicitly. WORKING may continue changing while a
frozen INDEX is reviewed; those later bytes cannot leak into the commit.

The approval endpoint is intentionally powerful and unauthenticated. Binding
the UI beyond loopback is an explicit operator choice and must be protected by
the surrounding deployment when access control is required.

ADR 0023's self-declared accepted package state and ADR 0025's read-only UI are
superseded by this decision. Journal write authority, workload admission,
runtime execution, and proof authority remain distinct.
