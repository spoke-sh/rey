# Plan 0026: Typed Workspace Ignore File

Implement ADR 0051 as a bounded, omission-visible WORKING scope contract.

## Completion Checklist

- [x] Parse a workspace-root `.reyignore` with comments, typed kinds, exact and
  `*`/`?` wildcard patterns, and strict path/encoding/size/count bounds.
- [x] Apply `workload` rules after validating workspace packages and drafts.
- [x] Apply `environment variable`, `application`, `input`, and `reference`
  rules to environment WORKING observations.
- [x] Bind relevant rule/file/match evidence into workload and environment
  WORKING identities.
- [x] Render source, rule count, match counts, source lines, and digest in
  human and structured status; retain UI omission summaries.
- [x] Test exact/wildcard matching, malformed input, symlink rejection, and
  end-to-end CLI filtering.
- [x] Pass full repository formatting, lint, UI, and Rust qualification (`just
  check`; 75 UI tests and 192 Rust/CLI tests through `just test`).

## Human Verification

```text
printf '%s\n' \
  'workload: context-anchor-survey' \
  'environment variable:*' > .reyignore

rey workloads status
rey env status
rey ui
```

Both status documents must identify `.reyignore`, its exact digest, relevant
rules, and match counts. The workload must disappear only from WORKING; process
environment variables must be absent from WORKING; already-retained HEAD and
INDEX state must remain intact.
