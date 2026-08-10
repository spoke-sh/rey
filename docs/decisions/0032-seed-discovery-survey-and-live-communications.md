# ADR 0032: Seed Discovery, Locator Survey, And Live Communications

- Status: Accepted
- Date: 2026-08-09
- Supersedes: the conventional-map bootstrap in [ADR 0020](0020-environment-mapping-graph.md) and the checked-in inventory decision in [ADR 0031](0031-desired-application-inventory-and-search-records.md)

## Context

Rey loaded `rey.env.yaml` by convention. This made a useful early environment
projection possible, but it made project configuration look like discovery
truth. In particular, `SPOKE_ENDPOINT` and `SPOKE_TOKEN` appeared because Rey's
own checked-in map named them, not because the current environment or an agent
had found evidence that this project uses Spoke.

The same collapse obscured four different runtime phases: deterministic
orientation, agent reasoning over that evidence, exact surface survey, and
ongoing artifact processing. The browser footer also described an abstract
pipeline instead of carrying current operator communication.

## Decision

Rey uses this ordered context lifecycle:

1. **Discovery.** The compiled Rey process observes the fixed seed set
   `HOME`, `PWD`, and `PATH` under explicit bounds. It may use those seeds only
   through declared adapters, such as the current `git` and `rg` identity
   adapters. It does not load a project configuration file, source a shell
   profile, traverse all of `HOME`, or infer Spoke configuration.
2. **Reasoning over discovery.** A coding harness or other policy receives the
   frozen discovery record and may propose a bounded environment mapping
   resource. `rey.env-map.v3` remains the current interchange format, but it
   is an agent-generatable resource rather than implicit bootstrap
   configuration. Rey observes it only when the caller supplies `--map`.
3. **Survey.** Admitted native locators identify exact anchors for source,
   environment, Git, workload, and provider evidence. Locators identify; they
   do not retrieve, authorize, or prove. A future `rey-locator` library owns
   canonical parsing, normalization, resolution dispatch, and matrix-style
   dimensions. Provider-owned Spoke locators remain opaque to Rey and may use
   schemes such as `spoke+local://...`.
4. **Process.** Rey incrementally processes artifacts produced by survey and
   by retained cadence ticks such as Git revisions, environment admissions,
   workload results, and scheduled scans. Processing derives deltas and typed
   attention; it does not invent a total order across independent clocks.

The desired-application declaration record and bounded search record remain
separate. The process-owned adapter inventory is recorded as `rey process`;
an explicit agent map extends that inventory and retains its own source.

The operator UI is live. Its fixed footer is a communications channel backed
by typed attention and passive-revalidation state. The left mailbox shows a
subtle count, the center chevrons reveal a bottom sheet, and the source
revision remains on the right. An empty mailbox explicitly means no operator
attention is requested. The UI never fabricates traffic to appear active.

Structured environment contracts advance to `rey.environment-status.v5`,
`rey.environment-operator-projection.v3`, and `rey.environment-diff.v4` because
process-owned seeds and application declarations now participate directly in
the operator projection.

## Consequences

- `SPOKE_ENDPOINT` and `SPOKE_TOKEN` are not default Rey environment inputs.
- A file named `rey.env.yaml` has no effect unless explicitly supplied.
- Agent-generated mappings remain bounded, diffable, admissible evidence
  without becoming hidden configuration authority.
- `rey env status`, `diff`, and `log -p` expose the process seed plane and the
  desired-inventory/search boundary without requiring code inspection.
- Survey needs a canonical locator library and CLI generation/validation
  surface before broad background processing or generic scheduling expands.
