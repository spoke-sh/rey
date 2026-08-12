import { approveWorkloadIndex, type FeedSources } from "./api";
import { useEffect, useState, type ReactNode } from "react";
import type { CadenceProjection, CadenceTick } from "./cadence";
import {
  shortDigest,
  type AttentionReadiness,
  type WorkloadList,
} from "./domain";
import { GitCommitLink } from "./git-commit-link";
import {
  journalEntrySlug,
  type JournalBlock,
  type JournalProjection,
  type RetainedJournalEntry,
} from "./journal";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { feedStyles as styles } from "./stylex/feed.stylex";
import { className as sx } from "./stylex/shared.stylex";

export type FeedUrgency = "NOW" | "WATCH" | "BOUND";
export const FEED_EVENT_LIMIT = 64;
export const FEED_STREAM_LIMIT = 8;

export type FeedStreamKind = "signals" | "admission" | "flow";
export type FeedStreamFilter =
  | "all"
  | "journal"
  | "git"
  | "environment"
  | "now"
  | "watch"
  | "bound"
  | "attention"
  | "failing"
  | "qualified";

export interface FeedStreamSpec {
  kind: FeedStreamKind;
  filter: FeedStreamFilter;
  name?: string;
}

export const DEFAULT_FEED_STREAMS: FeedStreamSpec[] = [
  { kind: "signals", filter: "all" },
  { kind: "admission", filter: "all" },
  { kind: "flow", filter: "all" },
];

const FEED_STREAM_FILTERS: Record<FeedStreamKind, FeedStreamFilter[]> = {
  signals: ["all", "journal", "git", "environment"],
  admission: ["all", "now", "watch", "bound"],
  flow: ["all", "attention", "failing", "qualified"],
};

export function parseFeedStreams(value: string | null): FeedStreamSpec[] {
  if (!value) return DEFAULT_FEED_STREAMS.map((stream) => ({ ...stream }));
  const streams = value
    .split(",")
    .slice(0, FEED_STREAM_LIMIT)
    .flatMap((token): FeedStreamSpec[] => {
      const [coordinate, encodedName, ...nameRest] = token.split("~");
      if (nameRest.length > 0) return [];
      const [kindValue, filterValue, ...coordinateRest] =
        coordinate!.split(".");
      if (coordinateRest.length > 0 || !isFeedStreamKind(kindValue)) return [];
      const filter = filterValue ?? "all";
      if (!isFeedStreamFilter(kindValue, filter)) return [];
      let name: string | undefined;
      if (encodedName !== undefined) {
        try {
          name = normalizeFeedStreamName(decodeURIComponent(encodedName));
        } catch {
          return [];
        }
      }
      return [{ kind: kindValue, filter, ...(name ? { name } : {}) }];
    });
  return streams.length > 0
    ? streams
    : DEFAULT_FEED_STREAMS.map((stream) => ({ ...stream }));
}

export function serializeFeedStreams(streams: FeedStreamSpec[]): string {
  return streams
    .slice(0, FEED_STREAM_LIMIT)
    .map((stream) => {
      const name = normalizeFeedStreamName(stream.name ?? "");
      const encodedName = name
        ? `~${encodeURIComponent(name).replaceAll("~", "%7E")}`
        : "";
      return `${stream.kind}.${stream.filter}${encodedName}`;
    })
    .join(",");
}

export function normalizeFeedStreamName(value: string): string | undefined {
  const normalized = value
    .toWellFormed()
    .replace(/\p{Cc}+/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!normalized) return undefined;
  return Array.from(normalized).slice(0, 48).join("");
}

export interface InspectionRow {
  id: string;
  source:
    | "ATTENTION"
    | "INDEX"
    | "QUALIFICATION"
    | "REPOSITORY"
    | "REQUEST"
    | "WORKING";
  subject: string;
  signal: string;
  detail: string;
  urgency: FeedUrgency;
  priority: number | null;
  sortPriority: number;
  basis: string;
  href: string;
  location: string;
}

export interface FeedEvent {
  id: string;
  stream: "GIT" | "JOURNAL" | "REY ENV";
  position: string;
  title: string;
  detail: string;
  state: string;
  revision: string;
  occurredAt: string | null;
  sortTime: number | null;
  sourceOrder: number;
  href: string;
  kind: "git" | "journal" | "environment";
  repository: string | null;
  journalEntry: RetainedJournalEntry | null;
  tick: CadenceTick | null;
}

export function deriveInspectionQueue(
  portfolio: WorkloadList,
  cadence: CadenceProjection,
): InspectionRow[] {
  const rows: InspectionRow[] = [];
  const revision = portfolio.revision;
  for (const change of revision?.staged.changes ?? []) {
    const packageSnapshot = revision?.index?.packages.find(
      (candidate) => candidate.workload_id === change.workload_id,
    );
    rows.push({
      id: `workload-index:${change.workload_id}:${revision?.index?.snapshot_revision ?? "missing"}`,
      source: "INDEX",
      subject: change.workload_id,
      signal: revision?.commit_ready ? "APPROVE" : "QUALIFY",
      detail: revision?.commit_ready
        ? `${packageSnapshot?.title ?? change.workload_id} is frozen, qualified, and awaiting human approval.`
        : `${packageSnapshot?.title ?? change.workload_id} is staged but cannot be admitted until its exact INDEX revision passes qualification.`,
      urgency: revision?.commit_ready ? "NOW" : "WATCH",
      priority: null,
      sortPriority: revision?.commit_ready ? 100 : 50,
      basis: `${change.change_kind} · ${packageSnapshot?.source_digest ?? change.target_revision ?? "missing digest"}`,
      href: `/workloads/${encodeURIComponent(change.workload_id)}`,
      location: "ADMISSION",
    });
  }
  for (const change of revision?.unstaged.changes ?? []) {
    const packageSnapshot = revision?.working.packages.find(
      (candidate) => candidate.workload_id === change.workload_id,
    );
    rows.push({
      id: `workload-working:${change.workload_id}:${revision?.working.snapshot_revision ?? "missing"}`,
      source: "WORKING",
      subject: change.workload_id,
      signal: "STAGE",
      detail: `${packageSnapshot?.title ?? change.workload_id} has agent-authored package changes that are not frozen for admission.`,
      urgency: "WATCH",
      priority: null,
      sortPriority: 25,
      basis: `${change.change_kind} · ${packageSnapshot?.source_digest ?? change.target_revision ?? "missing digest"}`,
      href: `/workloads/${encodeURIComponent(change.workload_id)}`,
      location: "WORKLOAD",
    });
  }
  rows.push(
    ...portfolio.attention.rows
      .filter((row) => row.readiness !== "excluded")
      .map(
        (row) =>
          ({
            id: `attention:${row.row_id}`,
            source: "ATTENTION",
            subject: row.subject_id,
            signal: row.action.toUpperCase(),
            detail: row.reason,
            urgency: urgencyForReadiness(row.readiness),
            priority: row.priority,
            sortPriority: row.priority,
            basis: `${row.evidence_ids.length} evidence · ${row.dependency_ids.length} dependencies · C${row.estimated_cost_units}`,
            href:
              row.subject_kind === "workload"
                ? `/workloads/${encodeURIComponent(row.subject_id)}`
                : "/explore",
            location: row.subject_kind === "workload" ? "WORKLOAD" : "EXPLORE",
          }) satisfies InspectionRow,
      ),
  );
  const subjectsWithAttention = new Set(
    portfolio.attention.rows.map(
      (row) => `${row.subject_kind}:${row.subject_id}`,
    ),
  );

  for (const draft of portfolio.drafts) {
    if (subjectsWithAttention.has(`workload:${draft.request.workload_id}`))
      continue;
    rows.push({
      id: `request:${draft.request.request_id}`,
      source: "REQUEST",
      subject: draft.request.workload_id,
      signal: "AUTHOR",
      detail: draft.request.intent ?? draft.request.title,
      urgency: "WATCH",
      priority: null,
      sortPriority: 0,
      basis: "graph pending · scenario oracle pending",
      href: `/workloads/${encodeURIComponent(draft.request.workload_id)}`,
      location: "HANDOFF",
    });
  }

  for (const workload of portfolio.workloads) {
    if (
      subjectsWithAttention.has(`workload:${workload.workload.id}`) ||
      workload.qualification === "qualified"
    ) {
      continue;
    }
    rows.push({
      id: `qualification:${workload.workload.id}`,
      source: "QUALIFICATION",
      subject: workload.workload.id,
      signal: qualificationSignal(workload.qualification),
      detail: `${workload.qualification} · ${workload.passed}/${workload.required} scenarios passing`,
      urgency:
        workload.qualification === "failing" ||
        workload.qualification === "inconclusive"
          ? "NOW"
          : "WATCH",
      priority: null,
      sortPriority: 0,
      basis: `${workload.failed} failed · ${workload.inconclusive} inconclusive · ${workload.stale} stale`,
      href: `/workloads/${encodeURIComponent(workload.workload.id)}`,
      location: "WORKLOAD",
    });
  }

  const repository = cadence.repository_state;
  if (repository?.working_tree_state === "dirty") {
    const changed =
      repository.staged_entries +
      repository.unstaged_entries +
      repository.untracked_entries +
      repository.conflicted_entries;
    rows.push({
      id: `repository:${repository.id}:working`,
      source: "REPOSITORY",
      subject: repository.branch ?? "detached HEAD",
      signal: repository.conflicted_entries > 0 ? "RESOLVE" : "REVIEW",
      detail: `${changed} working-tree paths require inspection before the current source state is quiet`,
      urgency: repository.conflicted_entries > 0 ? "NOW" : "WATCH",
      priority: null,
      sortPriority: repository.conflicted_entries > 0 ? 100 : 10,
      basis: `${repository.staged_entries} staged · ${repository.unstaged_entries} unstaged · ${repository.untracked_entries} untracked · ${repository.conflicted_entries} conflicted`,
      href: "/cadence",
      location: "CADENCE",
    });
  }
  if (repository && repository.push_state !== "pushed") {
    rows.push({
      id: `repository:${repository.id}:publication`,
      source: "REPOSITORY",
      subject: repository.upstream ?? repository.branch ?? "unbound ref",
      signal: publicationSignal(repository.push_state),
      detail: `local source and its retained upstream relation are ${repository.push_state.replaceAll("_", " ")}`,
      urgency: repository.push_state === "unpushed" ? "WATCH" : "BOUND",
      priority: null,
      sortPriority: repository.push_state === "diverged" ? 90 : 5,
      basis: `${repository.ahead ?? "—"} ahead · ${repository.behind ?? "—"} behind · local tracking ref`,
      href: "/cadence",
      location: "CADENCE",
    });
  }

  return rows.sort(
    (left, right) =>
      urgencyRank(left.urgency) - urgencyRank(right.urgency) ||
      right.sortPriority - left.sortPriority ||
      left.subject.localeCompare(right.subject) ||
      left.id.localeCompare(right.id),
  );
}

export function deriveFeedEvents(
  cadence: CadenceProjection,
  journal: JournalProjection,
): FeedEvent[] {
  const events: FeedEvent[] = [];
  let sourceOrder = 0;
  for (const entry of [...journal.log.entries].reverse()) {
    const sortTime = Date.parse(entry.admitted_at);
    events.push({
      id: `journal:${entry.entry_id}`,
      stream: "JOURNAL",
      position: `J@${entry.sequence}`,
      title: entry.title,
      detail: `${entry.author.kind} / ${entry.author.id}`,
      state: "ADMITTED",
      revision: entry.entry_id,
      occurredAt: Number.isFinite(sortTime) ? entry.admitted_at : null,
      sortTime: Number.isFinite(sortTime) ? sortTime : null,
      sourceOrder: sourceOrder++,
      href: `/journal/${journalEntrySlug(entry)}`,
      kind: "journal",
      repository: null,
      journalEntry: entry,
      tick: null,
    });
  }
  for (const lane of cadence.lanes) {
    for (const tick of lane.ticks) {
      const occurredAt =
        tick.occurred_at_unix === null
          ? null
          : new Date(tick.occurred_at_unix * 1_000).toISOString();
      events.push({
        id: `${lane.id}:${tick.id}`,
        stream: tick.kind === "git_commit" ? "GIT" : "REY ENV",
        position: tick.ordinal,
        title: tick.title,
        detail: tick.detail,
        state: [tick.state, tick.publication]
          .filter((value) => value !== null)
          .join(" · ")
          .toUpperCase(),
        revision: tick.revision,
        occurredAt,
        sortTime: occurredAt === null ? null : Date.parse(occurredAt),
        sourceOrder: sourceOrder++,
        href: tick.kind === "git_commit" ? "/cadence" : "/environment",
        kind: tick.kind === "git_commit" ? "git" : "environment",
        repository:
          tick.kind === "git_commit" ? cadence.source_repository : null,
        journalEntry: null,
        tick,
      });
    }
  }
  return events
    .sort((left, right) => {
      if (left.sortTime !== null && right.sortTime !== null) {
        return (
          right.sortTime - left.sortTime || left.sourceOrder - right.sourceOrder
        );
      }
      if (left.sortTime !== null) return -1;
      if (right.sortTime !== null) return 1;
      return left.sourceOrder - right.sourceOrder;
    })
    .slice(0, FEED_EVENT_LIMIT);
}

export function FeedPage({
  configuration,
  onConfigurationChange,
  portfolio,
  sources,
}: {
  configuration?: FeedStreamSpec[];
  onConfigurationChange?: (streams: FeedStreamSpec[]) => void;
  portfolio: WorkloadList;
  sources: FeedSources;
}) {
  const queue = deriveInspectionQueue(portfolio, sources.cadence);
  const events = deriveFeedEvents(sources.cadence, sources.journal);
  const [streams, setStreams] = useState<FeedStreamSpec[]>(
    () =>
      configuration?.map((stream) => ({ ...stream })) ?? parseFeedStreams(null),
  );
  const configurationKey = configuration
    ? serializeFeedStreams(configuration)
    : null;
  const [editing, setEditing] = useState<number | "new" | null>(null);
  const [draft, setDraft] = useState<FeedStreamSpec>({
    kind: "signals",
    filter: "all",
  });
  const sourceEventCount =
    sources.journal.log.entries.length +
    sources.cadence.lanes.reduce((count, lane) => count + lane.ticks.length, 0);
  const foldedEvents = Math.max(0, sourceEventCount - events.length);
  const omissions = boundedOmissions(sources.cadence, foldedEvents);

  useEffect(() => {
    if (configurationKey !== null)
      setStreams(parseFeedStreams(configurationKey));
  }, [configurationKey]);

  const publishStreams = (next: FeedStreamSpec[]) => {
    const bounded = next.slice(0, FEED_STREAM_LIMIT);
    setStreams(bounded);
    onConfigurationChange?.(bounded);
  };

  const openFirehose = (target: number | "new") => {
    setEditing(target);
    setDraft(
      target === "new"
        ? { kind: "signals", filter: "all" }
        : (streams[target] ?? { kind: "signals", filter: "all" }),
    );
  };

  const saveDraft = () => {
    if (editing === null) return;
    if (editing === "new") publishStreams([...streams, draft]);
    else
      publishStreams(
        streams.map((stream, index) => (index === editing ? draft : stream)),
      );
    setEditing(null);
  };

  const removeStream = (index: number) => {
    if (streams.length === 1) return;
    publishStreams(streams.filter((_, candidate) => candidate !== index));
    setEditing(null);
  };

  const moveStream = (index: number, offset: -1 | 1) => {
    const destination = index + offset;
    if (destination < 0 || destination >= streams.length) return;
    const next = [...streams];
    [next[index], next[destination]] = [next[destination]!, next[index]!];
    publishStreams(next);
  };

  const renameStream = (index: number, name: string) => {
    const normalized = normalizeFeedStreamName(name);
    const current = streams[index];
    if (!current) return;
    const derived = streamTitle({ ...current, name: undefined });
    const customName = normalized === derived ? undefined : normalized;
    if (current.name === customName) return;
    publishStreams(
      streams.map((stream, candidate) =>
        candidate === index ? { ...stream, name: customName } : stream,
      ),
    );
  };

  return (
    <main className={sx(styles.page)} data-feed-streams={streams.length}>
      {streams.map((stream, index) => (
        <FeedStream
          events={events}
          index={index}
          key={`${index}:${stream.kind}:${stream.filter}`}
          omissions={omissions}
          onMove={moveStream}
          onRename={renameStream}
          onTune={openFirehose}
          portfolio={portfolio}
          queue={queue}
          sources={sources}
          stream={stream}
          streamCount={streams.length}
        />
      ))}
      {editing === null ? (
        <button
          className={sx(styles.firehoseRail)}
          disabled={streams.length >= FEED_STREAM_LIMIT}
          onClick={() => openFirehose("new")}
          type="button"
        >
          <span aria-hidden="true">＋</span>
          <strong>FIREHOSE</strong>
          <small>
            {events.length + queue.length + portfolio.workloads.length}
          </small>
        </button>
      ) : (
        <FirehoseConfigurator
          draft={draft}
          editing={editing}
          onCancel={() => setEditing(null)}
          onChange={setDraft}
          onRemove={
            editing === "new" || streams.length === 1
              ? null
              : () => removeStream(editing)
          }
          onSave={saveDraft}
          sourceCounts={{
            admission: queue.length,
            flow: portfolio.workloads.length,
            signals: events.length,
          }}
        />
      )}
    </main>
  );
}

function FeedStream({
  events,
  index,
  omissions,
  onMove,
  onRename,
  onTune,
  portfolio,
  queue,
  sources,
  stream,
  streamCount,
}: {
  events: FeedEvent[];
  index: number;
  omissions: string[];
  onMove: (index: number, offset: -1 | 1) => void;
  onRename: (index: number, name: string) => void;
  onTune: (index: number) => void;
  portfolio: WorkloadList;
  queue: InspectionRow[];
  sources: FeedSources;
  stream: FeedStreamSpec;
  streamCount: number;
}) {
  const filteredEvents = filterEvents(events, stream.filter);
  const filteredQueue = filterQueue(queue, stream.filter);
  const filteredWorkloads = filterWorkloads(portfolio, stream.filter);
  const id = `feed-stream-${index + 1}`;
  return (
    <section
      className={sx(styles.lane)}
      aria-labelledby={id}
      data-feed-filter={stream.filter}
      data-feed-stream={stream.kind}
    >
      <LaneHeader
        id={id}
        index={String(index + 1).padStart(2, "0")}
        onMoveLeft={index > 0 ? () => onMove(index, -1) : null}
        onMoveRight={index < streamCount - 1 ? () => onMove(index, 1) : null}
        onRename={(name) => onRename(index, name)}
        onTune={() => onTune(index)}
        title={streamTitle(stream)}
      />
      <div className={sx(styles.laneScroll)} role="feed">
        {stream.kind === "signals" ? (
          <>
            <a className={sx(styles.composer)} href="/journal/new">
              <Avatar label="YOU" tone="human" />
              <div>
                <strong>Share an observation…</strong>
                <span>Write, query, map, frame, or diff.</span>
              </div>
              <b>CREATE ↗</b>
            </a>
            {filteredEvents.map((event) => (
              <FeedPost event={event} key={event.id} />
            ))}
            {filteredEvents.length === 0 ? (
              <QuietPost
                detail="No retained signal matches this stream lens."
                title="THIS SIGNAL STREAM IS QUIET"
              />
            ) : null}
            {stream.filter === "all" ? (
              <SourceBoundaryPost
                cadence={sources.cadence}
                journal={sources.journal}
                omissions={omissions}
                portfolio={portfolio}
              />
            ) : null}
          </>
        ) : null}
        {stream.kind === "admission" ? (
          <>
            {filteredQueue.length > 0 ? (
              <ReyBriefing queue={filteredQueue} />
            ) : null}
            <AdmissionControl portfolio={portfolio} />
            {filteredQueue.map((row) => (
              <AdmissionPost key={row.id} row={row} />
            ))}
            {filteredQueue.length === 0 ? (
              <QuietPost
                detail="No proposal or attention row matches this lens. No news is good news."
                title="NOTHING IS WAITING HERE"
              />
            ) : null}
          </>
        ) : null}
        {stream.kind === "flow" ? (
          <>
            {filteredWorkloads.map((workload) => (
              <FlowPost key={workload.workload.id} workload={workload} />
            ))}
            {filteredWorkloads.length === 0 ? (
              <QuietPost
                detail="No admitted workload revision matches this stream lens."
                title="NO MATCHING WORK IS IN FLOW"
              />
            ) : null}
          </>
        ) : null}
      </div>
    </section>
  );
}

function AdmissionControl({ portfolio }: { portfolio: WorkloadList }) {
  const revision = portfolio.revision;
  const [message, setMessage] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const index = revision?.index;
  const enabled = Boolean(revision?.commit_ready && index && message.trim());
  const approve = async () => {
    if (!revision || !index || !enabled) return;
    setSubmitting(true);
    setError(null);
    try {
      await approveWorkloadIndex({
        message: message.trim(),
        expected_head: revision.head?.commit_id ?? "EMPTY",
        expected_index: index.snapshot_revision,
      });
      window.location.reload();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setSubmitting(false);
    }
  };
  return (
    <div className={sx(styles.admissionBoundary)}>
      <span className={sx(chrome.micro)}>ADMISSION CONTROL</span>
      <strong>
        {revision?.commit_ready
          ? `${index?.packages.length ?? 0} QUALIFIED / READY`
          : index
            ? "INDEX REQUIRES QUALIFICATION"
            : "NO STAGED INDEX"}
      </strong>
      <p>
        {revision?.admission_boundary ??
          "No workload revision state is available."}
      </p>
      {index ? (
        <>
          <code title={index.snapshot_revision}>
            INDEX / {shortDigest(index.snapshot_revision)}
          </code>
          <input
            aria-label="Workload approval message"
            className={sx(styles.admissionMessage)}
            disabled={!revision?.commit_ready || submitting}
            maxLength={4096}
            onChange={(event) => setMessage(event.target.value)}
            placeholder="Why are you admitting this workload revision?"
            value={message}
          />
          <button
            className={sx(styles.admissionApprove)}
            disabled={!enabled || submitting}
            onClick={() => void approve()}
            type="button"
          >
            {submitting ? "ADMITTING…" : "APPROVE EXACT INDEX"}
          </button>
        </>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
    </div>
  );
}

function FirehoseConfigurator({
  draft,
  editing,
  onCancel,
  onChange,
  onRemove,
  onSave,
  sourceCounts,
}: {
  draft: FeedStreamSpec;
  editing: number | "new";
  onCancel: () => void;
  onChange: (stream: FeedStreamSpec) => void;
  onRemove: (() => void) | null;
  onSave: () => void;
  sourceCounts: Record<FeedStreamKind, number>;
}) {
  return (
    <section
      aria-labelledby="feed-firehose"
      className={sx(styles.lane, styles.firehose)}
    >
      <header className={sx(styles.firehoseHeader)}>
        <div>
          <span className={sx(chrome.micro)}>
            {editing === "new"
              ? "NEW STREAM"
              : `TUNING ${String(editing + 1).padStart(2, "0")}`}
          </span>
          <h1 id="feed-firehose">Firehose</h1>
        </div>
        <button
          aria-label="Close Firehose"
          className={sx(styles.iconButton)}
          onClick={onCancel}
          type="button"
        >
          ×
        </button>
      </header>
      <div className={sx(styles.firehoseBody)}>
        <p className={sx(styles.firehoseLead)}>
          Compose a stream as a bounded lens. The Firehose remains the union;
          lanes only select and arrange its projections.
        </p>
        <fieldset className={sx(styles.recipeGroup)}>
          <legend className={sx(chrome.micro)}>01 / SOURCE PLANE</legend>
          {(Object.keys(FEED_STREAM_FILTERS) as FeedStreamKind[]).map(
            (kind) => (
              <button
                aria-pressed={draft.kind === kind}
                className={sx(
                  styles.recipe,
                  draft.kind === kind && styles.recipeActive,
                )}
                key={kind}
                onClick={() =>
                  onChange({ kind, filter: "all", name: draft.name })
                }
                type="button"
              >
                <strong>{kind.toUpperCase()}</strong>
                <span>{streamDescription(kind)}</span>
                <b>{sourceCounts[kind]}</b>
              </button>
            ),
          )}
        </fieldset>
        <fieldset className={sx(styles.recipeGroup)}>
          <legend className={sx(chrome.micro)}>02 / LENS</legend>
          <div className={sx(styles.filterGrid)}>
            {FEED_STREAM_FILTERS[draft.kind].map((filter) => (
              <button
                aria-pressed={draft.filter === filter}
                className={sx(
                  styles.filterButton,
                  draft.filter === filter && styles.filterButtonActive,
                )}
                key={filter}
                onClick={() => onChange({ ...draft, filter })}
                type="button"
              >
                {filterLabel(filter)}
              </button>
            ))}
          </div>
        </fieldset>
        <div className={sx(styles.firehosePreview)}>
          <span className={sx(chrome.micro)}>STREAM COORDINATE</span>
          <code>{serializeFeedStreams([draft])}</code>
          <p>
            Saved in the Feed URL. Source records stay owned by their existing
            runtime contracts.
          </p>
        </div>
      </div>
      <footer className={sx(styles.firehoseActions)}>
        {onRemove ? (
          <button
            className={sx(styles.dangerButton)}
            onClick={onRemove}
            type="button"
          >
            REMOVE
          </button>
        ) : (
          <span />
        )}
        <div>
          <button
            className={sx(styles.secondaryButton)}
            onClick={onCancel}
            type="button"
          >
            CANCEL
          </button>
          <button
            className={sx(styles.primaryButton)}
            onClick={onSave}
            type="button"
          >
            {editing === "new" ? "ADD STREAM" : "APPLY LENS"}
          </button>
        </div>
      </footer>
    </section>
  );
}

function ReyBriefing({ queue }: { queue: InspectionRow[] }) {
  return (
    <article className={sx(styles.post, styles.briefing)}>
      <PostHeader
        avatar={<Avatar label="R" tone="rey" />}
        identity="REY / CURRENT PROJECTION"
        moment="PINNED"
        state={`${queue.length} PROPOSALS`}
      />
      <div className={sx(styles.postBody)}>
        <h2 className={sx(styles.postTitle)}>
          {queue.length === 0
            ? "The admission plane is quiet."
            : "Here is what may need admission next."}
        </h2>
        <p className={sx(styles.postLead)}>
          This briefing is derived from current evidence. It is not an agent
          assignment, a scheduler decision, or a claim that work began.
        </p>
        <div className={sx(styles.briefingCounts)}>
          <strong>
            {queue.filter((row) => row.urgency === "NOW").length} NOW
          </strong>
          <strong>
            {queue.filter((row) => row.urgency === "WATCH").length} WATCH
          </strong>
          <strong>
            {queue.filter((row) => row.urgency === "BOUND").length} BOUND
          </strong>
        </div>
      </div>
    </article>
  );
}

function AdmissionPost({ row }: { row: InspectionRow }) {
  return (
    <article className={sx(styles.post)} role="article">
      <PostHeader
        avatar={<Avatar label="R" tone="rey" />}
        identity={`REY / ${row.source}`}
        moment="CURRENT PROJECTION"
        state={`${row.signal} / ${row.urgency}`}
      />
      <div className={sx(styles.postBody)}>
        <h2 className={sx(styles.postTitle)}>{row.subject}</h2>
        <p className={sx(styles.postLead)}>{row.detail}</p>
        <div className={sx(styles.admissionEvidence)}>
          <span className={sx(chrome.micro)}>ADMISSION BASIS</span>
          <strong>{row.basis}</strong>
          <span>
            {row.priority === null
              ? "derived inspection posture"
              : `typed attention priority ${row.priority}`}
          </span>
        </div>
      </div>
      <footer className={sx(styles.postFooter)}>
        <span className={sx(chrome.micro)}>NO EFFECT AUTHORITY</span>
        <a className={sx(styles.postAction)} href={row.href}>
          INSPECT {row.location} →
        </a>
      </footer>
    </article>
  );
}

function FlowPost({
  workload,
}: {
  workload: WorkloadList["workloads"][number];
}) {
  const percent =
    workload.required === 0
      ? 0
      : Math.round((workload.passed * 100) / workload.required);
  return (
    <article className={sx(styles.post)} role="article">
      <PostHeader
        avatar={<Avatar label="W" tone="flow" />}
        identity={`WORKLOAD / ${workload.workload.id}`}
        moment={`REVISION ${workload.workload.revision}`}
        state={workload.qualification.toUpperCase()}
      />
      <div className={sx(styles.postBody)}>
        <h2 className={sx(styles.postTitle)}>{workload.title}</h2>
        <p className={sx(styles.postLead)}>
          {workload.passed}/{workload.required} scenarios passing ·{" "}
          {workload.attention_rows} current attention rows
        </p>
        <div className={sx(styles.flowProgress)}>
          <div>
            <strong>{percent}%</strong>
            <span className={sx(chrome.micro)}>LOCAL CONFORMANCE</span>
          </div>
          <i className={sx(styles.flowTrack)}>
            <b
              className={sx(styles.flowFill)}
              style={{ width: `${percent}%` }}
            />
          </i>
        </div>
        <div className={sx(styles.flowEvidence)}>
          <span>{workload.mining_results} mining results</span>
          <span>{workload.relation_deltas} directed deltas</span>
          <span>{workload.reasoning_surfaces} reasoning surfaces</span>
          <code title={workload.last_test_result_id ?? undefined}>
            TEST / {shortDigest(workload.last_test_result_id)}
          </code>
        </div>
      </div>
      <footer className={sx(styles.postFooter)}>
        <span className={sx(chrome.micro)}>
          RUN / {(workload.last_run_status ?? "NOT RUN").toUpperCase()}
        </span>
        <a
          className={sx(styles.postAction)}
          href={`/workloads/${encodeURIComponent(workload.workload.id)}`}
        >
          INSPECT FLOW →
        </a>
      </footer>
    </article>
  );
}

function SourceBoundaryPost({
  cadence,
  journal,
  omissions,
  portfolio,
}: {
  cadence: CadenceProjection;
  journal: JournalProjection;
  omissions: string[];
  portfolio: WorkloadList;
}) {
  return (
    <article className={sx(styles.post, styles.boundaryPost)} role="article">
      <PostHeader
        avatar={<Avatar label="∴" tone="boundary" />}
        identity="REY / SOURCE BOUNDARY"
        moment="CURRENT WINDOW"
        state="PARTIAL ORDER"
      />
      <div className={sx(styles.postBody)}>
        <h2 className={sx(styles.postTitle)}>
          Display order is not causal order.
        </h2>
        <p className={sx(styles.postLead)}>
          Wall-time posts appear newest first. Order-only posts retain their
          source position and follow the timestamped window.
        </p>
        <div className={sx(styles.sourceRecords)}>
          <SourceRecord
            identity={shortDigest(portfolio.attention.attention_id)}
            label="ATTENTION"
            state={`${portfolio.attention.rows.length} ROWS`}
          />
          <SourceRecord
            identity={shortDigest(journal.log.log_id)}
            label="JOURNAL"
            state={`${journal.log.entries.length} ENTRIES`}
          />
          <SourceRecord
            identity={cadence.ordering.toUpperCase()}
            label="CADENCE"
            state={`${cadence.lanes.length} CLOCKS`}
          />
        </div>
        <ul className={sx(styles.omissions)}>
          {omissions.map((omission) => (
            <li key={omission}>{omission}</li>
          ))}
        </ul>
      </div>
    </article>
  );
}

function QuietPost({ detail, title }: { detail: string; title: string }) {
  return (
    <article className={sx(styles.emptyPost)}>
      <Avatar label="R" tone="rey" />
      <div>
        <strong>{title}</strong>
        <p>{detail}</p>
      </div>
    </article>
  );
}

function LaneHeader({
  id,
  index,
  onMoveLeft,
  onMoveRight,
  onRename,
  onTune,
  title,
}: {
  id: string;
  index: string;
  onMoveLeft: (() => void) | null;
  onMoveRight: (() => void) | null;
  onRename: (name: string) => void;
  onTune: () => void;
  title: string;
}) {
  return (
    <header className={sx(styles.laneHeader)}>
      <div className={sx(styles.laneIdentity)}>
        <span className={sx(styles.laneIndex)}>{index}</span>
        <EditableStreamTitle id={id} onCommit={onRename} title={title} />
      </div>
      <div className={sx(styles.laneMeta)}>
        <div className={sx(styles.laneActions)}>
          <button
            aria-label={`Move ${title} left`}
            className={sx(styles.iconButton)}
            disabled={onMoveLeft === null}
            onClick={onMoveLeft ?? undefined}
            type="button"
          >
            ←
          </button>
          <button
            aria-label={`Move ${title} right`}
            className={sx(styles.iconButton)}
            disabled={onMoveRight === null}
            onClick={onMoveRight ?? undefined}
            type="button"
          >
            →
          </button>
          <button
            className={sx(styles.tuneButton)}
            onClick={onTune}
            type="button"
          >
            TUNE
          </button>
        </div>
      </div>
    </header>
  );
}

function EditableStreamTitle({
  id,
  onCommit,
  title,
}: {
  id: string;
  onCommit: (name: string) => void;
  title: string;
}) {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(title);

  useEffect(() => {
    if (!editing) setValue(title);
  }, [editing, title]);

  if (editing) {
    return (
      <input
        aria-label="Stream name"
        autoFocus
        className={sx(styles.streamTitleInput)}
        id={id}
        maxLength={96}
        onBlur={() => {
          onCommit(value);
          setEditing(false);
        }}
        onChange={(event) => setValue(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
          if (event.key === "Escape") {
            setValue(title);
            setEditing(false);
          }
        }}
        value={value}
      />
    );
  }

  return (
    <h1 className={sx(styles.laneTitle)} id={id}>
      <button
        aria-label={`Rename ${title}`}
        className={sx(styles.streamTitleButton)}
        onClick={() => {
          setValue(title);
          setEditing(true);
        }}
        title="Rename stream"
        type="button"
      >
        {title}
      </button>
    </h1>
  );
}

function FeedPost({ event }: { event: FeedEvent }) {
  return (
    <article
      className={sx(styles.post)}
      data-feed-kind={event.kind}
      role="article"
    >
      <PostHeader
        avatar={<EventAvatar event={event} />}
        identity={eventIdentity(event)}
        moment={
          event.occurredAt ? formatMoment(event.occurredAt) : "ORDER ONLY"
        }
        state={event.state}
      />
      <div className={sx(styles.postBody)}>
        <h2 className={sx(styles.postTitle)}>{event.title}</h2>
        <p className={sx(styles.postLead)}>{event.detail}</p>
        <EventAttachment event={event} />
      </div>
      <footer className={sx(styles.postFooter)}>
        <span className={sx(chrome.micro)}>
          {event.stream} / {event.position}
        </span>
        <a className={sx(styles.postAction)} href={event.href}>
          {postAction(event)} →
        </a>
      </footer>
    </article>
  );
}

function PostHeader({
  avatar,
  identity,
  moment,
  state,
}: {
  avatar: ReactNode;
  identity: string;
  moment: string;
  state: string;
}) {
  return (
    <header className={sx(styles.postHeader)}>
      {avatar}
      <div className={sx(styles.postIdentity)}>
        <strong>{identity}</strong>
        <span className={sx(chrome.micro)}>{moment}</span>
      </div>
      <span className={sx(chrome.micro, styles.stateChip)}>{state}</span>
    </header>
  );
}

function EventAvatar({ event }: { event: FeedEvent }) {
  if (event.journalEntry) {
    const author = event.journalEntry.author;
    return (
      <Avatar
        label={
          author.kind === "human" ? "H" : author.kind === "agent" ? "A" : "R"
        }
        tone={
          author.kind === "human"
            ? "human"
            : author.kind === "agent"
              ? "agent"
              : "rey"
        }
      />
    );
  }
  return (
    <Avatar
      label={event.kind === "git" ? "G" : "E"}
      tone={event.kind === "git" ? "git" : "environment"}
    />
  );
}

function Avatar({
  label,
  tone,
}: {
  label: string;
  tone: "agent" | "boundary" | "environment" | "flow" | "git" | "human" | "rey";
}) {
  return (
    <span
      aria-hidden="true"
      className={sx(
        styles.avatar,
        tone === "human" && styles.avatarHuman,
        tone === "agent" && styles.avatarAgent,
        tone === "git" && styles.avatarGit,
        tone === "environment" && styles.avatarEnvironment,
        tone === "flow" && styles.avatarFlow,
        tone === "boundary" && styles.avatarBoundary,
      )}
    >
      {label}
    </span>
  );
}

function EventAttachment({ event }: { event: FeedEvent }) {
  if (event.journalEntry)
    return <JournalAttachment entry={event.journalEntry} />;
  if (event.kind === "git") return <GitAttachment event={event} />;
  return <EnvironmentAttachment event={event} />;
}

function JournalAttachment({ entry }: { entry: RetainedJournalEntry }) {
  return (
    <details className={sx(styles.journalAttachment)}>
      <summary className={sx(styles.attachmentLabel)}>
        <span className={sx(chrome.micro)}>JOURNAL / RICH DOCUMENT</span>
        <span className={sx(chrome.micro)}>
          {entry.blocks.length} BLOCKS ＋
        </span>
      </summary>
      <div className={sx(styles.blockPreviews)}>
        {entry.blocks.slice(0, 2).map((block) => (
          <JournalBlockPreview block={block} key={block.id} />
        ))}
        {entry.blocks.length === 0 ? (
          <div className={sx(styles.blockBoundary)}>EMPTY DOCUMENT BODY</div>
        ) : null}
      </div>
      <a className={sx(styles.mapBinding)} href={entry.binding.coordinate}>
        <span className={sx(chrome.micro)}>EXACT EXPLORE BINDING</span>
        <code>{entry.binding.coordinate}</code>
        <strong>ENTER MAP →</strong>
      </a>
    </details>
  );
}

function JournalBlockPreview({ block }: { block: JournalBlock }) {
  if (block.kind === "prose") {
    return (
      <div className={sx(styles.prosePreview)}>
        {block.document.slice(0, 4).map((node, index) => {
          if (node.kind === "heading") return <h3 key={index}>{node.text}</h3>;
          if (node.kind === "code") return <pre key={index}>{node.text}</pre>;
          if (node.kind === "quote")
            return <blockquote key={index}>{node.text}</blockquote>;
          return (
            <p key={index}>
              {node.kind === "bullet" ? `• ${node.text}` : node.text}
            </p>
          );
        })}
      </div>
    );
  }
  if (block.kind === "diff") {
    return (
      <div className={sx(styles.diffPreview)}>
        <span className={sx(chrome.micro)}>
          DIRECTED DIFF / {block.assessment}
        </span>
        <div className={sx(styles.diffDirection)}>
          <code>− {block.source}</code>
          <strong>→</strong>
          <code>+ {block.target}</code>
        </div>
        <p>{block.summary}</p>
      </div>
    );
  }
  if (block.kind === "frame") {
    const columns = block.columns.slice(0, 5);
    return (
      <div className={sx(styles.framePreview)}>
        <span className={sx(chrome.micro)}>
          BOUNDED FRAME / {block.row_count} ROWS
          {block.truncated ? " / TRUNCATED" : ""}
        </span>
        <div className={sx(styles.previewTable)} role="table">
          <div
            className={sx(styles.previewRow, styles.previewHeader)}
            role="row"
          >
            {columns.map((column) => (
              <strong className={sx(styles.previewCell)} key={column.name}>
                {column.name}
              </strong>
            ))}
          </div>
          {block.preview_rows.slice(0, 3).map((row, index) => (
            <div className={sx(styles.previewRow)} key={index} role="row">
              {columns.map((column) => (
                <span className={sx(styles.previewCell)} key={column.name}>
                  {row[column.name] ?? "∅"}
                </span>
              ))}
            </div>
          ))}
        </div>
      </div>
    );
  }
  if (block.kind === "query") {
    return (
      <div className={sx(styles.queryPreview)}>
        <span className={sx(chrome.micro)}>
          READ-ONLY QUERY / {block.provider} / {block.language}
        </span>
        <pre>{block.statement}</pre>
      </div>
    );
  }
  if (block.kind === "explore") {
    return (
      <a className={sx(styles.explorePreview)} href={block.coordinate}>
        <span className={sx(chrome.micro)}>EXPLORE MAP</span>
        <strong>{block.caption ?? "Bound context topology"}</strong>
        <code>{block.coordinate}</code>
      </a>
    );
  }
  return (
    <div className={sx(styles.actionPreview)}>
      <span className={sx(chrome.micro)}>
        PROPOSED ACTION / {block.operation}
      </span>
      <strong>{block.desired_delta}</strong>
      <small>
        {block.evidence_ids.length} evidence · {block.dependency_ids.length}{" "}
        dependencies
      </small>
    </div>
  );
}

function GitAttachment({ event }: { event: FeedEvent }) {
  const parents = event.tick?.parent_revisions ?? [];
  return (
    <details className={sx(styles.journalAttachment)}>
      <summary className={sx(styles.attachmentLabel)}>
        <span className={sx(chrome.micro)}>GIT / EXACT REVISION</span>
        <span className={sx(chrome.micro)}>COMMIT + PARENTS ＋</span>
      </summary>
      <div className={sx(styles.commitAttachment)}>
        <div className={sx(styles.commitIdentity)}>
          <span className={sx(chrome.micro)}>EXACT COMMIT</span>
          <GitCommitLink
            className={sx(styles.commitLink)}
            fallback="GIT COMMIT / REPOSITORY UNBOUND"
            repository={event.repository}
            revision={event.revision}
          />
        </div>
        <div className={sx(styles.commitParents)}>
          <span className={sx(chrome.micro)}>PARENTS</span>
          {parents.length === 0 ? <code>ROOT OR BOUNDARY</code> : null}
          {parents.map((parent) => (
            <GitCommitLink
              className={sx(styles.parentLink)}
              fallback="REPOSITORY UNBOUND"
              key={parent}
              repository={event.repository}
              revision={parent}
            />
          ))}
        </div>
      </div>
    </details>
  );
}

function EnvironmentAttachment({ event }: { event: FeedEvent }) {
  return (
    <details className={sx(styles.journalAttachment)}>
      <summary className={sx(styles.attachmentLabel)}>
        <span className={sx(chrome.micro)}>ENV / {event.position}</span>
        <code title={event.revision}>{shortDigest(event.revision)} ＋</code>
      </summary>
      <div className={sx(styles.environmentAttachment)}>
        <div className={sx(styles.attachmentCell)}>
          <span className={sx(chrome.micro)}>ENVIRONMENT TRANSITION</span>
          <strong>{event.position}</strong>
        </div>
        <i aria-hidden="true">→</i>
        <div className={sx(styles.attachmentCell)}>
          <span className={sx(chrome.micro)}>EVIDENCE IDENTITY</span>
          <code title={event.revision}>{shortDigest(event.revision)}</code>
        </div>
      </div>
    </details>
  );
}

function SourceRecord({
  identity,
  label,
  state,
}: {
  identity: string;
  label: string;
  state: string;
}) {
  return (
    <article className={sx(styles.sourceRecord)}>
      <span className={sx(chrome.micro)}>{label}</span>
      <strong>{state}</strong>
      <code>{identity}</code>
    </article>
  );
}

function eventIdentity(event: FeedEvent): string {
  const author = event.journalEntry?.author;
  if (author) return `${author.kind} / ${author.id}`.toUpperCase();
  return event.kind === "git" ? "GIT / SOURCE REPOSITORY" : "REY / ENVIRONMENT";
}

function postAction(event: FeedEvent): string {
  if (event.kind === "journal") return "OPEN ENTRY";
  if (event.kind === "git") return "INSPECT CADENCE";
  return "OPEN ENVIRONMENT";
}

function boundedOmissions(
  cadence: CadenceProjection,
  foldedEvents: number,
): string[] {
  return [
    ...new Set([
      ...cadence.omissions,
      ...(cadence.repository_state?.omissions ?? []),
      ...cadence.lanes.flatMap((lane) => lane.omissions),
      "workload test and run results have no retained Feed clock",
      "operator read and unread state is not retained",
      ...(foldedEvents > 0
        ? [
            `${foldedEvents} older signals folded by the ${FEED_EVENT_LIMIT}-record Feed window`,
          ]
        : []),
    ]),
  ];
}

function urgencyForReadiness(readiness: AttentionReadiness): FeedUrgency {
  return readiness === "ready" ? "NOW" : "BOUND";
}

function urgencyRank(urgency: FeedUrgency): number {
  if (urgency === "NOW") return 0;
  if (urgency === "WATCH") return 1;
  return 2;
}

function qualificationSignal(qualification: string): string {
  if (qualification === "untested") return "TEST";
  if (qualification === "stale") return "RETEST";
  return "REFINE";
}

function publicationSignal(state: string): string {
  if (state === "unpushed") return "PUBLISH";
  if (state === "behind" || state === "diverged") return "RECONCILE";
  return "BOUND";
}

function formatMoment(value: string): string {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return value;
  return date.toISOString().replace("T", " ").slice(0, 16) + "Z";
}

function isFeedStreamKind(value: unknown): value is FeedStreamKind {
  return value === "signals" || value === "admission" || value === "flow";
}

function isFeedStreamFilter(
  kind: FeedStreamKind,
  value: unknown,
): value is FeedStreamFilter {
  return (
    typeof value === "string" &&
    FEED_STREAM_FILTERS[kind].some((filter) => filter === value)
  );
}

function filterEvents(
  events: FeedEvent[],
  filter: FeedStreamFilter,
): FeedEvent[] {
  if (filter === "all") return events;
  if (filter === "journal")
    return events.filter((event) => event.kind === "journal");
  if (filter === "git") return events.filter((event) => event.kind === "git");
  if (filter === "environment")
    return events.filter((event) => event.kind === "environment");
  return events;
}

function filterQueue(
  queue: InspectionRow[],
  filter: FeedStreamFilter,
): InspectionRow[] {
  if (filter === "now") return queue.filter((row) => row.urgency === "NOW");
  if (filter === "watch") return queue.filter((row) => row.urgency === "WATCH");
  if (filter === "bound") return queue.filter((row) => row.urgency === "BOUND");
  return queue;
}

function filterWorkloads(
  portfolio: WorkloadList,
  filter: FeedStreamFilter,
): WorkloadList["workloads"] {
  if (filter === "attention")
    return portfolio.workloads.filter(
      (workload) => workload.attention_rows > 0,
    );
  if (filter === "failing")
    return portfolio.workloads.filter(
      (workload) =>
        workload.qualification === "failing" ||
        workload.qualification === "inconclusive" ||
        workload.qualification === "stale",
    );
  if (filter === "qualified")
    return portfolio.workloads.filter(
      (workload) => workload.qualification === "qualified",
    );
  return portfolio.workloads;
}

function streamTitle(stream: FeedStreamSpec): string {
  if (stream.name) return stream.name;
  if (stream.filter === "all")
    return stream.kind[0]!.toUpperCase() + stream.kind.slice(1);
  return `${filterLabel(stream.filter)} ${stream.kind}`;
}

function streamDescription(kind: FeedStreamKind): string {
  if (kind === "signals") return "Journal, Git, and environment records";
  if (kind === "admission") return "Work proposed from current evidence";
  return "Admitted workloads and retained results";
}

function filterLabel(filter: FeedStreamFilter): string {
  return filter.toUpperCase();
}
