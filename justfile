set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

_default:
  @printf '%s\n' \
    'Rey development tasks:' \
    '  just setup          Verify the pinned toolchain' \
    '  just rey [args]     Run the Rey CLI' \
    '  just check          Check UI/Rust formatting, tests, lints, and flake evaluation' \
    '  just test           Run UI, Nextest workspace, and documentation tests' \
    '  just dist-check     Verify the release workflow and artifact plan' \
    '  just build          Build the UI and Rust workspace' \
    '  just fmt            Format UI, Rust, and Nix sources'

setup:
  @rustc --version
  @cargo --version
  @dist --version
  @cargo nextest --version
  @just --version
  @cargo fetch --locked
  @pnpm install --frozen-lockfile
  @pnpm exec turbo --version

rey *args:
  @if [[ "${1:-}" == "agent" ]]; then pnpm run build >&2; fi
  @cargo run --quiet -p rey --bin rey -- "$@"

check:
  @git diff --check
  @actionlint .github/workflows/ci.yml
  @actionlint -shellcheck= .github/workflows/release.yml
  @pnpm run check
  @cargo fmt --all -- --check
  @cargo clippy --workspace --all-targets --all-features -- -D warnings
  @dist generate --check
  @if command -v nix >/dev/null 2>&1; then \
    nix flake check "path:$PWD" --no-build; \
  else \
    printf '%s\n' 'Nix is unavailable; flake evaluation skipped.'; \
  fi

test:
  @pnpm run test
  @pnpm run build
  @cargo nextest run --workspace --all-features
  @cargo test --workspace --all-features --doc

dist-check:
  @dist generate --check
  @dist plan

build:
  @pnpm run build
  @cargo build --workspace --all-features

fmt:
  @pnpm run format
  @cargo fmt --all
  @if command -v nix >/dev/null 2>&1; then \
    nix fmt -- flake.nix; \
  else \
    printf '%s\n' 'Nix is unavailable; Nix formatting skipped.'; \
  fi
