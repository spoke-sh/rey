import { describe, expect, it } from "vitest";
import { deriveAgentRuntimes, deriveCollaborationTasks } from "./agents";
import type { WorkloadList } from "./domain";
import type {
  EnvironmentApplicationObservation,
  EnvironmentObjectStatus,
  EnvironmentStatus,
} from "./environment";

describe("agent collaboration projection", () => {
  it("indexes only process-discovered agent runtime applications", () => {
    const status = environmentStatus([
      application("tool.git.identity", "git", "tool.git.identity", "available"),
      application(
        "agent.runtime.codex.identity",
        "codex",
        "agent.runtime.codex.identity",
        "available",
      ),
      application(
        "agent.runtime.claude.identity",
        "claude",
        "agent.runtime.claude.identity",
        "unavailable",
      ),
    ]);

    expect(
      deriveAgentRuntimes(status).map((runtime) => [
        runtime.application.name,
        runtime.application.availability,
      ]),
    ).toEqual([
      ["claude", "unavailable"],
      ["codex", "available"],
    ]);
  });

  it("derives current tasks without duplicating a draft attention row", () => {
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
        priority: 4,
        estimated_cost_units: 2,
      },
    );

    expect(deriveCollaborationTasks(portfolio)).toMatchObject([
      {
        id: "request:alpha",
        operation: "AUTHOR",
        evidence_count: 1,
        workload_id: "alpha",
      },
      {
        id: "attention:surface",
        operation: "RESOLVE",
        dependency_count: 1,
        workload_id: null,
      },
    ]);
  });
});

function application(
  objectId: string,
  name: string,
  capability: string,
  availability: EnvironmentApplicationObservation["availability"],
): EnvironmentObjectStatus<EnvironmentApplicationObservation> {
  return {
    object_id: objectId,
    head: null,
    index: null,
    working: {
      name,
      purpose: "fixture",
      required: false,
      availability,
      resolved_path: availability === "available" ? `/bin/${name}` : null,
      content_digest: null,
      potential_capabilities: [capability],
      searched_path_count: 3,
      error_code: availability === "unavailable" ? "not_found" : null,
    },
    changes: {
      head_to_index: "unchanged",
      index_to_working: "inserted",
      head_to_working: "inserted",
    },
  };
}

function environmentStatus(
  applications: EnvironmentObjectStatus<EnvironmentApplicationObservation>[],
): EnvironmentStatus {
  return {
    schema: "rey.environment-status.v5",
    head_commit_id: null,
    head_sequence: null,
    head_snapshot_id: null,
    state: "unborn",
    working_snapshot: {
      semantic_digest: "blake3:environment",
      complete: true,
      profile: "standalone",
    },
    operator: {
      schema: "rey.environment-operator-projection.v3",
      source_label: "EMPTY",
      target_label: "WORKING",
      complete: true,
      mapping: null,
      application_inventory: { head: null, index: null, working: null },
      summary: {
        variables: 0,
        changed_variables: 0,
        applications_searched: applications.length,
        applications_found: applications.filter(
          (entry) => entry.working?.availability === "available",
        ).length,
        applications_not_found: applications.filter(
          (entry) => entry.working?.availability === "unavailable",
        ).length,
        application_errors: 0,
        changed_applications: applications.length,
        inputs: 0,
        changed_inputs: 0,
        references: 0,
      },
      variables: [],
      applications,
      inputs: [],
      references: [],
    },
    staged_delta: { changes: [] },
    unstaged_delta: { changes: [] },
  };
}

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
