import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AgentsPage, deriveJournalEntries, deriveWorkInsights } from "./agents";
import type { WorkloadList, WorkloadSummary } from "./domain";
import type { JournalOpportunitySurface, JournalProjection } from "./journal";
import type { AgentProcessDescriptor } from "./api";

describe("agent collaboration intelligence", () => {
  it("ranks typed recommendations without duplicating request attention", () => {
    const portfolio = emptyPortfolio();
    portfolio.drafts.push({
      request: {
        request_id: "request:alpha",
        workload_id: "alpha",
        title: "Author alpha",
        intent: "Create a graph over the surveyed artifacts",
        proposer: "coding_harness",
        target_package: "workloads/alpha/workload.yaml",
      },
      source: ".rey/workload-requests/alpha.yaml",
      source_digest: "blake3:request",
    });
    portfolio.attention.rows.push(
      {
        row_id: "attention:alpha",
        action: "create",
        subject_kind: "workload",
        subject_id: "alpha",
        reason: "workload graph missing",
        readiness: "ready",
        evidence_ids: ["evidence:survey"],
        dependency_ids: [],
        priority: 8,
        estimated_cost_units: 3,
      },
      {
        row_id: "attention:surface",
        action: "block",
        subject_kind: "surface",
        subject_id: "source:unknown",
        reason: "locator missing",
        readiness: "blocked",
        evidence_ids: [],
        dependency_ids: ["locator:required"],
        priority: 12,
        estimated_cost_units: 2,
      },
    );

    expect(deriveJournalEntries(portfolio)).toMatchObject([
      {
        id: "request:alpha",
        author: "rey",
        author_kind: "system",
        origin: "derived",
        operation: "AUTHOR",
        profile: "CODING HARNESS",
        source: "REQUEST + ATTENTION",
        evidence_count: 1,
        readiness: "ready",
      },
      {
        id: "attention:surface",
        operation: "RESOLVE",
        profile: "SURVEY / OPERATOR",
        dependency_count: 1,
        readiness: "blocked",
      },
    ]);
  });

  it("renders a quiet Journal with an honest shared-write affordance", () => {
    const markup = renderToStaticMarkup(
      createElement(AgentsPage, {
        agent: agentProcess(),
        journal: emptyJournal(),
        opportunities: emptyOpportunities(),
        portfolio: emptyPortfolio(),
      }),
    );

    expect(markup).toContain('data-rey-section="01 / JOURNAL"');
    expect(markup).toContain("Supervised agent topology");
    expect(markup).toContain("local-process:4200");
    expect(markup).toContain("rey.orchestrator");
    expect(markup).toContain("rey.operator-http");
    expect(markup).toContain("NO AUTONOMOUS WORKLOAD SCHEDULING");
    expect(markup).toContain('data-rey-section="02 / REY PROCESS"');
    expect(markup.indexOf("What should happen next")).toBeLessThan(
      markup.indexOf("Supervised agent topology"),
    );
    expect(markup).toContain("NO AGENT WORK RECOMMENDED BY CURRENT EVIDENCE");
    expect(markup).toContain('data-journal-admission="available"');
    expect(markup).toContain("WRITE A JOURNAL ENTRY");
    expect(markup).toContain("HUMAN + AGENT · EXPLORE-BOUND");
    expect(markup).toContain("UNAUTHENTICATED · VALIDATED DOCUMENT ADMISSION");
    expect(markup).toContain('href="/journal/new"');
    expect(markup).not.toContain("disabled");
    expect(markup).not.toContain("RECOMMENDATION BASIS");
  });

  it("keeps Journal admission available without an authentication boundary", () => {
    const markup = renderToStaticMarkup(
      createElement(AgentsPage, {
        agent: agentProcess(),
        journal: emptyJournal(),
        opportunities: emptyOpportunities(),
        portfolio: emptyPortfolio(),
      }),
    );

    expect(markup).toContain('data-journal-admission="available"');
    expect(markup).toContain('href="/journal/new"');
    expect(markup).not.toContain("AUTHENTICATION REQUIRED");
  });

  it("renders current authored actions as inert exact opportunities", () => {
    const opportunities = emptyOpportunities();
    opportunities.summary = {
      current_entries: 1,
      authored_actions: 1,
      projected: 1,
      omitted: 0,
    };
    opportunities.rows.push({
      schema: "rey.journal-opportunity.v1",
      opportunity_id: "blake3:opportunity",
      entry_id: "blake3:entry",
      entry_sequence: 4,
      document_path: "/journal/j4-bearing--blake3-entry",
      block_id: "next-bearing",
      fragment: "block-next-bearing",
      author: { kind: "agent", id: "codex" },
      binding: {
        coordinate: "rey+local://document/current?revision=blake3%3Asource",
        scale: 1,
        source_revision: "blake3:source",
      },
      operation: "refine",
      desired_delta: "Close the bounded coverage gap.",
      evidence_ids: ["blake3:evidence"],
      dependency_ids: [],
      readiness: "authored_only",
      authority: "none",
    });

    const markup = renderToStaticMarkup(
      createElement(AgentsPage, {
        agent: agentProcess(),
        journal: emptyJournal(),
        opportunities,
        portfolio: emptyPortfolio(),
      }),
    );

    expect(markup).toContain(
      'data-journal-opportunity-surface="blake3:opportunities"',
    );
    expect(markup).toContain("J@4#next-bearing");
    expect(markup).toContain("Close the bounded coverage gap.");
    expect(markup).toContain("AUTHORED ONLY");
    expect(markup).toContain("AUTHORITY / NONE");
    expect(markup).toContain(
      'href="/journal/j4-bearing--blake3-entry#block-next-bearing"',
    );
    expect(markup).toContain("NO ASSIGNMENT OR EXECUTION");
  });

  it("reports observed work from retained results rather than agent activity", () => {
    const portfolio = emptyPortfolio();
    portfolio.workloads.push(workload());

    expect(deriveWorkInsights(portfolio)).toEqual([
      expect.objectContaining({
        workload_id: "alpha",
        kind: "ADMITTED",
        observed_operation: "RUN",
        result: "PASSED",
        journey: "RUN COMPLETE",
        scenarios_passed: 3,
        scenarios_required: 4,
        artifact_summary: "2 mining · 1 deltas · 1 surfaces",
        evidence_id: "test:alpha",
      }),
    ]);
  });
});

function emptyJournal(): JournalProjection {
  return {
    schema: "rey.ui-journal.v2",
    write_enabled: true,
    authority: "unauthenticated_journal_admission",
    log: {
      schema: "rey.journal-log.v2",
      log_id: "blake3:empty",
      entries: [],
    },
  };
}

function agentProcess(): AgentProcessDescriptor {
  return {
    schema: "rey.agent-process.v1",
    state: "running",
    process: {
      schema: "rey.process.v1",
      process_id: "local-process:4200",
      os_pid: 4200,
      role: "orchestrator",
      topology_node_id: "rey.orchestrator",
      invocation: "rey agent",
      lifecycle: "foreground; owns every in-process background worker",
      shutdown: "cooperative SIGINT/SIGTERM at a bounded worker boundary",
      implementation_revision: "git:agent",
    },
    topology: {
      schema: "rey.agent-topology.v1",
      root_node_id: "rey.orchestrator",
      nodes: [
        {
          node_id: "rey.orchestrator",
          kind: "rey_process",
          parent_node_id: null,
          execution: "os_process",
          lifecycle: "foreground",
          state: "running",
          restart_policy: "external",
          authority:
            "background_lifecycle_only; no workload or agent-runtime authority",
          endpoint: null,
        },
        {
          node_id: "rey.operator-http",
          kind: "background_work",
          parent_node_id: "rey.orchestrator",
          execution: "supervised_thread",
          lifecycle: "bound_to_rey_process",
          state: "running",
          restart_policy: "never; fail the Rey process closed",
          authority: "operator HTTP projection and its declared bounded writes",
          endpoint: "http://127.0.0.1:4200/",
        },
      ],
      edges: [
        {
          source_node_id: "rey.orchestrator",
          target_node_id: "rey.operator-http",
          relationship: "supervises",
        },
      ],
      max_background_workers: 1,
      supervision_poll_interval_ms: 50,
      agent_runtime_invocation:
        "none; discovery, assignment, and execution authority remain separate",
    },
    operator: {
      source_repository: null,
      implementation_revision: "git:agent",
      journal_write_enabled: true,
      observation_write_enabled: true,
      workload_admission_enabled: true,
      channel_write_enabled: true,
      conversation_write_enabled: true,
      read_only: false,
    },
    authority: "local orchestration and operator projection only",
    omissions: ["no autonomous workload scheduling"],
  };
}

function emptyOpportunities(): JournalOpportunitySurface {
  return {
    schema: "rey.journal-opportunity-surface.v1",
    surface_id: "blake3:opportunities",
    source_log_id: "blake3:empty",
    ordering: "journal_sequence_then_block_order",
    completeness: "complete",
    limits: {
      max_rows: 128,
      max_log_entries: 256,
      max_blocks_per_entry: 32,
    },
    summary: {
      current_entries: 0,
      authored_actions: 0,
      projected: 0,
      omitted: 0,
    },
    rows: [],
    omissions: [],
    runtime_boundary:
      "requires_verified_selected_ready_create_attention_row_and_workload_admission",
  };
}

function workload(): WorkloadSummary {
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
    qualification: "qualified",
    required: 4,
    passed: 3,
    failed: 0,
    inconclusive: 0,
    evaluated: 3,
    stale: 0,
    optional: 1,
    mining_operations: 2,
    mining_results: 2,
    incomplete_mining_results: 0,
    relation_deltas: 1,
    reasoning_surfaces: 1,
    attention_rows: 0,
    topography_results: 0,
    topography_revision: null,
    topography_coverage: null,
    topography_frontier_rows: 0,
    topography_patch: null,
    topography_projection: null,
    scene_admission_results: 0,
    latest_scene_admission: null,
    last_run_status: "passed",
    last_test_result_id: "test:alpha",
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
