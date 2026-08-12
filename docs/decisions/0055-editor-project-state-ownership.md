# ADR 0055: Editor Project State Ownership

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0046](0046-read-first-scene-editor.md)
- Supersedes: [ADR 0048](0048-fresh-v1-contract-baseline.md) only for the
  `rey.editor-status` schema-version assignment

## Context

The first scene-editor slice treated `rey.editor-project.v1` as an ambient
workspace file named `rey.scene.json`. Every `status`, default `diff`, and
`add` therefore depended on that conventional filename, even before an editor
project existed. A read in an untouched workspace failed with a missing-file
error, while generation wrote Rey bookkeeping beside authored project files.

That ownership is inconsistent with the rest of the editor revision plane.
Project identity and the ordered source declaration are Rey's mutable WORKING
metadata. The authored GeoJSON documents are the workspace-native sources and
must remain outside a local cache; the manifest that binds them into an editor
candidate belongs with the editor INDEX, packages, requests, objects, and
history under the selected Rey state boundary.

## Decision

The selected `LocalEditorStore` owns exactly one project declaration named
`project.json`. With the default state directory its location is
`.rey/editor/project.json`. An explicit `--state-dir` moves the complete editor
state boundary, including that declaration. The public CLI has no `--project`
argument and never reads or writes `rey.scene.json`.

`rey editor generate terrain` initializes the internal declaration if it is
absent and continues to write its declared native GeoJSON output at the
explicit workspace-relative path. Source path validation remains bounded,
workspace-contained, regular-file-only, and symlink rejecting. The local
project declaration is also bounded, regular-file-only, symlink rejecting, and
atomically replaced when its source declarations change.

`rey editor status` is valid before initialization. It performs no write,
reports `initialized=false`, carries no WORKING snapshot, and derives empty
`HEAD → INDEX` and `INDEX → WORKING` changes. Human output directs the operator
to the generation surface. `diff` projects the same empty directed comparison.
Mutation through `add` still requires an initialized project. If the project
declaration disappears while retained INDEX or HEAD state exists, status fails
closed instead of presenting that corruption as an authored deletion.

The optional WORKING snapshot and initialization bit advance the status
contract to `rey.editor-status.v2`. This is a pre-alpha hard cut: there is no
reader, migration, fallback, or dual write for a workspace `rey.scene.json` or
`rey.editor-status.v1`. Existing local editor work must be regenerated into
the selected `.rey` state boundary.

## Consequences

- `rey editor status` succeeds in an untouched workspace without creating
  `.rey` or requiring a conventional project file.
- Project metadata, INDEX, history, packages, requests, and frozen objects have
  one explicit local-state owner.
- Authored native sources remain visible workspace files and the local store
  does not become their sole copy.
- Removing or changing `--state-dir` intentionally selects a different editor
  project and history.
- The checked Rey County native-source fixture remains reviewable, but it is no
  longer an ambient default editor project.
