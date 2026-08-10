# Plan 0017: Incremental Context Topography

- Status: Active
- Decision: [ADR 0041](../docs/decisions/0041-continuous-coordinate-topography.md)
- Extends: [Plan 0011](0011-local-operator-ui.md) and [Plan 0014](0014-seed-discovery-and-locator-survey.md)

## Outcome

Prove one complete seed-to-map voyage: an agent-generated, admitted workload
locates project anchors from bounded `AGENTS.md` and README seeds, emits a
typed topography patch and directed delta through the workloads CLI, and makes
that same evidence navigable through a continuous far-out Explorer lens.

## Completion Checklist

- [ ] Define a lossless provider-qualified coordinate binding that can carry
  opaque Spoke coordinates and explicitly local zero-Spoke bindings without
  conflating either with browser view state.
- [ ] Implement canonical locator parse/format and typed resolution outcomes
  with exact provider, source, revision, capability, limit, and completeness
  evidence.
- [ ] Define `rey.topography-patch.v1` anchors, classified edges, surveyed
  regions, coverage, frontier, omissions, lineage, and directed patch delta.
- [ ] Create `context-anchor-survey` through `rey workloads create`, with an
  external coding harness generating its graph and frozen scenario suite.
- [ ] Add fixture projects covering `AGENTS.md`, README variants, absolute and
  relative URI/reference candidates, duplicate anchors, malformed candidates,
  missing seeds, bounds, and deterministic replay.
- [ ] Make `rey workloads test context-anchor-survey -v|-vv` render seed,
  locator, resolution, anchor, relationship, coverage, omission, and patch
  evidence at the established verbosity levels.
- [ ] Make `rey workloads run` retain one exact patch result and make
  `list|status` surface its progress, topography revision, coverage, frontier,
  staleness, and next attention without reading implementation code.
- [ ] Derive an Explorer topography read model from admitted patch artifacts;
  do not introduce UI-owned scans, graph facts, or assessment state.
- [ ] Replace the fixed three-regime camera bound with a continuous scale and
  deterministic Atlas, Landscape, Neighborhood, Object, and Evidence
  projections that retain focus through level-of-detail changes.
- [ ] Render surveyed-empty, unexplored, omitted, stale, unsupported, and
  frontier regions distinctly without presenting interpolated terrain as
  evidence.
- [x] Hard-cut matrix routes to `/explore?coordinate=...&scale=...`, remove the
  legacy parser/route, and advance Journal v2 bindings to store semantic
  coordinate and numeric scale separately with no dual reader or migration.
- [ ] Add focused CLI, structured-output, read-model, camera, route, and live UI
  tests plus one captured high-fidelity human verification path.
- [ ] Exercise the coordinate carrier against a public Spoke contract, or
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
