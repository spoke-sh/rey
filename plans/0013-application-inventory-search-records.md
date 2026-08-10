# Plan 0013: Application Inventory And Search Records

- Status: Completed
- Decision: [ADR 0031](../docs/decisions/0031-desired-application-inventory-and-search-records.md)

## Outcome

Separate desired application intent from bounded environment search evidence
across the mapping DSL, `rey env` human interface, structured documents, and
`/environment`.

## Completion Checklist

- [x] Advance the mapping DSL to `rey.env-map.v3` and require a purpose for
  every desired executable.
- [x] Derive an exact identity over desired executable declarations and treat
  the target capability snapshot as the separate search record.
- [x] Render desired inventory before found, missing, errored, and removed
  search results in `env status`, `env diff`, and `env log -p`.
- [x] Render the same declaration/search boundary in `/environment`.
- [x] Remove Cargo from Rey's checked-in desired application inventory while
  retaining repository-development files as ordinary mapped inputs.
- [x] Advance affected structured schemas and add purpose/schema/interface
  fixtures.
- [x] Update foundational documentation and accepted decisions.

## Concrete Anchor

```text
rey env status

02 / BOUNDED SEARCH
DESIRED INVENTORY · 2 declared
  git · Inspect repository identity and activation inputs
  rg  · Extend bounded source mining with fast text search

SEARCH RECORD · WORKING @ <exact snapshot>
  bounded PATH identity resolution · no execution
```

The browser must present the same two records with the application-inventory
and working-snapshot identities visible before result groups.

## Current Proof

Captured on 2026-08-09:

```text
nix develop path:$PWD --command just check
# Prettier, TypeScript, 22/22 UI tests, Vite, Rustfmt, Clippy -D warnings,
# flake evaluation, and repository diff validation passed
nix develop path:$PWD --command just test
# 139/139 Rust tests, 22/22 UI tests, and every documentation test passed
nix develop path:$PWD --command just build
# production UI assets and the complete Rust workspace built
nix build path:$PWD#rey --no-link --print-out-paths
# /nix/store/2sqswhwb3vck88zmjawrxskyg1s3km42-rey
```

The focused inventory-identity fixture proves that found-to-missing search
drift changes observation evidence without changing the desired application
inventory id. CLI fixtures preserve stdout/stderr/JSON/exit behavior and expose
purpose, inventory identity, search snapshot identity, found/missing groups,
and the v3/v4/v2 schema cutovers. The embedded application test proves the same
records are present in the packaged browser surface.
