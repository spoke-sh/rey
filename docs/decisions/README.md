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
| [0007](0007-git-polling-and-delta-activation.md) | Accepted; public activation target narrowed by 0015 and env placement by 0033 | Git commit/ref/index snapshots as pollable frames and idempotent activation sources |
| [0008](0008-first-slice-contracts.md) | Accepted | First executable schemas, Polars/Arrow baseline, semantic hashing, and bounded local/Git observation |
| [0009](0009-just-rey-task.md) | Accepted | Rename the Just CLI task to `rey` while retaining the Nix `dev` wrapper |
| [0010](0010-capability-delta-and-certificate.md) | Accepted | Typed capability deltas, deterministic Tabular Diff projection, and required-capability certificates |
| [0011](0011-local-proof-bundle.md) | Accepted | Bounded content-addressed local proof bundles with explicit publication and retention guarantees |
| [0012](0012-delta-directed-orientation.md) | Accepted | Formal bootstrap/transition lifecycle and bounded delta-directed reasoning surfaces |
| [0013](0013-runtime-state-and-reasoning-surface-contracts.md) | Accepted; v1 schemas superseded by 0014 | Executable runtime-state reducer and bounded reasoning-surface contracts before scheduling |
| [0014](0014-frontier-progress-and-scheduling.md) | Accepted; public identity schemas superseded by 0016 | Canonical frontier/progress relations, deterministic bounded work selection, and decision-bound runtime/surface v2 |
| [0015](0015-workload-centered-product.md) | Accepted; first slice implemented by 0016 | Workload-centered product, scenario-qualified compute graphs, and four-command CLI contract |
| [0016](0016-first-workload-slice.md) | Accepted | Built-in zero-agent workload slice, local result index, typed scenario deltas, and workload identity cutover |
| [0017](0017-mining-capability-model.md) | Accepted | Relational and source mining operations, artifacts, diffs, visualization, and workload/runtime placement |
| [0018](0018-first-mining-workload.md) | Accepted | Source-search workload, typed relation/text deltas, evidence-linked CLI, and one delta-directed reasoning fixture |
| [0019](0019-git-shaped-environment-history.md) | Accepted; commit timestamp and human loop superseded by 0033 | Hard-cut `env` CLI, verified local environment commits, status, and patch-bearing linear history |
| [0020](0020-environment-mapping-graph.md) | Accepted | Human env diff, YAML variable/file/executable graph, safe observations, and removal of manual proof plumbing |
| [0021](0021-environment-admission-index.md) | Accepted | Unified environment status, HEAD-bound admission index, staged diff, partial add, and index-only commit |
| [0022](0022-portfolio-mining-and-workload-attention.md) | Accepted | Ongoing portfolio mining, typed workload attention, explicit coverage/blocker/exclusion evidence, and workload-centered CLI placement |
| [0023](0023-workspace-workload-packages.md) | Accepted | Workspace packages as the default product catalog, frozen generated scenarios, exact proposal provenance, and explicit built-in conformance catalog |
| [0024](0024-workload-creation-requests.md) | Accepted | Explicit content-addressed workload creation requests, external coding-harness handoff, visible drafts, and strict admission gating |
| [0025](0025-local-operator-ui.md) | Accepted | Read-only `rey ui`, loopback-first HTTP, embedded TanStack Router application, and pinned Hifi Kinetic Precision grammar |
| [0026](0026-context-topology-explorer.md) | Accepted | UI-first human operation, default context-topology Explorer, semantic zoom regimes, full-screen canvas, and passive revalidation |
| [0027](0027-environment-operator-delta.md) | Accepted; application presentation superseded by 0031 and status projection by 0033 | Shared CLI/UI environment delta, bounded value capture, env-shaped variable diff, and found/not-found application evidence |
| [0028](0028-environment-three-plane-diff.md) | Accepted | Three-plane human env diff for unstaged and staged directions with unchanged authoritative JSON |
| [0029](0029-environment-history-projection.md) | Accepted; commit header and schemas superseded by 0033 | Compact environment chronology and three-plane patch expansion over retained transitions |
| [0030](0030-operator-cadence-agents-and-explorer-coordinates.md) | Accepted; `/agents` superseded by 0034; matrix coordinates hard-cut by 0041; cadence v1 extended by 0036 | Historical partial-order cadence, provenance-derived agent registry, and matrix-style Explorer coordinates |
| [0031](0031-desired-application-inventory-and-search-records.md) | Accepted; conventional map superseded by 0032 | Exact desired-application declaration records separated from bounded search records |
| [0032](0032-seed-discovery-survey-and-live-communications.md) | Accepted | Process-owned seed discovery, explicit agent maps, locator survey, cadence processing, and a two-axis operator communication plane |
| [0033](0033-git-shaped-environment-loop-fidelity.md) | Accepted | Compact status, provenance-safe patch admission, dated v2 history, and Git/environment clock separation |
| [0034](0034-agent-runtime-inventory-and-derived-task-plane.md) | Accepted; `/agents` presentation superseded by 0035 | Process-owned agent runtime discovery, derived task/operation plane, and non-retained journeys |
| [0035](0035-agent-recommendations-and-observed-work.md) | Accepted; recommendation plane extended by 0037 | Evidence-ranked agent recommendations and observed-work ledger; runtime inventory remains in Environment |
| [0036](0036-cadence-repository-state-and-publication.md) | Accepted | Working-tree attention and exact local-upstream publication state in cadence |
| [0037](0037-explore-bound-collaboration-journal.md) | Accepted | Shared human, agent, and system Journal entries bound to exact Explorer coordinates |
| [0038](0038-unauthenticated-hyperlinkable-journal.md) | Accepted | Unauthenticated Journal writes plus exact entry and block hyperlinks |
| [0039](0039-bounded-operator-feed.md) | Accepted | Bounded high-cadence inspection queue and signal feed without inventing a global event log |
| [0040](0040-workspace-channels-and-operator-index.md) | Accepted | Standalone frontier observations, Journal seeds, workspace-local channel graph, Git-shaped operator index, Feed persistence, and relay boundary |
| [0041](0041-continuous-coordinate-topography.md) | Accepted | Hard-cut semantic coordinate/scale links, continuous lens, admitted survey voyages, and incremental evidence-backed context topography |
| [0042](0042-world-geometry-and-probe-navigation.md) | Accepted; transport/path projection superseded by 0043 | Six-level World projection and probe horizons |
| [0043](0043-emergent-natural-features-and-separate-paths.md) | Accepted | Anchor-field relief, unresolved weather, runoff hydrology and erosion, and separate discovered/constructed path evidence |

When a decision changes, add a superseding decision and link both documents.
Do not silently rewrite the context that led to an accepted choice.
