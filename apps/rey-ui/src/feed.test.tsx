import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { CadenceProjection } from "./cadence";
import type { WorkloadList } from "./domain";
import {
  DEFAULT_FEED_STREAMS,
  FEED_EVENT_LIMIT,
  FEED_STREAM_LIMIT,
  FeedPage,
  deriveFeedEvents,
  deriveInspectionQueue,
  parseFeedStreams,
  serializeFeedStreams,
} from "./feed";
import type { JournalProjection } from "./journal";

describe("high-cadence operator feed", () => {
  it("ranks current attention ahead of bounded repository state", () => {
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

    expect(deriveInspectionQueue(portfolio, cadence)).toMatchObject([
      {
        source: "ATTENTION",
        subject: "alpha",
        signal: "REFINE",
        urgency: "NOW",
        href: "/workloads/alpha",
      },
      {
        source: "REPOSITORY",
        signal: "REVIEW",
        urgency: "WATCH",
      },
      {
        source: "REPOSITORY",
        signal: "PUBLISH",
        urgency: "WATCH",
      },
    ]);
  });

  it("uses wall time only as display order and retains order-only signals", () => {
    const events = deriveFeedEvents(cadenceProjection(), journalProjection());

    expect(events.map((event) => [event.stream, event.position])).toEqual([
      ["JOURNAL", "J@1"],
      ["GIT", "HEAD~0"],
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

    const events = deriveFeedEvents(cadenceProjection(), journal);

    expect(events).toHaveLength(FEED_EVENT_LIMIT);
    expect(events[0]).toMatchObject({ position: `J@${FEED_EVENT_LIMIT + 10}` });
    expect(events.some((event) => event.position === "J@1")).toBe(false);
  });

  it("round-trips bounded stream lenses for deep-linked Feed composition", () => {
    const streams = parseFeedStreams(
      "signals.journal~Review%2C%20now%20%7E%20%CE%94,signals.git,admission.now,flow.failing",
    );

    expect(streams).toEqual([
      {
        kind: "signals",
        filter: "journal",
        name: "Review, now ~ Δ",
      },
      { kind: "signals", filter: "git" },
      { kind: "admission", filter: "now" },
      { kind: "flow", filter: "failing" },
    ]);
    expect(serializeFeedStreams(streams)).toBe(
      "signals.journal~Review%2C%20now%20%7E%20%CE%94,signals.git,admission.now,flow.failing",
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

  it("renders three calm, tunable streams with exact evidence links", () => {
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
        },
      }),
    );

    expect(markup).toContain('data-feed-stream="signals"');
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
    expect(markup).toContain("DIRECTED DIFF / different");
    expect(markup).toContain("INSPECT-ONLY");
    expect(markup).toContain("REY / ATTENTION");
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
    expect(markup).toContain(
      `href="https://github.com/spoke-sh/rey/commit/${"a".repeat(40)}"`,
    );
  });
});

function emptyPortfolio(): WorkloadList {
  return {
    schema: "rey.workload-list.v5",
    catalog: {
      schema: "rey.workload-catalog.v1",
      kind: "workspace_packages",
      root: "workloads",
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
    last_run_status: null,
    last_test_result_id: "blake3:test-alpha",
  };
}

function cadenceProjection(): CadenceProjection {
  return {
    schema: "rey.ui-cadence.v2",
    ordering: "partial",
    source_repository: "https://github.com/spoke-sh/rey",
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
      schema: "rey.journal-log.v1",
      log_id: "blake3:journal",
      entries: [
        {
          schema: "rey.journal-entry.v1",
          entry_id: "blake3:entry",
          sequence: 1,
          admitted_at: "1970-01-01T00:03:20.000Z",
          title: "Inspect the next bearing",
          author: { kind: "human", id: "operator" },
          binding: {
            coordinate:
              "/explore/portfolio/current;at=blake3%3Asource;lens=landscape",
            source_revision: "blake3:source",
          },
          supersedes: null,
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
