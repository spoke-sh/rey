# Collaboration Observations

Rey retains compact collaboration observations independently from Channel
topology, transport messages, Journal documents, runtime observations, and the
runtime frontier. This document defines the implemented local observation log,
CLI/browser surfaces, and authority boundary.

## Ownership And Separation

An observation is an immutable statement over an exact subject and evidence.
It may be admitted to one or more local Channels, superseded by one later
observation, or closed by one immutable resolution. These relations live in
`.rey/channels/observations.json`; they never enter Channel
`HEAD → INDEX → WORKING`, and they do not copy the observation body into each
Channel.

The collaboration frontier is a catch-up projection, not the runtime frontier.
An unresolved observation is not a workload, assignment, action, proof claim,
relay request, unread message, or authenticated assertion. A self-asserted
author label carries no identity guarantee.

## Contracts

`rey.observation.v1` binds:

- a finding, question, progress, blocker, or handoff kind;
- a bounded self-asserted human, agent, or Rey author label;
- an exact subject locator, compact statement, and optional desired delta;
- complete or partial posture, with explicit omissions required for partial
  observations and forbidden for complete ones;
- up to 32 exact evidence bindings, each with locator, source revision, and
  content digest; and
- an optional exact observation identity that this observation supersedes.

Admission retains the proposal identity, local sequence, admission time,
workspace-file source locator and content digest, and the effective limit
envelope. Proposal identity is content-derived; replaying the same proposal is
idempotent even when a later invocation names another source file.

`rey.observation-resolution.v1` names one exact open observation, a
`resolved` or `withdrawn` outcome, a bounded reason, self-asserted author, and
optional exact evidence. A resolution is append-only and idempotent. A closed
observation cannot receive a conflicting resolution or superseder.

## Channel Admission And Broadcast

A Channel admission is an immutable edge from one observation identity to one
local Channel id. It binds the exact Channel graph and optional Channel HEAD
commit used for admission. It carries no INDEX/HEAD mutation, relay, remote
delivery, action, assignment, or execution authority.

Broadcast takes a canonical explicit target set of at most 32 Channel ids.
Each target independently yields `admitted`, `already_admitted`,
`unknown_channel`, or `rejected_kind`. The complete partial-fan-out receipt is
retained with its exact request identity, graph binding, ordered target
outcomes, and any resulting admission identities. Replaying the same
observation, target set, and graph returns the same receipt rather than
duplicating admissions. A changed graph is a new broadcast request.

## Frontier

`rey.observation-frontier.v1` deterministically selects open observations in
ascending local observation sequence. It reports separate total, unresolved,
superseded, resolved, withdrawn, and unbroadcast counts. The projection binds
its source log, limit, completeness, omitted count, selected observation
records, exact evidence/source bindings, and admitted Channel ids. Reaching the
bound is incomplete catch-up evidence, not convergence.

No wall-time order is inferred across providers or other Rey logs. Observation
sequence orders this one local log only.

## Local Retention And Failure

The implemented store is bounded to 1,024 observations, 1,024 resolutions,
4,096 Channel admissions, 4,096 broadcast receipts, a 256-row maximum frontier,
and 4 MiB total state. It uses a separate lock and atomic replacement, rejects
symlinked state paths, verifies contiguous sequences and every semantic
identity on load, and leaves a missing store read-only and empty.

The implemented CLI is:

```text
rey observations [--workspace PATH] [--state-dir PATH] add OBSERVATION.yaml
  [--channel ID ...] [--no-broadcast] [--format table|json]
rey observations [--workspace PATH] [--state-dir PATH] list
  [-n COUNT] [--format table|json]
rey observations [--workspace PATH] [--state-dir PATH] show OBSERVATION_ID
  [--format table|json]
rey observations [--workspace PATH] [--state-dir PATH] resolve RESOLUTION.yaml
  [--format table|json]
```

With no explicit `--channel`, `add` selects the effective Channel graph's
bounded `broadcast_default` set; `--no-broadcast` retains the observation
locally without admissions. Human rendering exposes exact identities, source,
evidence, completeness, omissions, authority, per-target outcomes, frontier
coverage, and closure state. JSON returns the same typed documents. `list` and
`show` are read-only and do not create state.

The operator server exposes the same default 64-row frontier at read-only
`GET|HEAD /api/v1/observations`. Feed renders each unresolved observation as an
order-only `O@sequence` signal with its exact identity, source, subject,
evidence, omissions, limits, completeness, self-asserted author, and Channel
admission count. The footer mailbox renders observations in a separate source
section beside runtime attention. It does not synthesize cross-source order,
unread state, priority, assignment, action, or proof authority. Mounted
portfolio and Feed state passively revalidate this endpoint while retaining
the last good document on failure.

Selected exact unresolved identities can enter the deterministic unretained
Journal-seed projection through `rey journal seed` or
`/journal/new?observations=...`. The seed cites this log and its exact records;
it does not resolve, copy, mutate, or automatically retain an observation.
Ordinary Journal validation and admission remain the only retention boundary.
