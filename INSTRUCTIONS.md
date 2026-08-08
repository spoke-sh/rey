# Contributor Instructions

Procedural guidance for humans and agents working on Rey.

## Read First

1. `README.md` for scope, model, and current status.
2. `CONSTITUTION.md` for durable values and invariants.
3. `docs/ARCHITECTURE.md` for ownership and data flow.
4. `docs/ENVIRONMENT.md` before changing providers, discovery, tools, profiles,
   or capability admission.
5. `docs/GIT.md` before changing repository identity, commit/ref/index polling,
   cursors, triggers, or application activation.
6. `docs/DIFFS.md` before changing frames, comparison, normalization, or
   renderings.
7. `docs/PROOFS.md` before changing claims, certificates, staleness, or evidence.
8. `docs/INTERFACES.md` before changing the CLI or Spoke integration.
9. `docs/DEVELOPMENT.md` before changing the toolchain or root tasks.
10. `plans/README.md` and the active plan before implementation work.
11. `docs/decisions/README.md` for accepted choices that constrain the work.

## Working Loop

1. **Orient** — inspect current files, accepted decisions, active plans, the
   available environment, and relevant Spoke contracts when present.
2. **Bind** — identify the exact source revisions and observable claim affected
   by the work.
3. **Bound** — choose the smallest end-to-end behavior and explicit resource
   limits.
4. **Decide** — record consequential or hard-to-reverse choices before they
   spread through code and formats.
5. **Change** — preserve the boundary between deterministic runtime, policy,
   Spoke, and presentation.
6. **Prove** — run focused checks and retain useful, bounded evidence.
7. **Record** — update documentation and plan checklists in the same change.

## Current Development Interface

Rey has a pinned Nix Rust toolchain, a seven-crate Cargo workspace, an
environment-inspection and capability-delta/certificate executable, and a root
Just task surface. Do not generalize the implemented capability-specific delta
and required-capability certificate into generic frame, activation, durable
retention, or Spoke behavior.

Enter the environment and use:

```sh
nix develop
just setup
just rey
just check
just test
just build
just fmt
```

All six tasks are backed by the current Cargo workspace. `rey` runs the CLI;
`check`, `test`, and `build` execute real workspace verification. See
`docs/DEVELOPMENT.md` for the exact behavior. Documentation changes should pass
`just check` and manual link/repository-truth review.

## Runtime Work

- Keep frame construction, typed comparison, frontier selection, action
  admission, and proof evaluation deterministic and usable without an LLM.
- Keep the standalone runtime deterministic and useful without Spoke.
- Treat policy as an external decision source. Policy output is an untrusted
  proposal until the runtime validates identity, revisions, effects, and limits.
- Keep all queues, results, captures, traversals, iterations, and concurrency
  bounded.
- Make cancellation and partial failure visible at each observation and action
  boundary.
- Prefer one-way capability dependencies. Shared crates stay narrow and own
  semantics actually shared by multiple capabilities.
- Do not confuse standalone providers with a local Spoke storage bypass. When
  connected to Spoke, a same-host integration still uses the documented public
  or explicitly internal service contract.

## Frame And Diff Work

- Preserve one logical schema across Polars, Arrow, structured output, and
  terminal rendering.
- Require explicit comparison keys for unordered relations and validate their
  uniqueness before alignment.
- Include comparison direction, labels, source revisions, lens revisions,
  normalizers, and limits in delta identity.
- Keep typed before/after values in the structured delta even when a rendering
  uses strings.
- Use Tabular Diff 0.8 as a projection for compatible tabular comparisons, not
  as the only internal representation.
- Do not stringify relational data merely to obtain a text diff, and do not
  force source text or binary content into synthetic rows without a genuine
  relational contract.
- Treat an empty frame as a typed empty relation. Preserve its declared schema
  so missing rows can be represented rather than failing key resolution.

## Proof Work

- Separate `failed`, `inconclusive`, and `stale`; they lead to different next
  actions.
- Treat similarity, progress, confidence, and coverage as distinct quantities.
- Never infer coverage from a passing diff alone.
- Hash evaluator and normalizer implementations into proof inputs so changed
  semantics invalidate previous certificates.
- Make omitted frames, ignored fields, unsupported limits, and truncated output
  reviewable evidence.
- Verify a certificate by recomputing input identity and checking referenced
  evidence, not by trusting a stored `passed` field.

## Environment Awareness

- Discover environment capabilities through bounded provider contracts rather
  than arbitrary shell startup scripts.
- Keep discovery read-only. A known version probe has a timeout and output
  bound; discovery never executes arbitrary files merely because they are on
  `PATH`.
- Record provider identity, resolved executable path, version,
  digest/provenance when available, trust class, supported actions, and limits.
- Revalidate the capability snapshot at action admission. Tool or provider
  drift makes a proposal stale.
- Make standalone, connected, and required-capability behavior explicit in
  configuration and evidence.
- Missing capabilities remove actions or make dependent claims inconclusive;
  they do not silently select a weaker proof contract.

## Spoke Integration And Co-Evolution

- Use exact Spoke identities and revisions in source bindings; mutable paths or
  names alone are insufficient.
- Keep `QUERY` safe and idempotent. Effects use explicit resource methods or
  admitted compute runs.
- Let Spoke own tool resolution, process attempts, fencing, cancellation,
  captures, and durable process lineage.
- Let Rey own observation definitions, action rationale, frame/delta lineage,
  frontier selection, claim evaluation, and proof assembly.
- Preserve Spoke request ids, revisions, checkpoints, run ids, attempt ids,
  capture digests, and errors in Rey evidence where relevant.
- Test the direct Spoke contract and the same behavior through its routed public
  surface before claiming first-class integration.
- Keep the core standalone path capable of diagnosing Spoke while Spoke is
  unavailable, broken, or being changed.
- Turn Rey-discovered Spoke gaps into versioned conformance fixtures and proof
  artifacts. When Spoke closes a gap, expose the capability through discovery
  and exercise it from Rey.
- Do not make Spoke depend on Rey to boot or make Rey depend on Spoke to run its
  foundation tests.

## Git Polling And Activation

- Bind repository identity, object format, worktree identity, HEAD, watched
  refs, semantic index digest, and declared worktree state explicitly.
- Treat Git OIDs as opaque algorithm-qualified identities.
- Derive staged triggers from logical index entries, not the index file's mtime
  or stat-cache-only changes.
- Keep ref creation, deletion, fast-forward, rewind, rewrite/divergence, and
  unknown history distinct.
- Do not claim an ordered commit append across rebase, reset, force-push, or
  incomplete shallow history.
- Poll with no optional locks and never run hooks, aliases, filters, credential
  helpers, fsmonitor hooks, submodule commands, or mutations during discovery.
- Advance a cursor only after activation evidence commits. Expect replay after
  crashes and use deterministic activation ids plus action idempotency.
- A trigger creates an activation proposal; it never bypasses effect admission.

## Plans And Decisions

- Plans in `plans/` are executable checklists with a top-level completion list.
- Mark only repository-proven facts complete.
- Put open choices in the active plan; put accepted consequential choices in an
  ADR.
- Add a superseding ADR rather than silently rewriting why an accepted decision
  was made.
- Update the plan and decision indexes whenever status changes.

## Hygiene

- Prefer `rg` for repository search and the documented root tasks once they
  exist.
- Keep generated Arrow, CSV, trace, certificate, and benchmark outputs out of
  source control unless deliberately maintained as small fixtures.
- Never commit credentials, Spoke tokens, model provider keys, or private source
  snapshots.
- Avoid unresolved ambient host paths, environment variables, timestamps, or
  random values in semantic identities. Resolved environment capabilities are
  explicit inputs and must be recorded when they affect semantics.
- Do not claim deterministic, incremental, portable, or reproducible behavior
  without focused proof.
