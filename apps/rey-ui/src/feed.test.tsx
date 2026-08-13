import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { CadenceProjection } from "./cadence";
import type { ChannelGraphSnapshot, ChannelProjection } from "./channels";
import type { WorkloadList, WorkloadLog } from "./domain";
import {
  channelWorkingWriteForFeedLayout,
  DEFAULT_FEED_STREAMS,
  FEED_EVENT_LIMIT,
  FEED_STREAM_LIMIT,
  FeedPage,
  deriveFeedEvents,
  parseFeedStreams,
  persistFeedLayoutMovement,
  reorderFeedStreams,
  resolveFeedLayout,
  serializeFeedStreams,
} from "./feed";
import type { JournalProjection } from "./journal";
import type { ObservationFrontier } from "./observations";

describe("high-cadence operator feed", () => {
  it("does not turn mutable attention or repository posture into admission posts", () => {
    const portfolio = emptyPortfolio();
    portfolio.attention.rows.push({
      row_id: "attention:alpha",
      action: "refine",
      subject_kind: "workload",
      subject_id: "alpha",
      reason: "scenario delta remains unresolved",
      readiness: "ready",
      evidence_ids: ["evidence:delta"],
      dependency_ids: [],
      priority: 8,
      estimated_cost_units: 3,
    });
    const cadence = cadenceProjection();
    cadence.repository_state!.working_tree_state = "dirty";
    cadence.repository_state!.unstaged_entries = 2;
    cadence.repository_state!.push_state = "unpushed";
    cadence.repository_state!.ahead = 1;

    const markup = renderToStaticMarkup(
      createElement(FeedPage, {
        configuration: [{ kind: "admission", filter: "all" }],
        portfolio,
        sources: {
          cadence,
          journal: journalProjection(),
          observations: emptyObservationProjection(),
          workloadAdmissions: emptyWorkloadAdmissions(),
        },
      }),
    );

    expect(markup).toContain("NO WORKLOAD ADMISSIONS YET");
    expect(markup).not.toContain("REY / CURRENT PROJECTION");
    expect(markup).not.toContain("ADMISSION CONTROL");
    expect(markup).not.toContain("scenario delta remains unresolved");
  });

  it("uses wall time only as display order and retains order-only signals", () => {
    const events = deriveFeedEvents(
      cadenceProjection(),
      journalProjection(),
      observationProjection(),
    );

    expect(events.map((event) => [event.stream, event.position])).toEqual([
      ["JOURNAL", "J@1"],
      ["GIT", "HEAD~0"],
      ["OBSERVATION", "O@1"],
      ["REY ENV", "ENV@4"],
    ]);
    expect(events.at(-1)).toMatchObject({
      occurredAt: null,
      sortTime: null,
      stream: "REY ENV",
    });
  });

  it("bounds a high-cadence signal window to the newest retained records", () => {
    const journal = journalProjection();
    journal.log.entries = Array.from(
      { length: FEED_EVENT_LIMIT + 10 },
      (_, index) => ({
        ...journal.log.entries[0]!,
        entry_id: `blake3:entry-${index + 1}`,
        sequence: index + 1,
        admitted_at: new Date((index + 1_000) * 1_000).toISOString(),
        title: `Entry ${index + 1}`,
      }),
    );

    const events = deriveFeedEvents(
      cadenceProjection(),
      journal,
      emptyObservationProjection(),
    );

    expect(events).toHaveLength(FEED_EVENT_LIMIT);
    expect(events[0]).toMatchObject({ position: `J@${FEED_EVENT_LIMIT + 10}` });
    expect(events.some((event) => event.position === "J@1")).toBe(false);
  });

  it("round-trips bounded stream lenses for deep-linked Feed composition", () => {
    const streams = parseFeedStreams(
      "signals.journal~Review%2C%20now%20%7E%20%CE%94,signals.git,admission.all,flow.failing",
    );

    expect(streams).toEqual([
      {
        kind: "signals",
        filter: "journal",
        name: "Review, now ~ Δ",
      },
      { kind: "signals", filter: "git" },
      { kind: "admission", filter: "all" },
      { kind: "flow", filter: "failing" },
    ]);
    expect(serializeFeedStreams(streams)).toBe(
      "signals.journal~Review%2C%20now%20%7E%20%CE%94,signals.git,admission.all,flow.failing",
    );
    expect(
      parseFeedStreams(
        Array.from({ length: FEED_STREAM_LIMIT + 3 }, () => "signals.all").join(
          ",",
        ),
      ),
    ).toHaveLength(FEED_STREAM_LIMIT);
    expect(parseFeedStreams("unknown.all,flow.git")).toEqual(
      DEFAULT_FEED_STREAMS,
    );
  });

  it("resolves URL preview, WORKING, HEAD, and built-in layouts in order", () => {
    const builtIn = channelProjection();
    expect(resolveFeedLayout(null, builtIn)).toMatchObject({
      source: "built_in",
      detached: false,
      streams: [
        { id: "signals", kind: "signals", filter: "all" },
        { id: "admission", kind: "admission", filter: "all" },
        { id: "flow", kind: "flow", filter: "all" },
      ],
    });

    const admitted = channelProjection();
    admitted.status.head_commit = { sequence: 1, commit_id: "blake3:commit" };
    expect(resolveFeedLayout(null, admitted).source).toBe("channel_head");

    const working = channelProjection();
    working.status.working_present = true;
    working.status.working = channelSnapshot("working", [
      "flow",
      "signals",
      "admission",
    ]);
    expect(resolveFeedLayout(null, working)).toMatchObject({
      source: "channel_working",
      streams: [{ id: "flow" }, { id: "signals" }, { id: "admission" }],
    });

    expect(
      resolveFeedLayout(
        "review=signals.journal~Review,flow=flow.failing",
        working,
      ),
    ).toMatchObject({
      source: "url_preview",
      detached: true,
      streams: [
        { id: "review", kind: "signals", filter: "journal" },
        { id: "flow", kind: "flow", filter: "failing" },
      ],
    });
  });

  it("writes stable identity movement as one revisioned Channel semantic delta", () => {
    const projection = channelProjection();
    const resolved = resolveFeedLayout(null, projection);
    const moved = reorderFeedStreams(resolved.streams, "flow", "signals");
    const write = channelWorkingWriteForFeedLayout(projection, moved);

    expect(moved.map((stream) => stream.id)).toEqual([
      "flow",
      "signals",
      "admission",
    ]);
    expect(write.expected_head_snapshot_id).toBe("blake3:snapshot-built-in");
    expect(write.expected_working_snapshot_id).toBe("blake3:snapshot-built-in");
    expect(write.graph.layout).toEqual({
      id: "feed",
      revision: 2,
      stream_ids: ["flow", "signals", "admission"],
    });
    expect(
      write.graph.streams.map((stream) => [stream.id, stream.revision]),
    ).toEqual([
      ["admission", 1],
      ["flow", 1],
      ["signals", 1],
    ]);
  });

  it("rolls a rejected movement back to the last exact layout", async () => {
    const projection = channelProjection();
    const previous = resolveFeedLayout(null, projection).streams;
    const next = reorderFeedStreams(previous, "flow", "signals");
    const outcome = await persistFeedLayoutMovement(
      previous,
      next,
      async () => {
        throw new Error("WORKING snapshot changed");
      },
    );

    expect(outcome.streams.map((stream) => stream.id)).toEqual([
      "signals",
      "admission",
      "flow",
    ]);
    expect(outcome.result).toBeNull();
    expect(outcome.error?.message).toBe("WORKING snapshot changed");
  });

  it("renders three calm, tunable streams with exact evidence links", () => {
    const portfolio = emptyPortfolio();
    const ignoreRule = {
      kind: "workload",
      pattern: "context-anchor-survey",
      source_line: 2,
    };
    portfolio.revision = {
      schema: "rey.workload-revision-status.v1",
      state: "working",
      head: null,
      index: null,
      working: {
        schema: "rey.workload-admission-snapshot.v1",
        snapshot_revision: "blake3:working",
        packages: [],
        ignore: {
          schema: "rey.ignore.v1",
          source: ".reyignore",
          source_digest: "blake3:ignore",
          rules: [ignoreRule],
          omissions: [{ rule: ignoreRule, matched: 1 }],
          ignored: 1,
        },
      },
      staged: equalChangeSet("HEAD", "INDEX"),
      unstaged: {
        ...equalChangeSet("INDEX", "WORKING"),
        assessment: "different",
      },
      drafts: [],
      commit_ready: false,
      qualification_omissions: [],
      admission_boundary: "Only an exact qualified INDEX may advance HEAD.",
    };
    portfolio.attention.rows.push({
      row_id: "attention:alpha",
      action: "refine",
      subject_kind: "workload",
      subject_id: "alpha",
      reason: "scenario delta remains unresolved",
      readiness: "ready",
      evidence_ids: ["evidence:delta"],
      dependency_ids: [],
      priority: 8,
      estimated_cost_units: 3,
    });
    portfolio.workloads.push(workload());
    const markup = renderToStaticMarkup(
      createElement(FeedPage, {
        configuration: [
          { kind: "signals", filter: "all", name: "Source watch" },
          { kind: "admission", filter: "all" },
          { kind: "flow", filter: "all" },
        ],
        portfolio,
        sources: {
          cadence: cadenceProjection(),
          journal: journalProjection(),
          observations: observationProjection(),
          workloadAdmissions: workloadAdmissions(),
        },
      }),
    );

    expect(markup).toContain('data-feed-stream="signals"');
    expect(markup).toContain('data-feed-stream-id="signals"');
    expect(markup).not.toContain("FEED LAYOUT /");
    expect(markup).not.toContain("STABLE STREAM IDENTITIES");
    expect(markup).not.toContain("SNAPSHOT / DETACHED");
    expect(markup).not.toContain("MOVEMENT WRITES WORKING ONLY");
    expect(markup).toContain("PREVIEW ONLY · ADOPTION UNAVAILABLE");
    expect(markup).toContain("Alt+ArrowLeft Alt+ArrowRight");
    expect(markup).toContain('data-feed-stream="admission"');
    expect(markup).toContain('data-feed-stream="flow"');
    expect(markup.indexOf('data-feed-stream="signals"')).toBeLessThan(
      markup.indexOf('data-feed-stream="admission"'),
    );
    expect(markup.indexOf('data-feed-stream="admission"')).toBeLessThan(
      markup.indexOf('data-feed-stream="flow"'),
    );
    expect(markup.match(/role="feed"/g)).toHaveLength(3);
    expect(markup).toContain("Share an observation…");
    expect(markup).toContain("JOURNAL / RICH DOCUMENT");
    expect(markup).toContain("OBSERVATION / O@1 / ORDER ONLY");
    expect(markup).toContain("AUTHOR SELF-ASSERTED");
    expect(markup).toContain("NO ASSIGNMENT, ACTION, OR PROOF");
    expect(markup).toContain("DIRECTED DIFF / different");
    expect(markup).toContain("REY / WORKLOAD COMMIT");
    expect(markup).toContain("WORKLOAD@1");
    expect(markup).toContain("Approve exact workload snapshot");
    expect(markup).toContain("COMMITTED / RETAINED");
    expect(markup).not.toContain("ADMISSION CONTROL");
    expect(markup).not.toContain("REY / CURRENT PROJECTION");
    expect(markup).not.toContain("WORKING FILES");
    expect(markup).toContain("LOCAL CONFORMANCE");
    expect(markup).toContain("Display order is not causal order");
    expect(markup).toContain("ORDER ONLY");
    expect(markup.match(/>TUNE</g)).toHaveLength(3);
    expect(markup).toContain("FIREHOSE");
    expect(markup).toContain('data-feed-streams="3"');
    expect(markup).toContain('aria-label="Rename Source watch"');
    expect(markup).toContain(">Source watch</button>");
    expect(markup).toContain('aria-label="Rename Admission"');
    expect(markup).not.toContain("ALL LENS");
    expect(markup).not.toContain("3/3 POSTS");
    expect(markup).not.toContain("1 PROPOSALS · 1 READY");
    expect(markup).not.toContain("1 WORKLOADS");
    expect(markup).toContain(
      `href="https://github.com/example/rey/commit/${"a".repeat(40)}"`,
    );
  });
});

function equalChangeSet(
  source: string,
  target: string,
): import("./domain").WorkloadChangeSet {
  return {
    schema: "rey.workload-change-set.v1",
    source_label: source,
    target_label: target,
    source_revision: null,
    target_revision: null,
    assessment: "equal",
    inserted: 0,
    deleted: 0,
    modified: 0,
    changes: [],
  };
}

function channelSnapshot(
  label: string,
  streamIds: string[] = ["signals", "admission", "flow"],
): ChannelGraphSnapshot {
  return {
    schema: "rey.channel-graph-snapshot.v1",
    snapshot_id: `blake3:snapshot-${label}`,
    graph_id: `blake3:graph-${label}`,
    source: {
      kind: label === "built-in" ? "built_in" : "worktree",
      locator:
        label === "built-in"
          ? "builtin://rey/channel-graph/default"
          : "ui:///channels/working",
      content_digest: `blake3:content-${label}`,
    },
    limits: {
      max_channels: 32,
      max_subscriptions: 32,
      max_streams: 8,
      max_relays: 32,
      max_applications: 16,
      max_polling_beacons: 16,
      max_subscription_records: 256,
      max_relay_hops: 16,
    },
    graph: {
      schema: "rey.channel-graph.v1",
      channels: [
        {
          id: "workspace",
          revision: 1,
          name: "Workspace",
          scope: "workspace_local",
          accepted_observation_kinds: ["finding", "question"],
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
        {
          id: "signals",
          revision: 1,
          name: "Signals",
          subscription_id: "workspace",
          lens: "signals",
        },
      ],
      layout: { id: "feed", revision: 1, stream_ids: streamIds },
      applications: [],
      relays: [],
      beacons: [],
    },
  };
}

function channelProjection(): ChannelProjection {
  const snapshot = channelSnapshot("built-in");
  const delta = (source: string, target: string) => ({
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
  return {
    schema: "rey.ui-channels.v1",
    write_enabled: true,
    authority: "unauthenticated_channel_working_write",
    listener: {
      address: "127.0.0.1:5714",
      loopback_only: true,
      authentication: "none",
      warning: "local clients may replace WORKING",
    },
    status: {
      schema: "rey.channel-status.v1",
      state: "clean",
      working_present: false,
      head_commit: null,
      head: snapshot,
      index: null,
      working: snapshot,
      staged: delta("BUILT-IN", "INDEX"),
      unstaged: delta("INDEX", "WORKING"),
    },
  };
}

function workloadAdmissions(): WorkloadLog {
  return {
    schema: "rey.workload-log.v1",
    head_commit_id: "blake3:workload-commit",
    total_commits: 1,
    selected_commits: 1,
    patch: false,
    commits: [
      {
        schema: "rey.workload-commit.v1",
        commit_id: "blake3:workload-commit",
        sequence: 1,
        parent_commit_id: null,
        committed_at_unix: 400,
        message: "Approve exact workload snapshot",
        snapshot: {
          schema: "rey.workload-admission-snapshot.v1",
          snapshot_revision: "blake3:admitted-snapshot",
          packages: [],
          ignore: null,
        },
        qualification_ids: [],
      },
    ],
  };
}

function emptyWorkloadAdmissions(): WorkloadLog {
  return {
    schema: "rey.workload-log.v1",
    head_commit_id: null,
    total_commits: 0,
    selected_commits: 0,
    patch: false,
    commits: [],
  };
}

function emptyPortfolio(): WorkloadList {
  return {
    schema: "rey.workload-list.v1",
    semantic_atlas: null,
    semantic_atlas_history: [],
    semantic_atlas_deltas: [],
    catalog: {
      schema: "rey.workload-catalog.v1",
      kind: "workspace_packages",
      root: "sys",
      workload_count: 0,
      admitted_count: 0,
      draft_count: 0,
    },
    workloads: [],
    drafts: [],
    attention: {
      schema: "rey.workload-attention.v1",
      attention_id: "blake3:attention",
      source_snapshot_id: "blake3:source",
      rows: [],
      summary: {
        refine: 0,
        retest: 0,
        create: 0,
        blocked: 0,
        policy_excluded: 0,
        workloads: 0,
        surfaces: 0,
        owned_surfaces: 0,
        unowned_surfaces: 0,
      },
    },
  };
}

function workload(): WorkloadList["workloads"][number] {
  return {
    provenance: null,
    workload: { id: "alpha", revision: 2, semantic_digest: "blake3:workload" },
    title: "Alpha workload",
    candidate_graph: {
      id: "alpha.graph",
      revision: 3,
      semantic_digest: "blake3:graph",
    },
    freshness: "fresh",
    qualification: "failing",
    required: 4,
    passed: 3,
    failed: 1,
    inconclusive: 0,
    evaluated: 4,
    stale: 0,
    optional: 0,
    mining_operations: 2,
    mining_results: 2,
    incomplete_mining_results: 0,
    relation_deltas: 1,
    reasoning_surfaces: 1,
    attention_rows: 1,
    topography_results: 0,
    topography_revision: null,
    topography_coverage: null,
    topography_frontier_rows: 0,
    topography_patch: null,
    topography_projection: null,
    scene_admission_results: 0,
    latest_scene_admission: null,
    last_run_status: null,
    last_test_result_id: "blake3:test-alpha",
  };
}

function cadenceProjection(): CadenceProjection {
  return {
    schema: "rey.ui-cadence.v1",
    ordering: "partial",
    source_repository: "https://github.com/example/rey",
    repository_state: {
      id: "repository:rey",
      working_tree_state: "clean",
      staged_entries: 0,
      unstaged_entries: 0,
      untracked_entries: 0,
      conflicted_entries: 0,
      push_state: "pushed",
      branch: "main",
      head_revision: "a".repeat(40),
      upstream: "origin/main",
      upstream_revision: "a".repeat(40),
      ahead: 0,
      behind: 0,
      comparison_basis: "local_tracking_ref",
      complete: true,
      scope: "tracked_changes_and_untracked_files",
      omissions: [],
    },
    lanes: [
      {
        id: "git",
        label: "Git reachable HEAD",
        clock: "git_reachability",
        ordering: "newest_first",
        complete: true,
        ticks: [
          {
            id: "git:head",
            kind: "git_commit",
            state: "committed",
            ordinal: "HEAD~0",
            title: "Raise the operator plane",
            detail: "one exact source revision",
            revision: "a".repeat(40),
            parent_revisions: [],
            occurred_at_unix: 100,
            publication: "pushed",
          },
        ],
        omissions: [],
      },
      {
        id: "environment",
        label: "Environment admissions",
        clock: "rey_environment_sequence",
        ordering: "newest_first",
        complete: true,
        ticks: [
          {
            id: "environment:4",
            kind: "rey_admission",
            state: "committed",
            ordinal: "ENV@4",
            title: "Accept discovered tools",
            detail: "bounded capability transition",
            revision: "blake3:environment",
            parent_revisions: [],
            occurred_at_unix: null,
            publication: null,
          },
        ],
        omissions: ["environment commit has no retained wall time"],
      },
    ],
    schedules: [],
    omissions: ["cross-clock causality is not retained"],
  };
}

function journalProjection(): JournalProjection {
  return {
    schema: "rey.ui-journal.v2",
    write_enabled: true,
    authority: "unauthenticated_journal_admission",
    log: {
      schema: "rey.journal-log.v2",
      log_id: "blake3:journal",
      entries: [
        {
          schema: "rey.journal-entry.v2",
          entry_id: "blake3:entry",
          sequence: 1,
          admitted_at: "1970-01-01T00:03:20.000Z",
          title: "Inspect the next bearing",
          author: { kind: "human", id: "operator" },
          binding: {
            coordinate:
              "rey+local://portfolio/current?revision=blake3%3Asource",
            scale: 0.68,
            source_revision: "blake3:source",
          },
          supersedes: null,
          layout: {
            kind: "broadsheet",
            columns: 12,
            bands: [
              {
                id: "lead",
                cells: [
                  { block_id: "brief", span: 8 },
                  { block_id: "delta", span: 4 },
                ],
              },
            ],
          },
          blocks: [
            {
              kind: "prose",
              id: "brief",
              document: [
                { kind: "heading", text: "A richer collaboration post" },
                {
                  kind: "paragraph",
                  text: "The feed carries notebook structure into the stream.",
                },
              ],
            },
            {
              kind: "diff",
              id: "delta",
              source: "CURRENT",
              target: "EXPECTED",
              direction: "CURRENT_TO_EXPECTED",
              assessment: "different",
              summary: "One bounded delta remains.",
            },
          ],
        },
      ],
    },
  };
}

function observationProjection(): ObservationFrontier {
  const projection = emptyObservationProjection();
  projection.frontier_id = "blake3:frontier-one";
  projection.source_log_id = "blake3:observation-log-one";
  projection.summary = {
    ...projection.summary,
    observations: 1,
    unresolved: 1,
  };
  projection.rows = [
    {
      observation: {
        schema: "rey.observation-admission.v1",
        observation_id: "blake3:observation-one",
        sequence: 1,
        admitted_at_unix: 300,
        source: {
          locator: "workspace://notes/bearing.json",
          content_digest: "blake3:source-one",
        },
        limits: {
          max_body_bytes: 16_384,
          max_evidence_bindings: 32,
          max_omissions: 32,
          max_broadcast_targets: 32,
        },
        proposal: {
          schema: "rey.observation.v1",
          kind: "finding",
          author: { kind: "agent", id: "codex" },
          subject_locator: "rey+local://workload/alpha?revision=2",
          body: "A directed scenario delta remains unresolved.",
          desired_delta: "Make the expected and observed frames equal.",
          completeness: "partial",
          omissions: ["remote provider evidence was not requested"],
          evidence: [
            {
              locator: "rey+local://delta/alpha",
              source_revision: "blake3:test-alpha",
              content_digest: "blake3:delta-alpha",
            },
          ],
          supersedes: null,
        },
      },
      channel_ids: ["workspace"],
    },
  ];
  return projection;
}

function emptyObservationProjection(): ObservationFrontier {
  return {
    schema: "rey.observation-frontier.v1",
    frontier_id: "blake3:frontier-empty",
    source_log_id: "blake3:observation-log-empty",
    ordering: "observation_sequence_ascending",
    limit: 64,
    complete: true,
    omitted: 0,
    summary: {
      observations: 0,
      unresolved: 0,
      superseded: 0,
      resolved: 0,
      withdrawn: 0,
      unbroadcast: 0,
    },
    rows: [],
  };
}
