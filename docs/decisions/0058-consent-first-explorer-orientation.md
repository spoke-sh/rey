# ADR 0058: Consent-First Explorer Orientation

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0026](0026-context-topology-explorer.md), [ADR
  0049](0049-workload-admission-history.md), [ADR
  0050](0050-file-backed-workload-admission.md), and [ADR
  0056](0056-continuous-globe-mercator-county-grammar.md)

## Context

Clearing `.rey` leaves file-backed system workload packages visible in
WORKING but no admitted workload HEAD, retained survey patch, or semantic
atlas. Explorer previously interpreted that absence as a reason to reuse the
legacy portfolio graph. The result was a card-like diagram at the exact moment
when a new operator needed spatial orientation and a clear consensual next
step.

Rey must not solve that gap by pretending the project is already mapped. It
also cannot run a survey merely because the operator pans, zooms, selects an
object, or opens the UI. The pre-survey surface therefore needs a visual
grammar that can attract attention without converting candidate file state
into admitted geography or fabricated agent activity.

## Decision

`/explore` is the `rey ui` human entry route. When the workload revision loop
is initialized, no survey topography is admitted, and either no workload HEAD
exists or `context-anchor-survey` is admitted but unrun, Explorer renders
`rey.explore-orientation-globe.v1`.

The orientation globe is an abstract presentation sphere. It is not
`rey.semantic-atlas.v1`. It binds one exact source revision and contains
workload beacons derived from file-backed request, WORKING, INDEX, or admitted
workload state. Every beacon retains:

- workload identity, title, source path, and exact source digest;
- request, WORKING, INDEX, or admitted state;
- generator or producer provenance;
- whether the workload is the initial survey/mapping step; and
- the next explicit inspection, consent, hydration, or run step.

Beacon longitude and latitude are stable presentation coordinates derived
from workload identity. They carry no semantic-distance, neighborhood, Earth,
coverage, or project-boundary claim. They do not enter the semantic atlas and
are replaced, not reinterpreted, when admitted survey evidence produces the
real World geometry.

All pre-survey lens levels remain on the World orientation posture. Zoom may
change optical scale but cannot reveal portfolio cards, invented terrain, or
unadmitted details. Selecting a beacon focuses it and exposes exact workload
inspection. A WORKING or INDEX beacon also links to the existing Feed
admission control. That control remains the combined human action that freezes,
qualifies, and admits the exact reviewed file state. The globe itself remains
read-only.

The initial project loop is:

```text
system or agent authors an exact workload file
  → Explorer shows a workload beacon
  → human inspects and consents to the exact revision
  → qualification admits workload HEAD
  → agent runs the admitted survey over explicitly selected seeds
  → retained topography and projection packet
  → semantic World / Atlas / County grammar
```

An admitted-but-unrun `context-anchor-survey` remains a beacon and changes the
bearing to `SURVEY RUN REQUIRED`. A generic admitted workload without
topography retains the existing diagnostic workload topology; this onboarding
posture must not hide unrelated failures or incomplete work.

Workload beacons are distinct from Channel polling beacons. A workload beacon
is a browser attention projection with no process, transport, timing, action,
or execution authority.

## Consequences

- A fresh user enters Rey through spatial orientation rather than a queue or
  legacy card diagram.
- The UI explains why the project is unmapped and what exact consent is needed
  before an agent surveys it.
- WORKING remains visible without being treated as admitted evidence.
- The first globe can be visually rich while remaining honest about its lack
  of semantic geometry.
- Completing the loop still requires an admitted survey run and retained
  topography; this decision does not add browser workload execution.
