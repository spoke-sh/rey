# ADR 0004: Rust And Nix Development Foundation

- Status: Accepted
- Date: 2026-08-07

## Context

Rey will integrate closely with Spoke's Rust, Polars, and Arrow contracts and
needs deterministic comparison, bounded execution, and portable local tooling.
The repository needs a reproducible toolchain before a Cargo workspace is
scaffolded, while avoiding placeholder binaries or package outputs that imply
implementation exists.

## Decision

Rey's runtime is Rust-first. Nix pins the development toolchain and Just exposes
the root lifecycle.

The flake follows Spoke's development shape:

- `nixpkgs` provides tools and libraries;
- `rust-overlay` provides pinned stable Rust with Clippy, Rustfmt, and sources;
- `crane` will build filtered Cargo sources and reusable locked dependency
  artifacts once the workspace exists; and
- `flake-utils` generates outputs for the explicitly supported
  `x86_64-linux`, `aarch64-linux`, and `aarch64-darwin` systems. Current
  `nixpkgs` unstable no longer supports `x86_64-darwin`, so Rey does not claim
  that output.

The default shell adds rust-analyzer, cargo-nextest, Just, Git, curl, jq,
certificate roots, Alejandra, and mold on Linux. A smaller CI shell omits
rust-analyzer.

The root task surface is `setup`, `dev`, `check`, `test`, `build`, and `fmt`.
Tasks explicitly skip or reject runtime work while no Cargo workspace exists.
The flake exposes only development shell, wrapper, check, and formatter outputs
until an actual `rey` binary exists.

## Consequences

- `flake.lock` and the future `Cargo.lock` are reviewed dependency inputs.
- Documentation-only work can validate the same pinned environment that later
  compiles the runtime.
- There is no default package or app until implementation earns one.
- Intel macOS requires a separately supported package-set decision before it is
  advertised.
- Cold Nix evaluation and shell realization include the Rust/Polars-ready
  toolchain cost, while future Crane layers should preserve compiled dependency
  artifacts across application changes.
- Native dependencies require explicit Nix and portability proof.
