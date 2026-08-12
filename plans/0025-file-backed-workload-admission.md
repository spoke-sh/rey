# Plan 0025: File-Backed Workload Admission

Implement ADR 0050 so a fresh workspace can expose and admit visible workload
packages without pre-populated `.rey` state.

## Completion Checklist

- [x] Move the system survey package to `sys/context-anchor-survey/` and make
  `sys` the default CLI/UI catalog root.
- [x] Preserve exact request, package, generation-input, and source identities
  after the move.
- [x] Add an expected-WORKING precondition to exact package staging.
- [x] Make browser admission stage the reviewed WORKING snapshot, run its
  complete frozen scenario suites, and commit only qualified INDEX bytes.
- [x] Rename the browser mutation route and control to describe file-backed
  admission rather than pre-staged INDEX approval.
- [x] Prove browser admission from a workspace with visible package files and
  no local workload state.
- [x] Pass full repository formatting, lint, UI, and Rust qualification.

## Current Proof

Captured on 2026-08-11:

```text
rey workloads status
# working · INDEX empty · sys/context-anchor-survey/workload.yaml visible

just check
# UI formatting/typecheck/tests/build, Rust formatting/Clippy, and Nix flake
# evaluation passed

just test
# 75 UI tests and 187 Rust tests passed; all workspace doc tests passed
```

The server proof begins with package files and no workload state. It rejects a
changed expected-WORKING digest without creating state, then restores and
admits the exact reviewed file snapshot, observing `WORKLOAD@1`, an empty
INDEX, and clean HEAD/WORKING state.

## Human Verification

```text
# In a fresh workspace with no .rey/workloads/state.json:
rey workloads status
rey ui
```

`status` must show `sys/context-anchor-survey/workload.yaml` in WORKING with an
empty INDEX. The admission Feed must enable `ADMIT EXACT FILE SNAPSHOT`; the
action must qualify the exact file snapshot and advance `WORKLOAD@1`, or retain
diagnostic evidence while leaving HEAD unchanged.
