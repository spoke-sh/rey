# Plan 0012: Environment Operator Delta

- Status: Completed
- Decision: [ADR 0027](../docs/decisions/0027-environment-operator-delta.md)

## Outcome

Make the environment Rey's first first-class diff-directed human surface. One
typed `HEAD → INDEX → WORKING` projection must power both `rey env status` and
`/environment`, lead with an env-shaped variable diff, and preserve the full
bounded set of applications found and searched but not found.

## Completion Checklist

- [x] Accept ADR 0027 and define the value-capture, shared-projection, and
  read-only UI boundaries.
- [x] Hard-cut the mapping DSL to `rey.env-map.v1` with bounded non-sensitive
  value capture and secret-safe rejection fixtures.
- [x] Derive typed variable, application, input, and reference status across
  committed, admitted, and working planes.
- [x] Rewrite `rey env status` as a bounded environment diff rather than a
  generic capability inventory.
- [x] Apply the same three-plane operator grammar to default and staged
  `rey env diff` output while retaining the authoritative v1 JSON delta.
- [x] Reduce `rey env log` to an environment-native chronology and make `-p`
  expand exact retained transitions through the same three planes.
- [x] Add `GET|HEAD /api/v1/environment` from the same derivation used by the
  CLI.
- [x] Replace the portfolio-shaped `/environment` page with a Kinetic Precision
  variable diff and found/not-found application surface.
- [x] Prove CLI stdout/stderr/JSON/exit behavior, HTTP method and schema
  behavior, frontend derivation/rendering, secret safety, bounds, and packaged
  embedded assets.
- [x] Update foundational documents, examples, plan evidence, and repository
  truth after verification.

## Concrete Anchor

Use the checked-in `rey.env.yaml` to prove four variable modes and three
application outcomes without inventing execution authority. `PATH`,
`CARGO_HOME`, and `SPOKE_ENDPOINT` are explicit non-sensitive value captures;
`SPOKE_TOKEN` is presence-only. `cargo`, `git`, and `rg` remain bounded
executable searches whose current paths and potential capabilities are
evidence, not admitted operations.

The high-fidelity proof is visible through:

```text
rey env status
rey env status --format json
rey env diff
rey env diff --staged
rey env log
rey env log -p
rey ui
```

## Current Proof

Captured on 2026-08-09:

```text
nix develop path:$PWD --command just check
# Prettier, TypeScript, 12/12 UI tests, Vite, Rustfmt, Clippy -D warnings,
# git diff validation, and flake evaluation passed
nix develop path:$PWD --command just test
# 136/136 Rust tests, 12/12 UI tests, and every documentation test passed
nix develop path:$PWD --command just build
# deterministic UI assets and the complete Rust workspace built
nix build path:$PWD#rey --no-link --print-out-paths
# /nix/store/lciyr7i3bg9519x20psa2984z8rdwcfb-rey
```

The packaged `rey env status` rendered four tracked variables as a bounded
`ENV@1 → WORKING` text diff, three declared applications as found with exact
paths and search counts, three input identities, seven reference edges, and no
generic capability wall. Its live `/api/v1/environment` response returned
`rey.environment-status.v1` and
`rey.environment-operator-projection.v1` from the same workspace. The
`SPOKE_TOKEN` observation remained `capture: presence` with a null value.

The human `rey env diff` now renders its selected `INDEX → WORKING` or
`HEAD → INDEX` direction as directed variable text, bounded application search,
and input/reference topology. Focused fixtures prove both directions, literal
before/after values, found/not-found context, input identity changes, exact
edges, secret redaction, stdout/stderr/exit behavior, and unchanged
`rey.environment-diff.v1` JSON.

Plain `rey env log` now keeps each selected revision, authoritative evidence
coordinate, environment scope, mapped change counts, mapping, and message
visible without reopening provider bookkeeping. `log -p` derives the three
evidence planes from exact retained parent/commit snapshots, including
`EMPTY → ENV@1`; focused fixtures prove compact versus expanded output,
historical before/after values, found/not-found context, topology, redaction,
bounds, and unchanged `rey.environment-log.v1` JSON.

Focused fixtures additionally prove sensitive digest/value rejection,
non-sensitive value bounds and typed retention, found/not-found application
grouping, CLI redaction, read-only HTTP method behavior, SPA routing, and
byte-correct delivery of the embedded StyleX assets.

## Deferred

Interactive browser admission, commit/log controls, UI writes, automatic map
proposal acceptance, environment tick scheduling, executable version
invocation beyond admitted providers, and projecting exact environment objects
into `/explore` remain later slices.
