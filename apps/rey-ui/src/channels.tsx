import { KineticDenseTable, type KineticDenseTableColumn } from "@hifi/kinetic";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { channelsStyles as styles } from "./stylex/channels.stylex";
import { className as sx } from "./stylex/shared.stylex";

export interface ChannelDefinition {
  id: string;
  revision: number;
  name: string;
  scope: "workspace_local";
  accepted_observation_kinds: string[];
  broadcast_default: boolean;
}

export interface ChannelSubscription {
  id: string;
  revision: number;
  channel_ids: string[];
  observation_kinds: string[];
  filters: Record<string, string>;
  limit: number;
}

export interface FeedStreamDefinition {
  id: string;
  revision: number;
  name: string;
  subscription_id: string;
  lens: string;
}

export interface FeedLayout {
  id: string;
  revision: number;
  stream_ids: string[];
}

export interface ChannelApplicationDeclaration {
  id: string;
  revision: number;
  environment_capability_id: string;
  executable_path: string;
  executable_version: string | null;
  executable_digest: string;
  relay_argv: string[];
  timeout_ms: number;
  max_output_bytes: number;
}

export interface ChannelRelayDeclaration {
  id: string;
  revision: number;
  source_channel_id: string;
  target_channel_locator: string;
  provider_id: string;
  hop_limit: number;
}

export interface PollingBeaconDefinition {
  id: string;
  revision: number;
  application_id: string;
  relay_ids: string[];
  interval_seconds: number;
  batch_limit: number;
}

export interface ChannelGraph {
  schema: "rey.channel-graph.v1";
  channels: ChannelDefinition[];
  subscriptions: ChannelSubscription[];
  streams: FeedStreamDefinition[];
  layout: FeedLayout;
  applications: ChannelApplicationDeclaration[];
  relays: ChannelRelayDeclaration[];
  beacons: PollingBeaconDefinition[];
}

export interface ChannelGraphSnapshot {
  schema: "rey.channel-graph-snapshot.v1";
  snapshot_id: string;
  graph_id: string;
  source: {
    kind: "built_in" | "worktree";
    locator: string;
    content_digest: string;
  };
  limits: {
    max_channels: number;
    max_subscriptions: number;
    max_streams: number;
    max_relays: number;
    max_applications: number;
    max_polling_beacons: number;
    max_subscription_records: number;
    max_relay_hops: number;
  };
  graph: ChannelGraph;
}

export interface ChannelGraphChange {
  kind: "added" | "removed" | "modified" | "renamed" | "retargeted" | "moved";
  object_kind:
    | "channel"
    | "subscription"
    | "stream"
    | "layout"
    | "relay"
    | "application"
    | "beacon";
  object_id: string;
  before: string | null;
  after: string | null;
  detail: string;
}

export interface ChannelGraphDelta {
  schema: "rey.channel-graph-delta.v1";
  delta_id: string;
  source_label: string;
  target_label: string;
  source_graph_id: string;
  target_graph_id: string;
  assessment: "equal" | "different" | "incompatible" | "inconclusive";
  summary: {
    added: number;
    removed: number;
    modified: number;
    renamed: number;
    retargeted: number;
    moved: number;
    total: number;
  };
  changes: ChannelGraphChange[];
}

export interface ChannelStatus {
  schema: "rey.channel-status.v1";
  state: "clean" | "working" | "staged" | "mixed";
  working_present: boolean;
  head_commit: { sequence: number; commit_id: string } | null;
  head: ChannelGraphSnapshot;
  index: ChannelGraphSnapshot | null;
  working: ChannelGraphSnapshot;
  staged: ChannelGraphDelta;
  unstaged: ChannelGraphDelta;
}

export interface ChannelProjection {
  schema: "rey.ui-channels.v1";
  write_enabled: boolean;
  authority: string;
  listener: {
    address: string;
    loopback_only: boolean;
    authentication: "none";
    warning: string;
  };
  status: ChannelStatus;
}

export interface ChannelWorkingWriteRequest {
  schema: "rey.ui-channel-working-write.v1";
  expected_head_snapshot_id: string;
  expected_working_snapshot_id: string;
  graph: ChannelGraph;
}

export interface ChannelApplyResult {
  schema: "rey.channel-apply-result.v1";
  applied: boolean;
  snapshot: ChannelGraphSnapshot;
  delta: ChannelGraphDelta;
}

interface StreamRow extends FeedStreamDefinition {
  position: number;
}

const streamColumns: readonly KineticDenseTableColumn<StreamRow>[] = [
  {
    id: "stream",
    header: "STREAM / REVISION",
    rowHeader: true,
    width: "27%",
    render: (stream) => (
      <div className={sx(styles.stack)}>
        <strong>{stream.name}</strong>
        <code>{stream.id}</code>
        <span className={sx(chrome.micro)}>R{stream.revision}</span>
      </div>
    ),
  },
  {
    id: "subscription",
    header: "SUBSCRIPTION",
    width: "27%",
    render: (stream) => <code>{stream.subscription_id}</code>,
  },
  {
    id: "lens",
    header: "LENS",
    width: "26%",
    render: (stream) => <code>{stream.lens}</code>,
  },
  {
    align: "right",
    id: "position",
    header: "LAYOUT POSITION",
    width: "20%",
    render: (stream) => (
      <strong>{String(stream.position + 1).padStart(2, "0")}</strong>
    ),
  },
];

export function ChannelsPage({
  onWrite,
  projection,
  refreshError,
}: {
  onWrite: (request: ChannelWorkingWriteRequest) => Promise<ChannelProjection>;
  projection: ChannelProjection;
  refreshError: Error | null;
}) {
  const [draft, setDraft] = useState(() =>
    JSON.stringify(projection.status.working.graph, null, 2),
  );
  const [dirty, setDirty] = useState(false);
  const [writeError, setWriteError] = useState<Error | null>(null);
  const [writing, setWriting] = useState(false);
  useEffect(() => {
    if (!dirty) {
      setDraft(JSON.stringify(projection.status.working.graph, null, 2));
    }
  }, [dirty, projection.status.working]);
  const streams = useMemo(
    () =>
      projection.status.working.graph.layout.stream_ids.flatMap(
        (streamId, position) => {
          const stream = projection.status.working.graph.streams.find(
            (candidate) => candidate.id === streamId,
          );
          return stream ? [{ ...stream, position }] : [];
        },
      ),
    [projection.status.working.graph],
  );
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setWriteError(null);
    let graph: ChannelGraph;
    try {
      graph = JSON.parse(draft) as ChannelGraph;
    } catch (error) {
      setWriteError(
        error instanceof Error ? error : new Error("Channel graph is not JSON"),
      );
      return;
    }
    setWriting(true);
    try {
      const current = await onWrite({
        schema: "rey.ui-channel-working-write.v1",
        expected_head_snapshot_id: projection.status.head.snapshot_id,
        expected_working_snapshot_id: projection.status.working.snapshot_id,
        graph,
      });
      setDraft(JSON.stringify(current.status.working.graph, null, 2));
      setDirty(false);
    } catch (error) {
      setWriteError(error instanceof Error ? error : new Error(String(error)));
    } finally {
      setWriting(false);
    }
  };

  return (
    <main className={sx(chrome.page, styles.page)}>
      <section className={sx(styles.section)} data-rey-section="01 / REVISION">
        <ChannelHeading
          detail={`${projection.status.state.toUpperCase()} · ${refreshError ? "REVALIDATION DELAYED" : "LIVE"}`}
          index="01"
          kicker="REVISION"
          title="Channel operator index"
        />
        <div className={sx(styles.planeGrid)}>
          <RevisionPlane label="HEAD" snapshot={projection.status.head} />
          <RevisionPlane
            label="INDEX"
            snapshot={projection.status.index ?? projection.status.head}
          />
          <RevisionPlane label="WORKING" snapshot={projection.status.working} />
        </div>
        {refreshError ? (
          <p className={sx(styles.error)} role="status">
            LAST GOOD CHANNEL DOCUMENT RETAINED · {refreshError.message}
          </p>
        ) : null}
      </section>

      <section className={sx(styles.section)} data-rey-section="02 / TOPOLOGY">
        <ChannelHeading
          detail={`${projection.status.working.graph.channels.length} channels · ${projection.status.working.graph.subscriptions.length} subscriptions · ${streams.length}/${projection.status.working.limits.max_streams} streams`}
          index="02"
          kicker="TOPOLOGY"
          title="Feed streams and layout"
        />
        <KineticDenseTable
          ariaLabel="Channel Feed streams"
          className={sx(styles.table)}
          columns={streamColumns}
          emptyState="NO STREAMS ARE DECLARED"
          getRowClassName={() => sx(styles.row)}
          getRowKey={(stream) => stream.id}
          minWidth={820}
          rows={streams}
          theme="precision"
        />
        <div className={sx(styles.topologyFacts)}>
          {projection.status.working.graph.channels.map((channel) => (
            <article className={sx(styles.fact)} key={channel.id}>
              <span className={sx(chrome.micro)}>CHANNEL / {channel.id}</span>
              <strong>{channel.name}</strong>
              <span>{channel.scope.replaceAll("_", " ")}</span>
              <span>{channel.accepted_observation_kinds.join(" · ")}</span>
            </article>
          ))}
          {projection.status.working.graph.subscriptions.map((subscription) => (
            <article className={sx(styles.fact)} key={subscription.id}>
              <span className={sx(chrome.micro)}>
                SUBSCRIPTION / {subscription.id}
              </span>
              <strong>{subscription.channel_ids.join(" + ")}</strong>
              <span>{subscription.observation_kinds.join(" · ")}</span>
              <span>BOUND / {subscription.limit} RECORDS</span>
            </article>
          ))}
        </div>
      </section>

      <section className={sx(styles.section)} data-rey-section="03 / DELTAS">
        <ChannelHeading
          detail={`${projection.status.staged.summary.total} staged · ${projection.status.unstaged.summary.total} unstaged`}
          index="03"
          kicker="DIRECTED DELTAS"
          title="HEAD → INDEX → WORKING"
        />
        <div className={sx(styles.deltaGrid)}>
          <DeltaPanel delta={projection.status.staged} />
          <DeltaPanel delta={projection.status.unstaged} />
        </div>
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="04 / WORKING WRITE"
      >
        <ChannelHeading
          detail="VALIDATED GRAPH · EXPECTED SNAPSHOTS · WORKING ONLY"
          index="04"
          kicker="EXPLICIT MUTATION"
          title="Replace Channel WORKING"
        />
        <div
          className={sx(
            styles.warning,
            !projection.listener.loopback_only && styles.warningNetwork,
          )}
          role="note"
        >
          <strong>
            {projection.listener.loopback_only
              ? "LOOPBACK LISTENER"
              : "NETWORK-EXPOSED LISTENER"}
            {" · "}NO AUTHENTICATION
          </strong>
          <span>{projection.listener.warning}</span>
          <code>{projection.listener.address}</code>
          <span>{projection.authority}</span>
        </div>
        <form className={sx(styles.editor)} onSubmit={submit}>
          <label
            className={sx(styles.editorLabel)}
            htmlFor="channel-working-graph"
          >
            REY.CHANNEL-GRAPH.V1 / JSON
          </label>
          <textarea
            aria-describedby="channel-working-boundary"
            className={sx(styles.textarea)}
            id="channel-working-graph"
            onChange={(event) => {
              setDraft(event.target.value);
              setDirty(true);
            }}
            rows={24}
            spellCheck={false}
            value={draft}
          />
          <div className={sx(styles.editorFooter)}>
            <span className={sx(chrome.micro)} id="channel-working-boundary">
              NO INDEX · NO HEAD · NO RELAY · NO EXECUTION
            </span>
            <button
              className={sx(chrome.focusable, styles.submit)}
              disabled={!projection.write_enabled || !dirty || writing}
              type="submit"
            >
              {writing ? "VALIDATING…" : "WRITE CHANNEL WORKING →"}
            </button>
          </div>
          {writeError ? (
            <p className={sx(styles.error)} role="alert">
              WRITE REJECTED · {writeError.message}
            </p>
          ) : null}
        </form>
      </section>
    </main>
  );
}

function RevisionPlane({
  label,
  snapshot,
}: {
  label: string;
  snapshot: ChannelGraphSnapshot;
}) {
  return (
    <article className={sx(styles.plane)}>
      <span className={sx(chrome.micro)}>{label}</span>
      <strong>{snapshot.source.kind.replaceAll("_", " ")}</strong>
      <code title={snapshot.snapshot_id}>{snapshot.snapshot_id}</code>
      <span className={sx(chrome.micro)}>GRAPH</span>
      <code title={snapshot.graph_id}>{snapshot.graph_id}</code>
      <span>{snapshot.source.locator}</span>
    </article>
  );
}

function DeltaPanel({ delta }: { delta: ChannelGraphDelta }) {
  return (
    <article className={sx(styles.deltaPanel)}>
      <header>
        <strong>
          {delta.source_label} → {delta.target_label}
        </strong>
        <span className={sx(chrome.micro)}>{delta.assessment}</span>
      </header>
      <code title={delta.delta_id}>{delta.delta_id}</code>
      {delta.changes.length === 0 ? (
        <p>NO SEMANTIC CHANGES</p>
      ) : (
        <ol className={sx(styles.changeList)}>
          {delta.changes.map((change, index) => (
            <li
              key={`${change.object_kind}:${change.object_id}:${change.kind}:${index}`}
            >
              <strong>{change.kind.toUpperCase()}</strong>
              <span>
                {change.object_kind} / {change.object_id}
              </span>
              <small>{change.detail}</small>
            </li>
          ))}
        </ol>
      )}
    </article>
  );
}

function ChannelHeading({
  detail,
  index,
  kicker,
  title,
}: {
  detail: string;
  index: string;
  kicker: string;
  title: string;
}) {
  return (
    <header className={sx(styles.heading)}>
      <span className={sx(styles.index)}>{index}</span>
      <div>
        <p className={sx(chrome.micro, styles.kicker)}>{kicker}</p>
        <h1>{title}</h1>
      </div>
      <span className={sx(chrome.micro, styles.detail)}>{detail}</span>
    </header>
  );
}
