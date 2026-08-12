# Plan 0008: Environment Mapping Graph

- Status: Complete
- Completed: 2026-08-08
- Decision: [ADR 0020](../docs/decisions/0020-environment-mapping-graph.md)
- Extended by: [Plan 0009](0009-environment-admission-index.md) and
  [ADR 0021](../docs/decisions/0021-environment-admission-index.md);
  human log projection superseded by
  [ADR 0029](../docs/decisions/0029-environment-history-projection.md)

## Outcome

Make the human `env` surface describe and revision the environment graph Rey
actually cares about. Replace file-pair diff and manual proof plumbing with a
HEAD-to-working patch, then parse and observe a bounded agent-generatable YAML
mapping of variables, files, executables, and reference edges.

## Completion Checklist

- [x] Accept ADR 0020 and update the decision/plan indexes.
- [x] Remove `prove`, `verify`, and `verify-bundle` from the CLI without deleting
  their lower-level proof contracts.
- [x] Replace file-pair `env diff` with fresh `HEAD → WORKING` human and JSON
  output; keep `status` compact and `log -p` patch-bearing.
- [x] Define a closed, canonical, bounded `rey.env-map.v1` YAML graph schema.
- [x] Load `rey.env.yaml` by convention and support an explicit safe `--map`
  path beneath the workspace.
- [x] Observe variable presence/digests without retaining values or sensitive
  fingerprints.
- [x] Observe bounded regular files and resolve executables without invoking
  them or admitting proposed capabilities.
- [x] Project graph, node, and edge evidence into committed capability
  snapshots with exact map, source, limit, and observation lineage.
- [x] Render mapping counts and node/edge changes through `status`, `diff`,
  `commit`, and `log -p`.
- [x] Cover malformed/unknown YAML, duplicate ids/edges, missing endpoints,
  sensitive capture, path escape, symlinks, missing and changed inputs,
  executable resolution, bounds, deterministic replay, JSON, stderr, and exits.
- [x] Check in a truthful Rey mapping example and update foundational docs.
- [x] Run focused tests, full workspace tests, Clippy, build, Nix checks, link
  review, and repository-truth audit.

## Proof

Captured on 2026-08-08:

```text
cargo test -p rey-environment mapping
cargo test -p rey --test cli env_
just check
just test
# 123/123 tests passed; all workspace doc tests passed
just build
nix flake check "path:$PWD"
# package, workspace-test, and wrapper derivations passed
```

Human verification exercised `rey env status`, `rey env diff`, and
`rey env --help` against the checked-in mapping. CLI fixtures additionally
prove mapping counts and identity, file and non-sensitive-variable drift,
presence-only secret stability, raw-value omission, executable non-invocation,
JSON, stderr, and categorized exits.

## Deferred

Dynamic file paths derived from variable values, parsing references from file
contents, invoking executable version probes from the DSL, capability
action admission, revision selectors, graph query/visualization breadth, and
remote retention remains a later slice. Plan 0009 delivered history admission;
that does not admit mapped executable capabilities for action.
