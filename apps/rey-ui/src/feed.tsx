import { admitWorkloadFiles, type FeedSources } from "./api";
import {
  useEffect,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import type { CadenceProjection, CadenceTick } from "./cadence";
import type {
  ChannelApplyResult,
  ChannelGraph,
  ChannelGraphSnapshot,
  ChannelProjection,
  ChannelWorkingWriteRequest,
  FeedStreamDefinition,
} from "./channels";
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
import {
  observationPosition,
  type ObservationFrontier,
  type ObservationFrontierRow,
} from "./observations";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { feedStyles as styles } from "./stylex/feed.stylex";
import { className as sx } from "./stylex/shared.stylex";

export type FeedUrgency = "NOW" | "WATCH" | "BOUND";
export const FEED_EVENT_LIMIT = 64;
export const FEED_STREAM_LIMIT = 8;

export type FeedStreamKind = "signals" | "admission" | "flow";
export type FeedStreamFilter =
  | "all"
  | "observation"
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
  id?: string;
  kind: FeedStreamKind;
  filter: FeedStreamFilter;
  name?: string;
}

export interface ResolvedFeedStream extends FeedStreamSpec {
  id: string;
  revision: number | null;
  subscriptionId: string | null;
}

export type FeedLayoutSource =
  "url_preview" | "channel_working" | "channel_head" | "built_in";

export interface FeedLayoutResolution {
  source: FeedLayoutSource;
  detached: boolean;
  snapshotId: string | null;
  graphId: string | null;
  streams: ResolvedFeedStream[];
  omissions: string[];
}

export interface FeedLayoutWriteOutcome {
  result: ChannelApplyResult | null;
  projection: ChannelProjection;
  error: Error | null;
}

export interface FeedLayoutMovementOutcome {
  streams: ResolvedFeedStream[];
  result: ChannelApplyResult | null;
  error: Error | null;
}

export const DEFAULT_FEED_STREAMS: FeedStreamSpec[] = [
  { kind: "signals", filter: "all" },
  { kind: "admission", filter: "all" },
  { kind: "flow", filter: "all" },
];

const FEED_STREAM_FILTERS: Record<FeedStreamKind, FeedStreamFilter[]> = {
  signals: ["all", "observation", "journal", "git", "environment"],
  admission: ["all", "now", "watch", "bound"],
  flow: ["all", "attention", "failing", "qualified"],
};

interface ParsedFeedPreview {
  streams: FeedStreamSpec[];
  rejected: number;
}

export function parseFeedPreview(
  value: string | null,
): ParsedFeedPreview | null {
  if (!value) return null;
  let rejected = 0;
  const parsedStreams = value
    .split(",")
    .slice(0, FEED_STREAM_LIMIT)
    .flatMap((token): FeedStreamSpec[] => {
      const [identityAndCoordinate, encodedName, ...nameRest] =
        token.split("~");
      if (nameRest.length > 0) {
        rejected += 1;
        return [];
      }
      const separator = identityAndCoordinate!.indexOf("=");
      const id =
        separator < 0 ? undefined : identityAndCoordinate!.slice(0, separator);
      const coordinate =
        separator < 0
          ? identityAndCoordinate!
          : identityAndCoordinate!.slice(separator + 1);
      if (id !== undefined && !isFeedStreamId(id)) {
        rejected += 1;
        return [];
      }
      const [kindValue, filterValue, ...coordinateRest] =
        coordinate!.split(".");
      if (coordinateRest.length > 0 || !isFeedStreamKind(kindValue)) {
        rejected += 1;
        return [];
      }
      const filter = filterValue ?? "all";
      if (!isFeedStreamFilter(kindValue, filter)) {
        rejected += 1;
        return [];
      }
      let name: string | undefined;
      if (encodedName !== undefined) {
        try {
          name = normalizeFeedStreamName(decodeURIComponent(encodedName));
        } catch {
          rejected += 1;
          return [];
        }
      }
      return [
        {
          ...(id ? { id } : {}),
          kind: kindValue,
          filter,
          ...(name ? { name } : {}),
        },
      ];
    });
  if (value.split(",").length > FEED_STREAM_LIMIT) {
    rejected += value.split(",").length - FEED_STREAM_LIMIT;
  }
  const identities = new Set<string>();
  const streams = parsedStreams.filter((stream) => {
    if (!stream.id) return true;
    if (identities.has(stream.id)) {
      rejected += 1;
      return false;
    }
    identities.add(stream.id);
    return true;
  });
  return { streams, rejected };
}

export function parseFeedStreams(value: string | null): FeedStreamSpec[] {
  const preview = parseFeedPreview(value);
  return preview && preview.streams.length > 0
    ? preview.streams
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
      const identity =
        stream.id && isFeedStreamId(stream.id) ? `${stream.id}=` : "";
      return `${identity}${stream.kind}.${stream.filter}${encodedName}`;
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

export function resolveFeedLayout(
  urlPreview: string | null,
  channels: ChannelProjection,
): FeedLayoutResolution {
  const parsedPreview = parseFeedPreview(urlPreview);
  const base = channels.status.working_present
    ? channels.status.working
    : channels.status.head;
  if (parsedPreview) {
    return {
      source: "url_preview",
      detached: true,
      snapshotId: base.snapshot_id,
      graphId: base.graph_id,
      streams: materializeFeedStreamIds(parsedPreview.streams, base),
      omissions:
        parsedPreview.rejected > 0
          ? [
              `${parsedPreview.rejected} invalid or over-limit URL stream coordinates were omitted`,
            ]
          : [],
    };
  }
  const source: FeedLayoutSource = channels.status.working_present
    ? "channel_working"
    : channels.status.head_commit
      ? "channel_head"
      : "built_in";
  return resolutionFromSnapshot(source, base);
}

export function channelWorkingWriteForFeedLayout(
  channels: ChannelProjection,
  streams: readonly ResolvedFeedStream[],
): ChannelWorkingWriteRequest {
  const base = channels.status.working;
  const graph = graphWithFeedLayout(base.graph, streams);
  return {
    schema: "rey.ui-channel-working-write.v1",
    expected_head_snapshot_id: channels.status.head.snapshot_id,
    expected_working_snapshot_id: base.snapshot_id,
    graph,
  };
}

export function reorderFeedStreams(
  streams: readonly ResolvedFeedStream[],
  sourceId: string,
  targetId: string,
): ResolvedFeedStream[] {
  const sourceIndex = streams.findIndex((stream) => stream.id === sourceId);
  const targetIndex = streams.findIndex((stream) => stream.id === targetId);
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) {
    return streams.map((stream) => ({ ...stream }));
  }
  const next = streams.map((stream) => ({ ...stream }));
  const [source] = next.splice(sourceIndex, 1);
  next.splice(targetIndex, 0, source!);
  return next;
}

export async function persistFeedLayoutMovement(
  previous: readonly ResolvedFeedStream[],
  next: readonly ResolvedFeedStream[],
  write: (
    streams: readonly ResolvedFeedStream[],
  ) => Promise<FeedLayoutWriteOutcome>,
): Promise<FeedLayoutMovementOutcome> {
  try {
    const outcome = await write(next);
    return {
      streams: resolveFeedLayout(null, outcome.projection).streams,
      result: outcome.result,
      error: outcome.error,
    };
  } catch (error) {
    return {
      streams: previous.map((stream) => ({ ...stream })),
      result: null,
      error: error instanceof Error ? error : new Error(String(error)),
    };
  }
}

function resolutionFromSnapshot(
  source: Exclude<FeedLayoutSource, "url_preview">,
  snapshot: ChannelGraphSnapshot,
): FeedLayoutResolution {
  const omissions: string[] = [];
  const streams = snapshot.graph.layout.stream_ids.flatMap((streamId) => {
    const stream = snapshot.graph.streams.find(
      (candidate) => candidate.id === streamId,
    );
    if (!stream) {
      omissions.push(`layout references missing stream ${streamId}`);
      return [];
    }
    const lens = parseChannelLens(stream);
    if (!lens) {
      omissions.push(
        `stream ${stream.id} has unsupported Feed lens ${stream.lens}`,
      );
      return [];
    }
    return [
      {
        id: stream.id,
        revision: stream.revision,
        subscriptionId: stream.subscription_id,
        kind: lens.kind,
        filter: lens.filter,
        name: stream.name,
      } satisfies ResolvedFeedStream,
    ];
  });
  return {
    source,
    detached: false,
    snapshotId: snapshot.snapshot_id,
    graphId: snapshot.graph_id,
    streams,
    omissions,
  };
}

function parseChannelLens(
  stream: FeedStreamDefinition,
): Pick<FeedStreamSpec, "kind" | "filter"> | null {
  const [kind, filter = "all", ...rest] = stream.lens.split(".");
  if (
    rest.length > 0 ||
    !isFeedStreamKind(kind) ||
    !isFeedStreamFilter(kind, filter)
  ) {
    return null;
  }
  return { kind, filter };
}

function graphWithFeedLayout(
  base: ChannelGraph,
  streams: readonly ResolvedFeedStream[],
): ChannelGraph {
  const definitions = streams.map((stream) => {
    const current = base.streams.find(
      (candidate) => candidate.id === stream.id,
    );
    const desired = {
      id: stream.id,
      name: normalizeFeedStreamName(stream.name ?? "") ?? streamTitle(stream),
      subscription_id:
        current?.subscription_id ??
        stream.subscriptionId ??
        base.subscriptions[0]!.id,
      lens: feedLens(stream),
    };
    const changed =
      current !== undefined &&
      (current.name !== desired.name ||
        current.subscription_id !== desired.subscription_id ||
        current.lens !== desired.lens);
    return {
      ...desired,
      revision: current ? current.revision + (changed ? 1 : 0) : 1,
    };
  });
  definitions.sort((left, right) =>
    left.id < right.id ? -1 : left.id > right.id ? 1 : 0,
  );
  const streamIds = streams.map((stream) => stream.id);
  const layoutChanged =
    base.layout.stream_ids.length !== streamIds.length ||
    base.layout.stream_ids.some((id, index) => id !== streamIds[index]);
  return {
    ...base,
    streams: definitions,
    layout: {
      ...base.layout,
      revision: base.layout.revision + (layoutChanged ? 1 : 0),
      stream_ids: streamIds,
    },
  };
}

function materializeFeedStreamIds(
  streams: readonly FeedStreamSpec[],
  base: ChannelGraphSnapshot,
): ResolvedFeedStream[] {
  const used = new Set<string>();
  return streams.map((stream) => {
    let id = stream.id && !used.has(stream.id) ? stream.id : undefined;
    const expectedName =
      normalizeFeedStreamName(stream.name ?? "") ?? streamTitle(stream);
    const matched = id
      ? base.graph.streams.find((candidate) => candidate.id === id)
      : base.graph.streams.find(
          (candidate) =>
            !used.has(candidate.id) &&
            candidate.lens === feedLens(stream) &&
            candidate.name === expectedName,
        );
    id ??= matched?.id;
    id ??= allocateFeedStreamId(stream.kind, used, base.graph.streams);
    used.add(id);
    return {
      ...stream,
      id,
      revision: matched?.revision ?? null,
      subscriptionId: matched?.subscription_id ?? null,
    };
  });
}

function allocateFeedStreamId(
  kind: FeedStreamKind,
  used: ReadonlySet<string>,
  existing: readonly FeedStreamDefinition[],
): string {
  const unavailable = new Set(existing.map((stream) => stream.id));
  for (let sequence = 1; sequence <= FEED_STREAM_LIMIT + 1; sequence += 1) {
    const candidate = sequence === 1 ? kind : `${kind}-${sequence}`;
    if (!used.has(candidate) && !unavailable.has(candidate)) return candidate;
  }
  return `stream-${used.size + 1}`;
}

function feedLens(stream: FeedStreamSpec): string {
  return stream.filter === "all"
    ? stream.kind
    : `${stream.kind}.${stream.filter}`;
}

function isFeedStreamId(value: string): boolean {
  return /^[a-z0-9][a-z0-9._-]{0,79}$/.test(value);
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
  stream: "GIT" | "JOURNAL" | "OBSERVATION" | "REY ENV";
  position: string;
  title: string;
  detail: string;
  state: string;
  revision: string;
  occurredAt: string | null;
  sortTime: number | null;
  sourceOrder: number;
  href: string | null;
  kind: "git" | "journal" | "observation" | "environment";
  repository: string | null;
  journalEntry: RetainedJournalEntry | null;
  observation: ObservationFrontierRow | null;
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
      signal: "REVIEW",
      detail: `${packageSnapshot?.title ?? change.workload_id} is an incoming agent-authored file package awaiting human admission.`,
      urgency: "NOW",
      priority: null,
      sortPriority: 25,
      basis: `${change.change_kind} · ${packageSnapshot?.source_digest ?? change.target_revision ?? "missing digest"}`,
      href: `/workloads/${encodeURIComponent(change.workload_id)}`,
      location: "ADMISSION",
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
  observations: ObservationFrontier,
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
      observation: null,
      tick: null,
    });
  }
  for (const row of observations.rows) {
    const observation = row.observation;
    events.push({
      id: `observation:${observation.observation_id}`,
      stream: "OBSERVATION",
      position: observationPosition(row),
      title: observation.proposal.subject_locator,
      detail: observation.proposal.body,
      state:
        `${observation.proposal.kind} · ${observation.proposal.completeness}`.toUpperCase(),
      revision: observation.observation_id,
      occurredAt: null,
      sortTime: null,
      sourceOrder: sourceOrder++,
      href: null,
      kind: "observation",
      repository: null,
      journalEntry: null,
      observation: row,
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
        observation: null,
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
  layout,
  onAdopted,
  onConfigurationChange,
  onLayoutWrite,
  portfolio,
  sources,
}: {
  configuration?: FeedStreamSpec[];
  layout?: FeedLayoutResolution;
  onAdopted?: () => void;
  onConfigurationChange?: (streams: FeedStreamSpec[]) => void;
  onLayoutWrite?: (
    streams: readonly ResolvedFeedStream[],
  ) => Promise<FeedLayoutWriteOutcome>;
  portfolio: WorkloadList;
  sources: Pick<FeedSources, "cadence" | "journal" | "observations">;
}) {
  const queue = deriveInspectionQueue(portfolio, sources.cadence);
  const events = deriveFeedEvents(
    sources.cadence,
    sources.journal,
    sources.observations,
  );
  const resolvedLayout =
    layout ?? detachedFeedLayout(configuration ?? parseFeedStreams(null));
  const [streams, setStreams] = useState<ResolvedFeedStream[]>(() =>
    resolvedLayout.streams.map((stream) => ({ ...stream })),
  );
  const layoutKey = `${resolvedLayout.source}:${resolvedLayout.snapshotId ?? "none"}:${serializeFeedStreams(resolvedLayout.streams)}`;
  const [editing, setEditing] = useState<number | "new" | null>(null);
  const [draft, setDraft] = useState<FeedStreamSpec>({
    kind: "signals",
    filter: "all",
  });
  const [draggingStreamId, setDraggingStreamId] = useState<string | null>(null);
  const [writingLayout, setWritingLayout] = useState(false);
  const [layoutResult, setLayoutResult] = useState<ChannelApplyResult | null>(
    null,
  );
  const [layoutWriteError, setLayoutWriteError] = useState<Error | null>(null);
  const sourceEventCount =
    sources.journal.log.entries.length +
    sources.observations.rows.length +
    sources.cadence.lanes.reduce((count, lane) => count + lane.ticks.length, 0);
  const foldedEvents = Math.max(0, sourceEventCount - events.length);
  const omissions = boundedOmissions(
    sources.cadence,
    sources.observations,
    foldedEvents,
  );

  useEffect(() => {
    setStreams(resolvedLayout.streams.map((stream) => ({ ...stream })));
  }, [layoutKey]);

  const publishPreview = (next: ResolvedFeedStream[]) => {
    const bounded = next.slice(0, FEED_STREAM_LIMIT);
    setStreams(bounded);
    onConfigurationChange?.(bounded);
    setLayoutResult(null);
    setLayoutWriteError(null);
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
    if (editing === "new") {
      const used = new Set(streams.map((stream) => stream.id));
      publishPreview([
        ...streams,
        {
          ...draft,
          id: allocateFeedStreamId(draft.kind, used, []),
          revision: null,
          subscriptionId: null,
        },
      ]);
    } else {
      publishPreview(
        streams.map((stream, index) =>
          index === editing
            ? {
                ...stream,
                kind: draft.kind,
                filter: draft.filter,
                ...(draft.name ? { name: draft.name } : { name: undefined }),
              }
            : stream,
        ),
      );
    }
    setEditing(null);
  };

  const removeStream = (index: number) => {
    if (streams.length === 1) return;
    publishPreview(streams.filter((_, candidate) => candidate !== index));
    setEditing(null);
  };

  const persistMovement = async (
    previous: ResolvedFeedStream[],
    next: ResolvedFeedStream[],
  ) => {
    setStreams(next);
    setLayoutResult(null);
    setLayoutWriteError(null);
    if (resolvedLayout.detached || !onLayoutWrite) {
      onConfigurationChange?.(next);
      return;
    }
    if (resolvedLayout.omissions.length > 0 || writingLayout) {
      setStreams(previous);
      return;
    }
    setWritingLayout(true);
    const outcome = await persistFeedLayoutMovement(
      previous,
      next,
      onLayoutWrite,
    );
    setStreams(outcome.streams);
    setLayoutResult(outcome.result);
    setLayoutWriteError(outcome.error);
    setWritingLayout(false);
  };

  const moveStream = (streamId: string, offset: -1 | 1) => {
    const index = streams.findIndex((stream) => stream.id === streamId);
    const destination = index + offset;
    if (destination < 0 || destination >= streams.length) return;
    const next = [...streams];
    [next[index], next[destination]] = [next[destination]!, next[index]!];
    void persistMovement(streams, next);
  };

  const moveStreamTo = (sourceId: string, targetId: string) => {
    const next = reorderFeedStreams(streams, sourceId, targetId);
    if (next.every((stream, index) => stream.id === streams[index]?.id)) return;
    void persistMovement(streams, next);
  };

  const renameStream = (index: number, name: string) => {
    const normalized = normalizeFeedStreamName(name);
    const current = streams[index];
    if (!current) return;
    const derived = streamTitle({ ...current, name: undefined });
    const customName = normalized === derived ? undefined : normalized;
    if (current.name === customName) return;
    publishPreview(
      streams.map((stream, candidate) =>
        candidate === index ? { ...stream, name: customName } : stream,
      ),
    );
  };

  const adoptPreview = async () => {
    if (
      !resolvedLayout.detached ||
      !onLayoutWrite ||
      resolvedLayout.omissions.length > 0 ||
      writingLayout
    ) {
      return;
    }
    setWritingLayout(true);
    setLayoutResult(null);
    setLayoutWriteError(null);
    try {
      const outcome = await onLayoutWrite(streams);
      setStreams(
        resolveFeedLayout(null, outcome.projection).streams.map((stream) => ({
          ...stream,
        })),
      );
      setLayoutResult(outcome.result);
      setLayoutWriteError(outcome.error);
      if (!outcome.error) onAdopted?.();
    } catch (error) {
      setLayoutWriteError(
        error instanceof Error ? error : new Error(String(error)),
      );
    } finally {
      setWritingLayout(false);
    }
  };

  return (
    <main className={sx(styles.page)} data-feed-streams={streams.length}>
      <FeedLayoutBoundary
        layout={resolvedLayout}
        onAdopt={onLayoutWrite ? () => void adoptPreview() : null}
        result={layoutResult}
        writeError={layoutWriteError}
        writing={writingLayout}
      />
      <div className={sx(styles.streamDeck)}>
        {streams.map((stream, index) => (
          <FeedStream
            dragging={draggingStreamId === stream.id}
            events={events}
            index={index}
            key={stream.id}
            movementDisabled={writingLayout}
            omissions={omissions}
            onDragEnd={() => setDraggingStreamId(null)}
            onDragStart={() => setDraggingStreamId(stream.id)}
            onDrop={(sourceId, targetId) => {
              moveStreamTo(sourceId, targetId);
              setDraggingStreamId(null);
            }}
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
      </div>
    </main>
  );
}

function detachedFeedLayout(
  configuration: readonly FeedStreamSpec[],
): FeedLayoutResolution {
  const used = new Set<string>();
  return {
    source: "url_preview",
    detached: true,
    snapshotId: null,
    graphId: null,
    streams: configuration.slice(0, FEED_STREAM_LIMIT).map((stream) => {
      const id =
        stream.id && !used.has(stream.id)
          ? stream.id
          : allocateFeedStreamId(stream.kind, used, []);
      used.add(id);
      return {
        ...stream,
        id,
        revision: null,
        subscriptionId: null,
      };
    }),
    omissions: [],
  };
}

function FeedLayoutBoundary({
  layout,
  onAdopt,
  result,
  writeError,
  writing,
}: {
  layout: FeedLayoutResolution;
  onAdopt: (() => void) | null;
  result: ChannelApplyResult | null;
  writeError: Error | null;
  writing: boolean;
}) {
  const source = layout.source.replaceAll("_", " ").toUpperCase();
  return (
    <aside
      className={sx(styles.layoutBoundary)}
      aria-label="Feed layout source"
    >
      <div className={sx(styles.layoutIdentity)}>
        <span className={sx(chrome.micro)}>FEED LAYOUT / {source}</span>
        <strong>
          {layout.detached
            ? "DETACHED PREVIEW · NOT RETAINED"
            : `${layout.streams.length} STABLE STREAM IDENTITIES`}
        </strong>
      </div>
      <div className={sx(styles.layoutRevision)}>
        {layout.snapshotId ? (
          <code title={layout.snapshotId}>
            SNAPSHOT / {shortDigest(layout.snapshotId)}
          </code>
        ) : (
          <code>SNAPSHOT / DETACHED</code>
        )}
        {layout.graphId ? (
          <code title={layout.graphId}>
            GRAPH / {shortDigest(layout.graphId)}
          </code>
        ) : null}
      </div>
      {layout.omissions.map((omission) => (
        <span className={sx(styles.layoutOmission)} key={omission}>
          OMITTED / {omission}
        </span>
      ))}
      {result ? (
        <span className={sx(styles.layoutResult)} title={result.delta.delta_id}>
          DELTA / {result.delta.assessment.toUpperCase()} ·{" "}
          {result.delta.summary.moved} MOVED · {result.delta.summary.modified}{" "}
          MODIFIED
        </span>
      ) : null}
      {writeError ? (
        <span className={sx(styles.layoutError)} role="alert">
          WORKING WRITE REJECTED · VIEW ROLLED BACK · {writeError.message}
        </span>
      ) : null}
      {layout.detached ? (
        onAdopt ? (
          <button
            className={sx(styles.adoptButton)}
            disabled={writing || layout.omissions.length > 0}
            onClick={onAdopt}
            type="button"
          >
            {writing ? "ADOPTING…" : "ADOPT INTO CHANNEL WORKING"}
          </button>
        ) : (
          <span className={sx(styles.layoutAuthority)}>
            PREVIEW ONLY · ADOPTION UNAVAILABLE
          </span>
        )
      ) : (
        <span className={sx(styles.layoutAuthority)}>
          {writing ? "WRITING EXACT WORKING…" : "MOVEMENT WRITES WORKING ONLY"}
        </span>
      )}
    </aside>
  );
}

function FeedStream({
  dragging,
  events,
  index,
  movementDisabled,
  omissions,
  onDragEnd,
  onDragStart,
  onDrop,
  onMove,
  onRename,
  onTune,
  portfolio,
  queue,
  sources,
  stream,
  streamCount,
}: {
  dragging: boolean;
  events: FeedEvent[];
  index: number;
  movementDisabled: boolean;
  omissions: string[];
  onDragEnd: () => void;
  onDragStart: () => void;
  onDrop: (sourceId: string, targetId: string) => void;
  onMove: (streamId: string, offset: -1 | 1) => void;
  onRename: (index: number, name: string) => void;
  onTune: (index: number) => void;
  portfolio: WorkloadList;
  queue: InspectionRow[];
  sources: Pick<FeedSources, "cadence" | "journal" | "observations">;
  stream: ResolvedFeedStream;
  streamCount: number;
}) {
  const filteredEvents = filterEvents(events, stream.filter);
  const filteredQueue = filterQueue(queue, stream.filter);
  const filteredWorkloads = filterWorkloads(portfolio, stream.filter);
  const id = `feed-stream-${stream.id}`;
  return (
    <section
      className={sx(styles.lane)}
      aria-labelledby={id}
      data-feed-filter={stream.filter}
      data-feed-stream-id={stream.id}
      data-feed-stream={stream.kind}
      data-feed-stream-revision={stream.revision ?? "detached"}
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => {
        event.preventDefault();
        const sourceId = event.dataTransfer.getData("text/plain");
        if (sourceId) onDrop(sourceId, stream.id);
      }}
    >
      <LaneHeader
        dragging={dragging}
        id={id}
        index={String(index + 1).padStart(2, "0")}
        movementDisabled={movementDisabled}
        onDragEnd={onDragEnd}
        onDragStart={onDragStart}
        onMoveLeft={index > 0 ? () => onMove(stream.id, -1) : null}
        onMoveRight={
          index < streamCount - 1 ? () => onMove(stream.id, 1) : null
        }
        onRename={(name) => onRename(index, name)}
        onTune={() => onTune(index)}
        streamId={stream.id}
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
                observations={sources.observations}
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
  const working = revision?.working;
  const hasPendingFiles = Boolean(
    working &&
    working.packages.length > 0 &&
    revision?.head?.snapshot.snapshot_revision !== working.snapshot_revision,
  );
  const enabled = Boolean(hasPendingFiles && message.trim());
  const approve = async () => {
    if (!revision || !working || !enabled) return;
    setSubmitting(true);
    setError(null);
    try {
      await admitWorkloadFiles({
        message: message.trim(),
        expected_head: revision.head?.commit_id ?? "EMPTY",
        expected_working: working.snapshot_revision,
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
        {hasPendingFiles
          ? `${working?.packages.length ?? 0} FILE PACKAGE${working?.packages.length === 1 ? "" : "S"} / READY FOR REVIEW`
          : "NO INCOMING FILE CHANGES"}
      </strong>
      <p>
        {revision?.admission_boundary ??
          "No workload revision state is available."}
      </p>
      {revision?.working.ignore ? (
        <code title={revision.working.ignore.source_digest}>
          {revision.working.ignore.source} /{" "}
          {revision.working.ignore.rules.length} RULES /{" "}
          {revision.working.ignore.ignored} OMITTED
        </code>
      ) : null}
      {hasPendingFiles && working ? (
        <>
          <code title={working.snapshot_revision}>
            WORKING FILES / {shortDigest(working.snapshot_revision)}
          </code>
          <input
            aria-label="Workload approval message"
            className={sx(styles.admissionMessage)}
            disabled={submitting}
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
            {submitting
              ? "QUALIFYING & ADMITTING…"
              : "ADMIT EXACT FILE SNAPSHOT"}
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
            Applying creates a detached Feed URL preview. Adopt that preview
            explicitly to replace Channel WORKING; source records stay owned by
            their existing runtime contracts.
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
  observations,
  omissions,
  portfolio,
}: {
  cadence: CadenceProjection;
  journal: JournalProjection;
  observations: ObservationFrontier;
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
            identity={shortDigest(observations.frontier_id)}
            label="OBSERVATIONS"
            state={`${observations.rows.length} OPEN / ${observations.omitted} OMITTED`}
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
  dragging,
  id,
  index,
  movementDisabled,
  onDragEnd,
  onDragStart,
  onMoveLeft,
  onMoveRight,
  onRename,
  onTune,
  streamId,
  title,
}: {
  dragging: boolean;
  id: string;
  index: string;
  movementDisabled: boolean;
  onDragEnd: () => void;
  onDragStart: () => void;
  onMoveLeft: (() => void) | null;
  onMoveRight: (() => void) | null;
  onRename: (name: string) => void;
  onTune: () => void;
  streamId: string;
  title: string;
}) {
  const moveWithKeyboard = (event: KeyboardEvent<HTMLElement>) => {
    if (
      !event.altKey ||
      movementDisabled ||
      event.target !== event.currentTarget
    )
      return;
    if (event.key === "ArrowLeft" && onMoveLeft) {
      event.preventDefault();
      onMoveLeft();
    }
    if (event.key === "ArrowRight" && onMoveRight) {
      event.preventDefault();
      onMoveRight();
    }
  };
  return (
    <header
      aria-keyshortcuts="Alt+ArrowLeft Alt+ArrowRight"
      className={sx(styles.laneHeader, dragging && styles.laneHeaderDragging)}
      data-feed-drag-identity={streamId}
      draggable={!movementDisabled}
      onDragEnd={onDragEnd}
      onDragStart={(event: DragEvent<HTMLElement>) => {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", streamId);
        onDragStart();
      }}
      onKeyDown={moveWithKeyboard}
      tabIndex={0}
    >
      <div className={sx(styles.laneIdentity)}>
        <span
          className={sx(styles.laneIndex)}
          title={`Stable stream ${streamId}`}
        >
          {index}
        </span>
        <EditableStreamTitle id={id} onCommit={onRename} title={title} />
      </div>
      <div className={sx(styles.laneMeta)}>
        <div className={sx(styles.laneActions)}>
          <button
            aria-label={`Move ${title} left`}
            aria-keyshortcuts="Alt+ArrowLeft"
            className={sx(styles.iconButton)}
            disabled={movementDisabled || onMoveLeft === null}
            onClick={onMoveLeft ?? undefined}
            type="button"
          >
            ←
          </button>
          <button
            aria-label={`Move ${title} right`}
            aria-keyshortcuts="Alt+ArrowRight"
            className={sx(styles.iconButton)}
            disabled={movementDisabled || onMoveRight === null}
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
        {event.href ? (
          <a className={sx(styles.postAction)} href={event.href}>
            {postAction(event)} →
          </a>
        ) : (
          <span className={sx(chrome.micro)}>NO EFFECT AUTHORITY</span>
        )}
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
  const author =
    event.journalEntry?.author ??
    event.observation?.observation.proposal.author;
  if (author) {
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
  if (event.observation)
    return <ObservationAttachment row={event.observation} />;
  if (event.kind === "git") return <GitAttachment event={event} />;
  return <EnvironmentAttachment event={event} />;
}

function ObservationAttachment({ row }: { row: ObservationFrontierRow }) {
  const observation = row.observation;
  const proposal = observation.proposal;
  return (
    <details className={sx(styles.journalAttachment)}>
      <summary className={sx(styles.attachmentLabel)}>
        <span className={sx(chrome.micro)}>
          OBSERVATION / {observationPosition(row)} / ORDER ONLY
        </span>
        <code title={observation.observation_id}>
          {shortDigest(observation.observation_id)} ＋
        </code>
      </summary>
      <div className={sx(styles.blockPreviews)}>
        <div className={sx(styles.prosePreview)}>
          <span className={sx(chrome.micro)}>EXACT SUBJECT</span>
          <code>{proposal.subject_locator}</code>
          {proposal.desired_delta ? (
            <p>
              <strong>Desired delta:</strong> {proposal.desired_delta}
            </p>
          ) : null}
          <p>
            {proposal.evidence.length} evidence bindings ·{" "}
            {proposal.omissions.length} omissions · {row.channel_ids.length}{" "}
            Channel admissions
          </p>
        </div>
        <div className={sx(styles.sourceRecords)}>
          <SourceRecord
            identity={shortDigest(observation.source.content_digest)}
            label="SOURCE"
            state={observation.source.locator}
          />
          <SourceRecord
            identity={proposal.completeness.toUpperCase()}
            label="COVERAGE"
            state={`${proposal.evidence.length}/${observation.limits.max_evidence_bindings} EVIDENCE BINDINGS`}
          />
          <SourceRecord
            identity="HARD BOUNDS"
            label="LIMITS"
            state={`${observation.limits.max_body_bytes} BODY BYTES · ${observation.limits.max_evidence_bindings} EVIDENCE · ${observation.limits.max_omissions} OMISSIONS · ${observation.limits.max_broadcast_targets} TARGETS`}
          />
        </div>
        {row.channel_ids.length > 0 ? (
          <p>
            <strong>Admitted Channels:</strong> {row.channel_ids.join(", ")}
          </p>
        ) : (
          <p>UNBROADCAST / LOCAL OBSERVATION ONLY</p>
        )}
        {proposal.evidence.map((evidence) => (
          <div
            className={sx(styles.diffPreview)}
            key={`${evidence.locator}:${evidence.source_revision}`}
          >
            <span className={sx(chrome.micro)}>BOUND EVIDENCE</span>
            <code>{evidence.locator}</code>
            <p>
              {evidence.source_revision} · {evidence.content_digest}
            </p>
          </div>
        ))}
        {proposal.omissions.length > 0 ? (
          <ul className={sx(styles.omissions)}>
            {proposal.omissions.map((omission) => (
              <li key={omission}>{omission}</li>
            ))}
          </ul>
        ) : null}
        <p className={sx(chrome.micro)}>
          AUTHOR SELF-ASSERTED · UNRESOLVED · NO ASSIGNMENT, ACTION, OR PROOF
          AUTHORITY
        </p>
      </div>
    </details>
  );
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
  const author =
    event.journalEntry?.author ??
    event.observation?.observation.proposal.author;
  if (author) return `${author.kind} / ${author.id}`.toUpperCase();
  return event.kind === "git" ? "GIT / SOURCE REPOSITORY" : "REY / ENVIRONMENT";
}

function postAction(event: FeedEvent): string {
  if (event.kind === "observation") return "UNRESOLVED";
  if (event.kind === "journal") return "OPEN ENTRY";
  if (event.kind === "git") return "INSPECT CADENCE";
  return "OPEN ENVIRONMENT";
}

function boundedOmissions(
  cadence: CadenceProjection,
  observations: ObservationFrontier,
  foldedEvents: number,
): string[] {
  return [
    ...new Set([
      ...cadence.omissions,
      ...(cadence.repository_state?.omissions ?? []),
      ...cadence.lanes.flatMap((lane) => lane.omissions),
      "workload test and run results have no retained Feed clock",
      "operator read and unread state is not retained",
      "observation, Journal, Git, and environment source clocks have no proven total ordering",
      ...(observations.complete
        ? []
        : [
            `${observations.omitted} unresolved observations omitted by the ${observations.limit}-record frontier`,
          ]),
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
  if (filter === "observation")
    return events.filter((event) => event.kind === "observation");
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
  if (kind === "signals")
    return "Open observations, Journal, Git, and environment records";
  if (kind === "admission") return "Work proposed from current evidence";
  return "Admitted workloads and retained results";
}

function filterLabel(filter: FeedStreamFilter): string {
  return filter.toUpperCase();
}
