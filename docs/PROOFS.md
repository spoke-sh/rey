# Proofs And Evidence

Rey produces scoped computational proofs: reproducible assessments that a
declared claim holds for exact observations under explicit comparison rules,
coverage, and limits. Rey is not a theorem prover, and a proof certificate is
not a universal correctness claim.

## Claim Contract

A claim declares:

- stable id and revision;
- human purpose;
- space and source scope;
- required lenses and baseline/target selection;
- predicate and comparison direction;
- required coverage and completeness;
- permitted normalizers, tolerances, and ignored fields;
- effective resource limits;
- pass, fail, and inconclusive rules; and
- evidence retention requirements.

Examples include:

- two keyed relations are equal;
- no new error diagnostics appear relative to a baseline;
- every changed public symbol has an expected compatibility decision;
- selected test and runtime evidence matches reviewed expectations; or
- a code mutation changes only an allowed dependency frontier.

Claims compose only through explicit boolean or quantified rules. One passing
child claim does not imply its siblings ran.

## Evidence

Evidence is the retained, addressable material used to evaluate a claim. It may
include:

- exact Spoke source versions and query revisions;
- local source identities and the capability snapshot used to establish them;
- Git repository/worktree identity, object format, commit/ref OIDs, semantic
  index digest, poll cursor, and activation identities where applicable;
- frame schemas, Arrow data, and content digests;
- structured deltas and Tabular Diff projections;
- lens, normalizer, comparator, and evaluator implementation digests;
- action proposals and admission decisions;
- local or Spoke run, attempt, event, capture, and tool-resolution lineage;
- fixtures, expected values, and reviewed exceptions;
- coverage and completeness relations;
- limits, truncation, unsupported controls, and failures; and
- trace edges that connect causes to observations.

An evidence address is credential-free and identifies provenance rather than a
connection string. Secrets, signed URLs, private hostnames, ephemeral database
credentials, and raw provider keys do not belong in proof artifacts.

## Status

Proof status has five values:

| Status | Meaning |
| --- | --- |
| `pending` | Required evaluation has not completed |
| `passed` | Every required check passed with sufficient complete evidence |
| `failed` | At least one required predicate was evaluated and did not hold |
| `inconclusive` | The claim could not be decided under available evidence or limits |
| `stale` | Previously evaluated evidence no longer matches current bound inputs |

`inconclusive` is not a softer pass. Common causes include an unavailable
source, unsupported runtime enforcement, incompatible frame, duplicate key,
truncated required observation, exhausted budget, or lost run.

## Proof Manifest

A proof manifest contains, at minimum:

```text
schema and proof id
claim id and revision
status and assessment time
space id and revision
source bindings and input digest
lenses, schemas, normalizers, comparators, and evaluator digests
actions, tools, runs, attempts, and captures used
Git snapshots, poll cursors, triggers, and activations used
required and observed coverage
limits, omissions, ignored fields, tolerances, and unsupported controls
frame and delta evidence addresses and digests
individual check outcomes
certificate digest
```

Assessment time is lineage, not proof identity, unless time is a declared claim
input. The certificate digest covers a canonical unsigned manifest. A later
signature may attest who issued that manifest; content hashing alone does not
establish issuer identity or trust.

## Input Identity And Staleness

The proof input digest covers all semantics capable of changing the result:

- exact sources and candidates;
- environment providers, capability snapshot, resolved tools, trust classes,
  and guarantees used by required evidence;
- Git repository/worktree identity, watched refs, exact OIDs, semantic index,
  trigger revision, and activation inputs;
- fixtures and expected evidence;
- space, lens, claim, and policy revisions where policy affects scope;
- schema, key, order, normalizer, tolerance, and ignore definitions;
- tool and execution-contract identities;
- comparator, evaluator, and relevant runner implementation digests; and
- limits that affect completeness.

Verification recomputes this identity. A mismatch yields `stale` even when the
stored manifest says `passed`. Changing presentation-only color or a
non-semantic timestamp need not invalidate proof.

If changed inputs exceed the retained certificate's evaluation bounds, the
verifier can still classify the exact snapshot mismatch as `stale`, but it
reports that no recomputed input digest is available because no authoritative
delta exists beyond the bound.

Remote or later-stage evidence depends on the proof inputs that authorized it.
If a local baseline proof becomes stale, downstream assessments that cite its
old input digest are stale as well.

### Implemented Required-Capability Certificate

ADR 0010 implements one deliberately narrow claim:
`rey.environment.required-capabilities.v1`. Each named capability passes when
at least one target row reports it available, fails when complete evidence
shows it absent or unavailable, and is inconclusive when an error or incomplete
unknown absence prevents a decision. The conjunction fails on any known false,
then is inconclusive on any remaining unknown.

The certificate binds verified source and target snapshot ids, the exact
capability delta, normalized requirements, labels, limits, and versioned
comparator and evaluator contract digests. Verification checks the certificate
digest, recomputes the delta and check results, and reports `stale` when a
snapshot or current contract input changes. The certificate is emitted to
stdout only; it does not yet claim a local retention profile, signatures,
Spoke durability, or a generic proof manifest.

## Coverage And Completeness

Coverage identifies what part of a declared scope was observed. It can be
modeled branches, entities, files, schemas, changed rows, requirements, or
another versioned domain measure. Coverage must name its unit and denominator.

Completeness answers whether a particular observation satisfied its lens
contract. A complete frame under a narrow lens does not imply broad code or
behavioral coverage.

Progress, similarity, confidence, and coverage may help prioritize the
frontier. They remain separate from proof status:

- high similarity can still fail one critical predicate;
- 100% execution coverage can contain mismatches;
- a zero tabular diff can cover only one fixture; and
- a passing process exit status can leave semantic claims failed.

## Verification

Verification must be possible without trusting the policy that requested the
work. A verifier:

1. parses and bounds the manifest;
2. resolves all required immutable evidence;
3. checks content digests and format versions;
4. recomputes proof input identity;
5. validates claim, lens, comparator, and evaluator compatibility;
6. recomputes or validates individual check outcomes;
7. checks coverage, completeness, limits, and omissions; and
8. recomputes the certificate digest and final status.

Missing retained evidence makes verification inconclusive unless the claim
explicitly permits deterministic replay and every replay input is still
available at its exact revision.

## Retention Profiles

Rey has two initial evidence-retention profiles:

- **local** writes a content-addressed, bounded proof bundle to an explicit
  caller-selected artifact directory. It may use atomic host-file operations
  but does not claim transactional multi-artifact publication, remote
  durability, restart coordination, or Spoke revision semantics unless those
  are separately proved.
- **Spoke-backed** persists proof inputs and artifacts using public Spoke
  resources with immutable versions and explicit media types.

Candidate Spoke-backed mappings include:

- files or objects for canonical manifests, Arrow frames, deltas, CSV, and text
  patches;
- streams for ordered transition and proof events;
- tables for queryable proof, check, frontier, and lineage indexes; and
- documents for human-authored claims and review material.

The authoritative local bundle layout, Spoke mapping, and atomic publication
boundaries remain open Plan 0001 decisions. A proof names its retention profile
and must not claim durability stronger than the operations actually used.

## Acceptance And Mutation

Proof can gate an effect, but a passing proof never grants ambient authority.
An explicit policy decides which claim is required for which action, and
runtime admission revalidates that proof's identity and staleness immediately
before the effect.

Acceptance records are new evidence. They do not rewrite failed attempts,
discard exceptions, or mutate an old certificate from failed to passed.

## Required Proof Fixtures

The first proof engine must demonstrate:

- passed, failed, inconclusive, pending, and stale states;
- changed source, fixture, lens, normalizer, tool, evaluator, and limit inputs;
- changed capability snapshots and loss of required Spoke guarantees;
- changed Git refs, index semantics, trigger definitions, or replay cursor;
- honest verification of both local-only and Spoke-backed retention profiles;
- missing and tampered evidence;
- incomplete coverage and truncated observations;
- downstream proof staleness;
- deterministic canonical manifest and certificate digests;
- clear distinction between similarity, coverage, and status; and
- verification that does not trust a stored status field.
