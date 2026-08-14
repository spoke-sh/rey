# Renderer Lifecycle

`ReactThreeFiberRendererAdapter` places Three.js's asynchronous
`WebGPURenderer` behind a bounded lifecycle and immutable status surface.

## Backend Selection

| Preference  | Behavior                                                                                      |
| ----------- | --------------------------------------------------------------------------------------------- |
| `auto`      | Prefer WebGPU; accept the Three.js WebGL2 compatibility backend with visible degraded status. |
| `webgpu`    | Require WebGPU; fall back to reference status if Three.js selects another backend.            |
| `webgl2`    | Force the WebGL2 backend, primarily for compatibility qualification.                          |
| `reference` | Skip accelerated initialization and report the application reference renderer.                |

Lifecycle states are `idle`, `initializing`, `ready`, `failed`, and
`disposed`. Status snapshots are immutable and identify the selected backend
and renderer revision.

## Loss And Fallback

WebGPU device loss disposes the renderer and reports degraded reference
status. WebGL context loss unmounts the R3F root and does the same.

The package does not draw fallback pixels. `@rey/agent` retains its
deterministic reference surface beneath the accelerated canvas, reveals
acceleration only after a valid submitted frame, and reveals the reference
surface again after loss.

This separation means backend failure changes visible fidelity, not semantic
assessment or scene identity.

## Viewport Bounds

`boundedViewport` prevents accidental unbounded pixel work:

| Limit                      |   Default |
| -------------------------- | --------: |
| Maximum device pixel ratio |         2 |
| Maximum logical dimension  |   2048 px |
| Maximum physical pixels    | 8,388,608 |

It preserves aspect ratio while reducing width and height until all limits are
satisfied. Internal callers may supply stricter bounds.

## Reporting Semantics

`ExplorerCanvasReport` exposes lifecycle, selected backend, renderer revision,
draw submission, and bounded resource observations to the application.

Submission timing measures only the synchronous CPU call boundary. Draw-call
counts come from Three.js renderer information. Neither value is GPU execution
time or a frame-rate measurement, and neither participates in immutable frame
identity.

See [Architecture](ARCHITECTURE.md) for frame invalidation and
[Extending and qualifying](EXTENDING.md) for lifecycle proof requirements.
