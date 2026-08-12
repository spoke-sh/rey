# ADR 0053: GitHub CI And Cargo-dist Releases

- Status: Accepted
- Date: 2026-08-11

## Context

Rey has reproducible Nix development and package outputs plus canonical
quality and test tasks, but no hosted continuous-integration or binary-release
path. A release process must preserve those local verification contracts,
produce native artifacts for common operator platforms, and keep publication
an explicit human decision.

The sibling Keel project demonstrates a useful split: Nix-backed GitHub CI for
quality and tests, and cargo-dist planning/building/hosting driven by semantic
version tags. Rey does not yet define Keel's additional Debian, RPM, Homebrew
tap, or documentation-publishing contracts.

## Decision

GitHub Actions is Rey's hosted CI and release orchestrator.

The CI workflow runs on pull requests and `main`, executes `just check` and
`just test` inside the pinned CI shell, and independently builds the Nix
package. It holds read-only repository permission.

Cargo-dist 0.32.0 is pinned in `dist-workspace.toml` and supplied by every Nix
development surface. The `rey` package is the only distributable application;
workspace libraries remain implementation crates and Cargo publication is
disabled. Package metadata declares the repository's checked-in MIT license.
Cargo-dist generates the release workflow, and drift from that generated form
fails `just check`.

Pull requests plan releases without building or publishing artifacts. A pushed
semantic-version tag matching the `rey` package version builds native archives
for aarch64 and x86-64 macOS, x86-64 Linux, and x86-64 Windows, plus shell and
PowerShell installers, checksums, and a source archive. GitHub Releases is the
only host. The release workflow alone receives `contents: write` through the
repository-scoped token.

## Consequences

- The local and hosted quality gates use the same `just` commands and pinned
  toolchain.
- A release requires an intentional version change and matching pushed tag;
  merges to `main` cannot publish a release.
- Pull requests continuously prove that the release configuration and
  generated workflow agree.
- Release archives are available for the four configured native targets; Nix
  remains an independent source/package installation path.
- Debian, RPM, MSI, Homebrew, Cargo registry, signing, provenance attestation,
  and external documentation publishing remain out of scope until their
  metadata, credentials, ownership, and qualification contracts are accepted.
