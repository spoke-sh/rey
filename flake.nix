{
  description = "Rey Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    crane,
    flake-utils,
    nixpkgs,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ] (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          "clippy"
          "rust-src"
          "rustfmt"
        ];
      };

      shellHook = ''
        export RUST_BACKTRACE="''${RUST_BACKTRACE:-1}"
        export CARGO_TARGET_DIR="''${REY_CARGO_TARGET_DIR:-''${XDG_CACHE_HOME:-$HOME/.cache}/cargo-target/rey}"
        export TMPDIR="''${REY_TMPDIR:-/var/tmp}"
        mkdir -p "$CARGO_TARGET_DIR" "$TMPDIR"
        ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
          export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-arg=-fuse-ld=mold"
          export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-arg=-fuse-ld=mold"
        ''}
      '';

      basePackages =
        [
          rustToolchain
          pkgs.cacert
          pkgs.curl
          pkgs.git
          pkgs.jq
          pkgs.just
        ]
        ++ pkgs.lib.optionals pkgs.stdenv.isLinux [pkgs.mold];

      developmentPackages =
        basePackages
        ++ [
          pkgs.alejandra
          pkgs.cargo-nextest
          pkgs.rust-analyzer
        ];

      reyDev = pkgs.writeShellApplication {
        name = "rey-dev";
        runtimeInputs =
          basePackages
          ++ [
            pkgs.alejandra
            pkgs.cargo-nextest
            pkgs.nix
          ];
        text = ''
          exec just "$@"
        '';
      };
    in {
      packages.dev = reyDev;

      apps.dev = {
        type = "app";
        program = "${reyDev}/bin/rey-dev";
        meta.description = "Run Rey development tasks through just";
      };

      checks.dev-wrapper = reyDev;

      devShells = {
        default = pkgs.mkShell {
          packages = developmentPackages;
          inherit shellHook;
        };

        ci = pkgs.mkShell {
          packages =
            basePackages
            ++ [
              pkgs.alejandra
              pkgs.cargo-nextest
            ];
          inherit shellHook;
        };
      };

      formatter = pkgs.alejandra;
    });
}
