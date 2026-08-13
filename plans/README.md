# Current Plans

Plans contain only current executable work and logically ordered follow-on
work. Completed and superseded delivery history lives in Git; implemented
truth lives in the foundational docs and tests. The
[current decision plane](../docs/decisions/README.md) explains the accepted
structure these plans must preserve.

## Execution Order

| Plan | State | Next verifiable closure | Depends on |
| --- | --- | --- | --- |
| [0003 — Admit scenes and complete Explorer](0003-scene-to-explorer.md) | Active | Finish projection-engine ownership separation and renderer qualification. | Implemented editor-to-admission-to-Explorer voyage, deterministic bounded native source registration and review, exact source-bound regional height/material samples with no interpolation, projection packets, retained survey/regional atlas history with exact scene back-references, stable occupied sectors, typed deltas, reversible Mercator geometry, immutable/continuous World/Atlas transition, bounded wrapping/inverse picking/recentering, focus-preserving deterministic label layout, explicit regional selection, a verified reversible envelope-centered County frame, exact admitted footprint fabric/validity, independent admitted native layer kinds, exact selected-object evidence routes, and bounded haloed absolute-coordinate survey-terrain patches with exact seams and a byte/cell-bounded retained cache. |

The plans may advance in parallel when they do not share a contract, but their
authority dependencies remain ordered:

```text
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
- an embedded operator UI with Channel-backed Feed layout adoption, Explorer,
  Cadence, Environment, Workloads, Journal, bounded conversation transcripts,
  passive revalidation, exact Git links, and exact scenario/delta routes;
- immutable Channel messages, explicit relay and one-shot beacon commands;
- immutable local conversation sessions and append-only messages with exact
  declared writers, conditional browser admission, and no delivery or
  execution claim;
- a bounded Git cursor/pending-transition loop with independently classified
  HEAD and exact watched refs, bounded reachability/path deltas, complete
  supported semantic-index evidence, retained cadence ticks/receipts,
  explicit retry/cancellation/partial-failure stops, proposal-only activations,
  exact workload admission, replay-stable selected-scenario execution, and
  strict same-transition coalescing;
- candidate-only native scene authoring, procedural terrain generation, and a
  qualified scene-admission bridge whose latest accepted production result is
  the only regional scene input to Explorer; and
- a read-first Explorer with consent-first orientation, semantic World globe,
  synthetic atlas, bounded admitted County object projection, continuous survey
  terrain, WebGPU/WebGL2 acceleration, and a deterministic accessible fallback.

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
