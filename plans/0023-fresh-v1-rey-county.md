# Plan 0023: Fresh V1 Rey County

- Status: Complete
- Decision: [ADR 0048](../docs/decisions/0048-fresh-v1-contract-baseline.md)
- Editor storage and status contract superseded by: [ADR
  0055](../docs/decisions/0055-editor-project-state-ownership.md)
- Extends: [Plan 0021](0021-read-first-scene-editor.md) and [Plan
  0022](0022-semantic-spherical-atlas.md)

## Outcome

Establish one destructive fresh v1 public-contract baseline and prove the
editor end to end by creating a bounded Rey County candidate from tools and
workloads observed and executed through Rey. Preserve the boundary between
authored native scene sources, immutable editor packages, admitted survey
evidence, and `/explore`.

## Completion Checklist

### 1. Establish the fresh v1 baseline

- [x] Reset every current Rey-owned public schema name and numeric schema
      version to v1/1.
- [x] Remove environment-history dual reading and value-rendering fallbacks.
- [x] Require timestamped complete v1 environment commits and reject other
      shapes.
- [x] Record the destructive cut in ADR 0048 and current foundational docs.
- [x] At this checkpoint, prove no Rey-owned `.v2` or later public schema
      literal remains. ADR 0055 later advances only `rey.editor-status` to v2
      for explicit absent WORKING state.

### 2. Rebuild evidence through Rey

- [x] Observe the process-owned environment with `rey env status`, then stage
      and commit the exact fresh v1 tool snapshot.
- [x] Qualify the then-current workspace workload set through the former
      pre-admission `rey workloads test` surface; ADR 0049 supersedes that
      lifecycle with exact INDEX qualification.
- [x] Run the context-anchor survey over an explicit bounded corpus and run
      the then-checked-in normalization proof over an explicit input. ADR 0049
      removes that proof from the product catalog.
- [x] Record exact tool, workload, graph, package, result, and topography
      identities used to author the scene.

### 3. Author native county sources

- [x] Create stable-ID OGC CRS84 GeoJSON sources for county boundary, terrain
      controls, hydrology, general features, and markers.
- [x] Make every named district and POI traceable to fresh Rey observations or
      workload results without presenting authored geometry as observed Earth
      geography or semantic distance.
- [x] Keep hydrology as an authored natural-feature candidate; do not encode
      seed edges as paths, roads, or discovered traversability.
- [x] Retain the native sources and a human-readable provenance/reproduction
      guide outside `.rey`.

### 4. Exercise the editor contract

- [x] At this checkpoint, retain the project and exact agent-authored native
      sources as one WORKING scene. ADR 0055 later moves the declaration out of
      the workspace and into `.rey/editor/project.json`; only the native source
      fixture remains checked.
- [x] Inspect WORKING, stage the exact snapshot, inspect the staged diff, and
      exercise commit-time validation through human CLI renderings.
- [x] Commit only INDEX and inspect the exact immutable candidate package
      through `rey editor log -p`.
- [x] Preserve candidate-only authority and document that `/explore` cannot
      consume the package before scene admission exists.

### 5. Qualification

- [x] Add or update focused tests for the hard cut and required v1 fields.
- [x] Run `just check` and `just test` and record exact evidence.
- [x] Verify the fresh environment, workload, and editor state through the
      high-fidelity human CLI without reading implementation files.

## Current Implementation Checkpoint

The fresh environment is `ENV@1` at
`blake3:6070a3c40b82ce885f912b522dfd9977d219412267a75dcf4de6b428bedce831`
over capability snapshot
`blake3:fc2e6648a63ba3df153b710b8616f0eff8602cd2f5fb2a41da0055a3549493d8`.
It retains 14 capabilities, including identity-probed Git 2.55.0 and ripgrep
15.2.0, seven found application runtimes/tools, one missing application, the
literal UTF-8 source miner, and Arrow frame interchange.

Both workspace workloads qualified. The retained context survey run
`blake3:73d579860d01e1d334204db433d875e8d730c1af2c259628786fec47f04c8369`
used five explicit sources and produced topography
`blake3:c1d64839c0d95eb4886ecfd24759806bf72bf65391afe0a09aba429b5db5427f`
with 56 anchors, 76 excluded source edges, one unsupported probe horizon, and
nine field channels. Label-normalization run
`blake3:70e6615ebcd02f121f506c5e0bb34389255a72d51019f5de846f28dc55922d7a`
produced `REY COUNTY`.

The editor snapshot
`blake3:168229cb1c1cabb30cfa1f4597873cf17a3307fcb673abe7e1413f1c9a5ce993`
contains five native sources, 34 stable-ID features, 12 markers, and 137
positions with complete coverage and no omissions. `SCENE@1`
`blake3:9c571b5e713d7ad3b347ffe10bba9cb244f633b6e1206cadb071d045c11a8099`
retains the message `establish Rey County`, candidate package
`blake3:3c683ad8675549b5dba753a8174d7382d865c2c3a7d313e969fbf079741c275d`,
and request
`blake3:4154e143175a8c9030164732825e485f3743252b4969e38ecb9bf15bd2b2c038`.
At this checkpoint, `rey editor status` was clean and `rey editor log -p`
resolved the exact commit, package, and parent-to-commit delta. The request
still reports
`requires_workload`, `admitted=false`, and `/explore` unchanged; that is the
intended authority boundary rather than an omitted implementation claim.

Repository-wide qualification on 2026-08-11:

- `just check` passed Git whitespace validation, Prettier, TypeScript, 75 UI
  tests across 24 files, the production Vite build, Rust formatting, workspace
  Clippy with warnings denied, and Nix flake evaluation on x86_64 Linux.
- `just test` passed the same 75 UI tests, 185 Rust tests across 14 binaries,
  and every workspace doc-test target with no failures or skips.
