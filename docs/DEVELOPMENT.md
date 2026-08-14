# Development Environment

Nix defines Rey's development toolchain, the Cargo workspace defines Rust
dependencies and build metadata, the root pnpm workspace and Turborepo define
the JavaScript monorepo graph, and `just` provides the canonical cross-language
task surface. Crane builds the locked Rust dependency graph once and reuses it
for the binary and Nextest workspace tests. Its filtered source includes the
checked-in scene, workload, and topography resources embedded or opened by
those tests.

## Enter The Environment

```sh
nix develop
just setup
```

With direnv:

```sh
direnv allow
```

The checked-in `.envrc` delegates to the flake. Do not add project setup logic
to `.envrc`; that would create a second environment path.

## Flake Inputs

The flake pins four development inputs:

- `nixpkgs` for tools and libraries;
- `rust-overlay` for a pinned stable Rust toolchain;
- `crane` for filtered Cargo builds and reusable dependency artifacts; and
- `flake-utils` for supported system output generation.

`flake.lock` pins their complete dependency graph. The stable Rust selection is
resolved at lock-update time, so all users of the lock receive the same
compiler and components.

The flake declares `x86_64-linux`, `aarch64-linux`, and `aarch64-darwin`.
Current `nixpkgs` unstable has dropped `x86_64-darwin`, so Rey does not
advertise that output. Reintroducing Intel macOS requires a supported pinned
package set and explicit verification rather than an evaluation failure hidden
behind `eachDefaultSystem`.

The default shell contains:

- Rust compiler, Cargo, standard sources, Rustfmt, and Clippy;
- `rust-analyzer`;
- `cargo-dist`;
- `cargo-nextest`;
- Node.js 24 and pnpm for the embedded operator UI;
- `actionlint` for GitHub Actions workflow validation;
- `just`, Git, curl, jq, and certificate roots;
- `mold` on Linux; and
- Alejandra as the Nix formatter.

The CI shell omits `rust-analyzer` but keeps the compiler, formatter, Clippy,
Actionlint, test and distribution runners, Nix formatter, and basic
command-line tools.

## Cache And Temporary Directories

Both shells establish:

- `RUST_BACKTRACE=1` unless already selected;
- `CARGO_TARGET_DIR` at
  `${XDG_CACHE_HOME:-$HOME/.cache}/cargo-target/rey` by default; and
- `TMPDIR=/var/tmp` by default.

Set `REY_CARGO_TARGET_DIR` or `REY_TMPDIR` before entering the shell to override
those project-specific defaults. The shell does not repurpose `HOME` or infer a
private service data root. Linux shells select `mold` consistently for x86_64 and aarch64
GNU targets.

## Flake Outputs

```text
devShells.default       complete local Rust development shell
devShells.ci            smaller CI-oriented shell
packages.default/rey    locked `rey` binary built through Crane
packages.agent          immutable `@rey/agent` browser assets
packages.dev            self-contained Rust/Just/Nix wrapper for root tasks
apps.default/rey        `nix run . -- <rey arguments>`
apps.dev                `nix run .#dev -- <just arguments>`
checks.rey              proves the packaged binary
checks.agent            proves the browser application and Explorer package
checks.workspace-tests  runs locked offline Nextest workspace tests and doctests
checks.dev-wrapper      proves the development wrapper
formatter               Alejandra
```

The development wrapper includes Rust, Cargo, Just, Nix, Alejandra, Actionlint,
cargo-dist, nextest, and the base command-line tools in its runtime closure, so
`nix run .#dev -- setup` works without first entering `nix develop`. It
deliberately omits editor-only rust-analyzer.

## Canonical Tasks

```sh
just setup
just rey
just rey agent
just check
just test
just dist-check
just build
just fmt
```

Current behavior is:

- `setup` prints pinned Rust, Cargo, cargo-dist, cargo-nextest, and Just
  versions, fetches locked Cargo dependencies, installs the frozen root pnpm
  graph, and prints the repository-local Turbo version.
- `check` runs `git diff --check`, Turbo-scheduled TypeScript
  formatting/type/tests/build,
  GitHub Actions structure/expression linting, cargo-dist generation drift,
  Rustfmt, Clippy with warnings denied, and flake evaluation when Nix is
  available. Actionlint retains ShellCheck for the authored CI workflow and
  disables it only for cargo-dist's generated release shell fragments.
- `test` runs Turbo-scheduled UI tests, regenerates the embedded UI assets,
  requires cargo-nextest for all Rust workspace test binaries, and then uses
  Cargo for Rust documentation tests because Nextest does not execute doctests.
- `dist-check` verifies that cargo-dist's generated release workflow matches
  `dist-workspace.toml` and renders the complete release artifact plan without
  building or publishing it.
- `build` builds deterministic UI assets through Turbo before every workspace
  crate and feature so the Rust binary embeds the current application.
- `fmt` formats authored TypeScript/StyleX, root pnpm/Turbo policy, Rust, and
  `flake.nix`; installed Hifi packages remain immutable dependency artifacts.
- `rey` runs the `rey` binary through Cargo with Rust build progress suppressed
  so the terminal surface is Rey's output; compiler diagnostics and failures
  still reach stderr. When its first argument is `agent`, the recipe first runs
  the static UI build without suppressing TypeScript/Vite output, routes that
  build transcript to diagnostics so JSON stdout stays machine-clean, then
  starts the binary that embeds those freshly built assets.

## JavaScript Monorepo

The repository root owns the private `rey` package, the exact pnpm version,
the single `pnpm-lock.yaml`, and `turbo.json`. `pnpm-workspace.yaml` admits
`apps/*` and `packages/*`; `@rey/agent` is the browser application and
`@rey/explorer` is its reusable canvas/rendering package. Add future
applications or shared packages beneath those roots rather than creating
nested workspace files or lockfiles. The dependency is one-way:
`@rey/agent → @rey/explorer`; the canvas package cannot import the application.

Root `pnpm` scripts are the JavaScript task surface. Turbo schedules and caches
package `build`, `format:check`, `test`, and `typecheck` tasks, retains declared
`dist/**` build outputs in its disposable cache, and leaves mutating `format`
tasks uncached. Both `.turbo` and application `dist` directories are ignored;
generated bundles are never source inputs from version control. `just` remains
the repository-wide surface that composes this JavaScript graph with Cargo,
Nix, release, and documentation checks.

Nix packages the same graph explicitly. A fixed-output dependency derivation
materializes the root pnpm lock, an agent-assets derivation runs Turbo/Vite, and Crane
receives that immutable output through `REY_UI_DIST_DIR` before compiling the
Rust workspace. The cargo-dist release workflow performs its pinned Node/pnpm
setup and UI build before `dist build`. The Rust build script only validates
and embeds the supplied output; it does not install packages or access the
network. Outside those composed entry points, a direct Cargo build requires a
prior `pnpm run build` and reports that boundary if the assets are absent.

## Rust Conventions

- use the workspace edition and the flake-provided stable toolchain;
- commit `Cargo.lock` and use `--locked` in reproducible builds;
- prefer pure-Rust dependencies and Rustls-based clients where they meet the
  contract;
- keep `unsafe` isolated, justified with safety comments, and covered by focused
  tests;
- keep cancellation, backpressure, and allocation bounds explicit;
- avoid a catch-all core crate or composition binary;
- preserve one-way dependency flow toward core contracts; and
- use Polars features narrowly enough that Nix builds prove the intended
  closure rather than an accidental feature set.

The current data/runtime closure uses Polars 0.55.2 with only `fmt` and
`ipc_streaming`, Arrow IPC stream transport, BLAKE3 length-framed semantic
identity, Serde JSON documents, Clap, and the small `csv` encoder used by the
Tabular Diff 0.8 projection. The pure runtime, frontier, reasoning-surface,
workload, graph/scenario, qualification, and result-state contracts reuse that
closure. They do not add a general persistence engine, async runtime, or agent
transport.

`rey-mining` owns provider-neutral operation, request, result, artifact,
completeness, dependency, lineage, and limit contracts. It adds no third-party
dependency beyond the existing Serde, BLAKE3 identity, and error closure; it is
not a query engine, parser bundle, tool runner, visualization library, or
persistence layer.

The first source provider reuses the workspace's existing `base64` dependency
for reversible Unix-byte and Windows-UTF-16LE path identities and the existing
Polars/Arrow closure for the typed match relation. It adds no regex, parser,
process, traversal, storage, or rendering dependency. Deterministic pure
projections leave optional observed wall time absent from semantic consumption;
tool-backed probes may record it explicitly.

Ordered text and source-match relation deltas, typed source-match graph values,
portfolio mining, and `rey.workload-attention.v1` reuse the existing
Polars/DataFrame closure. The composition crate reuses bounded `serde-saphyr`
to load `rey.workload-package.v1` and JSON-compatible YAML creation requests.
The default catalog is workspace-authored; compiled graphs remain explicit
conformance. Local workload state stores runtime results rather than mutable
catalog projections, and drafts remain catalog state rather than execution
state.

New HTTP, async-runtime, Git parsing, parser, regex, database, or broader
Polars features require a concrete active-plan need and dependency review.

Current Rey-owned pre-alpha schemas fail closed at their declared versions.
Earlier `.rey` state must be discarded when a hard cut changes a schema; there
are no automatic aliases, migration readers, or partially populated decoders
unless an active plan explicitly accepts them. External protocol versions such
as Git porcelain v2 and renderer backend capability names are not Rey document
versions.

The composition binary uses `tiny_http` 0.12 for a narrow synchronous local
operator listener. Its bounded Journal POST, expected-snapshot Channel WORKING
POST, and exact workload-admission POST are intentionally unauthenticated and
have no origin check on every explicitly configured listener. This adds no
async runtime, TLS, authenticated identity, background scheduler, or general
persistence/service topology.
The TypeScript application uses locked React, React Three Fiber 9.7, TanStack
Router, TypeScript, Vite, Vitest, StyleX 0.19, and the official StyleX
unplugin. Authored UI rules
live only in `src/stylex/*.stylex.ts`; the build extracts one layered atomic
CSS asset while typed Kinetic material values remain runtime custom
properties. Exact MIT-licensed Hifi core/Kinetic Git packages are pinned to one
GitHub revision in `apps/agent/package.json` and the root `pnpm-lock.yaml`.
The pnpm workspace policy admits build scripts only for those exact two codeload
artifacts; no ambient sibling checkout or arbitrary dependency build is
trusted. Nix fetches the same revision and content hash, rebuilds only those two
packages inside the agent-assets derivation, and then runs the root Turbo build. Crane
passes the resulting immutable asset directory to the Rust build script for
`include_bytes!`; neither `apps/agent/dist` nor any other generated UI bundle
is checked in. The packaged Rey binary does not need Node at runtime.

`crates/rey/src/journal.rs` owns the shared typed entry validator, semantic
identity, idempotent ordered log, hard limits,
symlink checks, file lock, and atomic local publication. `rey journal add`
admits agent YAML; `POST /api/v1/journal` admits validated human JSON without
authentication on every explicit bind. Both retain beneath `.rey/journal` by
default and execute no notebook block. Exact entry routes and block fragments
make the retained document interface deeply hyperlinkable. The Journal format
is specified in `docs/JOURNAL.md`.

`src/topology.ts` retains shared scene types, orientation, high-level dispatch,
and admitted regional World/Atlas/County projection. Revisioned survey,
survey-layout, survey-terrain, and portfolio projection modules derive the
bounded Landscape, Neighborhood, Object, and Evidence scenes from admitted
patches and projection packets in `rey.workload-list.v1`; World additionally
compiles the optional `rey.semantic-atlas.v1` into a synthetic globe whose
React Three Fiber/Three.js and reference paths share one scene revision.
`src/explore/engine/camera.ts` owns camera math,
`src/explore/engine/scene.ts` freezes the current scene, and
`src/explore/engine/fields.ts` owns bounded typed scalar, vector, mask, and
material buffers. `src/explore/terrain` derives anchor elevation, validity,
finite-horizon hydrology, erosion, physical-scale normals, curvature, and
material inputs. Its compiler splits camera working sets into
absolute-coordinate patches with explicit hydrology/relief halos, proves
shared render-channel seams, and retains patch identities within packet-owned
cell and byte limits.
`apps/agent/src/explore/renderers/reference.tsx` owns the accessible SVG/DOM overlays and
fallback beneath the React canvas shell. Seed edges
remain deep inspection evidence and do not become relief, natural features, or
paths.
`src/explore/engine/render-graph.ts` owns the renderer-neutral ordered pass
manifest and its evidence/derived/presentation/interface authority labels.
`packages/explorer/src/renderer.ts` owns exact frame invalidation; both surfaces
consume the immutable snapshot graph, and the accelerated adapter leaves an
identical frame quiet. `packages/explorer/src/three-terrain.ts` measures every
uploaded position, normal, material attribute, curvature, and index byte before
allocation and fails visibly above its explicit 64 MiB GPU budget. It copies
upload arrays away from immutable CPU fields and verifies every sample under a
revisioned parity contract before React Three Fiber constructs the declared
Three.js geometry in `packages/explorer/src/fiber-scenes.tsx`.
The live diagnostics time CPU field evaluation, geometry construction, and
renderer submission with `performance.now()`. These values are explicitly
transient and unretained; renderer submission is not GPU completion.
`src/explore/engine/picking.ts` compiles the immutable semantic candidate index
and owns inverse scene selection across repeated Atlas chart copies.
`src/explore/projection/survey-terrain.ts` owns revisioned survey reference
field evaluation, contour extraction, and projected natural-feature
derivation; its compiler revision participates in immutable scene lineage.
The R3F/Three.js lifecycle adapter observes both WebGL context loss and WebGPU
`device.lost`; either returns the live surface to a degraded reference-renderer
status without changing scene assessment.
Topology-model tests prove semantic lens ordering, zoom bounds, identity
retention, and omission disclosure without requiring a browser graph library.
The embedded asset remains the HTTP proof for `/explore`, `/environment`, and
the root redirect.

The production UI build uses Vite 8's Rolldown output contract. Route surfaces
are loaded through dynamic imports, WebGPU uses Three.js's modular source graph,
and explicit React, React Three Fiber, router, and Three.js chunk groups keep
each emitted JavaScript object at or below 450 KiB. The build fails if that
bound is crossed
and emits `apps/agent/dist/bundle-report.json` beside the disposable bundle,
including each chunk's exact byte size, imports, and entry or dynamic-entry
role. Nix retains that report in the immutable agent-assets derivation used to build the
binary. The named production route-bundle workload reduced `app.js` from the
650.17 kB baseline to 233,819 bytes and removed the prior 1,032.25 kB
`three.webgpu.js` monolith. Rolldown preserves source execution order across
the bounded Three.js WebGPU/TSL chunks so cyclic node modules cannot evaluate a
subclass before its base class. The generated manifest embeds and serves every
emitted JavaScript and CSS object rather than assuming a fixed asset list.

Explorer is a high-fidelity spatial game engine for evidence-bound projection.
Its current boundary separates projection packets, revisioned scene
projections, immutable scenes, data-oriented fields, camera/LOD, render-pass
selection, backend lifecycle, and the application React shell. Remaining
direct-transport qualification is [Plan
0003](../plans/0003-scene-to-explorer.md) work. React Three Fiber `9.7.0`
declaratively reconciles the scene inside `@rey/explorer` over a pinned Three.js `WebGPURenderer` and
TSL adapter that prefers WebGPU and uses Three.js's WebGL2 backend as
compatibility fallback. The current package pins Three.js `0.185.1`; its narrow
lifecycle adapter has deterministic Vitest coverage
for asynchronous initialization, WebGPU selection, forced WebGL2 selection,
failure, device loss, frame submission, and disposal; engine tests separately
own viewport bounds. It is mounted lazily as `/explore`'s
continuous base-terrain surface, while the reference renderer remains active
through initialization and on failure. R3F test-renderer fixtures verify the
declared terrain and globe scene graph without treating it as semantic proof.
Vite binds R3F's exact `three` import to `@rey/explorer`'s modular WebGPU runtime bridge so
R3F props, declared objects, and the renderer share constructor and color
identities; the bridge exposes the source-graph `WebGLRenderer` only for R3F's
headless Vitest harness and compatibility contract. Regression assertions retain
the globe's warm light and stipple-material colors rather than accepting
Three.js defaults.
The TSL graph consumes typed tint,
occlusion, roughness, curvature, and normal attributes. Retained voyages now
cover both real backends and the reference renderer at both target viewports,
semantic-first rendered parity, WebGL context loss, and WebGPU device loss.
The fulfilled transport used in the current socket-restricted environment does
not qualify direct browser networking. A separately named voyage qualifies the
five-second passive publish path with one bounded generated attention stimulus;
the manifest identifies that row as a test input rather than observed runtime
activity. A named local SwiftShader budget now bounds observable CPU, resource,
interaction, and browser-presentation measurements across the full matrix; it
makes no GPU execution or frame-rate claim. Rey's deterministic reference
renderer remains independent of React Three Fiber and Three.js.

Backend-independent tests own semantic correctness: field values, validity
masks, scene manifests, stable ordering, LOD selection, render-pass order,
picking, limits, omissions, and exact evidence links. Browser capture owns
perceptual fidelity. GPU pixels and frame timing are not semantic identities;
performance results must name the fixture, browser/backend, viewport, DPR,
hardware, warm/cold posture, revisions, and budgets.

Run a retained Explorer browser voyage against an explicitly started CLI
surface with:

```sh
pnpm --filter @rey/agent qualify:explorer -- \
  --base-url http://127.0.0.1:5714 \
  --backend webgpu \
  --width 1920 \
  --height 1080
```

The harness uses an isolated Chrome/Chromium profile, traverses World → Atlas
→ County → Objects → Evidence through browser controls, and retains a typed
manifest plus exact PNG digests beneath ignored
`.rey/qualification/explorer`. Repeat the named voyage for `webgpu`, `webgl2`,
and `reference` at both required viewports. The manifest binds the admitted
region, atlas and workload inputs, machine/browser identity, requested and
observed backend, scene/focus/source/compiler lineage, render passes, bounded
GPU/source measurements, exact evidence links, console failures, omissions,
and capture identities. It is local qualification evidence, not semantic
evidence, GPU execution timing, a frame-rate claim, action authority, or proof
authority.

Direct browser HTTP is the default transport. A socket-restricted execution
environment may add `--transport fulfilled`; that posture runs the exact built
JS/CSS assets in Chrome and supplies only the bounded root-loader GET documents
read from the named `rey agent` origin through a local-file bootstrap. The
manifest discloses that it did not exercise direct browser networking. Such a
voyage remains useful for rendering and interaction qualification but cannot
close a direct-transport claim.

Add `--revalidation attention` to one fulfilled reference voyage to supply a
two-document workload sequence: the exact initial document followed by one
bounded generated attention row. After completing the semantic traversal, the
harness waits for the mounted UI's five-second passive refresh, verifies the
mailbox increment, preserves the exact Explorer scene, and retains a final
capture. The sequence tests publish behavior only; it is not observed runtime
activity or admitted evidence.

After retaining one bound trio, record structural and rendered parity with:

```sh
pnpm --filter @rey/agent qualify:explorer-parity -- \
  --reference .rey/qualification/explorer/<reference>/manifest.json \
  --webgl2 .rey/qualification/explorer/<webgl2>/manifest.json \
  --webgpu .rey/qualification/explorer/<webgpu>/manifest.json
```

This optional ImageMagick-backed check first requires exact scene snapshot,
focus, source revision, render graph/pass, and evidence-link equality. It then
retains per-stage normalized RMSE for all backend pairs and bounds WebGL2 ↔
WebGPU output at `0.02`. Reference and accelerated materials intentionally
differ, so their pixel measurements are retained without inventing a semantic
pass threshold.

Retain separately named loss voyages by adding `--loss webgl-context` to a
forced WebGL2 run and `--loss webgpu-device` to a WebGPU run. The former uses
the live context's `WEBGL_lose_context` extension; the latter destroys the
live qualification device and observes its real `device.lost` promise. Both
must retain a visibly degraded reference surface before continuing to exact
Evidence. The WebGPU destroy hook exists only on the renderer adapter and is
reachable from the canvas only through the explicit qualification event; it
does not change scene evidence or expose a production action path.

After retaining all six viewport/backend voyages with performance fields,
evaluate the versioned local ceiling with:

```sh
pnpm --filter @rey/agent qualify:explorer-performance -- \
  --machine-name rey-local-swiftshader \
  --manifest .rey/qualification/explorer/<voyage>/manifest.json \
  --manifest .rey/qualification/explorer/<voyage>/manifest.json
```

Supply one `--manifest` for every required backend and viewport. The aggregator
requires one exact machine/browser and transport posture, exact admitted input
and scene/source lineage within each viewport, required interactions, positive
accelerated draw/upload observations, and every ceiling in
`qualification/explorer-performance-budget.json`. It retains maximum scene,
field, geometry, and synchronous render-submission CPU duration; upload bytes;
backend-reported draw calls; label candidates; JavaScript heap; interaction
convergence; and `requestAnimationFrame` presentation cadence. The last two
timing families do not measure GPU completion or establish a frame-rate claim.

The mapping parser hard-cuts to `rey.env-map.v2`; the process-owned discovery
seed set is `HOME`, `PWD`, and `PATH`; a map is loaded only through explicit
`--map`; desired executables require a bounded purpose; and bounded
UTF-8 values are retained only for explicit
non-sensitive `capture: value` nodes. `crates/rey/src/env.rs` derives the
shared `rey.environment-operator-projection.v2` from the same frozen HEAD,
index, and working capability snapshots used by the authoritative deltas.
`GET|HEAD /api/v1/environment`, `rey env status`, and the React environment
workbench consume that common derivation. TypeScript projection tests and the
Rust HTTP/CLI tests are the interface proof; the browser never probes the host
independently.

`apps/agent/src/passive.ts` owns passive browser revalidation independently of
TanStack route lifecycle. Route loaders establish the initial typed document;
the mounted React projection publishes later successful reads in place, rejects
overlapping refreshes, retains the last good document after failure, and never
uses router invalidation as a polling mechanism. Focused fake-timer tests prove
the scheduling and failure behavior without a browser timing dependency.

`crates/rey/build.rs` binds the composition binary to its implementation Git
revision. A clean Nix build supplies `self.rev`; local Cargo builds resolve the
repository HEAD and register both HEAD and its symbolic ref as rebuild inputs.
Unavailable or non-hex revisions fail to `unknown`, in which case the UI does
not manufacture a GitHub commit link.

The next mining implementation should continue to prefer existing
Polars/Arrow, Serde, BLAKE3, and bounded-process infrastructure. A parser
framework, regex engine, tree/graph library, visualization library, async
runtime, database, or broader Polars feature set requires a concrete named
workload, fixture need, CLI verification surface, and dependency review.
The same rule applies to a browser rendering engine, shader toolchain, WebGL or
WebGPU abstraction, label-placement package, or GPU test harness.
Discovering or adapting the existing `rg` executable does not make it a
packaged runtime dependency unless that deployment contract is explicitly
accepted and tested.

## Cargo And Crane Outputs

The flake filters sources through `craneLib.cleanCargoSource`, compiles the
locked dependency graph with `buildDepsOnly`, reuses those artifacts for the
workspace package and tests, and exposes only the implemented `rey` binary.
Documentation edits do not invalidate Cargo dependency builds.

The workspace-test derivation explicitly supplies Bash, coreutils, and Git for
the bounded-process and repository fixtures. They are test inputs, not runtime
dependencies of the packaged `rey` binary; environment inspection discovers
available tools from the caller's configured search path.

## Updating Dependencies

For Rust dependencies:

1. change workspace or crate manifests;
2. update `Cargo.lock` intentionally;
3. run focused tests and `just check`;
4. include manifest and lock changes; and
5. update a decision when the dependency fixes a semantic or durable format
   choice.

For the Nix toolchain:

1. explain the version or tool need in the active plan;
2. change `flake.nix` if necessary;
3. update only intended lock inputs where practical;
4. run `nix flake check` and a command in both relevant shells; and
5. update this document when inputs, outputs, or cache policy change.

## Verification

For the current executable foundation, run:

```sh
nix develop path:$PWD#ci --command just check
nix develop path:$PWD#ci --command just test
nix develop path:$PWD#ci --command just build
nix flake check path:$PWD
nix run path:$PWD -- channels list --format table
nix run path:$PWD -- channels status --format table
nix run path:$PWD -- channels diff --format table
nix run path:$PWD -- channels apply path/to/channel-graph.yaml --format table
nix run path:$PWD -- channels add --format table
nix run path:$PWD -- channels diff --staged --format table
nix run path:$PWD -- channels commit -m "Admit channel topology" --format table
nix run path:$PWD -- channels log -n 3 -p --format table
nix run path:$PWD -- channels message add path/to/message.yaml --format table
nix run path:$PWD -- channels relay MESSAGE_ID --relay RELAY_ID --format table
nix run path:$PWD -- channels beacon BEACON_ID --format table
nix run path:$PWD -- channels poll GITHUB_APPLICATION_ID --format table
nix run path:$PWD -- env status
nix run path:$PWD -- env diff
nix run path:$PWD -- env add -p
nix run path:$PWD -- env add .
nix run path:$PWD -- env commit -m 'accept local toolchain'
nix run path:$PWD -- env log -n 3
nix run path:$PWD -- env log -p
nix run path:$PWD -- env status --format json
nix run path:$PWD -- workloads list --format table
nix run path:$PWD -- workloads test rey.fixture.text-normalize --format table -vv
nix run path:$PWD -- workloads test rey.portfolio.attention --format table -vv
nix run path:$PWD -- workloads run rey.portfolio.attention --format table
nix run path:$PWD -- ui
nix run path:$PWD -- ui --host 0.0.0.0 --port 5714
```
