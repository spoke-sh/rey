# Plan 0024: Workload Admission History

Implement ADR 0049 as the smallest end-to-end human/agent admission loop.

## Completion Checklist

- [x] Remove package-owned admission declarations and reject the old shape.
- [x] Implement verified workload HEAD, INDEX, WORKING snapshots and semantic
  change sets.
- [x] Implement `status`, `diff`, `add`, `commit`, and `log` with human and JSON
  renderings.
- [x] Bind workspace qualification to `test --staged` and the exact complete
  INDEX snapshot.
- [x] Resolve `run` only from admitted HEAD and make `list` preserve plane
  boundaries.
- [x] Enter the browser through the admission Feed, show INDEX/WORKING
  candidates, and approve with exact HEAD/INDEX preconditions.
- [x] Keep browser approval enabled for explicitly selected non-loopback binds
  and expose the unauthenticated authority in startup evidence.
- [x] Remove `rey.portfolio.label-normalization` from the product catalog.
- [x] Add unit and CLI proofs for frozen-index commit, continued WORKING edits,
  exact-index requalification, and non-loopback approval exposure.
- [x] Stage and qualify the repository's fresh `context-anchor-survey` local
  proposal without admitting it, leaving the final decision to the operator.
- [x] Pass repository formatting, lint, UI, and full test qualification.

## Human Verification

```text
rey workloads status
rey workloads diff
rey workloads add
rey workloads test --staged context-anchor-survey -vv
rey workloads status
rey ui --host 0.0.0.0
```

The final status must say the exact INDEX is qualified and awaiting human
approval. The UI must open on the admission Feed and show
`context-anchor-survey`; this plan does not approve it.

## Current Proof

Captured on 2026-08-11:

```text
rey workloads status
# no commits yet; context-anchor-survey staged; exact INDEX qualified and
# awaiting human approval

just check
# Rust formatting, Clippy -D warnings, 75 UI tests, production UI build, and
# Nix flake evaluation passed

just test
# 75 UI tests and 186 Rust tests passed; all workspace doc tests passed
```

The server proof stages and qualifies a survey, posts an exact expected
HEAD/INDEX approval to `/api/v1/workloads/commit`, and observes `WORKLOAD@1`.
The repository-local operator state is a separate candidate and remains
uncommitted for human review.
