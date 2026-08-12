# Current Plans

Plans contain only current executable work and logically ordered follow-on
work. Completed and superseded delivery history lives in Git; implemented
truth lives in the foundational docs and tests. The
[current decision plane](../docs/decisions/README.md) explains the accepted
structure these plans must preserve.

## Execution Order

| Plan | State | Next verifiable closure | Depends on |
| --- | --- | --- | --- |
| [0001 — Close the runtime loop](0001-runtime-loop.md) | Active | Add path-level activation evidence, then close retry, cancellation, and partial-failure recurrence bounds. | Implemented ownership/invalidation edge proofs, exact watched-ref, bounded reachability, and complete supported semantic-index polling, attention/frontier and harness handoffs, bounded Git cadence, exact activation admission/execution, compatible result reuse, and selected-versus-full recomputation proof. |
| [0002 — Close the collaboration loop](0002-collaboration-loop.md) | Active | Project the implemented Channel graph/messages into the operator surface, then admit one exact observation and derive its unresolved frontier without dirtying topology. | Implemented Channel CLI, Journal, Feed, and runtime evidence. Plan 0001 only where an authored opportunity becomes runtime work. |
| [0003 — Admit scenes and complete Explorer](0003-scene-to-explorer.md) | Active | Qualify one exact editor package through a scene-admission workload and expose the admitted regional result through the CLI before broadening the browser grammar. | Implemented editor, workload admission, projection packet, semantic atlas, and renderer boundary. |

The plans may advance in parallel when they do not share a contract, but their
authority dependencies remain ordered:

```text
runtime ownership/invalidation ───────────────┐
                                              ▼
Channel or Journal proposal ────────→ admitted runtime work

editor candidate → scene admission → retained atlas delta
                 → World/Atlas/County projection → fidelity proof
```

## Delivered Baseline

The current repository already provides:

- a twelve-crate Rust workspace, pinned Nix/Just development surface, GitHub
  CI, and cargo-dist release plan;
- explicit local environment, workload, editor, and Channel
  `HEAD → INDEX → WORKING` loops;
- deterministic workload DAG/scenario qualification, literal source mining,
  typed deltas, topography survey, portfolio attention, frontier/scheduling,
  reasoning surfaces, and local proof mechanisms;
- an embedded operator UI with Feed, Explorer, Cadence, Environment,
  Workloads, Journal, passive revalidation, and exact Git links;
- immutable Channel messages, explicit relay and one-shot beacon commands;
- a bounded Git cursor/pending-transition loop with independently classified
  HEAD and exact watched refs, complete supported semantic-index evidence,
  retained cadence ticks/receipts, proposal-only
  activations, exact workload admission, replay-stable selected-scenario
  execution, and strict same-transition coalescing;
- candidate-only native scene authoring and procedural terrain generation; and
- a read-first Explorer with consent-first orientation, semantic World globe,
  synthetic atlas, continuous terrain, WebGPU/WebGL2 acceleration, and a
  deterministic accessible fallback.

Those facts are not repeated as completed checklists in this directory.

## Later Roadmap

[Roadmap](../docs/ROADMAP.md) retains future bearings that are not yet bounded
enough for an executable plan: general admitted mutation, codebase spaces,
provider-neutral agent policies, and scaled deployment. Promote one of those
bearings into a numbered plan only when it has an exact current gap, a smallest
end-to-end CLI slice, explicit bounds, and qualification criteria.

## Plan Rules

- State the current implementation boundary before proposed work.
- Keep a top-level completion checklist ordered by dependency.
- Mark only repository-proven facts complete; do not preserve old command
  transcripts as a substitute for current proof.
- Close the high-fidelity human CLI path before treating browser projection or
  provider advertisement as feature completion.
- Put open choices in the plan. Put accepted durable structure in the owning
  foundational document and current decision plane.
- Remove a plan when its required acceptance boundary is complete; Git retains
  the delivery history.
