# Plan 0018: World Context Navigation

- Status: Superseded by [Plan 0019](0019-emergent-context-features.md)
- Decision: [ADR 0042](../docs/decisions/0042-world-geometry-and-probe-navigation.md)
- Extends: [Plan 0017](0017-incremental-context-topography.md)

Plan 0019 removes this slice's literal edge corridors and curation-path
projection. The checklist below records the historical implementation that led
to that correction; it is not the current map contract.

## Outcome

Make the admitted survey feel like one chart within a larger context world.
Zooming out reveals charted geometry, exact transport structure, and honest
probe horizons; zooming in reveals prospecting signals and an exact curation
path without executing or recommending mining from the canvas.

## Completion Checklist

- [x] Add World before Atlas and extend exact Explorer and Journal scales to
  `0.05..=5.4` with a canonical `0.1` World stop.
- [x] Derive charted-land and probe-horizon envelopes only from displayed
  admitted anchors and retained frontier rows.
- [x] Render containment roads, directed reference flows, shared-coordinate
  passages, and unresolved probe trails as distinct labeled corridor classes.
- [x] Keep anchor, corridor, and frontier coordinates stable across all six
  levels of detail.
- [x] Classify frontier prerequisites without turning connectivity into a
  mining recommendation or read authority.
- [x] Derive a bounded directed path from the survey origin to a selected
  anchor or frontier, separating exact route steps from probe crossings.
- [x] Add map-style Relief, Routes, and Probes visibility controls plus a
  bearing, legend, and level-of-detail scale.
- [x] Keep every canvas gesture read-only and disclose that selection does not
  reshape relief; only changed admitted evidence can do so.
- [x] Add equivalent world, transport, mining-bearing, and probe-prerequisite
  evidence to the verbose workloads CLI.
- [x] Add focused lens, geometry, route, probe, deep-link, Journal-bound, CLI,
  formatting, type, production-build, and workspace tests.
- [ ] Capture and inspect a high-fidelity World → Neighborhood browser voyage
  over a retained real-project patch.

## Verification

```text
$ just check
# formatting, TypeScript, 52 UI tests, production build, Clippy, and Nix evaluation passed

$ just test
# 52 UI tests, 172 Rust tests, and all documentation tests passed

$ cargo run -q -p rey -- workloads --workspace . status context-anchor-survey --format table
# emitted admitted world geometry, classified transport corridors, mining bearing,
# and prerequisite-bearing PROBE rows over the retained repository survey
```

The browser voyage must show that a probe trail stays visually distinct from
an exact route and that selecting it reports the prerequisite without running
a locator or changing the retained patch revision. Isolated Chrome and Firefox
capture attempts on 2026-08-10 both reached the local `rey ui` process but their
host screenshot compositors did not return an image. This is retained as an
open human-verification boundary rather than counted as feature proof.
