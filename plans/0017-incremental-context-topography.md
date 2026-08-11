# Plan 0017: Incremental Context Topography

- Status: Complete
- Decision: [ADR 0041](../docs/decisions/0041-continuous-coordinate-topography.md)
- Extends: [Plan 0011](0011-local-operator-ui.md) and [Plan 0014](0014-seed-discovery-and-locator-survey.md)
- Extended by: [Plan 0018](0018-world-context-navigation.md)

## Outcome

Prove one complete seed-to-map voyage: an agent-generated, admitted workload
locates project anchors from bounded `AGENTS.md` and README seeds, emits a
typed topography patch and directed delta through the workloads CLI, and makes
that same evidence navigable as anchor-shaped relief and stable points of
interest through a continuous far-out Explorer lens.

## Completion Checklist

- [x] Define a lossless provider-qualified coordinate binding that can carry
  opaque Spoke coordinates and explicitly local zero-Spoke bindings without
  conflating either with browser view state.
- [x] Implement canonical locator parse/format and typed resolution outcomes
  with exact provider, source, revision, capability, limit, and completeness
  evidence.
- [x] Define `rey.topography-patch.v1` anchors, classified edges, surveyed
  regions, coverage, frontier, omissions, lineage, and directed patch delta.
- [x] Create `context-anchor-survey` through `rey workloads create`, with an
  external coding harness generating its graph and frozen scenario suite.
- [x] Add fixture projects covering `AGENTS.md`, README variants, absolute and
  relative URI/reference candidates, duplicate anchors, malformed candidates,
  missing seeds, bounds, and deterministic replay.
- [x] Make `rey workloads test context-anchor-survey -v|-vv` render seed,
  locator, resolution, anchor, relationship, coverage, omission, and patch
  evidence at the established verbosity levels.
- [x] Make `rey workloads run` retain one exact patch result and make
  `list|status` surface its progress, topography revision, coverage, frontier,
  staleness, and next attention without reading implementation code.
- [x] Derive an Explorer topography read model from admitted patch artifacts;
  do not introduce UI-owned scans, graph facts, or assessment state.
- [x] Replace the fixed three-regime camera bound with a continuous scale and
  deterministic Atlas, Landscape, Neighborhood, Object, and Evidence layers
  over one persistent scene that retains focus and spatial identity through
  level-of-detail changes.
- [x] Derive nested terrain isolines from a bounded scalar field over admitted
  anchor prominence and exact classified edges; render anchors as stable POIs,
  preserve ridges and saddles across zoom, and disclose that current local
  relief is relational concentration rather than semantic similarity.
- [x] Render surveyed-empty, unexplored, omitted, stale, unsupported, and
  frontier regions distinctly without presenting interpolated terrain as
  evidence.
- [x] Hard-cut matrix routes to `/explore?coordinate=...&scale=...`, remove the
  legacy parser/route, and advance Journal v2 bindings to store semantic
  coordinate and numeric scale separately with no dual reader or migration.
- [x] Add focused CLI, structured-output, read-model, camera, route, and live UI
  tests plus one captured high-fidelity human verification path.
- [x] Exercise the coordinate carrier against a public Spoke contract, or
  preserve an explicit conformance gap without claiming connected semantics.

## Concrete Anchor

The first implementation slice is one deterministic fixture voyage, not a
generic scheduler:

```text
PWD
 └─ seed-name inventory: AGENTS.md + README variants
     └─ admitted context-anchor-survey workload
         └─ URI/reference locator candidates
             └─ typed resolution outcomes
                 └─ rey.topography-patch.v1
                     ├─ directed delta + frontier in rey workloads ...
                     └─ Atlas → Evidence projection in /explore
```

The acceptance path starts at the existing CLI:

```text
rey workloads create context-anchor-survey \
  --title "Survey project context anchors" \
  --intent "Mine bounded AGENTS.md and README seeds for exact URI and reference anchors"
rey workloads test context-anchor-survey -vv
rey workloads run context-anchor-survey --source AGENTS.md --source README.md
rey workloads status context-anchor-survey
rey ui
```

If the coding harness has not yet generated and admitted the package,
`test|run` must report that boundary rather than substituting runtime-owned
scenarios. Zooming the canvas must never advance the voyage.

## Current Proof

Captured on 2026-08-10:

```text
cargo test --workspace
# 172 locator, patch, runtime, workload, CLI, UI-listener, and related tests passed

pnpm --dir apps/rey-ui check
# formatting, typecheck, 51 read-model/route/component tests, and production build passed

rey workloads test context-anchor-survey --format table -vv
# required scenarios passed; bounded optional scenario remained explicitly inconclusive

rey workloads run context-anchor-survey --source AGENTS.md --source README.md
# one exact rey.topography-patch.v1 result retained with directed prior → target delta
```

A high-fidelity 1440×1000 static browser capture over the real retained patch
verified Atlas, Neighborhood, and Object renderings. Atlas projected 38 anchor
POIs through seven nested contour levels; Neighborhood retained the same relief
and added screen-sized classified relationships; Object filtered those edges
to the selected anchor and added bounded inspection cards. A dense two-patch
fixture separately proves a 3000×1000 world, 128 unique visible anchor POIs,
fourteen contour levels, and explicit folding beyond the 64-POI per-scene
bound.

The end-to-end CLI test also starts a real ephemeral `rey ui` listener and
proves that `GET /api/v1/workloads` returns the identical retained patch used by
Explorer. The local coordinate carrier round-trips opaque provider-owned Spoke
payloads, but no public Spoke coordinate contract is available here. That
connected-semantics conformance gap is explicit in `docs/LOCATORS.md`; no Spoke
resolution, durability, or federation claim is made.
