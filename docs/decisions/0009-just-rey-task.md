# ADR 0009: Name The CLI Task `rey`

- Status: Accepted
- Date: 2026-08-07
- CLI spelling updated by: [ADR 0019](0019-git-shaped-environment-history.md)
- Amends: [ADR 0004](0004-rust-and-nix-development-foundation.md)

## Context

ADR 0004 reserved `just dev` before a runtime executable existed. Rey now has a
real CLI, while `dev` also names the Nix development wrapper and shell concept.
The Just task should state what it runs rather than overload that development
terminology.

## Decision

Rename the Just CLI recipe from `dev` to `rey`. At the time of this decision,
the demonstration was:

```sh
just rey env inspect --format table
```

ADR 0021 later removes `inspect`; the equivalent current demonstration is
`just rey env status`.

The Nix `packages.dev` and `apps.dev` development-task wrapper retain their
names. The other root Just recipes remain `setup`, `check`, `test`, `build`,
and `fmt`.

## Consequences

- Contributor and quick-start documentation uses `just rey` for the CLI.
- `nix run .#dev -- rey <arguments>` continues to invoke the renamed recipe
  through the development wrapper.
- `just dev` is removed rather than retained as an indefinite alias.
