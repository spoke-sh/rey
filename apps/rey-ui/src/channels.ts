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
  github_inbox: {
    channel_id: string;
    hostname: string;
    poll_interval_seconds: number;
    notification_limit: number;
    pull_request_limit: number;
    comment_limit: number;
    credential_environment: string[];
  } | null;
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
  mailbox: ChannelMailboxProjection;
}

export type ChannelMessageSource =
  | { kind: "local_admission" }
  | {
      kind: "git_hub_notification";
      application_id: string;
      application_revision: number;
      external_id: string;
      source_revision: string;
      repository: string;
      subject_type: string;
      subject_title: string;
      reason: string;
      provider_unread: boolean;
      occurred_at_unix: number;
      html_url: string;
    }
  | {
      kind: "git_hub_issue_comment";
      application_id: string;
      application_revision: number;
      external_id: string;
      source_revision: string;
      repository: string;
      pull_number: number;
      author: string;
      occurred_at_unix: number;
      html_url: string;
    }
  | {
      kind: "git_hub_review_comment";
      application_id: string;
      application_revision: number;
      external_id: string;
      source_revision: string;
      repository: string;
      pull_number: number;
      author: string;
      occurred_at_unix: number;
      html_url: string;
      path: string;
    };

export interface ChannelMessage {
  schema: "rey.channel-message-admission.v1";
  message_id: string;
  sequence: number;
  admitted_at_unix: number;
  channel_head_commit_id: string;
  channel_graph_id: string;
  proposal: {
    schema: "rey.channel-message.v1";
    channel_id: string;
    kind: string;
    body: string;
    evidence_locators: string[];
  };
  source: ChannelMessageSource;
}

export interface GitHubPollReceipt {
  schema: "rey.github-channel-poll-receipt.v1";
  poll_id: string;
  sequence: number;
  polled_at_unix: number;
  channel_head_commit_id: string;
  channel_graph_id: string;
  application_id: string;
  application_revision: number;
  environment_commit_id: string;
  environment_capability_id: string;
  hostname: string;
  api_version: string;
  request_count: number;
  notification_count: number;
  pull_request_count: number;
  issue_comment_count: number;
  review_comment_count: number;
  admitted_message_count: number;
  reused_message_count: number;
  current_message_ids: string[];
  complete: boolean;
  omissions: string[];
}

export interface ChannelMailboxProjection {
  schema: "rey.channel-mailbox.v1";
  ordering: "provider_updated_desc";
  messages: ChannelMessage[];
  polls: GitHubPollReceipt[];
  complete: boolean;
  omissions: string[];
  max_messages: number;
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
