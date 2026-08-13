# Research References

This document is Rey's reusable register of external research that materially
informs its architecture or delivery. Each entry identifies the reviewed
source, summarizes the relevant mechanisms, maps them onto Rey's contracts,
and records promising incorporation paths and non-fit boundaries.

References are inputs to design reasoning, not accepted decisions, plans, or
implementation evidence. The [Constitution](../CONSTITUTION.md), foundational
contracts, [current decision plane](decisions/README.md), active
[plans](../plans/README.md), code, and tests remain authoritative in that
order. A research result cannot by itself establish a provider guarantee,
qualify a workload, admit a scene, or prove a Rey claim.

## Entry Format

Each entry should include:

- exact primary sources, publication or artifact revision, and review date;
- the research problem and the mechanisms that matter to Rey;
- a mapping from those mechanisms to existing Rey concepts;
- incorporation candidates ordered by value and dependency;
- boundaries where the source's assumptions conflict with Rey; and
- an evidence posture covering evaluation, reproducibility, and open
  questions.

Prefer primary sources. Pin a paper version, release, commit, or access date
when the source can change. Clearly separate what the source demonstrates from
what Rey infers or proposes.

## Index

| Reference | Area | Reviewed source | Rey bearings |
| --- | --- | --- | --- |
| [WorldClaw: Agentic 3D Open-World Generation at Scale](#worldclaw-agentic-3d-open-world-generation-at-scale) | Agentic world construction, procedural terrain, regional realization, visual refinement | arXiv `2608.05248v1`, 2026-08-05 | Explorer, scene editor, mining, workloads, runtime, proof |

## WorldClaw: Agentic 3D Open-World Generation at Scale

### Source Identity

- Chunchao Guo, Jinpeng Li, Yang Li, and Zilong Huang, *WorldClaw: Agentic 3D
  Open-World Generation at Scale*.
- Primary artifacts: [project page](https://tencent-hunyuan.github.io/Hunyuan3D-WorldClaw/),
  [arXiv `2608.05248v1`](https://arxiv.org/abs/2608.05248), and the linked
  [project repository](https://github.com/Tencent-Hunyuan/Hunyuan3D-WorldClaw).
- Reviewed: 2026-08-13.

### Research Summary

WorldClaw builds an explicit, explorable, editable 3D world from an open-ended
prompt through a coarse-to-fine agentic pipeline. It separates three stages:
intent analysis and scene planning, global terrain construction, and selective
regional object generation and placement. The stages communicate through
structured intermediate representations rather than repeatedly interpreting
the original prompt.

The planning stage first extracts only constraints stated by the user, then
separately completes information required by later modules. Its shared scene
specification names regions, terrain constraints, object constraints,
appearance, and spatial relationships. Terrain planning lowers those concepts
into a semantic layout, numerical landform parameters, reusable assets, and
materials. A region-aware height field combines base elevations,
multi-frequency noise, geomorphic operators, and softened regional weights.

Detailed content is realized only in selected regions. WorldClaw renders the
existing local terrain from a recorded camera, uses that image as a constrained
composition surface, segments individual objects, reconstructs editable
textured meshes, and recovers each placement through explicit camera and crop
transforms. It then runs bounded render-inspect-edit loops over terrain,
materials, object pose and scale, and object-terrain contact. The result keeps
global terrain and instance-level objects independently editable. The project
page presents appearance, instance-mask, normal, and depth channels from global,
regional, and walk-level views.

The central result is architectural rather than a new monolithic world model:
global coherence can be established once while local detail is progressively
and selectively realized against that shared structure.

### Fit With Rey

| WorldClaw mechanism | Rey-aligned interpretation |
| --- | --- |
| Global-to-regional, coarse-to-fine construction | Reinforces Rey's World → Atlas → County → Object → Evidence grammar and its sparse camera-relative working sets. Rey should compile only admitted detail needed by a lens or declared workload, while keeping one stable global semantic identity. |
| Shared structured intermediate representations | Supports explicit, versioned candidate scene plans between intent, terrain, regional objects, and rendering. In Rey these remain native artifacts plus genuinely relational indexes, with exact producer, source, schema, limits, and revision lineage. |
| Explicit-constraint extraction before missing-information completion | Suggests a provenance ledger that distinguishes source-asserted constraints from policy-proposed completions and deterministic defaults. Only the former are observations; completions remain reviewable candidate choices until qualified and admitted. |
| Semantic region layout driving terrain, materials, and scattering | Suggests one typed region relation plus native boundary or mask artifacts reused by field compilation and layer placement. Rey must keep semantic membership, surveyed validity, visual blend weights, and material selection as separate channels. |
| Selective regional realization | Matches delta/frontier-directed work: unresolved regional requirements may justify a bounded scene workload without regenerating the whole atlas. Camera navigation can expose the opportunity but must never schedule or execute it. |
| Independent meshes and placement transforms | Supports separating stable object identity, native asset revision, appearance revision, and placement transform so local replacement or movement produces a narrow directed delta rather than a new opaque scene. |
| Render-inspect-local-edit loops | Maps to a bounded candidate-side diagnostic workload: render exact viewpoints, derive typed issues, propose local WORKING edits, re-render, compare, and stop on qualification or an explicit budget. An agent may propose the edit but cannot pass its own scenario or admit the result. |
| Appearance, instance, normal, and depth render channels | Suggests richer Explorer qualification bundles. Object-id, depth, normal, validity/no-data, and final-color captures can verify picking, occlusion, contact, layer separation, and fallback parity more precisely than color screenshots alone. |
| Procedural and generative tools used together | Reinforces code-native, parameterized scene programs where they improve reproducibility and editability. Generated programs, meshes, textures, and material graphs stay in the editor candidate plane and bind their tool/model capability snapshot before admission. |

### Incorporation Candidates

#### 1. Add semantic render-diagnostic bundles

After the current direct-browser transport closure, extend named Explorer
voyages with a versioned diagnostic manifest over the exact scene, camera,
viewport, backend, render graph, and source revisions. Retain bounded final
color, object/instance identity, depth, normal, and validity/no-data channels.
Use structural assertions for identities, masks, bounds, and channel
relationships; use pixel comparison only for explicitly qualified visual
claims.

This is the highest-value near-term transfer because it strengthens Rey's
existing renderer proof without changing evidence authority. It can catch an
object that renders correctly but picks as the wrong identity, a visually
hidden validity leak, an incorrect depth ordering, or a backend fallback that
preserves color while losing semantic layers.

#### 2. Preserve constraint origin in scene planning

Introduce a candidate-side scene-plan artifact only when an editor workload
needs it. Every constraint should carry an origin such as `source_asserted`,
`human_authored`, `policy_proposed`, or `deterministic_default`, together with
its source or proposal identity. Ambiguity and missing information should
remain explicit instead of being silently filled.

The plan may organize regions, desired layers, appearance, object roles,
spatial relationships, and budgets, but it grants no coordinate, retrieval,
execution, or evidence authority. Qualification should reject a plan that
relabels a proposed completion as observed context.

#### 3. Make refinement a typed bounded workload

Represent visual and geometric defects as a typed issue relation rather than
an agent's free-form success report. A row can bind the target object or terrain
support region, issue class, diagnostic viewpoint and channels, expected
predicate, proposed local edit, attempt count, consumed budget, and status.

The loop should be:

```text
exact candidate + diagnostic views
  → typed issue delta
  → bounded candidate edit proposal
  → deterministic re-render and checks
  → residual issue delta
  → qualify, continue, or stop at an explicit bound
```

Edits remain in editor WORKING. Only ordinary INDEX review, scene commit,
workload qualification, and admission can affect Explorer.

#### 4. Separate reusable asset identity from instances

When repeated scene objects become a concrete requirement, add an explicit
prototype/instance contract. A prototype binds native geometry, material,
generator or source revision, and intrinsic bounds. Each instance binds the
prototype revision, stable semantic identity, placement transform, regional
membership, and local overrides. Deltas can then distinguish replacement,
movement, appearance change, and insertion/removal.

This should follow a real workload need; Rey should not introduce a general
asset database or force native artifacts into DataFrames in anticipation.

#### 5. Let frontiers propose regional realization

A future scene or survey workload may derive an attention row when an admitted
region has unresolved object requirements, insufficient valid terrain support,
or stale diagnostic evidence. The generic scheduler may select that row, and a
policy may propose a bounded regional workload. The reason, readiness,
required capability, source scope, expected delta, and total budget must remain
typed and reviewable.

This is progressive realization directed by evidence, not view-dependent
generation. Panning, zooming, selecting, or opening a route remains read-only.

### Boundaries And Non-Fit

- WorldClaw generates plausible world content from user intent. Rey projects
  admitted context and may author candidates, but it cannot treat plausible
  completion, a semantic layout image, or a generated surface as observed
  evidence.
- Soft region weights are useful for continuous candidate terrain and visual
  materials. They must never soften Rey's surveyed-validity mask, expand a
  County footprint, or convert unknown space into support.
- A render-based agent's judgment is a proposal or diagnostic observation, not
  qualification. Deterministic scenarios and independently evaluated deltas
  decide whether a Rey candidate passes.
- WorldClaw may choose regions because they can support requested functions.
  Rey requires exact readiness evidence and capability/admission checks; visual
  suitability alone cannot authorize work.
- Generated code and Blender/MCP access are provider actions with tool,
  capability, effect, timeout, and output bounds. Discovery or model
  preference grants no execution authority.
- Image-conditioned reconstruction is useful for authoring but loses direct
  source semantics unless every crop, mask, camera, model, and placement
  transform remains in lineage. It must not become an evidence shortcut.
- Physics, navigation, interaction logic, unrestricted free orbit, and a
  general game-engine runtime remain outside Rey until a bounded workload and
  human-verifiable CLI contract require them.

### Evidence Posture

The paper provides detailed mechanisms and extensive qualitative examples, but
its reported comparison is qualitative rather than a quantitative benchmark or
ablation. The stated implementation uses Blender 5.1.1, four NVIDIA H20 GPUs,
one agent model, and several image and 3D foundation models. The authors also
identify model dependence, generated-program instability, long-horizon
latency, and object-by-object cost as limitations.

At review time, the linked public repository contains the project README and
presentation assets rather than a reproducible implementation of the pipeline.
Rey should therefore adopt the architectural ideas as hypotheses to qualify,
not import performance, correctness, reproducibility, or provider claims from
the paper.
