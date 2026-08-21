# Scheduling

Rey scheduling is a supervised runtime mechanism, not a browser polling
convention. The foreground `rey agent` process owns a bounded orchestrator
topology containing the operator HTTP server and one `rey scheduler` child OS
process. If either required process exits unexpectedly, supervision cancels the
other and the agent fails closed.

## Ownership

The scheduler owns schedule intervals, enabled state, next-run calculation,
retries, last errors, source revisions, semantic change detection, and bounded
run receipts. It owns both static runtime scans and dynamic provider-backed
schedules.

The browser owns no recurring scan timer. It loads route documents over HTTP,
subscribes to `/api/v1/events`, and revalidates mounted projections only after
the scheduler publishes a semantic invalidation. The first event on each stream
is a wildcard resynchronization boundary, so reconnecting clients do not rely
on durable event replay.

## Schedule families

The initial static schedules observe portfolio, environment, channels,
observations, and cadence projections. A GitHub inbox admitted in the current
Channel HEAD creates a dynamic schedule named
`provider.github-inbox/<application-id>`. That schedule invokes the same
bounded `rey channels poll` interface used by an explicit CLI poll; discovery
does not grant provider authority.

An evidence-link click requests `run_now` for the exact GitHub schedule. It
does not start a second browser polling loop. A run resets the next deadline
for that schedule.

## Control and retention

Schedule controls require an exact schedule ID and expected schedule revision.
Enablement changes are retained below `.rey/scheduler/state.json`. Run receipts
are bounded and local. Process-local server-sent events are delivery hints,
not the retained source of truth; route projections and scheduler state remain
authoritative after reconnect.

The public control surfaces are:

- `GET /api/v1/schedules` for the current scheduler snapshot;
- `POST /api/v1/schedules/control` to enable, disable, or run a schedule now;
- `GET /api/v1/events` for the live semantic invalidation stream.

Disabling a schedule removes future automatic runs without deleting retained
receipts, its last source revision, or provider-owned evidence.
