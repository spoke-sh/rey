# ADR 0039: Bounded Operator Feed

- Status: Accepted
- Date: 2026-08-10
- Extends: [ADR 0030](0030-operator-cadence-agents-and-explorer-coordinates.md), [ADR 0035](0035-agent-recommendations-and-observed-work.md), and [ADR 0037](0037-explore-bound-collaboration-journal.md)
- Extended by: [ADR 0040](0040-workspace-channels-and-operator-index.md), which supersedes URL-only Feed-layout retention

## Context

Explorer is the right map for spatial orientation and Cadence is the right
place to prove order on independent clocks. Neither is the natural first
inspection plane for a high-cadence project. An operator should not need to
reconstruct the current bearing by visiting the workload portfolio, Journal,
repository instruments, and every cadence lane in turn.

Rey also cannot solve that problem by inventing an unread counter or a
universal event stream. The current standalone runtime has no durable operator
cursor, cross-clock causal edges, complete activation log, or retained browser
read history. Git commit wall time, Journal admission time, and Rey environment
sequence have different semantics.

## Decision

`/feed` is a read-only, bounded inspection projection and the first item in the
primary navigation. It composes the already authoritative workload-list,
cadence, and Journal documents; it does not add another API, store, scheduler,
attention relation, or event owner.

Feed starts with three side-by-side, independently scrolling vertical streams
in the spirit of TweetDeck, but they are a default composition rather than
fixed page regions. A narrow Firehose rail opens the common stream composer.
It adds new streams and tunes, reorders, or removes existing ones. Every stream
is a bounded lens over the same source union and uses rich authored posts rather
than a dashboard or flattened event table:

1. **Signals** projects bounded Git, environment-admission, and Journal records.
   Git posts embed exact commit and parent links; Journal posts can embed prose,
   query, frame, diff, Explore, and proposed-action previews; environment posts
   retain their order-only identity. The source-boundary post identifies the
   exact attention snapshot, Journal log, cadence contract, and omissions.
2. **Admission** projects current non-excluded portfolio attention, missing
   draft/qualification follow-up, dirty or conflicted working-tree state, and
   the retained local-upstream publication relation. These posts orient an
   admission decision but remain inspect-only until Rey defines an explicit
   browser admission contract.
3. **Flow** projects admitted workload revisions and their observed
   qualification, scenario progress, run posture, mining output, deltas, and
   reasoning surfaces. It is retained-result flow, not live process telemetry.

The first lens grammar is deliberately small. Signals selects
`all|journal|git|environment`, Admission selects `all|now|watch|bound`, and Flow
selects `all|attention|failing|qualified`. A composition contains at most eight
lanes and is encoded in the browser URL as
`?streams={plane}.{filter}[~{percent-encoded-name}],...`. Clicking the displayed
stream title enters a bounded inline editor. Blur or Enter autosaves the
normalized optional name into the URL, Escape cancels, and an empty name
restores the derived title. This makes a composed inspection plane deep-linkable
without introducing a server-side preference document, feed store, or
configuration API. The Feed route owns and validates the search document;
autosave replaces that typed route location instead of bypassing TanStack
Router through the raw browser History API. Invalid entries are ignored; an
absent or entirely invalid composition returns to the three defaults.

High-fidelity evidence remains present but does not compete with stream-level
scanning. Journal blocks, Git lineage, and environment transition details are
collapsed by default and expand in place. The stream header retains only its
editable identity, ordering controls, and entry into the Firehose; it does not
repeat the active lens as an eyebrow or surface per-stream count summaries.

The intended later transition is explicit: an admitted proposal may move from
Admission into Flow only after validated action admission and a retained
result. Moving a card in the browser, viewing a post, or writing a Journal entry
cannot cause that transition.

The first signal window renders at most 64 records after display ordering and
reports how many older source records were folded. Admission is not silently
capped inside Feed; it preserves the source attention bound so current
actionable work is not hidden by a presentation limit.

Every Git SHA continues to use the shared exact GitHub commit-link boundary.
Journal, environment, workload, Explorer, and Cadence rows link to their
existing human surfaces. Feed uses the same five-second passive revalidation
and last-good-document behavior as the mounted source projections. It does not
claim read/unread state, exhaustiveness, remote freshness, causal order, live
agent telemetry, or durable stream retention.

`GET /` continues to redirect to `/explore`. Feed precedes Explorer in the nav
because it is likely to become the normal inspection plane on high-cadence
projects; changing the default entry remains an explicit operator-product
decision once real use validates that bearing.

## Consequences

- The operator can watch several logical high-cadence streams at once, compose
  repeated source lenses for different concerns, and share the exact
  composition without losing rich content or source deep links.
- `/feed`, `/cadence`, and `/explore` remain distinct: ranked inspection,
  clock/order proof, and topology navigation respectively.
- A quiet Admission stream is meaningful no-news output, while Signals and Flow
  can still show retained activity and results.
- Wall-time sorting cannot be interpreted as causality or as a complete global
  chronology.
- URL stream state is projection-only in this slice; ADR 0040 defines the later
  workspace-local Channel index while preserving URLs as detached previews.
- Durable operator cursors, read state, stream pagination, aggregation windows,
  cross-clock causal edges, and remote feeds remain later contracts.
