# ADR 0024: Workload Creation Requests

- Status: Accepted
- Date: 2026-08-09
- Extends: [ADR 0023](0023-workspace-workload-packages.md)

## Context

Workspace workload packages made generated graphs and frozen scenario suites
visible, but Rey still had no user-facing way to ask a coding harness to create
one. Treating `workloads test` as that interface would mix two authorities:
test deterministically evaluates an admitted package, while a harness mines
context and proposes new source-controlled artifacts.

Rey must not fabricate placeholder scenarios or hide harness work inside the
runtime. A creation request also needs to remain visible before a package is
admitted, otherwise users cannot inspect the lifecycle through the CLI.

## Decision

`rey workloads create <id>` creates a strict
`rey.workload-creation-request.v1` document at
`workloads/<id>/request.yaml`. It is a local-state mutation and never invokes
an embedded model. The request is the provider-neutral handoff to an external
coding harness and binds:

- stable workload id, title, optional bounded intent, and semantic request id;
- `coding_harness` as the proposer class;
- exact catalog root and target `workload.yaml` path;
- requirements to mine authoritative revisioned inputs, use admitted bounded
  operations, generate independent scenarios, freeze the oracle before
  admission, and preserve the request as lineage; and
- package, graph-node, scenario, and string-size limits.

The command refuses invalid ids, escaping or symlinked catalog paths, and any
existing workload directory. It writes only `request.yaml` with create-new
semantics; it does not invent a graph, scenario, oracle, or accepted admission.

A workspace catalog child may contain either a creation request or an admitted
package. `list` and `status` expose requests as draft workloads with journey
`HYDRATE`, graph `MISSING`, oracle `NOT ADMITTED`, and admission
`AWAITING CODING HARNESS`. `test` and `run` reject a selected draft. When a
harness later materializes `workload.yaml`, Rey validates both documents and
requires their workload ids to match; the admitted package becomes the
executable catalog entry while `request.yaml` remains creation lineage.

The human create result prints the mutation plane, created path, admission
state, agent instructions, and next action. JSON returns
`rey.workload-create-result.v1`. Catalog, list, status, test, and run projection
schemas advance because catalog counts now distinguish total, admitted, and
draft entries.

## Consequences

- Workload authors now have one explicit agentic entry point without making an
  LLM part of deterministic runtime mechanism.
- Draft creation is inspectable through the same portfolio interface as
  admitted workloads.
- Scenario generation is clearly outside `test`; expected values cannot be
  rewritten by candidate execution.
- The request half of the coding-harness handshake is implemented. Harness
  response validation, attention-row binding, and measured re-mining remain
  subsequent Plan 0010 work.
