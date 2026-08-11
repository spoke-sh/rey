# Plan 0014: Seed Discovery And Locator Survey

- Status: Active
- Decisions: [ADR 0032](../docs/decisions/0032-seed-discovery-survey-and-live-communications.md) and [ADR 0041](../docs/decisions/0041-continuous-coordinate-topography.md)

## Outcome

Separate process-owned discovery, agent-generated reasoning maps, exact
locator survey, and ongoing cadence processing, with the same state visible in
the agent CLI and live operator UI.

## Completion Checklist

- [x] Make `HOME`, `PWD`, and `PATH` the fixed process-owned discovery seeds.
- [x] Stop loading `rey.env.yaml` by convention; accept mapping resources only
  through explicit `--map` input.
- [x] Remove checked-in Spoke endpoint/token assumptions and project the
  process-owned application inventory into `rey env` and `/environment`.
- [x] Advance environment structured contracts and add CLI/provider fixtures.
- [x] Replace the decorative UI pipeline footer with a live attention mailbox,
  quiet state, revalidation error message, and source revision link.
- [x] Add the live-UI invariant to `AGENTS.md` and formalize the four phases.
- [x] Implement canonical locator types and parse/format fixtures in a
  dependency-light `rey-locator` crate.
- [x] Generate the first survey graph and frozen scenarios through
  `rey workloads create context-anchor-survey`; do not hard-code product
  scenarios into the runtime.
- [x] Expose seed, locator, resolution, patch, delta, frontier, omission, and
  lineage evidence through `rey workloads list|test|run|status`.
- [x] Bind survey artifacts to exact discovery, map, provider, and source
  revisions with bounded resolver outcomes.
- [ ] Feed admitted survey artifacts and independent cadence ticks into the
  processing frontier without fabricating a global event log.

## Concrete Anchor

Plan 0017 owns the first concrete proof:

```text
PWD → AGENTS.md + README variants
    → agent-generated context-anchor-survey workload
    → locator candidates + typed resolution outcomes
    → admitted topography patch + directed delta + frontier
    → rey workloads ... + /explore
```

Environment reasoning-map generation remains a valid later agent interface,
but it is not the shortest proof of incremental context survey. The acceptance
surface must show process seeds, generated workload provenance, locator
parse/resolution outcomes, bounds, omissions, source revisions, patch delta,
and frontier in both human and structured output.
