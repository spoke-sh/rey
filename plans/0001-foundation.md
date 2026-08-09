# Plan 0001: Foundation And First Delta Proof

## Outcome

Establish the Rust/Nix workspace and deliver the smallest end-to-end Rey slice:
inventory a bounded local environment, materialize two typed frames, compute an
authoritative directed delta, render Tabular Diff 0.8, and evaluate a scoped
proof with zero Spoke. Then run the same evidence contract with a routed local
Spoke provider and record the first external-client conformance delta. A Git
commit/ref/index poll activates the relevant workload scenario selection or
graph entry point without assuming append-only history or exactly-once
delivery.

The slice must be deterministic without an LLM and must prove that changed
inputs or evaluator code make an earlier certificate stale.

## Completion Checklist

- [x] Foundational documents distinguish current truth from target architecture.
- [x] Accepted decisions fix the environment-aware, diff-directed, DataFrame,
  optional Spoke, proof, co-evolution, and development boundaries.
- [x] A pinned Nix flake provides default and CI Rust development shells.
- [x] The root Just tasks expose honest lifecycle behavior for repository state.
- [x] A Cargo workspace and narrow first-slice crates are scaffolded.
- [x] A bounded environment probe emits a typed capability snapshot with no
  Spoke present.
- [ ] A Git snapshot and semantic index delta produce one deterministic,
  replay-safe workload activation.
- [x] Two typed capability fixture frames produce a deterministic structured
  delta.
- [x] The same capability delta produces a valid bounded Tabular Diff 0.8
  artifact.
- [x] A scoped required-capability certificate verifies and detects stale or
  tampered inputs.
- [x] A local-only proof bundle exposes its exact retention guarantees.
- [ ] One frame/delta/proof bundle round-trips through routed Spoke contracts.
- [ ] The first Rey–Spoke conformance delta can direct work in either project.
- [ ] Root Cargo, Nix, CLI, standalone, and Spoke integration checks pass.

## Milestone 1 — Foundation

- [x] Write `README.md`, `CONSTITUTION.md`, `INSTRUCTIONS.md`, and `AGENTS.md`.
- [x] Add architecture, environment, Git, diff, proof, interface, development,
  and roadmap documents.
- [x] Add plan and architecture-decision indexes.
- [x] Accept ADRs 0001–0011 for the foundation and first proof slice.
- [x] Add a locked stable Rust flake with default and CI shells.
- [x] Add `.envrc`, `.gitignore` rules, and the six root Just lifecycle tasks.
- [x] Capture successful foundation verification commands below.

Foundation proof captured on 2026-08-07:

```text
nix flake check "path:$PWD"
nix flake check "path:$PWD" --all-systems --no-build
nix develop .#ci --command just check
nix develop "path:$PWD#ci" --command just setup
nix develop "path:$PWD" --command just setup
nix develop "path:$PWD#ci" --command alejandra --check flake.nix
nix run "path:$PWD#dev" -- setup
# local Markdown links and trailing whitespace checked with a bounded shell pass
```

The explicit `path:` form lets Nix evaluate newly created files before they are
added to Git. The ordinary `.#ci` form was also exercised with `flake.nix` and
`flake.lock` staged temporarily, then the index was restored.

## Milestone 2 — Freeze First-Slice Schemas

- [x] Select initial Polars and Arrow crate versions/features in an ADR.
- [x] Define canonical frame metadata and content identity.
- [x] Define the environment provider, capability snapshot, trust, operation,
  guarantee, and discovery-error schemas.
- [ ] Define Git repository/worktree, ref, commit/parent, path-change, semantic
  index, worktree-status, poll cursor, trigger, and activation schemas.
- [x] Define and implement the minimum workload, graph, scenario, campaign,
  qualification, and result contracts through ADR 0016 and Plan 0005;
  Git-triggered activation remains Milestone 5 work.
- [x] Define capability comparison compatibility, unique composite keys,
  ordering, normalization, and completeness behavior.
- [x] Define capability typed-delta Arrow and serialized schemas without losing
  before/after types.
- [x] Define capability structured, Arrow, JSON, summary, and Tabular Diff
  output bounds.
- [x] Define the required-capability claim, checks, input digest, status, and
  certificate schemas.
- [x] Select canonical serialization and hashing rules.
- [x] Decide which capability semantic fields invalidate frames, deltas, and
  proofs.
- [x] Decide the initial local evidence bundle and its publication guarantees.
- [x] Define the bootstrap and steady-state runtime lifecycle, including
  transition versus residual deltas and orthogonal execution/semantic/evidence
  state (ADRs 0012 and 0013; implemented separately in Plan 0002).
- [x] Define provider-owned retrieval and the bounded delta-directed reasoning
  surface at the architecture and interface-contract level (ADRs 0012 and
  0013; implemented separately in Plan 0002).
- [ ] Decide the Spoke artifact mappings and their distinct publication
  guarantees.

## Milestone 3 — Scaffold

- [x] Add a Cargo workspace using the flake-provided stable toolchain.
- [x] Add `rey` as the CLI/composition crate without collecting domain logic.
- [x] Add only the capability crates needed for the first slice from the target
  ownership map.
- [x] Add a narrow `rey-git` provider without coupling core runtime semantics to
  one Git CLI or library implementation.
- [x] Keep the Spoke adapter optional so core workspace tests have no running
  Spoke requirement.
- [x] Keep shared contracts narrow and dependency direction one-way.
- [x] Add locked dependency-only, workspace package, workspace test, `rey`
  package, and app outputs through Crane.
- [x] Wire every root task to real Cargo behavior without silent skips.
- [x] Add formatting, Clippy, nextest, doc-test, and Nix checks.

First executable proof captured on 2026-08-07:

```text
nix develop path:$PWD#ci --command just check
nix develop path:$PWD#ci --command just test
# 16 tests passed through nextest; all five crate doc-test suites passed
nix develop path:$PWD#ci --command just build
nix flake check path:$PWD
# packaged rey, offline workspace tests, and dev wrapper built successfully
nix flake check path:$PWD --all-systems --no-build
# x86_64-linux, aarch64-linux, and aarch64-darwin outputs evaluated
nix run path:$PWD -- environment inspect --format json
# standalone rey.capabilities.v1 snapshot contained five local provider rows
```

## Milestone 4 — Environment Capability Snapshot

- [ ] Add built-in, explicit-workspace, known-tool, and optional Spoke provider
  contracts behind bounded discovery.
- [x] Resolve only configured roots and known executable names/paths.
- [x] Bound metadata and version probes by time and bytes.
- [x] Record provider/tool path, version, digest or provenance, trust,
  operations, enforcement, limits, and errors in one typed frame.
- [ ] Freeze the snapshot for action admission and detect provider/tool drift.
- [x] Prove zero-Spoke startup and explicit standalone selection.
- [ ] Prove optional Spoke appearance/disappearance without changing a local
  claim's meaning.
- [x] Prove required-capability failure before side effects in the standalone
  certificate command.
- [ ] Test missing tools, duplicate candidates, malformed versions, timeouts,
  path changes, and unsupported enforcement.

## Milestone 5 — Git Poll And Activation

- [x] Discover one explicit repository/worktree without running hooks or
  mutating the index.
- [ ] Record object format, common repository identity, worktree identity,
  HEAD, watched refs, shallow/sparse/split-index facts, and completeness.
- [ ] Materialize bounded ref, commit, parent, path-change, semantic index, and
  declared worktree-status frames.
- [ ] Derive semantic index identity from logical entries rather than raw file
  metadata.
- [ ] Classify ref creation, deletion, fast-forward, rewind,
  rewrite/divergence, and unknown movement.
- [ ] Define a trigger that maps one Git delta subset to one workload revision
  plus scenario selection or graph entry point.
- [ ] Create deterministic activation identity from trigger,
  workload/graph/scenario selection, snapshots, and matched delta.
- [ ] Advance the poll cursor only after required transition evidence is
  retained.
- [ ] Prove crash replay and idempotent activation without an exactly-once
  claim.
- [ ] Cover detached/unborn HEAD, merge, shallow history, conflict stages,
  linked worktrees, gitlinks, stat-cache-only index refresh, and bounded
  overflow.

## Milestone 6 — Local Frame And Delta

- [x] Load two bounded reviewed capability snapshot fixtures with exact schemas.
- [x] Validate declared unique composite keys before alignment.
- [x] Compare equal, inserted, deleted, and modified capability relations and
  reject incompatible snapshot schemas.
- [x] Preserve typed before/after values and deterministic source/target labels.
- [x] Retain typed Arrow schema and lineage for an empty capability delta.
- [x] Reject duplicate keys and invalid schemas, and report incomplete
  observations as inconclusive.
- [x] Emit deterministic structured capability delta and summary artifacts.
- [x] Render bounded, ANSI-free Tabular Diff 0.8 CSV.
- [x] Prove that context elision does not change capability delta semantics.
- [ ] Generalize these contracts to arbitrary compatible typed frames,
  including schema changes, normalizers, and terminal rendering.

## Milestone 7 — Proof And Staleness

- [x] Evaluate a required-capability claim over fixture snapshots and delta.
- [x] Emit a canonical scoped certificate with content digests.
- [x] Verify a certificate without trusting its stored status.
- [x] Distinguish passed, failed, inconclusive, and stale for the implemented
  scoped claim.
- [ ] Add pending to the future retained/asynchronous proof lifecycle.
- [x] Bind capability source/target snapshots, normalized claim, limits, and
  comparator/evaluator contracts into the proof input digest.
- [ ] Make source, schema, key, normalizer, comparator, evaluator, fixture, and
  limit changes invalidate proof inputs.
- [ ] Make provider, tool, version, digest/provenance, trust, and guarantee
  changes invalidate dependent proof inputs.
- [ ] Make Git source snapshots, semantic index, trigger/workload selection,
  and cursor/activation inputs invalidate dependent proof state.
- [x] Detect tampered snapshot and certificate evidence.
- [ ] Keep similarity, coverage, completeness, and status distinct.
- [x] Write and verify a content-addressed local bundle without claiming Spoke
  durability or process lineage.

Capability delta/certificate/local-bundle proof captured on 2026-08-07:

```text
nix develop path:$PWD#ci --command just check
nix develop path:$PWD#ci --command just test
# 37 tests passed through nextest; all seven crate doc-test suites passed
nix develop path:$PWD#ci --command just build
nix flake check path:$PWD
nix flake check path:$PWD --all-systems --no-build
nix run path:$PWD -- environment inspect --format json
nix run path:$PWD -- environment prove baseline.json candidate.json \
  --require-capability frame.arrow-stream --bundle proof.bundle
nix run path:$PWD -- environment verify-bundle proof.bundle
# identical replay emitted the same certificate and reused the verified bundle
```

## Milestone 8 — Optional Routed Spoke Proof

- [ ] Add explicit and safe configured discovery of a Spoke endpoint.
- [ ] Project advertised Spoke capabilities into the common capability snapshot.
- [ ] Use Arrow IPC for one bounded typed query or table observation.
- [ ] Bind the resulting frame to exact Spoke revision/checkpoint metadata.
- [ ] Publish retained frame, delta, evidence, and certificate artifacts through
  public Spoke contracts with content identity and idempotency.
- [ ] Ensure the certificate is not visible before required evidence is durable.
- [ ] Verify the bundle after Rey and Spoke process restart.
- [ ] Prove revision drift, missing capability, unavailable service, truncation,
  and routed error behavior.
- [ ] Prove that disconnected automatic mode retains the standalone contract.
- [ ] Prove that a Spoke-required claim fails closed while a local claim remains
  evaluable.
- [ ] Emit a typed conformance delta between the Rey-required and
  Spoke-advertised public surfaces.
- [ ] Activate that conformance workload from a watched Spoke Git commit while
  retaining exact Git and Spoke runtime identities.
- [ ] Preserve exact Rey revision, Spoke revision, fixture, and evaluator
  identity so either project's change makes conformance state stale.
- [ ] Do not access Spoke's data directory or capability internals.
- [ ] Keep Spoke build/start independent from Rey and Rey foundation tests
  independent from Spoke.

## Milestone 9 — CLI And Acceptance

- [x] Expose the implemented environment-inspection command.
- [x] Expose a capability-diff command.
- [ ] Expose implemented Git inspect, diff, and one-shot poll commands.
- [x] Expose only the implemented inspection, capability-diff, scoped proof,
  and local-bundle commands.
- [x] Preserve machine data on stdout and diagnostics on stderr.
- [x] Support bounded terminal, Arrow, and JSON output for the implemented
  capability schema, plus structured/Arrow/summary/Tabular Diff delta output.
- [x] Define and test categorized exit codes for pass, fail, inconclusive,
  stale, and invalid input/runtime failure.
- [ ] Add property tests and routed Spoke integration tests; deterministic
  fixtures and focused resource-limit tests exist for the standalone slice.
- [x] Make `just check`, `just test`, `just build`, and `nix flake check` pass.

## Acceptance Criteria

- [x] Identical frozen capability inputs produce byte-identical structured
  semantic artifacts.
- [ ] Direction, labels, keys, schemas, types, normalizers, limits, and source
  revisions survive every supported representation.
- [x] Duplicate keys and incompatible or incomplete capability inputs never
  produce a guessed passing delta.
- [ ] Tabular Diff insertion, deletion, modification, schema, null, and context
  markers match the selected 0.8 contract.
- [ ] A zero diff passes only the declared scoped claim.
- [x] Changed capability snapshot inputs or evaluator code make the old
  certificate stale.
- [ ] Capability drift makes affected actions/proofs stale without invalidating
  unrelated frames.
- [x] Missing or tampered local evidence cannot verify as passed.
- [x] Zero-Spoke mode produces useful local capability frames, deltas, and
  scoped proofs.
- [ ] Commit/ref/index deltas activate only declared affected workload entries.
- [ ] Ref rewrites and incomplete history never appear as false append events.
- [ ] Poll replay does not duplicate an idempotent workload effect.
- [ ] Poll evidence does not claim unobserved intermediate index/worktree states.
- [ ] Polling executes no repository hook and makes no Git mutation.
- [ ] Missing required Spoke capability never silently weakens a claim.
- [x] Local-only evidence and execution never claim Spoke durability, fencing,
  or query semantics.
- [ ] Routed Spoke persistence retains exact revision and request lineage across
  restart.
- [x] CLI structured stdout is byte-clean and diagnostics remain on stderr.
- [ ] Every advertised limit is enforced and budget exhaustion is
  inconclusive, not convergence.

## Deferred

Long-running Git polling, reasoning-surface retrieval/materialization,
workload-specific frontier derivation/invalidation, recurring scheduling,
multi-step loops, agent policies, mutation, local and Spoke compute actions,
full codebase lenses, incremental physical execution, a Rey service,
multi-user operation, and managed deployment remain later plans. Plan 0002
implements the pure lifecycle and bounded surface contracts; Plan 0003 adds
canonical frontier/progress and deterministic work-selection contracts. Neither
claims those runtime behaviors are implemented in Plan 0001.

Plan 0006 now owns the first bounded mining operation/request/result,
source-search, relational/text-delta, visualization, workload-frontier, and
reasoning-surface implementation slice. Broader codebase mining remains
deferred until that plan proves the common standalone contract.
