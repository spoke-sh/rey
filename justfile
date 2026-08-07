set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

rust_workspace := `test -f Cargo.toml && printf present || true`

_default:
  @printf '%s\n' \
    'Rey development tasks:' \
    '  just setup          Verify the pinned toolchain' \
    '  just dev [args]     Run Rey once the runtime is scaffolded' \
    '  just check          Check docs, Rust when present, and flake evaluation' \
    '  just test           Run tests when the Cargo workspace is present' \
    '  just build          Build when the Cargo workspace is present' \
    '  just fmt            Format Rust and Nix sources'

setup:
  @rustc --version
  @cargo --version
  @just --version
  @if [ -n "{{rust_workspace}}" ]; then \
    cargo fetch --locked; \
  else \
    printf '%s\n' 'No Cargo workspace is scaffolded yet; dependency fetch skipped.'; \
  fi

dev *args:
  @if [ -f crates/rey/Cargo.toml ]; then \
    cargo run -p rey --bin rey -- "$@"; \
  else \
    printf '%s\n' 'The Rey runtime is not scaffolded yet; see plans/0001-foundation.md.' >&2; \
    exit 1; \
  fi

check:
  @git diff --check
  @if [ -n "{{rust_workspace}}" ]; then \
    cargo fmt --all -- --check; \
    cargo clippy --workspace --all-targets --all-features -- -D warnings; \
  else \
    printf '%s\n' 'No Cargo workspace is scaffolded yet; Rust checks skipped.'; \
  fi
  @nix flake check --no-build

test:
  @if [ -n "{{rust_workspace}}" ]; then \
    cargo nextest run --workspace --all-features; \
    cargo test --workspace --all-features --doc; \
  else \
    printf '%s\n' 'No Cargo workspace is scaffolded yet; tests skipped.'; \
  fi

build:
  @if [ -n "{{rust_workspace}}" ]; then \
    cargo build --workspace --all-features; \
  else \
    printf '%s\n' 'No Cargo workspace is scaffolded yet; build skipped.'; \
  fi

fmt:
  @if [ -n "{{rust_workspace}}" ]; then cargo fmt --all; fi
  @nix fmt -- flake.nix
