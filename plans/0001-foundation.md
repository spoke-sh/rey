# Plan 0001: Foundation And First Delta Proof

## Outcome

Establish the Rust/Nix workspace and deliver the smallest end-to-end Rey slice:
inventory a bounded local environment, materialize two typed frames, compute an
authoritative directed delta, render Tabular Diff 0.8, and evaluate a scoped
proof with zero Spoke. Then run the same evidence contract with a routed local
Spoke provider and record the first external-client conformance delta. A Git
commit/ref/index poll activates the relevant application component without
assuming append-only history or exactly-once delivery.

The slice must be deterministic without an LLM and must prove that changed
inputs or evaluator code make an earlier certificate stale.

## Completion Checklist

- [x] Foundational documents distinguish current truth from target architecture.
- [x] Accepted decisions fix the environment-aware, diff-directed, DataFrame,
  optional Spoke, proof, co-evolution, and development boundaries.
- [x] A pinned Nix flake provides default and CI Rust development shells.
- [x] The root Just tasks report the unscaffolded runtime honestly.
- [ ] A Cargo workspace and narrow first-slice crates are scaffolded.
- [ ] A bounded environment probe emits a typed capability snapshot with no
  Spoke present.
- [ ] A Git snapshot and semantic index delta produce one deterministic,
  replay-safe application activation.
- [ ] Two typed fixture frames produce a deterministic structured delta.
- [ ] The same delta produces a valid bounded Tabular Diff 0.8 artifact.
- [ ] A scoped proof manifest verifies and detects stale or tampered inputs.
- [ ] A local-only proof bundle exposes its exact retention guarantees.
- [ ] One frame/delta/proof bundle round-trips through routed Spoke contracts.
- [ ] The first Rey–Spoke conformance delta can direct work in either project.
- [ ] Root Cargo, Nix, CLI, standalone, and Spoke integration checks pass.

## Milestone 1 — Foundation

- [x] Write `README.md`, `CONSTITUTION.md`, `INSTRUCTIONS.md`, and `AGENTS.md`.
- [x] Add architecture, environment, Git, diff, proof, interface, development,
  and roadmap documents.
- [x] Add plan and architecture-decision indexes.
- [x] Accept ADRs 0001–0007.
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

- [ ] Select initial Polars and Arrow crate versions/features in an ADR.
- [ ] Define canonical frame metadata and content identity.
- [ ] Define the environment provider, capability snapshot, trust, operation,
  guarantee, and discovery-error schemas.
- [ ] Define Git repository/worktree, ref, commit/parent, path-change, semantic
  index, worktree-status, poll cursor, trigger, and activation schemas.
- [ ] Define application and independently activatable component declarations,
  dependency edges, entry points, and budgets.
- [ ] Define comparison compatibility, unique composite keys, ordering,
  normalization, and completeness behavior.
- [ ] Define the typed delta Arrow and serialized schemas without losing
  heterogeneous before/after types.
- [ ] Define structured, Arrow, JSON, terminal, and Tabular Diff output bounds.
- [ ] Define claim, check, evidence manifest, input digest, status, and
  certificate schemas.
- [ ] Select canonical serialization and hashing rules.
- [ ] Decide which semantic fields invalidate frames, deltas, and proofs.
- [ ] Decide the initial local evidence bundle and Spoke artifact mappings plus
  their distinct publication guarantees.

## Milestone 3 — Scaffold

- [ ] Add a Cargo workspace using the flake-provided stable toolchain.
- [ ] Add `rey` as the CLI/composition crate without collecting domain logic.
- [ ] Add only the capability crates needed for the first slice from the target
  ownership map.
- [ ] Add a narrow `rey-git` provider without coupling core runtime semantics to
  one Git CLI or library implementation.
- [ ] Keep the Spoke adapter optional so core workspace tests have no running
  Spoke requirement.
- [ ] Keep shared contracts narrow and dependency direction one-way.
- [ ] Add locked dependency-only, workspace package, workspace test, `rey`
  package, and app outputs through Crane.
- [ ] Wire every root task to real Cargo behavior without silent skips.
- [ ] Add formatting, Clippy, nextest, doc-test, and Nix checks.

## Milestone 4 — Environment Capability Snapshot

- [ ] Add built-in, explicit-workspace, known-tool, and optional Spoke provider
  contracts behind bounded discovery.
- [ ] Resolve only configured roots and known executable names/paths.
- [ ] Bound metadata and version probes by time and bytes.
- [ ] Record provider/tool path, version, digest or provenance, trust,
  operations, enforcement, limits, and errors in one typed frame.
- [ ] Freeze the snapshot for action admission and detect provider/tool drift.
- [ ] Prove zero-Spoke startup and explicit standalone selection.
- [ ] Prove optional Spoke appearance/disappearance without changing a local
  claim's meaning.
- [ ] Prove required-capability failure before side effects.
- [ ] Test missing tools, duplicate candidates, malformed versions, timeouts,
  path changes, and unsupported enforcement.

## Milestone 5 — Git Poll And Activation

- [ ] Discover one explicit repository/worktree without running hooks or
  mutating the index.
- [ ] Record object format, common repository identity, worktree identity,
  HEAD, watched refs, shallow/sparse/split-index facts, and completeness.
- [ ] Materialize bounded ref, commit, parent, path-change, semantic index, and
  declared worktree-status frames.
- [ ] Derive semantic index identity from logical entries rather than raw file
  metadata.
- [ ] Classify ref creation, deletion, fast-forward, rewind,
  rewrite/divergence, and unknown movement.
- [ ] Define a trigger that maps one Git delta subset to one application
  component revision.
- [ ] Create deterministic activation identity from trigger, component,
  snapshots, and matched delta.
- [ ] Advance the poll cursor only after required transition evidence is
  retained.
- [ ] Prove crash replay and idempotent activation without an exactly-once
  claim.
- [ ] Cover detached/unborn HEAD, merge, shallow history, conflict stages,
  linked worktrees, gitlinks, stat-cache-only index refresh, and bounded
  overflow.

## Milestone 6 — Local Frame And Delta

- [ ] Load two bounded reviewed Arrow or fixture relations with exact schemas.
- [ ] Validate declared unique keys before alignment.
- [ ] Compare equal, inserted, deleted, modified, and schema-changed relations.
- [ ] Preserve typed before/after values and deterministic source/target labels.
- [ ] Retain typed schemas for empty source and target frames.
- [ ] Reject duplicate keys, incompatible schemas, invalid normalizers, and
  incomplete required observations explicitly.
- [ ] Emit deterministic structured delta and summary artifacts.
- [ ] Render bounded, ANSI-free Tabular Diff 0.8 CSV plus a terminal view.
- [ ] Prove that color and context elision do not change semantics.

## Milestone 7 — Proof And Staleness

- [ ] Evaluate an equality claim over the fixture frames and delta.
- [ ] Emit canonical evidence and proof manifests with content digests.
- [ ] Verify a certificate without trusting its stored status.
- [ ] Distinguish passed, failed, inconclusive, pending, and stale.
- [ ] Make source, schema, key, normalizer, comparator, evaluator, fixture, and
  limit changes invalidate proof inputs.
- [ ] Make provider, tool, version, digest/provenance, trust, and guarantee
  changes invalidate dependent proof inputs.
- [ ] Make Git source snapshots, semantic index, trigger/component revision,
  and cursor/activation inputs invalidate dependent proof state.
- [ ] Detect missing and tampered evidence.
- [ ] Keep similarity, coverage, completeness, and status distinct.
- [ ] Write and verify a content-addressed local bundle without claiming Spoke
  durability or process lineage.

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
- [ ] Activate that conformance component from a watched Spoke Git commit while
  retaining exact Git and Spoke runtime identities.
- [ ] Preserve exact Rey revision, Spoke revision, fixture, and evaluator
  identity so either project's change makes conformance state stale.
- [ ] Do not access Spoke's data directory or capability internals.
- [ ] Keep Spoke build/start independent from Rey and Rey foundation tests
  independent from Spoke.

## Milestone 9 — CLI And Acceptance

- [ ] Expose implemented environment inspection and capability-diff commands.
- [ ] Expose implemented Git inspect, diff, and one-shot poll commands.
- [ ] Expose only the implemented frame, diff, and proof commands.
- [ ] Preserve machine data on stdout and diagnostics on stderr.
- [ ] Support bounded terminal, Arrow, JSON, structured delta, and Tabular Diff
  output where the corresponding schema is implemented.
- [ ] Define and test categorized exit codes for pass, fail, inconclusive,
  stale, invalid input, and runtime failure.
- [ ] Add fixture, property, determinism, resource-limit, and routed integration
  tests.
- [ ] Make `just check`, `just test`, `just build`, and `nix flake check` pass.

## Acceptance Criteria

- [ ] Identical frozen inputs produce byte-identical semantic artifacts.
- [ ] Direction, labels, keys, schemas, types, normalizers, limits, and source
  revisions survive every supported representation.
- [ ] Duplicate keys and incompatible or incomplete inputs never produce a
  guessed passing delta.
- [ ] Tabular Diff insertion, deletion, modification, schema, null, and context
  markers match the selected 0.8 contract.
- [ ] A zero diff passes only the declared scoped claim.
- [ ] Changed proof inputs or evaluator code make the old certificate stale.
- [ ] Capability drift makes affected actions/proofs stale without invalidating
  unrelated frames.
- [ ] Missing or tampered evidence cannot verify as passed.
- [ ] Zero-Spoke mode produces useful local frames, deltas, and scoped proofs.
- [ ] Commit/ref/index deltas activate only declared affected components.
- [ ] Ref rewrites and incomplete history never appear as false append events.
- [ ] Poll replay does not duplicate an idempotent component effect.
- [ ] Poll evidence does not claim unobserved intermediate index/worktree states.
- [ ] Polling executes no repository hook and makes no Git mutation.
- [ ] Missing required Spoke capability never silently weakens a claim.
- [ ] Local-only evidence and execution never claim Spoke durability, fencing,
  or query semantics.
- [ ] Routed Spoke persistence retains exact revision and request lineage across
  restart.
- [ ] CLI structured stdout is byte-clean and diagnostics remain on stderr.
- [ ] Every advertised limit is enforced and budget exhaustion is
  inconclusive, not convergence.

## Deferred

Long-running Git polling, frontier scheduling, multi-step loops, agent policies,
mutation, local and Spoke compute actions, full codebase lenses, incremental
physical execution, a Rey service, multi-user operation, and managed deployment
remain later plans.
