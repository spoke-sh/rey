# Development Environment

Nix defines Rey's development toolchain, the Cargo workspace defines Rust
dependencies and build metadata, and `just` provides the canonical root task
surface. Crane builds the locked dependency graph once and reuses it for the
binary and workspace tests.

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

The flake follows Spoke's Rust environment shape and pins four inputs:

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
- `cargo-nextest`;
- Node.js 24 and pnpm for the embedded operator UI;
- `just`, Git, curl, jq, and certificate roots;
- `mold` on Linux; and
- Alejandra as the Nix formatter.

The CI shell omits `rust-analyzer` but keeps the compiler, formatter, linter,
test runner, Nix formatter, and basic command-line tools.

## Cache And Temporary Directories

Both shells establish:

- `RUST_BACKTRACE=1` unless already selected;
- `CARGO_TARGET_DIR` at
  `${XDG_CACHE_HOME:-$HOME/.cache}/cargo-target/rey` by default; and
- `TMPDIR=/var/tmp` by default.

Set `REY_CARGO_TARGET_DIR` or `REY_TMPDIR` before entering the shell to override
those project-specific defaults. The shell does not repurpose `HOME` or infer a
Spoke data root. Linux shells select `mold` consistently for x86_64 and aarch64
GNU targets.

## Flake Outputs

```text
devShells.default       complete local Rust development shell
devShells.ci            smaller CI-oriented shell
packages.default/rey    locked `rey` binary built through Crane
packages.dev            self-contained Rust/Just/Nix wrapper for root tasks
apps.default/rey        `nix run . -- <rey arguments>`
apps.dev                `nix run .#dev -- <just arguments>`
checks.rey              proves the packaged binary
checks.workspace-tests  runs locked offline workspace tests
checks.dev-wrapper      proves the development wrapper
formatter               Alejandra
```

The development wrapper includes Rust, Cargo, Just, Nix, Alejandra, nextest,
and the base command-line tools in its runtime closure, so `nix run .#dev --
setup` works without first entering `nix develop`. It deliberately omits
editor-only rust-analyzer.

## Canonical Tasks

```sh
just setup
just rey
just check
just test
just build
just fmt
```

Current behavior is:

- `setup` prints pinned Rust, Cargo, and Just versions, fetches locked Cargo
  dependencies, and installs the frozen pnpm graph.
- `check` runs `git diff --check`, TypeScript formatting/type/tests/build,
  Rustfmt, Clippy with warnings denied, and flake evaluation when Nix is
  available.
- `test` runs UI tests, nextest when available, falls back to Cargo's test
  runner, and always runs Rust documentation tests.
- `build` builds deterministic UI assets before every workspace crate and
  feature so the Rust binary embeds the current application.
- `fmt` formats authored TypeScript/StyleX, pnpm workspace policy, Rust, and
  `flake.nix`; installed Hifi packages remain immutable dependency artifacts.
- `rey` runs the `rey` binary through Cargo with build progress suppressed so
  the terminal surface is Rey's output; compiler diagnostics and failures still
  reach stderr.

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

ADR 0008 selects Polars 0.55.2 with only `fmt` and `ipc_streaming`, Arrow IPC
stream transport, BLAKE3 length-framed semantic identity, Serde JSON documents,
and Clap for the first CLI. ADR 0010 adds the small `csv` encoder for the
Tabular Diff 0.8 projection. ADR 0013 adds no external dependency: its pure
runtime reducer uses existing identity/Serde contracts, and its reasoning
surface uses the existing Polars/DataFrame closure. ADR 0014 adds
`rey-frontier` using that same closure and no new external dependency. New
HTTP, async-runtime, Git parsing, or broader Polars features require an
explicit plan need and dependency review.

ADR 0015 adds no dependency or executable behavior. It accepts the
workload-centered product and command contracts while deferring manifest
encoding, catalog/result storage, graph execution, scenario campaigns, and the
required runtime/frontier/surface schema cutover to a later implementation
slice.

ADR 0016 implements that first slice with no new third-party dependency. The
existing Rey crates now provide built-in typed UTF-8 graph operations,
scenario-output deltas, workload qualification/run results, and bounded local
JSON result state. The workload identity cutover advances frontier, progress,
and scheduling to v2 and reasoning surfaces to v3; it does not add a general
manifest parser, persistence engine, async runtime, or agent transport.

ADR 0017 accepts the relational/source mining capability model. Plan 0006 now
implements the narrow `rey-mining` ownership boundary for provider-neutral
operation, request, result, artifact, completeness, dependency, lineage, and
limit contracts. The crate adds no third-party dependency beyond the existing
Serde, BLAKE3 identity, and error closure; it is not a query engine, parser
bundle, tool runner, visualization library, or persistence layer.

The first source provider reuses the workspace's existing `base64` dependency
for reversible Unix-byte and Windows-UTF-16LE path identities and the existing
Polars/Arrow closure for the typed match relation. It adds no regex, parser,
process, traversal, storage, or rendering dependency. Deterministic pure
projections leave optional observed wall time absent from semantic consumption;
tool-backed probes may record it explicitly. That semantic correction is the
pre-alpha `rey.mining-result.v2` hard cut; operation and request remain v1 and
no compatibility alias silently relabels the earlier result document.

ADR 0018 completes the first mining workload without a new third-party
dependency. It adds ordered text and source-match relation deltas, extends the
runtime graph with a typed source-match value and built-in source operations,
and advances workload, graph, scenario, result, qualification, run, local
state, list/status/batch, and scenario-output schemas to v2. The hard cut has no
v1 decoder because pre-alpha retained workload state is local, bounded, and
must never be silently reinterpreted under new evidence semantics.

ADR 0022 and Plan 0010 add portfolio mining without expanding the workspace's
third-party dependency closure. `rey-runtime` now directly uses the existing
Polars and `rey-dataframe` workspace dependencies for the canonical
`rey.workload-attention.v1` relation. It adds typed portfolio snapshot and
attention values, two deterministic built-in graph operations, one compiled
system workload, and additive retained attention evidence in scenario/run
results. Existing v2 workload documents remain replay-compatible when the
additive attention collection is empty.
ADR 0022 advanced the public list, status, and status-batch envelopes to v3 because they
now require a portfolio-attention document and add per-workload attention
counts; the local state and test/run schemas remain v2.

ADR 0023 reuses the workspace's existing bounded `serde-saphyr` dependency in
the `rey` composition crate to load `rey.workload-package.v1`. The default
catalog is now workspace-authored; compiled graphs remain explicit
conformance. Exact package bytes/path and proposal provenance advance the
workload definition to v3. Catalog/provenance fields advance list and status
envelopes to v4 and the test batch to v3; `rey.workload-run-view.v1` wraps the
unchanged verified v2 run result. Local workload state remains v2 because it
stores runtime results, not mutable catalog projections.

ADR 0024 adds no dependency. `workloads create` serializes its strict request
as JSON-compatible YAML with existing Serde support and loads it through the
same bounded YAML parser as packages. The request and create-result documents
are v1. Draft-aware catalog descriptors advance to v2; list/status advance to
v5, test batch to v4, and run view to v2. Runtime results and local retained
state remain unchanged because a draft is catalog state, not execution state.

ADR 0025 adds `tiny_http` 0.12 to the composition binary for a narrow
synchronous local operator listener. ADR 0037 adds one bounded Journal POST;
ADR 0038 admits that document without authentication or an origin check on
every explicitly configured listener. It deliberately adds no async runtime,
TLS, authenticated identity, background scheduler, or general
persistence/service topology.
The TypeScript application uses locked React, TanStack Router, TypeScript,
Vite, Vitest, StyleX 0.19, and the official StyleX unplugin. Authored UI rules
live only in `src/stylex/*.stylex.ts`; the build extracts one layered atomic
CSS asset while typed Kinetic material values remain runtime custom
properties. Exact MIT-licensed Hifi core/Kinetic Git packages are pinned to one
GitHub revision in `apps/rey-ui/package.json` and `pnpm-lock.yaml`. The pnpm
workspace policy admits build scripts only for those exact two codeload
artifacts; no ambient sibling checkout or arbitrary dependency build is
trusted. Crane's filtered source includes built `apps/rey-ui/dist` assets
consumed by Rust `include_bytes!` calls; the packaged Rey binary does not need
Node at runtime.

ADR 0037 adds no dependency. `crates/rey/src/journal.rs` owns the shared typed
entry validator, semantic identity, idempotent ordered log, hard limits,
symlink checks, file lock, and atomic local publication. `rey journal add`
admits agent YAML; `POST /api/v1/journal` admits validated human JSON without
authentication on every explicit bind. Both retain beneath `.rey/journal` by
default and execute no notebook block. Exact entry routes and block fragments
make the retained document interface deeply hyperlinkable. The Journal format
is specified in `docs/JOURNAL.md`.

ADR 0026 adds no runtime dependency. `src/topology.ts` deterministically
derives bounded World, Atlas, Landscape, Neighborhood, Object, and Evidence
scenes from admitted patches and projection packets in `rey.workload-list.v7`;
`src/explore/engine/camera.ts` owns the extracted camera math,
`src/explore/engine/scene.ts` freezes the current reference scene, and
`src/explore/renderers/reference.tsx` owns the SVG/DOM passes beneath the React
canvas shell. Anchor-only fields and remaining scene assembly still live in
`src/topology.ts`. Seed edges remain deep inspection evidence and do not become
relief, natural features, or paths.
Topology-model tests prove semantic lens ordering, zoom bounds, identity
retention, and omission disclosure without requiring a browser graph library.
The embedded asset remains the HTTP proof for `/explore`, `/environment`, and
the root redirect.

ADR 0044 formalizes Explorer as a high-fidelity spatial game engine for
evidence-bound projection and adds no graphics dependency by itself. Plan 0020
first extracts typed evidence adapters, projection packets, immutable scenes,
data-oriented fields, camera/LOD/invalidation, render-graph, picking, backend,
and React-shell
boundaries while retaining the existing SVG/DOM output as a reference path.
ADR 0045 selects a pinned Three.js `WebGPURenderer` and TSL adapter with WebGPU
preferred and Three.js's WebGL2 backend as compatibility fallback. The current
package pins Three.js `0.185.1`; its adapter has deterministic lifecycle tests
for asynchronous initialization, WebGPU selection, forced WebGL2 selection,
viewport bounds, failure, and disposal, but is not yet mounted as the live
terrain surface. Its remaining bounded
qualification must cover browser and device support, bundle and Nix closure
size, asynchronous initialization, graphics-resource ownership and disposal,
device/context loss, both backends, determinism, accessibility, security policy,
licensing, maintenance, and named performance evidence. Rey's deterministic
reference renderer remains independent of Three.js.

Backend-independent tests own semantic correctness: field values, validity
masks, scene manifests, stable ordering, LOD selection, render-pass order,
picking, limits, omissions, and exact evidence links. Browser capture owns
perceptual fidelity. GPU pixels and frame timing are not semantic identities;
performance results must name the fixture, browser/backend, viewport, DPR,
hardware, warm/cold posture, revisions, and budgets.

ADRs 0027, 0031, and 0032 add no dependency. The mapping parser hard-cuts to
`rey.env-map.v3`; the process-owned discovery seed set is `HOME`, `PWD`, and
`PATH`; a map is loaded only through explicit `--map`; desired executables
require a bounded purpose; and bounded
UTF-8 values are retained only for explicit
non-sensitive `capture: value` nodes. `crates/rey/src/env.rs` derives the
shared `rey.environment-operator-projection.v3` from the same frozen HEAD,
index, and working capability snapshots used by the authoritative deltas.
`GET|HEAD /api/v1/environment`, `rey env status`, and the React environment
workbench consume that common derivation. TypeScript projection tests and the
Rust HTTP/CLI tests are the interface proof; the browser never probes the host
independently.

`apps/rey-ui/src/passive.ts` owns passive browser revalidation independently of
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
nix run path:$PWD -- env status
nix run path:$PWD -- env diff
nix run path:$PWD -- env add -p
nix run path:$PWD -- env add
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
