# Plan 0014: Seed Discovery And Locator Survey

- Status: Active
- Decision: [ADR 0032](../docs/decisions/0032-seed-discovery-survey-and-live-communications.md)

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
- [ ] Implement canonical locator types and parse/format fixtures in a
  dependency-light `rey-locator` crate.
- [ ] Add high-fidelity agent CLI commands to generate and validate reasoning
  map resources and display their locator anchors.
- [ ] Bind survey artifacts to exact discovery, map, provider, and source
  revisions with bounded resolver outcomes.
- [ ] Feed admitted survey artifacts and independent cadence ticks into the
  processing frontier without fabricating a global event log.

## Concrete Anchor

```text
rey env map generate --from-discovery <snapshot>
rey env map validate <resource>

DISCOVERY → REASONING MAP → LOCATOR SURVEY → PROCESS
```

The exact command names remain plan-owned until the locator/resource types are
implemented. The acceptance surface must show process seeds, generated-resource
identity and provenance, locator parse/resolution outcomes, bounds, omissions,
and source revisions in both human and structured output.
