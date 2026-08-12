# Releases

Rey uses GitHub Actions for continuous integration and cargo-dist for tagged
binary releases. CI verifies the repository through the same Nix and `just`
interfaces used locally. Release automation packages the `rey` binary but does
not replace or weaken that quality gate.

## Continuous Integration

`.github/workflows/ci.yml` runs for pull requests, pushes to `main`, and manual
dispatches. Its quality job enters `devShells.ci`, runs `just check`, and then
runs `just test`. A separate job builds the Nix `rey` derivation. Workflow
permissions default to read-only repository contents.

The Cargo cache is an optimization only. Nix, Cargo, pnpm, Nextest, and the
checked-in lock files remain authoritative when the cache is absent or stale.

## Release Planning

`dist-workspace.toml` is the authored cargo-dist configuration and
`.github/workflows/release.yml` is generated from it. Pull requests run the
release plan but do not build or publish release artifacts. `just check`
rejects a generated workflow that has drifted from its configuration and uses
Actionlint to verify workflow structure and GitHub expressions. ShellCheck is
retained for the authored CI workflow and disabled for the generated release
pass because cargo-dist owns those shell fragments.

The current plan distributes only the `rey` application and produces:

- native archives for Apple Silicon macOS, Intel macOS, x86-64 Linux, and
  x86-64 Windows;
- SHA-256 checksums and a combined checksum manifest;
- POSIX shell and PowerShell installers; and
- a source archive.

The release workflow uses the repository-scoped `GITHUB_TOKEN` and requires
write access only to GitHub Release contents. It does not publish Cargo crates,
packages, a Homebrew tap, or a documentation site.

## Cutting A Release

The release tag is the explicit publication boundary. Before tagging:

1. set `[workspace.package].version` in `Cargo.toml` to the intended semantic
   version and update `Cargo.lock` if Cargo changes it;
2. run `just check`, `just test`, and `just dist-check`;
3. commit the version and release-bearing changes; and
4. create and push an annotated `v<version>` tag, for example:

   ```sh
   git tag -a v0.1.0 -m 'Release v0.1.0'
   git push origin main
   git push origin v0.1.0
   ```

Cargo-dist rejects a tag whose version does not match the distributable
package. A matching pushed tag builds every configured target, creates the
GitHub Release, uploads the archives, installers, checksums, source archive,
and final distribution manifest, then announces the release. Prerelease
semantic versions such as `v0.2.0-rc.1` become GitHub prereleases.

Do not create or push a release tag merely to test the workflow. Pull-request
planning and `just dist-check` are the non-publishing verification surfaces.

## Updating Cargo-dist

Update the Nix pin through `flake.lock`/`nixpkgs` and
`cargo-dist-version` in `dist-workspace.toml` intentionally. Then regenerate
the release workflow with:

```sh
dist generate
```

Review the generated action and target changes, then run `just dist-check` and
the normal repository checks.
