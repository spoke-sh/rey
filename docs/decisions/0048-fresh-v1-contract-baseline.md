# ADR 0048: Fresh V1 Contract And Candidate-State Baseline

- Status: Accepted
- Date: 2026-08-11
- Supersedes: every earlier Rey-owned public schema-version assignment and all
  pre-alpha compatibility or migration behavior
- Extends: [ADR 0046](0046-read-first-scene-editor.md) and [ADR
  0047](0047-semantic-spherical-atlas.md)

## Context

Rey is pre-alpha and its local `.rey` state has been intentionally cleared.
Successive implementation slices had advanced individual document names while
the product was still changing as one system. That history made the current
contract harder to read and encouraged compatibility branches for state that
is neither durable nor admitted proof.

The reset is also the right point to exercise the editor as intended: observe
the current environment through Rey, qualify and run the current workspace
workloads, and author a bounded native county candidate from that exact
evidence. The candidate must remain separate from admitted topography and must
not appear in `/explore` until a scene-admission workload exists.

## Decision

Every current Rey-owned public document, relation, envelope, local-state file,
API response, workload package, editor project, and editor package uses its
complete `.v1` schema. Numeric schema-version metadata is `1`. An operation or
graph revision is still ordinary instance identity and may advance without
renaming the containing public schema.

This is a destructive pre-alpha baseline:

- no reader accepts an earlier Rey schema name;
- no alias relabels older bytes as v1;
- no optional field exists solely to decode a prior shape;
- no migration, downgrade, rollback, or dual-write path is provided; and
- a user with earlier `.rey` state must remove it and rebuild evidence through
  the current CLI.

The v1 environment commit requires its integer commit timestamp. Missing or
partially populated commit documents fail verification. The same fail-closed
rule applies to all other v1 documents.

External version labels are outside this reset. Git porcelain v2 names a Git
wire format, WebGPU and WebGL2 name renderer capabilities, library versions
name dependencies, and RFC or OGC revisions name external standards. They are
not Rey document compatibility promises.

The first fresh-state editor proof authors **Rey County** as an OGC CRS84
candidate composed from exact native GeoJSON sources for boundary, terrain
control, hydrology, general features, and markers. Its labels and properties
may refer only to tool observations and workload executions actually exposed
through the fresh Rey CLI. The geometry is an authored scene layout, not an
Earth survey, semantic-distance claim, discovered path, or admitted
topography. `rey editor` must validate, stage, package, and inspect the
candidate; package authority remains candidate-only.

## Consequences

- Current code and foundational documentation have one public version
  vocabulary.
- Historical ADRs remain historical context, but their public version labels
  are superseded by this decision and do not describe readable formats.
- Clearing `.rey` is an intentional prerequisite for this cut rather than a
  migration failure.
- Tests prove rejection of non-v1 and incomplete-v1 documents instead of
  replaying earlier layouts.
- Rey County provides a concrete editor surface and provenance fixture while
  leaving the missing scene-admission boundary visible.
