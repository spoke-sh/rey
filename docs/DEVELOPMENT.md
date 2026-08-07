# Development Environment

Nix defines Rey's development toolchain. The future Cargo workspace will define
Rust dependencies and build metadata. `just` provides a small root task surface
that is honest about the current documentation-and-toolchain-only state.

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
- `crane` for future filtered Cargo builds and reusable dependency artifacts;
  and
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

## Current Flake Outputs

```text
devShells.default   complete local Rust development shell
devShells.ci        smaller CI-oriented shell
packages.dev        self-contained Rust/Just/Nix wrapper for root tasks
apps.dev            `nix run .#dev -- <just arguments>`
checks.dev-wrapper  proves the development wrapper evaluates and builds
formatter           Alejandra
```

There is intentionally no default `rey` package or app before a Cargo binary
exists. Plan 0001 will add Crane dependency, package, test, and binary outputs
with the same pinned toolchain.

The development wrapper includes Rust, Cargo, Just, Nix, Alejandra, nextest,
and the base command-line tools in its runtime closure, so `nix run .#dev --
setup` works without first entering `nix develop`. It deliberately omits
editor-only rust-analyzer.

## Canonical Tasks

```sh
just setup
just dev
just check
just test
just build
just fmt
```

Current behavior is:

- `setup` prints pinned Rust, Cargo, and Just versions; it fetches locked Cargo
  dependencies after a workspace exists.
- `check` runs `git diff --check`, Rustfmt and Clippy when a Cargo workspace is
  present, and `nix flake check --no-build`.
- `test` runs nextest and Rust doc tests when a workspace is present.
- `build` builds the Cargo workspace when present.
- `fmt` formats Rust when present and always formats `flake.nix` with the Nix
  formatter.
- `dev` fails with a direct explanation until `crates/rey` exists.

Skipping a nonexistent Cargo workspace is explicit output, not a successful
runtime or test claim.

## Rust Conventions

Once scaffolded:

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

Selecting concrete Polars, Arrow, HTTP client, hashing, serialization, or CLI
dependencies belongs in the active plan and relevant ADR before broad use.

## Adding Cargo And Crane Outputs

When the workspace is scaffolded:

1. filter sources through `craneLib.cleanCargoSource`;
2. compile the locked dependency graph with `buildDepsOnly`;
3. reuse those artifacts for workspace builds and tests;
4. expose the `rey` binary through named package and app outputs;
5. keep documentation edits from invalidating binary dependency builds; and
6. make `nix flake check` execute real offline workspace tests.

Do not add a placeholder default package that implies the runtime exists.

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

For the current foundation, run:

```sh
nix flake check --no-build
nix flake check --all-systems --no-build
nix develop .#ci --command just check
nix develop --command just setup
```

After packages exist, `nix flake check` must build and test them rather than
only evaluate the shell and wrapper.
