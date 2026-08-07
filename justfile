set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

_default:
  @printf '%s\n' \
    'Rey development tasks:' \
    '  just setup          Verify the pinned toolchain' \
    '  just rey [args]     Run the Rey CLI' \
    '  just check          Check formatting, lints, and flake evaluation' \
    '  just test           Run workspace and documentation tests' \
    '  just build          Build the Rust workspace' \
    '  just fmt            Format Rust and Nix sources'

setup:
  @rustc --version
  @cargo --version
  @just --version
  @cargo fetch --locked

rey *args:
  @cargo run -p rey --bin rey -- "$@"

check:
  @git diff --check
  @cargo fmt --all -- --check
  @cargo clippy --workspace --all-targets --all-features -- -D warnings
  @if command -v nix >/dev/null 2>&1; then \
    nix flake check "path:$PWD" --no-build; \
  else \
    printf '%s\n' 'Nix is unavailable; flake evaluation skipped.'; \
  fi

test:
  @if command -v cargo-nextest >/dev/null 2>&1; then \
    cargo nextest run --workspace --all-features; \
  else \
    cargo test --workspace --all-features; \
  fi
  @cargo test --workspace --all-features --doc

build:
  @cargo build --workspace --all-features

fmt:
  @cargo fmt --all
  @if command -v nix >/dev/null 2>&1; then \
    nix fmt -- flake.nix; \
  else \
    printf '%s\n' 'Nix is unavailable; Nix formatting skipped.'; \
  fi
