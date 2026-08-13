import type {
  WorkloadDraft,
  WorkloadList,
  WorkloadSummary,
} from "../../domain";
import { deriveAgentIndex } from "../../domain";
import type {
  TopologyEdge,
  TopologyNode,
  TopologyProjection,
  TopologyTone,
} from "../../topology";
import { admittedTopographies } from "./topography-projector";
import { buildSurveyScene } from "./survey-scene";

const NEIGHBORHOOD_LIMIT = 8;

export function buildPortfolioLandscape(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length > 0)
    return buildSurveyScene(topographies, focusId, "landscape");
  const miningResults = portfolio.workloads.reduce(
    (total, workload) => total + workload.mining_results,
    0,
  );
  const reasoningSurfaces = portfolio.workloads.reduce(
    (total, workload) => total + workload.reasoning_surfaces,
    0,
  );
  const attention = portfolio.attention.rows.length;
  const agents = deriveAgentIndex(portfolio);

  return {
    regime: "landscape",
    label: "CONTEXT LANDSCAPE",
    detail: "bounded portfolio topology",
    focus_id: focusId,
    regions: [],
    nodes: [
      node(
        "context",
        "cluster:context",
        "CONTEXT",
        "Declared surfaces",
        `${portfolio.attention.summary.surfaces} mapped · ${portfolio.attention.summary.unowned_surfaces} unowned`,
        190,
        180,
        245,
        portfolio.attention.summary.unowned_surfaces > 0
          ? "attention"
          : "neutral",
      ),
      node(
        "workloads",
        "cluster:workloads",
        "WORKLOADS",
        "Compute neighborhoods",
        `${portfolio.catalog.admitted_count} admitted · ${portfolio.catalog.draft_count} draft`,
        585,
        155,
        285,
        "accent",
      ),
      node(
        "evidence",
        "cluster:evidence",
        "EVIDENCE",
        "Mined observations",
        `${miningResults} results · ${reasoningSurfaces} reasoning surfaces`,
        980,
        245,
        275,
        "neutral",
      ),
      node(
        "attention",
        "cluster:attention",
        "ATTENTION",
        "Directed frontier",
        `${attention} unresolved row${attention === 1 ? "" : "s"}`,
        865,
        555,
        275,
        attention > 0 ? "attention" : "healthy",
      ),
      node(
        "agents",
        "cluster:agents",
        "AGENTS",
        "Exact producer coordinates",
        `${agents.length} identified · ${portfolio.catalog.draft_count} unassigned`,
        315,
        555,
        265,
        portfolio.catalog.draft_count > 0 ? "blocked" : "neutral",
      ),
      node(
        "portfolio",
        "cluster:portfolio",
        "PORTFOLIO",
        "Current coordinate",
        shortCoordinate(portfolio.attention.source_snapshot_id),
        585,
        355,
        245,
        "healthy",
      ),
    ],
    edges: [
      edge("context-workloads", "context", "workloads", "observes", "binds"),
      edge("workloads-evidence", "workloads", "evidence", "produces", "mines"),
      edge("evidence-attention", "evidence", "attention", "produces", "diffs"),
      edge(
        "attention-portfolio",
        "attention",
        "portfolio",
        "directs",
        "orients",
      ),
      edge(
        "portfolio-workloads",
        "portfolio",
        "workloads",
        "directs",
        "selects",
      ),
      edge("agents-workloads", "agents", "workloads", "produces", "proposes"),
      edge("context-portfolio", "context", "portfolio", "contains", "bounds"),
    ],
    omissions: [],
  };
}

export function buildPortfolioNeighborhoods(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length > 0 && !focusId.startsWith("agent:"))
    return buildSurveyScene(topographies, focusId, "neighborhoods");
  const workloadCandidates: Array<WorkloadSummary | WorkloadDraft> = [
    ...portfolio.workloads,
    ...portfolio.drafts,
  ];
  const agentCandidates = deriveAgentIndex(portfolio);
  const showAgents =
    focusId === "cluster:agents" || focusId.startsWith("agent:");
  const candidates = showAgents ? agentCandidates : workloadCandidates;
  const visibleCandidates = candidates.slice(0, NEIGHBORHOOD_LIMIT);
  const visibleAttention = portfolio.attention.rows.slice(
    0,
    NEIGHBORHOOD_LIMIT,
  );
  const nodes: TopologyNode[] = [];

  visibleCandidates.forEach((candidate, index) => {
    if ("producer" in candidate) {
      nodes.push(
        node(
          `agent:${candidate.id}`,
          `agent:${candidate.id}`,
          "AGENT",
          candidate.producer,
          `${candidate.kind.replaceAll("_", " ")} · ${candidate.producer_revision} · ${candidate.workload_ids.length} outputs`,
          205 + (index % 2) * 255,
          150 + Math.floor(index / 2) * 150,
          220,
          candidate.attention_rows > 0 ? "attention" : "healthy",
        ),
      );
      return;
    }
    const isDraft = "request" in candidate;
    const workloadId = isDraft
      ? candidate.request.workload_id
      : candidate.workload.id;
    const qualification = isDraft ? "draft" : candidate.qualification;
    nodes.push(
      node(
        `workload:${workloadId}`,
        `workload:${workloadId}`,
        isDraft ? "REQUEST" : "WORKLOAD",
        workloadId,
        isDraft
          ? "awaiting coding harness"
          : `${candidate.passed}/${candidate.required} scenarios · ${candidate.qualification}`,
        205 + (index % 2) * 255,
        150 + Math.floor(index / 2) * 150,
        220,
        isDraft ? "blocked" : qualificationTone(qualification),
        workloadId,
      ),
    );
  });

  visibleAttention.forEach((row, index) => {
    nodes.push(
      node(
        `attention:${row.row_id}`,
        `attention:${row.row_id}`,
        row.subject_kind === "surface" ? "SURFACE" : "ATTENTION",
        row.subject_id,
        `${row.action} · ${row.reason.replaceAll("_", " ")}`,
        750 + (index % 2) * 255,
        150 + Math.floor(index / 2) * 150,
        220,
        row.readiness === "blocked"
          ? "blocked"
          : row.readiness === "excluded"
            ? "neutral"
            : "attention",
      ),
    );
  });

  const visibleIds = new Set(nodes.map((candidate) => candidate.id));
  const edges = visibleAttention.flatMap((row) => {
    const from = `attention:${row.row_id}`;
    const to = `workload:${row.subject_id}`;
    return visibleIds.has(to)
      ? [edge(`${from}-${to}`, from, to, "directs", row.action)]
      : [];
  });
  const omissions: string[] = [];
  const candidateOmissions = candidates.length - visibleCandidates.length;
  const attentionOmissions =
    portfolio.attention.rows.length - visibleAttention.length;
  if (candidateOmissions > 0)
    omissions.push(
      `${candidateOmissions} ${showAgents ? "agent" : "workload"} neighborhoods omitted`,
    );
  if (attentionOmissions > 0)
    omissions.push(`${attentionOmissions} attention neighborhoods omitted`);

  return {
    regime: "neighborhoods",
    label: "CONTEXT NEIGHBORHOODS",
    detail: `${candidates.length} ${showAgents ? "agent" : "workload"} · ${portfolio.attention.rows.length} attention`,
    focus_id: focusId,
    regions: [
      {
        id: "workload-region",
        label: showAgents ? "AGENT NEIGHBORHOODS" : "WORKLOAD NEIGHBORHOODS",
        detail: `${candidates.length} bounded ${showAgents ? "generator" : "compute"} contexts`,
        x: 55,
        y: 55,
        width: 520,
        height: 610,
        tone: "accent",
      },
      {
        id: "attention-region",
        label: "ATTENTION NEIGHBORHOODS",
        detail: `${portfolio.attention.rows.length} directed deltas`,
        x: 625,
        y: 55,
        width: 520,
        height: 610,
        tone: portfolio.attention.rows.length > 0 ? "attention" : "healthy",
      },
    ],
    nodes,
    edges,
    omissions,
  };
}

function qualificationTone(qualification: string): TopologyTone {
  if (qualification === "qualified") return "healthy";
  if (qualification === "failing" || qualification === "inconclusive")
    return "blocked";
  if (qualification === "stale" || qualification === "untested")
    return "attention";
  return "neutral";
}

function shortCoordinate(value: string): string {
  const coordinate = value.startsWith("blake3:") ? value.slice(7) : value;
  return coordinate.length > 22
    ? `${coordinate.slice(0, 10)}…${coordinate.slice(-8)}`
    : coordinate;
}

function node(
  id: string,
  focusId: string,
  family: string,
  label: string,
  detail: string,
  x: number,
  y: number,
  width: number,
  tone: TopologyTone,
  workloadId?: string,
  coordinateUri?: string,
): TopologyNode {
  return {
    id,
    focus_id: focusId,
    family,
    label,
    detail,
    x,
    y,
    width,
    tone,
    workload_id: workloadId,
    coordinate_uri: coordinateUri,
  };
}

function edge(
  id: string,
  from: string,
  to: string,
  kind: TopologyEdge["kind"],
  label: string,
): TopologyEdge {
  return { id, from, to, kind, label };
}
