# Current Decision Plane

This directory is a projection of Rey's accepted decisions as they constrain
the repository now. It is not a chronological ADR archive and does not repeat
the complete subject contracts. Superseded rationale remains available in Git
history; current structure belongs in the foundational document that owns it.

Use this projection after the [Constitution](../../CONSTITUTION.md) and the
relevant subject documents. When a summary here conflicts with a foundational
contract, the foundational contract wins and this projection must be corrected
in the same change.

## Plane Map

| Plane | Accepted structural choice | Authoritative contract | Implementation posture |
| --- | --- | --- | --- |
| Product and evidence | Rey is a client-oriented, diff-directed mining and compute runtime. Deltas direct subsequent bounded work; proof remains scoped to exact evidence, coverage, and limits. | [Architecture](../ARCHITECTURE.md), [Diffs](../DIFFS.md), [Proofs](../PROOFS.md) | Typed relational and text deltas, capability certificates, and local proof bundles exist; general structural comparison and complete proof orchestration remain partial. |
| Data and artifacts | Polars DataFrames are canonical bounded in-process relations and Arrow is preferred typed interchange. Native source, documents, trees, graphs, geometry, raster, and binary artifacts retain their own identity rather than being counterfeited as tables. | [Architecture](../ARCHITECTURE.md), [Mining](../MINING.md), [Diffs](../DIFFS.md) | Frame/Arrow helpers and typed relations exist beside native text and GeoJSON artifacts. |
| Environment and providers | Bootstrap begins only from process-owned `HOME`, `PWD`, and `PATH`, compiled adapters, and explicitly supplied maps. Discovery is bounded and read-only; finding an executable grants no action authority. Providers own their actual guarantees. | [Environment](../ENVIRONMENT.md), [Locators](../LOCATORS.md), [Interfaces](../INTERFACES.md) | The implemented profile is local and provider-independent. No remote provider is privileged or implied. |
| Admission and revision state | Environment, workload, editor, and Channel topology use explicit `HEAD → INDEX → WORKING` comparison planes. `add` freezes exact INDEX; `commit` consumes INDEX without rereading WORKING. Each plane retains distinct authority. | [CLI](../CLI.md), [Environment](../ENVIRONMENT.md), [Workloads](../WORKLOADS.md), [Interfaces](../INTERFACES.md) | All four local loops exist. Workloads additionally require fresh complete staged qualification before HEAD. |
| Workloads and mining | A workload is the public unit of computation: one graph, scenario suite, policy boundary, qualification contract, and total budget. Relational and source mining are peer capability families. Workspace packages are visible file-backed proposals under `sys/`; compiled fixtures are explicit conformance-only inputs. | [Workloads](../WORKLOADS.md), [Mining](../MINING.md), [Runtime](../RUNTIME.md) | Workspace admission, deterministic DAG/scenario execution, literal source mining, topography survey, and portfolio-attention conformance paths exist. Harness invocation and the recurring improvement loop do not. |
| Frontier, runtime, and policy | Portfolio attention explains why work matters; the frontier represents bounded unresolved work; deterministic scheduling selects ready rows. Policy may propose but cannot redefine evidence, bypass admission, or declare its own proposal resolved. | [Frontier](../FRONTIER.md), [Runtime](../RUNTIME.md), [Workloads](../WORKLOADS.md) | Canonical frontier, progress, scheduling, runtime-state, reasoning-surface, live environment/Git invalidation, generic attention handoff, and one immutable harness response cycle exist. Recurring policy execution remains planned. |
| Git and activation | Commit, ref, semantic index, and declared worktree state are first-class inputs. Ref movement is classified rather than flattened into append events; cursors advance only after retained transition evidence and replay is idempotent. | [Git](../GIT.md), [Runtime](../RUNTIME.md) | Bounded observation, exact retained watched-ref scope and movement, bounded added/removed reachability and tree path deltas, complete supported semantic-index flags, retry/cancellation/partial-failure Cadence receipts, local cursor/pending/history retention, proposal-only exact-ref/raw-path-prefix trigger matching, acknowledged-cursor invalidation, exact admission/selected-scenario execution, strict same-transition result reuse, and bounded selected-versus-full recomputation proof exist. Graph-entry activation, cross-poll debounce, and autonomous activation scheduling remain planned. |
| Operator surface | Humans normally collaborate through `rey ui`; agents use the `rey` CLI. Both project the same typed evidence and authority boundaries. The UI is an explicitly started local operator surface, not a public Rey service. | [CLI](../CLI.md), [Explorer](../EXPLORER.md), [Interfaces](../INTERFACES.md) | The embedded UI, passive revalidation, Feed, Cadence, Channels, Environment, Workloads, Journal, and Explorer routes exist. Browser writes remain bounded to Journal admission, Channel WORKING, and exact workload approval; scenario/delta deep inspection remains incomplete. |
| Collaboration | Feed is a bounded high-cadence projection, Channel topology and observations are separate retained planes, Journal is retained synthesis, and the footer conversation axis is a separate transport boundary. Feed resolves detached URL preview before WORKING, HEAD, and built-in layouts; adoption and stable movement write only WORKING. Observation broadcast retains local admission edges and partial outcomes without granting relay authority. None aliases another. | [Observations](../OBSERVATIONS.md), [Journal](../JOURNAL.md), [Explorer](../EXPLORER.md), [CLI](../CLI.md), [Interfaces](../INTERFACES.md) | Channel graph history, browser status/WORKING replacement, Feed layout resolution/adoption/movement, immutable observation/admission/resolution/frontier storage and CLI, transport messages, explicit relay, one-shot beacons, Journal broadsheets, and Feed projection exist. Observation browser projection, Journal seeding, and a conversation transport do not. |
| Explorer and coordinates | Explorer is a read-first projection engine over admitted evidence. Semantic coordinate identity is separate from camera and view state. A fresh workspace shows exact workload beacons on an explicitly unmapped orientation globe; it does not infer an atlas or run a survey. | [Explorer](../EXPLORER.md), [Locators](../LOCATORS.md), [Mining](../MINING.md) | Orientation, admitted survey topography, a synthetic semantic atlas, World globe rotation, local terrain, and exact view links exist. Semantic Mercator, retained sector deltas, and admitted County entry remain planned. |
| Scene authoring and rendering | Editor commits are immutable candidate packages, never Explorer evidence. A qualified scene-admission workload is the only path from a package to an admitted regional scene. Terrain is a deterministic program with disposable camera-relative working sets. Three.js WebGPU/TSL is the accelerated renderer, WebGL2 its compatibility path, and the deterministic accessible renderer the semantic fallback. | [Explorer](../EXPLORER.md), [Mining](../MINING.md), [CLI](../CLI.md), [Interfaces](../INTERFACES.md) | Candidate GeoJSON authoring, terrain generation, projection packets, procedural working sets, continuous relief, and globe rendering exist. Scene admission, complete render-graph separation, County layers, and retained visual/performance qualification do not. |
| Delivery | Rust and the pinned Nix shell define the implementation and development boundary. Root `just` tasks are canonical. GitHub Actions runs the same checks; cargo-dist publishes only from an intentional matching semantic-version tag. | [Development](../DEVELOPMENT.md), [Releases](../RELEASES.md) | The twelve-crate workspace, embedded UI build, CI, release planning, and native artifact configuration exist. |

## Cross-Plane Invariants

The current choices compose through four paths:

```text
context → mine → evidence → delta → attention/frontier
        → schedule → reason → propose → admit → act → observe

WORKING → INDEX → qualification where required → HEAD

editor WORKING → SCENE@n candidate → scene-admission workload
               → admitted regional scene → projection packet → /explore

runtime evidence → Feed / mailbox / Journal / Channel projections
                 ≠ conversation transport or execution authority
```

- Exact inputs, revisions, capabilities, limits, omissions, completeness, and
  lineage survive every path.
- Read, admission, execution, assignment, publication, and proof authority
  remain separate.
- A renderer, model, process exit, author label, or visual analogy cannot mint
  evidence or convergence.
- Current local guarantees are never silently upgraded to remote durability,
  authentication, federation, or provider semantics.
- Pre-alpha contract changes are hard cutovers unless an active plan explicitly
  accepts migration behavior. Compatibility code is not retained by default.

## Changing The Decision Plane

A consequential change updates three places together:

1. the owning foundational document, including current versus target posture;
2. the corresponding row or invariant in this projection; and
3. the active plan and proof path that will make the change repository truth.

Do not create a chronological decision file for each implementation slice.
Git preserves superseded rationale. Add a separate record only when a retained
external compatibility, governance, or migration obligation cannot be stated
coherently in the current plane and its owning contract.
