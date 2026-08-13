# Collaboration Journal

The Rey Journal is the shared, retained synthesis surface between a human
operator, an agent, and Rey's exact evidence. It sits after transient
conversation and compact Channel observations but before executable workload
admission:

```text
conversation ───────────────┐
Channel observations → Journal seed → entry proposal → validate → retain
                                                │
                                                └─ no execution authority
```

A Journal entry can explain context, carry a read-only query declaration,
show a bounded frame or directed diff, point into `/explore`, and propose a
next action. Admission means only that Rey accepted and ordered the document.
It does not execute a query, assign an agent, mutate a workload, prove an
observation, or let an author qualify its own claim.

## Entry Contract

`rey.journal-entry-proposal.v2` is the authored contract. Admission derives a
content identity and produces `rey.journal-entry.v2`; the ordered retained
collection is `rey.journal-log.v2`.

| Field | Meaning |
| --- | --- |
| `schema` | Exact proposal schema |
| `title` | Human-readable entry identity, 1–240 characters |
| `author` | Typed `human`, `agent`, or `system` identity |
| `binding` | Canonical semantic coordinate, numeric Explorer scale, and the matching exact source revision |
| `supersedes` | Optional identity of an earlier retained entry; history is never rewritten |
| `layout` | Required bounded 12-column broadsheet of ordered bands and cells |
| `blocks` | Ordered bounded document of 1–32 typed blocks |

The retained entry adds `entry_id`, one-based `sequence`, and `admitted_at`.
The entry identity is derived from the canonical proposal rather than its
admission time. Re-admitting identical content is idempotent and returns the
existing entry. A supersession must point backward within the same log.

Every entry binding keeps semantic address and presentation state separate:

```text
coordinate: rey+local://{kind}/{identity}?revision={revision}[&role={agent-role}]
scale:      0.05..=5.4
```

Coordinate dimensions are lexically ordered `revision`, `role`. The decoded
`revision` must equal `binding.source_revision`; `scale` must be finite within
the current Explorer bound. The UI derives
`/explore?coordinate={percent-encoded-coordinate}&scale={canonical-number}`.
The former matrix grammar and all earlier Journal documents are rejected with
no dual reader or automatic migration. The binding may intentionally name
historical context; Explorer resolution determines whether it is current,
stale, or missing. See [Context Topology Explorer](EXPLORER.md) and
[Locators](LOCATORS.md).

## Document Addresses

Every retained entry and typed block has a stable browser address:

```text
/agents                                      Journal index
/journal/new                                 new human entry
/journal/{slug}                              exact retained entry
/journal/{slug}#block-{block-id}             exact typed block
```

The canonical slug is
`j{sequence}-{bounded-ascii-title}--{normalized-full-entry-id}`. The readable
title aids recognition; the complete content identity prevents two documents
from collapsing onto one route. Entry routes resolve exactly and never fall
back to a title, prefix, sequence, or current document. Selecting an entry in
the index enters its canonical route, and successful creation replaces
`/journal/new` with the admitted entry route.

Block ids are unique within an entry and become fragment permalinks. A block
heading links to its own fragment so prose, query, frame, diff, Explore, and
action context can be cited without copying the document. These addresses are
presentation coordinates over retained Journal state; they do not turn a URL
into admission or execution authority.

## Notebook Blocks

The ordered block document is the high-fidelity communication grammar. It is
typed data, never arbitrary HTML or executable browser content.

| Block | Purpose | Important bounds |
| --- | --- | --- |
| `prose` | WYSIWYG-style notebook nodes: heading, paragraph, bullet, quote, or code | 1–128 nodes; 64 KiB total text |
| `explore` | Embedded map/lens reference with its own exact source revision | Semantic coordinate; numeric scale; optional caption |
| `query` | SQL or another provider query declaration | `mode` must be `read_only`; 32 KiB statement; admission never executes it |
| `frame` | Bounded tabular result preview tied to an earlier query block and immutable snapshot | 64 typed columns; 100 preview rows; explicit nulls, row count, and truncation |
| `diff` | Directed comparison between exact source and target locators | Explicit direction and `equal`, `different`, or `inconclusive` assessment |
| `action` | Proposed operation and desired delta | Bounded evidence and dependency locator sets |

Frame, diff, and action blocks are communication surfaces, not proof merely
because an author submitted them. Exact evidence locators and the runtime's
normal validators remain authoritative.

The live browser authoring surface creates prose, read-only query, directed
diff, and proposed-action cells. Every new document also starts with an exact
Explore cell. Frame cells remain exact retained previews authored through the
agent contract; they render and survive a human superseding revision without
becoming editable browser-produced evidence.

## Broadsheet Grammar

Every block is placed exactly once in a required `broadsheet` layout:

```yaml
layout:
  kind: broadsheet
  columns: 12
  bands:
    - id: lead
      cells:
        - block_id: context
          span: 8
        - block_id: map
          span: 4
```

The column count is always 12. A document contains 1–32 uniquely named ordered
bands. Every band contains one or more ordered cells with a span from 1–12;
the spans in one band cannot exceed 12. Blank columns are explicit whitespace,
not missing evidence. Band/cell traversal must equal `blocks` order, so the
semantic reading order stays deterministic and every fragment remains stable.

Small viewports stack cells in that same order. Width, adjacency, whitespace,
and responsive stacking are projection choices only: they cannot introduce a
relationship, change a diff assessment, hide a completeness boundary, or
grant an action authority.

The browser supports every 1–12-column span. A narrower cell can join the
preceding band when the combined span remains within 12. These controls compile
the same exact grammar accepted by agent YAML; the browser does not retain an
independent layout model.

## Live Editing And Revisions

`/journal/new` and `/journal/{slug}` use the same live broadsheet surface. A
new entry compares against an empty base. A retained route compares the live
document against that exact entry and reports inserted, modified, removed, and
layout changes. Recording is disabled until the proposal is both admissible
and changed.

Retained routes expose the exact predecessor and up to three direct
superseding branches as canonical document links; additional branches are
counted rather than silently omitted. This is an immutable revision plane, not
a mutable branch selector or Git repository claim.

Editing never updates an entry in place. Recording a retained document creates
a new human-authored proposal with `supersedes` set to the exact current
`entry_id`, appends it through ordinary Journal admission, and enters the new
canonical route. Earlier entries, slugs, fragments, sequences, authors, and
timestamps remain unchanged. The browser has no retained draft or autosave
contract in this slice.

Prose editing is a deterministic shorthand over typed nodes. Blank-line-
separated units starting with `# `, `- `, or `> ` become heading, bullet, or
quote nodes; triple-backtick fenced units become code; other units become
paragraphs. Only the typed nodes are retained.

## Authoring Surfaces

Agents write a workspace-contained YAML proposal through the CLI:

```sh
rey journal add path/to/entry.yaml
rey journal list
```

`rey journal add` admits only `author.kind: agent`. Proposal files must be
regular, non-symlinked, contained beneath the workspace, and at most 1 MiB.
The command does not execute any block.

Humans enter through `/journal/new`. `POST /api/v1/journal` accepts a bounded
JSON proposal on every address the operator explicitly binds. It requires no
session, credential, token, cookie, or matching `Origin`, and it accepts only
`author.kind: human`. Author ids are therefore self-asserted labels, not
authenticated principals. Whoever can reach a non-loopback listener can
submit a Journal document; Rey warns about that write boundary at startup.

Rey's system entries remain deterministic projections from workload requests
and attention. They are not copied into the authored log, so retained human or
agent statements stay distinguishable from current derived frontier state.

Both authoring paths use the same validator, content identity, ordered local
log, entry/block limits, and atomic locked publication beneath
`.rey/journal/journal.json`. This local log is standalone runtime state, not a
claim of remote durability, multi-user consistency, authenticated identity, or
remote retention. Unauthenticated admission does not weaken validation,
content identity, limits, atomic publication, or the rule that Journal blocks
carry no execution authority.

## Channel Relationship

Journal remains separate from Channel topology and immutable Channel messages.
The implemented `rey channels message|relay|beacon` paths do not create or
revise Journal documents. The standalone observation frontier retains exact
evidence and source bindings; a Journal is deliberate notebook synthesis that
may cite several observations. A broadcast associates one retained observation
identity with explicit local channels without creating or duplicating a
Journal entry.

`rey journal seed <observation-id>... --author <agent-id>` and
`/journal/new?observations=<id>[,<id>]` use the same deterministic
`rey.journal-seed.v1` projection. It accepts 1–16 unique exact unresolved
identities, canonicalizes them by local observation sequence, binds the exact
observation-log revision, and emits one valid broadsheet proposal containing
exact observation/source/evidence citations within the ordinary 1 MiB proposal
bound. Seed identity covers that source log,
canonical selection, author, and complete proposal. The CLI accepts an agent
author label; the browser derives the self-asserted human `operator` label and
opens the same live editor for review.

Seeding is read-only and creates no Journal state. A closed, unknown,
duplicate, malformed, or over-limit selection is rejected. Only ordinary
`rey journal add` or browser `POST /api/v1/journal` validation and admission
can retain the proposal. [Observations](OBSERVATIONS.md) owns the source log;
this document owns the deterministic seed and Journal-admission boundary.

## Authored Opportunity Surface

`rey journal opportunities` and read-only
`GET|HEAD /api/v1/journal/opportunities` derive the same bounded
`rey.journal-opportunity-surface.v1`. The surface selects action blocks only
from current Journal leaves: an entry disappears from this projection as soon
as any retained entry supersedes it, while independent supersession branches
remain current. Rows retain Journal sequence and entry identity, the exact
document and block permalink, self-asserted author, semantic binding, proposed
operation and desired delta, and exact evidence/dependency citations.

The projection is ordered by Journal sequence and block reading order and
keeps the newest 128 rows by default, with a hard maximum of 256. It binds the
exact `rey.journal-log.v2` identity, effective Journal entry/block limits,
completeness, and an explicit oldest-row omission when truncated. Surface and
row identities are deterministic, and replay against a changed or tampered log
fails.

Every row is explicitly `authored_only` with `authority: none`. `/agents`
renders these rows as authored opportunities separately from Rey-derived
attention and links each one to its exact Journal fragment. The surface does
not infer readiness, priority, cost, assignment, policy selection, or observed
work. If the proposed idea becomes runtime work, it must first appear as a
verified selected ready `CREATE` attention row and then cross the existing
`rey workloads create --attention-row ...` generation and workload admission
boundary. Journal authors cannot manufacture or bypass that scheduler state.

## Separately Admitted Read-Only Query

The first executable Journal query is deliberately narrow:

```yaml
kind: query
id: open-observations
language: rey
provider: rey.observations
mode: read_only
statement: frontier
parameters:
  limit: "64"
```

Only the optional canonical decimal `limit` is accepted, from 1 through 100.
The provider reads the already retained local observation log and projects its
oldest-open-first frontier; it performs no broadcast, resolution, locator
read, workload operation, or mutation.

Execution crosses three distinct gates:

```text
retained current query cell
  → journal query admit       exact Journal + observation log/frontier
  → journal query execute     retained frame/delta evidence
                             + create-new unretained superseding proposal
  → journal add               ordinary validation and Journal retention
```

`rey journal query admit <entry-id> <block-id>` accepts only a query cell on an
unsuperseded entry. `rey.journal-query-admission.v1` binds the exact Journal
log, entry and block, complete declaration, observation log and frontier,
effective frame bounds, and read-only authority. Admission is retained and
idempotent but does not execute the query or change the Journal.

`rey journal query execute <admission-id> --author <agent-id>
--proposal-out <workspace-relative.json>` revalidates every admitted input,
executes only that bounded projection, and retains
`rey.journal-query-execution.v1`. Its exact frame contains at most nine typed
columns and 100 preview rows; `row_count`, truncation, omitted rows, source and
target snapshots, and the directed empty-to-observed delta remain explicit.
Changed Journal or observation input fails before a new execution. Exact
execution replay returns retained evidence without rerunning the projection.

Execution still does not write the Journal. It uses create-new semantics for a
workspace-contained JSON proposal that preserves the original query, appends
its frame and diff blocks, and sets `supersedes` to the exact source entry.
JSON is valid input to `rey journal add`; only that existing ordinary admission
can append the result. The browser read endpoint
`GET|HEAD /api/v1/journal/queries` exposes retained admissions and executions,
and the existing exact Journal route renders frame/diff cells after admission.
The browser grants no query-admission or execution write.

## Example Agent Proposal

```yaml
schema: rey.journal-entry-proposal.v2
title: Source coverage moved after survey
author:
  kind: agent
  id: codex
binding:
  coordinate: rey+local://workload/source-mining?revision=blake3%3Aabc
  scale: 2.05
  source_revision: blake3:abc
layout:
  kind: broadsheet
  columns: 12
  bands:
    - id: evidence
      cells:
        - block_id: context
          span: 8
        - block_id: coverage-query
          span: 4
    - id: bearing
      cells:
        - block_id: next-bearing
          span: 12
blocks:
  - kind: prose
    id: context
    document:
      - kind: paragraph
        text: Two unowned source surfaces remain after the latest survey.
  - kind: query
    id: coverage-query
    language: sql
    provider: local
    mode: read_only
    statement: select * from coverage where owner is null
    parameters: {}
  - kind: action
    id: next-bearing
    operation: refine
    desired_delta: Reduce unowned source surfaces from two to zero.
    evidence_ids:
      - rey+local://coverage/latest
    dependency_ids: []
```

See [Context Topology Explorer](EXPLORER.md) for coordinate resolution,
[Interfaces](INTERFACES.md) for HTTP authority, and the
[current decision plane](decisions/README.md) for the collaboration boundary.
