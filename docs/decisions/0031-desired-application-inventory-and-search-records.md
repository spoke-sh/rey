# ADR 0031: Desired Application Inventory And Search Records

- Status: Accepted
- Date: 2026-08-09
- Supersedes: the undifferentiated application presentation in [ADR 0027](0027-environment-operator-delta.md)

## Context

`rey.env-map.v2` declared executable names and Rey immediately rendered their
observations as “applications searched.” This preserved found and missing
results, but it collapsed two different facts:

1. which applications the operator wants an agent to investigate, and why;
2. what one bounded environment observation actually searched and found.

That collapse made `cargo` look like a current Rey extension target merely
because Cargo is part of this Rust repository's development environment.
Build availability is not application-inventory intent.

## Decision

The mapping DSL advances to `rey.env-map.v3`. Every desired executable must
declare a bounded human-readable `purpose` in addition to its name, requirement
level, and potential capabilities. Rey derives a dedicated semantic identity
over the canonical executable-declaration subset; that is the desired
inventory record. Its source path and inventory digest identify exactly what
was presented to the agent or deterministic caller without changing merely
because an unrelated mapped variable or file changed.

The target `rey.capabilities` snapshot is a separate search record. Executable
observation remains deterministic bounded PATH identity resolution: Rey does
not invoke the executable, infer adapter semantics, or grant authority. The
snapshot digest identifies the exact found, missing, errored, and bounded
search results. `env add` and `env commit` remain the explicit admission and
retention boundary for that observation.

Human CLI and UI projections render these records in order:

```text
DESIRED INVENTORY · exact application declaration record
SEARCH RECORD · exact target capability snapshot
FOUND / SEARCHED, NOT FOUND / ERRORS / NO LONGER SEARCHED
```

The structured contracts advance to `rey.environment-status.v4`,
`rey.environment-operator-projection.v2`, and `rey.environment-diff.v3` because
application observations now retain declared purpose.

An agent may propose or edit `rey.env.yaml`, but deterministic discovery does
not claim that an agent authored the map. Authorship remains bound by the
surrounding source revision and review process. Future agent-generation
provenance can extend the declaration record without merging it with search
evidence.

The checked-in Rey inventory removes `cargo` and `CARGO_HOME`. It currently
desires `git` for repository context and `rg` as a text-mining extension
candidate. `Cargo.toml`, `flake.nix`, and `justfile` remain mapped inputs; a
mapped input is not automatically a desired application.

## Consequences

- Development dependencies cannot silently become application intent.
- The agent sees the bounded desired inventory before inspecting results.
- Operators can independently verify declaration drift and search-result drift.
- Missing applications remain useful first-class evidence.
- A found executable remains an unadmitted candidate until an adapter freezes
  its operations, trust, effects, and limits.
