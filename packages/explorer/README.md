# `@rey/explorer`

`@rey/explorer` is Rey's reusable accelerated rendering package. It owns the
React Three Fiber canvas, declarative globe and terrain scenes, pure GPU-data
compilers, bounded Three.js renderer lifecycle, and renderer diagnostics.

Start with the [Explorer product principles](../../docs/EXPLORER.md) for the
spatial journey, evidence rules, and fidelity standard. The documents here
describe the package as it exists today.

## Package At A Glance

```text
@rey/agent
  admitted evidence · semantic projection · fields · camera · picking · UI
                                  │ typed immutable inputs
                                  ▼
@rey/explorer
  pure compilers · declarative R3F scenes · bounded renderer · reports
                                  │
                                  ▼
                         WebGPU / WebGL2 pixels
```

The dependency is one-way: `@rey/agent → @rey/explorer`. This package does not
fetch evidence, assign semantic coordinates, choose semantic level of detail,
own application controls, or qualify what it renders.

## Technical Documentation

Read only the contract relevant to the change:

| Document                                      | Use it for                                                                                               |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| [Architecture](docs/ARCHITECTURE.md)          | Ownership, component map, immutable frame flow, invalidation, and render-graph boundary.                 |
| [Globe projection](docs/GLOBE.md)             | Globe compilation, shared sphere-to-Mercator projection, stipple, atmosphere, and attachment invariants. |
| [Terrain](docs/TERRAIN.md)                    | Field inputs, validity-safe mesh compilation, materials, parity, and GPU limits.                         |
| [Renderer](docs/RENDERER.md)                  | Backend selection, lifecycle, loss handling, viewport bounds, reporting, and fallback.                   |
| [Extending and qualifying](docs/EXTENDING.md) | Capability checklist, engine direction, Vitest coverage, and browser qualification.                      |

## Public API

The package root exports:

- canvas and renderer contracts such as `ExplorerCanvas`,
  `ExplorerCanvasContent`, `ExplorerCanvasReport`, `RendererPreference`, and
  `RendererStatus`;
- globe contracts such as `compileContextGlobe`, `CompiledContextGlobe`,
  `CONTEXT_GLOBE_PROJECTION_REVISION`, and polar/material revision identities;
- terrain contracts such as `compileContinuousRelief`,
  `buildTerrainMeshData`, `verifyTerrainMeshParity`,
  `createContinuousReliefMaterial`, and the GPU-budget contracts; and
- structural inputs including `ExplorerGlobe`, `GlobeCameraView`,
  `TerrainFieldSetInput`, and `TerrainCameraView`.

Pure globe projection and fabric primitives are also exported through
`@rey/explorer/globe-projection` and `@rey/explorer/globe-samples`. The
reference renderer can therefore share view-center and pattern contracts
without importing Three.js.

See [`src/index.ts`](src/index.ts) for the exact current export surface.

## Development

From the repository root:

```sh
pnpm --filter @rey/explorer test
pnpm --filter @rey/explorer typecheck
pnpm --filter @rey/explorer build
pnpm check
```

The package uses Vitest for compiler, projection, scene, lifecycle, loss, and
resource-bound contracts. Real browser and fallback behavior is qualified by
the `@rey/agent` Explorer voyage. See
[Extending and qualifying](docs/EXTENDING.md) for the complete proof boundary.

## Related Documents

- [Explorer product principles](../../docs/EXPLORER.md)
- [Repository architecture](../../docs/ARCHITECTURE.md)
- [Mining and visualization](../../docs/MINING.md)
- [Interfaces](../../docs/INTERFACES.md)
- [Plan 0003](../../plans/0003-scene-to-explorer.md)
