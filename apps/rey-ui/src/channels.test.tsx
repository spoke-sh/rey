import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { ChannelsPage, type ChannelProjection } from "./channels";

const graph = {
  schema: "rey.channel-graph.v1" as const,
  channels: [
    {
      id: "workspace",
      revision: 1,
      name: "Workspace",
      scope: "workspace_local" as const,
      accepted_observation_kinds: [
        "finding",
        "question",
        "progress",
        "blocker",
        "handoff",
      ],
      broadcast_default: true,
    },
  ],
  subscriptions: [
    {
      id: "workspace",
      revision: 1,
      channel_ids: ["workspace"],
      observation_kinds: ["finding", "question"],
      filters: {},
      limit: 64,
    },
  ],
  streams: [
    {
      id: "signals",
      revision: 1,
      name: "Signals",
      subscription_id: "workspace",
      lens: "signals",
    },
    {
      id: "admission",
      revision: 1,
      name: "Admission",
      subscription_id: "workspace",
      lens: "admission",
    },
    {
      id: "flow",
      revision: 1,
      name: "Flow",
      subscription_id: "workspace",
      lens: "flow",
    },
  ],
  layout: {
    id: "feed",
    revision: 1,
    stream_ids: ["signals", "admission", "flow"],
  },
  applications: [],
  relays: [],
  beacons: [],
};

const limits = {
  max_channels: 32,
  max_subscriptions: 32,
  max_streams: 8,
  max_relays: 32,
  max_applications: 16,
  max_polling_beacons: 16,
  max_subscription_records: 256,
  max_relay_hops: 16,
};

const snapshot = {
  schema: "rey.channel-graph-snapshot.v1" as const,
  snapshot_id: "blake3:channel-snapshot",
  graph_id: "blake3:channel-graph",
  source: {
    kind: "built_in" as const,
    locator: "builtin://rey/channel-graph/default",
    content_digest: "blake3:channel-graph",
  },
  limits,
  graph,
};

const emptyDelta = (source: string, target: string) => ({
  schema: "rey.channel-graph-delta.v1" as const,
  delta_id: `blake3:${source}-${target}`,
  source_label: source,
  target_label: target,
  source_graph_id: snapshot.graph_id,
  target_graph_id: snapshot.graph_id,
  assessment: "equal" as const,
  summary: {
    added: 0,
    removed: 0,
    modified: 0,
    renamed: 0,
    retargeted: 0,
    moved: 0,
    total: 0,
  },
  changes: [],
});

const projection: ChannelProjection = {
  schema: "rey.ui-channels.v1",
  write_enabled: true,
  authority:
    "unauthenticated_channel_working_write; no INDEX, HEAD, relay, or execution authority",
  listener: {
    address: "127.0.0.1:5714",
    loopback_only: true,
    authentication: "none",
    warning:
      "any local client that can reach this listener may replace Channel WORKING",
  },
  status: {
    schema: "rey.channel-status.v1",
    state: "clean",
    working_present: false,
    head_commit: null,
    head: snapshot,
    index: null,
    working: snapshot,
    staged: emptyDelta("BUILT-IN", "INDEX"),
    unstaged: emptyDelta("INDEX", "WORKING"),
  },
};

describe("Channel operator projection", () => {
  it("keeps revision identities, stream order, bounds, and write authority visible", () => {
    const markup = renderToStaticMarkup(
      createElement(ChannelsPage, {
        onWrite: vi.fn(async () => projection),
        projection,
        refreshError: null,
      }),
    );

    expect(markup).toContain("Channel operator index");
    expect(markup).toContain("blake3:channel-snapshot");
    expect(markup).toContain("Feed streams and layout");
    expect(markup.indexOf("Signals")).toBeLessThan(markup.indexOf("Admission"));
    expect(markup.indexOf("Admission")).toBeLessThan(markup.indexOf("Flow"));
    expect(markup).toContain("3/8 streams");
    expect(markup).toContain("LOOPBACK LISTENER");
    expect(markup).toContain("NO AUTHENTICATION");
    expect(markup).toContain("NO INDEX · NO HEAD · NO RELAY · NO EXECUTION");
    expect(markup).toContain("WRITE CHANNEL WORKING");
    expect(markup).toMatch(/<button[^>]*disabled=""/);
  });

  it("raises the stronger unauthenticated warning for a network listener", () => {
    const networkProjection: ChannelProjection = {
      ...projection,
      listener: {
        ...projection.listener,
        address: "0.0.0.0:5714",
        loopback_only: false,
        warning:
          "any network client that can reach this listener may replace Channel WORKING without authentication",
      },
    };
    const markup = renderToStaticMarkup(
      createElement(ChannelsPage, {
        onWrite: vi.fn(async () => networkProjection),
        projection: networkProjection,
        refreshError: null,
      }),
    );

    expect(markup).toContain("NETWORK-EXPOSED LISTENER");
    expect(markup).toContain("0.0.0.0:5714");
    expect(markup).toContain("without authentication");
  });
});
