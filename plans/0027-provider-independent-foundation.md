# Plan 0027: Provider-Independent Foundation

## Goal

Remove the privileged external-platform model from Rey's contracts,
implementation, documentation, examples, and roadmap so the project is driven
only by Rey's own requirements.

## Completion

- [x] Remove platform-specific coordinate and retention variants.
- [x] Remove platform-specific proof guarantee fields.
- [x] Remove hard-coded repository ownership from the CLI and UI contract.
- [x] Remove the dedicated integration and co-development decisions.
- [x] Define the provider-independent hard cut in ADR 0052.
- [x] Rewrite foundational documents and historical plans around Rey-owned
  local contracts.
- [x] Qualify Rust, TypeScript, CLI, and browser behavior.
- [x] Prove a case-insensitive tracked-source scan contains no removed platform
  references.

## Contract

The implemented v1 profile is local. A future provider may implement a narrow
Rey-owned interface only after a concrete requirement, accepted decision, and
human-verifiable CLI slice exist. No provider receives architectural priority
by anticipation.
