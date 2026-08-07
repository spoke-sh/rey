# ADR 0005: Environment Awareness And Optional Spoke

- Status: Accepted
- Date: 2026-08-07
- Supersedes in part: [ADR 0003](0003-spoke-and-proof-boundary.md) where it made
  an explicit Spoke endpoint a requirement for Rey development and integration

## Context

Rey explores context surfaces that vary by workspace, machine, deployment, and
task. Useful tools may include version control, fast text search, compilers,
language analyzers, build systems, test runners, and a Spoke deployment.

Treating those tools as invisible ambient state would make actions difficult to
reproduce and proofs easy to overstate. Requiring a healthy Spoke for all useful
work would also prevent Rey from diagnosing or improving Spoke precisely when
Spoke is absent, broken, or under development.

Rey needs one runtime whose capabilities can expand and contract without
silently changing the meaning of a claim.

## Decision

Rey discovers context surfaces through bounded, versioned environment
providers. Discovery produces a typed capability snapshot before action
planning. Each capability records provider identity and revision, kind,
resolved location, version, digest or provenance when available, availability,
trust class, supported operations, enforcement claims, limits, observation
time, and errors.

Discovery is read-only and allowlisted by provider. It may resolve configured
paths or `PATH`, inspect metadata, and run a known bounded identity probe such
as `--version`. It does not execute arbitrary discovered files, project startup
hooks, installers, or mutations. Action admission separately freezes and
revalidates the selected capability row.

Rey supports:

- a standalone profile using built-in capabilities, an explicit local context,
  and admitted local tools;
- a connected profile that adds a Spoke provider and its advertised query,
  compute, lineage, and persistence capabilities; and
- per-space or per-claim required capabilities that fail closed when absent.

Spoke is optional by default. Its absence removes capabilities rather than
changing runtime semantics. A Spoke-backed claim cannot silently become a
local-only claim. Local evidence bundles state their weaker retention and
execution guarantees and never mint Spoke identities.

When connected, ADR 0003 still applies: Rey uses public Spoke contracts, never
opens Spoke storage, and does not resolve Spoke paths through the host.

Capability snapshots participate in frame, action, trace, and proof identity.
Provider, tool, version, path, digest, trust, or guarantee drift can invalidate
an action or make prior proof stale.

## Consequences

- Environment discovery becomes an explicit capability with schemas, limits,
  fixtures, and failure behavior.
- Rey's foundation and deterministic tests run with zero Spoke.
- Local process execution must disclose that it is not a sandbox and must not
  imitate Spoke compute fencing or durability.
- `auto` discovery is allowed only where detection is safe and bounded; an
  explicit configuration can disable or require providers.
- A missing optional capability narrows the action set. A missing required
  capability stops admission or makes the dependent claim inconclusive.
- Connected and standalone results can be compared because both carry explicit
  capability snapshots and provider guarantees.
