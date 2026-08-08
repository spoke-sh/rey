# ADR 0011: Local Content-Addressed Proof Bundle

- Status: Accepted
- Date: 2026-08-07

## Context

ADR 0010 emits a verified capability delta and required-capability certificate,
but the certificate exists only on stdout. Plan 0001 needs a local retention
boundary before Git polling can advance a cursor after evidence publication and
before the same semantic artifacts can be mapped onto Spoke resources.

The local profile must remain useful without pretending a group of host files
has Spoke revisions, remote durability, fenced execution lineage, or a
multi-process transaction. It also needs deterministic identities, bounded
verification, and safe behavior when publication is retried after interruption.

## Decision

The first local bundle schema is `rey.local-proof-bundle.v1`. A caller selects
one output directory. Its fixed layout is:

```text
<bundle>/
  manifest.json
  objects/
    blake3/
      <lowercase-hex-digest>
```

Objects retain six roles for the implemented claim: source and target snapshot
JSON, authoritative capability-delta JSON, capability-delta Arrow, Tabular Diff
CSV, and required-capability certificate JSON. JSON artifacts use the canonical
Serde struct field order plus one trailing LF. Artifact identity is the
domain-separated, length-framed BLAKE3 digest of the exact retained bytes. The
manifest orders roles canonically and binds artifact media types, byte lengths,
object paths, semantic snapshot/delta/certificate identities, effective bundle
limits, total logical artifact bytes, and the fixed local retention contract.
The bundle id is a semantic digest of those manifest fields rather than a hash
of presentation whitespace.

Bundle creation first verifies the supplied certificate against the exact
snapshots and current comparator/evaluator contracts, recomputes every retained
projection, and enforces artifact-count, per-artifact byte, total-byte, and
capability-row limits before filesystem publication. It writes a fresh staging
directory beside the selected destination, writes content-addressed objects,
writes `manifest.json` last, and then renames the complete staging directory to
the destination. An already present, fully verified bundle with the same bundle
id is an idempotent success. Any different, incomplete, symlinked, or invalid
destination is rejected rather than overwritten.

Verification never trusts the stored certificate status, delta, projections,
manifest id, file lengths, or object names. It bounds and validates the
manifest, admits only the fixed canonical roles and paths, hashes every regular
file without following symlinks, reloads and verifies both snapshots,
recomputes the certificate, delta, Arrow, and Tabular Diff artifacts, and then
checks the bundle identity.

The manifest declares the exact local contract:

- objects are content-addressed and the manifest is written after them;
- publication uses a same-parent staging-directory rename;
- an existing destination is never overwritten;
- verification is read-only; and
- no process-crash durability, multi-process transactionality, remote
  durability, authenticated writer identity, Spoke durability, fenced
  execution, query semantics, revision lineage, or process lineage is claimed.

The implementation does not call `fsync`, coordinate concurrent writers, clean
abandoned staging directories owned by another process, or claim protection
against a concurrently malicious local filesystem. A crash before the rename
may leave an unreferenced staging directory; a crash after the rename leaves
the completely constructed namespace but still has only the host filesystem's
unflushed durability guarantees. Retry is safe because only a fully verified
identical destination is accepted.

`rey environment prove` gains an optional explicit bundle destination while
retaining its certificate stdout behavior. A separate
`rey environment verify-bundle` command verifies retained evidence without
changing the existing three-file certificate verification contract.

## Consequences

- Standalone Rey gains a self-contained, bounded, replayable evidence boundary
  without inventing a storage service.
- Git activation can use successful bundle publication as the local evidence
  precondition for cursor advancement.
- Exact artifact roles and media types can later be mapped through public Spoke
  contracts without treating the local directory layout as a Spoke API.
- The certificate itself remains retention-neutral; the enclosing manifest
  states the local guarantees.
- Spoke artifact mapping, signatures, shared object stores, garbage collection,
  concurrent writers, and crash-durable flushing remain later decisions.
