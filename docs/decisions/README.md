# Architecture Decisions

Accepted decisions constrain current implementation. Proposed decisions may
guide experiments but do not outrank the architecture until accepted.

| Decision | Status | Summary |
| --- | --- | --- |
| [0001](0001-diff-directed-runtime.md) | Accepted | Deltas direct compute; deterministic runtime and policy remain separate |
| [0002](0002-dataframes-typed-deltas-and-tabular-diff.md) | Accepted | Polars frames, Arrow interchange, authoritative typed deltas, and Tabular Diff projection |
| [0003](0003-spoke-and-proof-boundary.md) | Accepted; narrowed by 0005 | Connected Spoke owns durable reasoning/compute; Rey owns scoped proof and transition semantics |
| [0004](0004-rust-and-nix-development-foundation.md) | Accepted | Rust-first implementation and pinned Nix/Just/Crane development foundation |
| [0005](0005-environment-awareness-and-optional-spoke.md) | Accepted | Bounded capability discovery, standalone operation, and optional Spoke amplification |
| [0006](0006-rey-spoke-recursive-improvement.md) | Accepted | Rey as Spoke's first external runtime application and conformance feedback loop |
| [0007](0007-git-polling-and-delta-activation.md) | Accepted; public activation target narrowed by 0015 | Git commit/ref/index snapshots as pollable frames and idempotent activation sources |
| [0008](0008-first-slice-contracts.md) | Accepted | First executable schemas, Polars/Arrow baseline, semantic hashing, and bounded local/Git observation |
| [0009](0009-just-rey-task.md) | Accepted | Rename the Just CLI task to `rey` while retaining the Nix `dev` wrapper |
| [0010](0010-capability-delta-and-certificate.md) | Accepted | Typed capability deltas, deterministic Tabular Diff projection, and required-capability certificates |
| [0011](0011-local-proof-bundle.md) | Accepted | Bounded content-addressed local proof bundles with explicit publication and retention guarantees |
| [0012](0012-delta-directed-orientation.md) | Accepted | Formal bootstrap/transition lifecycle and bounded delta-directed reasoning surfaces |
| [0013](0013-runtime-state-and-reasoning-surface-contracts.md) | Accepted; v1 schemas superseded by 0014 | Executable runtime-state reducer and bounded reasoning-surface contracts before scheduling |
| [0014](0014-frontier-progress-and-scheduling.md) | Accepted; public identity cutover required by 0015 | Canonical frontier/progress relations, deterministic bounded work selection, and decision-bound runtime/surface v2 |
| [0015](0015-workload-centered-product.md) | Accepted | Workload-centered product, scenario-qualified compute graphs, and four-command CLI contract |

When a decision changes, add a superseding decision and link both documents.
Do not silently rewrite the context that led to an accepted choice.
