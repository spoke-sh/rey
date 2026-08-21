# Rey Documentation

This directory contains Rey's foundational contracts, implementation bearings,
and current decision plane. Start with the project scope and constitution, then
follow the subject document for the plane being changed. The documents describe
both implemented behavior and target boundaries; each document must say which
is which.

## Start Here

| Document | Purpose |
| --- | --- |
| [Project README](../README.md) | Product landmark, installation, first bearing, runtime loop, admission grammar, and foundational bearings. |
| [Constitution](../CONSTITUTION.md) | Durable values and invariants that outrank architecture, plans, and implementation details. |
| [Contributor Instructions](../INSTRUCTIONS.md) | Required reading order, working loop, verification expectations, and repository conventions. |
| [Architecture](ARCHITECTURE.md) | System ownership, planes, data flow, crate boundaries, security boundaries, and current implementation posture. |
| [Glossary](GLOSSARY.md) | Canonical vocabulary and the semantic distinctions that other documents rely on. |
| [CLI](CLI.md) | Agent-facing command philosophy, `HEAD → INDEX → WORKING` revision loops, command groups, formats, streams, colors, and exit behavior. |

## Runtime And Evidence

| Document | Purpose |
| --- | --- |
| [Environment](ENVIRONMENT.md) | Process-owned discovery, reasoning maps, providers, capability snapshots, environment admission history, trust, and limits. |
| [Mining](MINING.md) | Relational and source mining families, request/result lineage, native artifacts, visualization, and provider boundaries. |
| [Workloads](WORKLOADS.md) | The public unit of computation: workspace packages, graphs, scenarios, qualification, admission, execution, retained results, and exact scenario/delta inspection. |
| [Runtime](RUNTIME.md) | Deterministic transition state, nested campaigns, reasoning surfaces, stop guards, and semantic convergence. |
| [Scheduling](SCHEDULING.md) | Supervised schedules, semantic invalidations, controls, and retention. |
| [Frontier](FRONTIER.md) | Unresolved work, directional progress, readiness, deterministic scheduling, and workload attention placement. |
| [Diffs](DIFFS.md) | Authoritative typed relational, text, and structural deltas, direction, normalization, invalidation, and portable projections. |
| [Proofs](PROOFS.md) | Claims, evidence manifests, qualification, certificates, completeness, staleness, verification, and retention profiles. |

## Context And Collaboration

| Document | Purpose |
| --- | --- |
| [Explorer](EXPLORER.md) | First principles and the globe → map → terrain fidelity standard for the evidence-bound spatial engine. |
| [Locators](LOCATORS.md) | Canonical candidate addresses, bounded resolution outcomes, and the distinction between locating and reading. |
| [Git](GIT.md) | Repository identity, refs, commits, semantic index state, polling cursors, ref movement, and workload activation. |
| [Journal](JOURNAL.md) | Retained human/agent synthesis, typed notebook blocks, exact browser addresses, authored opportunities, separately admitted read-only query evidence, and deliberately narrow authority. |
| [Observations](OBSERVATIONS.md) | Immutable collaboration statements, exact evidence/source bindings, Channel-admission edges, retained partial broadcast outcomes, resolution/supersession, and the bounded catch-up frontier. |
| [Conversations](CONVERSATIONS.md) | Bounded workspace-local sessions and append-only transcripts, declared participants/writers, ordering, retention, availability, and effect boundaries. |

## Research Context

| Document | Purpose |
| --- | --- |
| [Research References](REFERENCES.md) | Versioned external research summaries, mappings to Rey, incorporation candidates, non-fit boundaries, and evidence posture; references are design inputs rather than accepted decisions. |

## Interfaces And Delivery

| Document | Purpose |
| --- | --- |
| [Interfaces](INTERFACES.md) | High-level ownership map across CLI, API, browser, evidence projections, provider/policy authority, identity, errors, and persistence. |
| [Agent HTTP API](API.md) | Axum server model, API-root discovery, Swagger/OpenAPI, versioned routes, transport, writes, exposure, and typed errors. |
| [`@rey/explorer` technical guide](../packages/explorer/README.md) | React Three Fiber package boundary, shared projection contract, globe and terrain pipelines, renderer lifecycle, limits, APIs, and tests. |
| [Development](DEVELOPMENT.md) | Pinned Nix/Rust/TypeScript environment, root `just` tasks, build outputs, dependency updates, and qualification commands. |
| [Releases](RELEASES.md) | GitHub Actions quality gates, cargo-dist artifact planning, version tags, release permissions, and operator procedure. |
| [Roadmap](ROADMAP.md) | Delivery sequence from local environment evidence through workloads, topography, admitted mutation, and policy. |
| [Current Decision Plane](decisions/README.md) | Accepted structural choices grouped by ownership plane, with current implementation posture and authoritative contract links. |
| [Current Plans](../plans/README.md) | Current executable slices, dependency order, completion criteria, and human verification paths. |

## How The Documents Relate

Use [Architecture](ARCHITECTURE.md) to decide ownership,
[Interfaces](INTERFACES.md) to find the responsible surface, [CLI](CLI.md) for
the agent contract, [API](API.md) for HTTP, and the subject document to define
semantic behavior. The current
[decision plane](decisions/README.md) summarizes accepted structure;
[plans](../plans/README.md) track only bounded current and logically queued
work. Superseded rationale and completed delivery history remain in Git. When a
higher-level decision changes a lower-level description, update the stale
document in the same change.
