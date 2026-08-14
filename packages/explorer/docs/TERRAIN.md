# Terrain Pipeline

The accelerated terrain path turns admitted, bounded field grids into a
continuous-relief R3F scene. It never converts missing support, control
geometry, or application cards into terrain.

## Pipeline

```text
TerrainFieldSetInput[]
  → buildTerrainMeshData
  → verifyTerrainMeshParity
  → enforce GPU byte budget
  → CompiledContinuousRelief
  → ContinuousReliefScene
  → orthographic camera + TSL material + indexed relief meshes
```

## Field Contract

Each `TerrainFieldSetInput` supplies one bounded regular grid:

| Channel   | Representation                  | Use                                                 |
| --------- | ------------------------------- | --------------------------------------------------- |
| Grid      | Columns, rows, and local bounds | Maps cells into local scene coordinates.            |
| Validity  | `Uint8Array`                    | Determines whether a triangle has admitted support. |
| Elevation | `Float32Array` plus scale       | Becomes the mesh Y coordinate.                      |
| Normal    | `Float32Array` or `Int8Array`   | Drives lighting in Three.js coordinates.            |
| Curvature | `Float32Array`                  | Enhances ridges and valleys.                        |
| Tint      | RGB `Float32Array`              | Supplies base material color.                       |
| Occlusion | `Float32Array`                  | Darkens bounded valleys and ambient support.        |
| Roughness | `Float32Array`                  | Controls the TSL surface response.                  |

`@rey/agent` derives these fields from admitted survey terrain programs and
camera-relative patch requests. Field generation, hydrology, validity
classification, LOD selection, halo evaluation, and patch caching remain
outside this package.

## Validity-Safe Mesh Compilation

Grid X/Y becomes mesh X/Z; admitted elevation becomes mesh Y. Each grid quad
uses alternating diagonals. A triangle is emitted only when all three vertices
have valid support, so the GPU surface cannot bridge an invalid or unexplored
cell.

Compilation creates separate upload arrays for positions, normals, tint,
occlusion, roughness, curvature, and indices. Upload storage is disposable and
cannot mutate the authoritative CPU fields.

`verifyTerrainMeshParity` checks every uploaded field sample after the
coordinate transform and rejects any index touching invalid support. The
current parity identity is `rey.terrain.cpu-mesh-upload-parity@1`.

## Material And Lighting

`createContinuousReliefMaterial` produces a `MeshStandardNodeMaterial` with
TSL. It combines:

- source tint;
- world-space multidirectional hillshade;
- explicit occlusion;
- curvature-based ridge brightening and valley darkening; and
- bounded roughness.

`ContinuousReliefScene` shares the material across compiled meshes and adds
warm and cool directional lights plus ambient fill. The accelerated camera is
currently bounded, orthographic, and overhead. A unified bounded
County-isometric 3D camera remains engine direction, not current package fact.

## Bounds And Accounting

The compiler measures exact typed-array byte length before rendering and
rejects output above `MAX_ACCELERATED_TERRAIN_GPU_BYTES`, currently 64 MiB.
Statistics retain field-set, vertex, triangle, source-byte, GPU-byte, budget,
parity-sample, and geometry-compilation counts.

## Current Boundary

The package currently owns the accelerated base continuous-relief surface. It
does not yet upload or draw contours, water, weather, boundaries, roads,
structures, labels, selection, or evidence overlays. Those passes exist in the
application render graph and reference renderer.

Regional packets without a terrain program do not enter this pipeline. The
application renders their exact footprint, point locations, and native bounds
as sparse County vectors; it does not substitute cards or turn terrain-control
geometry into observed elevation.
