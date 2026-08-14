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
    self,
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

      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
      agentSource = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
          ./package.json
          ./pnpm-lock.yaml
          ./pnpm-workspace.yaml
          ./turbo.json
          ./apps/agent/index.html
          ./apps/agent/package.json
          ./apps/agent/src
          ./apps/agent/tsconfig.json
          ./apps/agent/vite.config.ts
          ./packages/explorer/package.json
          ./packages/explorer/src
          ./packages/explorer/tsconfig.json
          ./packages/explorer/vitest.config.ts
        ];
      };
      agentPnpmDeps = pkgs.fetchPnpmDeps {
        pname = "rey-agent-dependencies";
        version = "0.1.0";
        src = agentSource;
        pnpm = pkgs.pnpm;
        fetcherVersion = 4;
        hash = "sha256-B+eQ26UB6dEJtvtWGTomTNJ/WsLfLDwjyJbE4O1g8sY=";
      };
      hifiSource = pkgs.fetchurl {
        url = "https://codeload.github.com/rupurt/hifi/tar.gz/058c6504fc10740360717e97e687fd77bef6a5c5";
        hash = "sha256-PzJsF7nxHWaVuVUsFo+XeszzjRFtOBI8SI/6sdeYRUM=";
      };
      agentAssets = pkgs.stdenvNoCC.mkDerivation {
        pname = "rey-agent";
        version = "0.1.0";
        src = agentSource;
        pnpmDeps = agentPnpmDeps;
        nativeBuildInputs = [
          pkgs.nodejs_24
          pkgs.pnpm
          pkgs.pnpmConfigHook
        ];
        TURBO_TELEMETRY_DISABLED = "1";
        buildPhase = ''
          runHook preBuild
          # fetchPnpmDeps suppresses dependency lifecycle scripts. Rebuild the
          # two explicitly admitted Hifi packages from the same pinned archive
          # before compiling Rey's UI against their package outputs.
          mkdir hifi
          tar -xzf ${hifiSource} --strip-components=1 -C hifi
          rm -rf apps/agent/node_modules/@hifi
          mkdir -p apps/agent/node_modules/@hifi
          ln -s "$PWD/hifi/packages/core" apps/agent/node_modules/@hifi/core
          ln -s "$PWD/hifi/packages/kinetic" apps/agent/node_modules/@hifi/kinetic
          ln -s "$PWD/apps/agent/node_modules" hifi/node_modules
          apps/agent/node_modules/.bin/tsc -p hifi/packages/core/tsconfig.build.json
          apps/agent/node_modules/.bin/tsc -p hifi/packages/kinetic/tsconfig.build.json
          pnpm run build
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          mkdir -p "$out"
          cp -R apps/agent/dist/. "$out/"
          runHook postInstall
        '';
      };
      cargoSource = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
          (craneLib.fileset.commonCargoSources ./.)
          ./crates/rey-environment/tests/fixtures
          ./crates/rey-runtime/tests/fixtures
          ./crates/rey/tests/fixtures
          ./docs/decisions/README.md
          ./plans/0003-scene-to-explorer.md
          ./scenes/rey-county
          ./sys/context-anchor-survey
        ];
      };
      commonCargoArgs = {
        pname = "rey-workspace";
        version = "0.1.0";
        src = cargoSource;
        strictDeps = true;
        cargoExtraArgs = "--locked --workspace --all-features";
        REY_BUILD_REVISION = self.rev or self.dirtyRev or "unknown";
        REY_UI_DIST_DIR = agentAssets;
      };
      cargoArtifacts = craneLib.buildDepsOnly (commonCargoArgs
        // {
          pname = "rey-workspace-deps";
        });
      workspacePackage = craneLib.buildPackage (commonCargoArgs
        // {
          inherit cargoArtifacts;
          cargoBuildExtraArgs = "--bins";
          doCheck = false;
          doInstallCargoArtifacts = true;
        });
      workspaceTests = craneLib.cargoNextest (commonCargoArgs
        // {
          cargoArtifacts = workspacePackage;
          nativeBuildInputs = [
            pkgs.bash
            pkgs.coreutils
            pkgs.git
          ];
          postCheck = ''
            cargo test --locked --workspace --all-features --doc --offline
          '';
        });
      reyPackage =
        pkgs.runCommand "rey" {
          meta = {
            description = "Environment-aware diff-directed compute runtime";
            license = pkgs.lib.licenses.mit;
            mainProgram = "rey";
          };
        } ''
          install -Dm755 ${workspacePackage}/bin/rey "$out/bin/rey"
        '';

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
          pkgs.actionlint
          pkgs.cacert
          pkgs.curl
          pkgs.git
          pkgs.jq
          pkgs.just
          pkgs.nodejs_24
          pkgs.pnpm
        ]
        ++ pkgs.lib.optionals pkgs.stdenv.isLinux [pkgs.mold];

      developmentPackages =
        basePackages
        ++ [
          pkgs.actionlint
          pkgs.alejandra
          pkgs.cargo-dist
          pkgs.cargo-nextest
          pkgs.rust-analyzer
        ];

      reyDev = pkgs.writeShellApplication {
        name = "rey-dev";
        runtimeInputs =
          basePackages
          ++ [
            pkgs.actionlint
            pkgs.alejandra
            pkgs.cargo-dist
            pkgs.cargo-nextest
            pkgs.nix
          ];
        text = ''
          exec just "$@"
        '';
      };
    in {
      packages = {
        default = reyPackage;
        rey = reyPackage;
        agent = agentAssets;
        dev = reyDev;
      };

      apps = {
        default = {
          type = "app";
          program = "${reyPackage}/bin/rey";
          meta.description = "Run Rey";
        };
        rey = {
          type = "app";
          program = "${reyPackage}/bin/rey";
          meta.description = "Run Rey";
        };
        dev = {
          type = "app";
          program = "${reyDev}/bin/rey-dev";
          meta.description = "Run Rey development tasks through just";
        };
      };

      checks = {
        rey = reyPackage;
        agent = agentAssets;
        workspace-tests = workspaceTests;
        dev-wrapper = reyDev;
      };

      devShells = {
        default = pkgs.mkShell {
          packages = developmentPackages;
          inherit shellHook;
        };

        ci = pkgs.mkShell {
          packages =
            basePackages
            ++ [
              pkgs.actionlint
              pkgs.alejandra
              pkgs.cargo-dist
              pkgs.cargo-nextest
            ];
          inherit shellHook;
        };
      };

      formatter = pkgs.alejandra;
    });
}
