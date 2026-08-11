# Plan 0019: Emergent Context Features

- Status: In progress
- Decision: [ADR 0043](../docs/decisions/0043-emergent-natural-features-and-separate-paths.md)
- Corrects: [Plan 0018](0018-world-context-navigation.md)
- Extended by: [Plan 0020](0020-high-fidelity-projection-engine.md)

## Outcome

Make the survey shape a world field without drawing its source graph as world
geometry. Anchor samples create relief, unresolved conditions create weather,
and accumulated runoff may carve projected streams and rivers. Discovered and
constructed paths remain a separate future evidence family.

## Completion Checklist

- [x] Remove containment, reference, shared-coordinate, probe, and selected
  curation routes from the relief projection.
- [x] Remove exact seed-edge influence from anchor relief height.
- [x] Derive anchor station prominence from admitted seed and resolution
  samples rather than graph degree.
- [x] Project unresolved frontier conditions as local weather fronts without a
  line back to their source coordinate.
- [x] Derive deterministic rainfall, eight-neighbor downslope accumulation,
  stream and river thresholds, and runoff erosion over the anchor-only field.
- [x] Expose separate Relief, Water, Weather, and Probes visibility controls.
- [x] Keep exact source relationships in CLI and deep evidence inspection.
- [x] Make the CLI disclose atmospheric inputs, excluded edge provenance,
  hydrology projection limits, and the absence of a path claim.
- [x] Add focused natural-feature, no-edge-path, frontier-bearing, CLI,
  formatting, type, build, and workspace tests.
- [ ] Capture and inspect the World → Landscape natural-feature progression in
  a high-fidelity browser voyage.

## Verification

```text
$ just check
# formatting, TypeScript, 52 UI tests, production build, Clippy, and Nix evaluation passed

$ just test
# 52 UI tests, 172 Rust tests, and all documentation tests passed

$ cargo run -q -p rey -- workloads --workspace . status context-anchor-survey --format table
# emitted survey atmosphere, excluded edge provenance, hydrology/erosion limits,
# exact retained EDGE evidence, probe prerequisites, and no-path disclosure
```

The browser voyage must confirm that water responds to the relief field,
weather remains boundary-condition geometry, no source relationship appears as
a far-map line, and selecting a probe neither creates nor executes a path. The
host browser screenshot compositor limitation recorded by Plan 0018 remains,
so this visual voyage is not counted as completed proof.
