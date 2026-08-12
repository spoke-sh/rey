# ADR 0052: Provider-Independent Foundation

- Status: Accepted
- Date: 2026-08-11
- Supersedes: the removed external-platform integration decisions formerly
  numbered 0003, 0005, and 0006

## Context

Rey's foundation had accumulated identities, retention profiles, proof fields,
coordinates, roadmap phases, and source links for one particular external
platform. Those assumptions made an unimplemented integration influence core
semantics and obscured the guarantees Rey actually provides today.

## Decision

Rey is designed from its own workload, evidence, projection, and operator
requirements. No external platform is a privileged reasoning, query, compute,
storage, or durability plane.

The current v1 contract is local and explicit:

- semantic coordinates use Rey's local provider-qualified carrier;
- retained workload, environment, editor, Journal, and proof state disclose
  their implemented local guarantees;
- proof schemas contain only guarantees Rey can evaluate;
- the runtime and UI do not advertise an unbound source repository; and
- roadmaps contain no speculative integration track.

Future providers may implement narrow public Rey contracts. They must arise
from a concrete Rey requirement, declare exact identities and guarantees, and
earn first-class status through a new decision and end-to-end CLI proof. A
provider cannot redefine Rey's semantic model or become a build, startup, or
design dependency.

This is a pre-alpha hard cut. Removed enum variants, serialized fields, command
options, examples, and historical integration documents have no compatibility
reader or alias.

## Consequences

- Local evidence is the only implemented retention profile.
- Provider ownership remains explicit without naming a preferred external
  implementation.
- The source repository stays unbound until configuration supplies an exact
  repository identity.
- Historical plans and decisions that privileged an external platform are
  removed rather than retained as current design context.
