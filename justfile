set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

_default:
  @printf '%s\n' \
    'Rey development tasks:' \
    '  just setup          Verify the pinned toolchain' \
    '  just rey [args]     Run the Rey CLI' \
    '  just check          Check UI/Rust formatting, tests, lints, and flake evaluation' \
    '  just test           Run UI, workspace, and documentation tests' \
    '  just build          Build the UI and Rust workspace' \
    '  just fmt            Format UI, Rust, and Nix sources'

setup:
  @rustc --version
  @cargo --version
  @just --version
  @cargo fetch --locked
  @pnpm --dir apps/rey-ui install --frozen-lockfile

rey *args:
  @cargo run --quiet -p rey --bin rey -- "$@"

check:
  @git diff --check
  @pnpm --dir apps/rey-ui run check
  @cargo fmt --all -- --check
  @cargo clippy --workspace --all-targets --all-features -- -D warnings
  @if command -v nix >/dev/null 2>&1; then \
    nix flake check "path:$PWD" --no-build; \
  else \
    printf '%s\n' 'Nix is unavailable; flake evaluation skipped.'; \
  fi

test:
  @pnpm --dir apps/rey-ui run test
  @if command -v cargo-nextest >/dev/null 2>&1; then \
    cargo nextest run --workspace --all-features; \
  else \
    cargo test --workspace --all-features; \
  fi
  @cargo test --workspace --all-features --doc

build:
  @pnpm --dir apps/rey-ui run build
  @cargo build --workspace --all-features

fmt:
  @pnpm --dir apps/rey-ui run format
  @cargo fmt --all
  @if command -v nix >/dev/null 2>&1; then \
    nix fmt -- flake.nix; \
  else \
    printf '%s\n' 'Nix is unavailable; Nix formatting skipped.'; \
  fi
