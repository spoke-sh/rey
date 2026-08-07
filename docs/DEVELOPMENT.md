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

- `setup` prints pinned Rust, Cargo, and Just versions and fetches the locked
  dependencies.
- `check` runs `git diff --check`, Rustfmt, Clippy with warnings denied, and
  flake evaluation when Nix is available.
- `test` runs nextest when available, falls back to Cargo's test runner, and
  always runs Rust documentation tests.
- `build` builds every workspace crate and feature.
- `fmt` formats Rust and formats `flake.nix` when Nix is available.
- `rey` runs the `rey` binary through Cargo.

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
and Clap for the first CLI. New HTTP, async-runtime, Git parsing, or broader
Polars features require an explicit plan need and dependency review.

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
nix run path:$PWD -- environment inspect --format json
```
