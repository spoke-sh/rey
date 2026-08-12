# ADR 0051: Typed Workspace Ignore File

Status: Accepted

## Context

Rey observes a growing set of typed workspace resources. Humans need a durable
way to narrow local WORKING scope—for example excluding one incoming workload
or all process environment variables—without deleting authored files, changing
global process state, or relying on surface-specific hidden configuration.

A Git-style path-only ignore grammar is insufficient because `workload`,
`environment variable`, and `application` identities are different resource
families. Silently filtering them would also violate Rey's requirement to show
omissions and exact scope.

## Decision

- Rey recognizes one optional workspace-root `.reyignore` file.
- V1 syntax is one `kind: pattern` rule per line. Blank lines and `#` comments
  are inert. `*` matches zero or more bytes and `?` matches one byte;
  matching is case-sensitive.
- Implemented kinds are `workload`, `environment variable`, `application`,
  `input`, and `reference`. Resource kinds remain literal and cannot contain
  wildcard metacharacters.
- The file is workspace-owned, UTF-8, regular, non-symlinked, and bounded to
  64 KiB, 256 rules, and 4096 bytes per line. Malformed or unsafe files fail
  closed.
- Rey validates discovered or authored candidates before ignoring them. An
  ignore rule cannot hide malformed package input or turn discovery authority
  into execution authority.
- Filtering applies only while deriving WORKING. It does not mutate source
  files or rewrite retained HEAD and INDEX.
- Relevant rules, exact `.reyignore` source digest, source lines, and match
  counts participate in the filtered snapshot identity and remain visible as
  explicit omissions. Environment snapshots retain this as a typed synthetic
  policy capability; workload snapshots carry the same projection directly.
- Rules owned by another surface are preserved by the parser but do not affect
  or enter the identity of the current surface.

## Consequences

The operator can write:

```text
workload: context-anchor-survey
environment variable:*
```

and both CLI and UI projections omit those exact WORKING objects while
disclosing the active scope. Changing `.reyignore` is semantic drift even when
the visible filtered object set remains empty. Removing a rule can reveal a
new WORKING object, but cannot execute or admit it.

This policy is not `.gitignore`, source-mining ignore parity, a regular
expression engine, or a recursive configuration cascade. New kinds or pattern
semantics require a versioned contract change and corresponding status/UI
proof.
