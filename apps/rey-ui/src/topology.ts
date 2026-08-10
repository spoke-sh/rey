import type {
  AttentionRow,
  WorkloadDraft,
  WorkloadList,
  WorkloadSummary,
} from "./domain";

export type LensRegime = "landscape" | "neighborhoods" | "objects";
export type TopologyTone =
  "neutral" | "accent" | "healthy" | "attention" | "blocked";

export interface TopologyRegion {
  id: string;
  label: string;
  detail: string;
  x: number;
  y: number;
  width: number;
  height: number;
  tone: TopologyTone;
}

export interface TopologyNode {
  id: string;
  focus_id: string;
  family: string;
  label: string;
  detail: string;
  x: number;
  y: number;
  width: number;
  tone: TopologyTone;
  workload_id?: string;
}

export interface TopologyEdge {
  id: string;
  from: string;
  to: string;
  kind: "contains" | "directs" | "produces" | "observes" | "depends";
  label: string;
}

export interface TopologyScene {
  regime: LensRegime;
  label: string;
  detail: string;
  focus_id: string;
  regions: TopologyRegion[];
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  omissions: string[];
}

export const TOPOLOGY_WORLD = { width: 1200, height: 720 } as const;
export const MIN_LENS_ZOOM = 0.55;
export const MAX_LENS_ZOOM = 2;
export const DEFAULT_LENS_ZOOM = 0.68;
export const NEIGHBORHOOD_LENS_ZOOM = 1;
export const OBJECT_LENS_ZOOM = 1.46;

const NEIGHBORHOOD_LIMIT = 8;

export function lensRegimeForZoom(zoom: number): LensRegime {
  if (zoom < 0.88) return "landscape";
  if (zoom < 1.32) return "neighborhoods";
  return "objects";
}

export function clampLensZoom(zoom: number): number {
  return Math.min(MAX_LENS_ZOOM, Math.max(MIN_LENS_ZOOM, zoom));
}

export function stepLensZoom(zoom: number, direction: 1 | -1): number {
  const regime = lensRegimeForZoom(zoom);
  if (direction > 0) {
    if (regime === "landscape") return NEIGHBORHOOD_LENS_ZOOM;
    if (regime === "neighborhoods") return OBJECT_LENS_ZOOM;
    return MAX_LENS_ZOOM;
  }
  if (regime === "objects") return NEIGHBORHOOD_LENS_ZOOM;
  if (regime === "neighborhoods") return DEFAULT_LENS_ZOOM;
  return MIN_LENS_ZOOM;
}

export function buildTopologyScene(
  portfolio: WorkloadList,
  zoom: number,
  focusId = "cluster:portfolio",
): TopologyScene {
  const regime = lensRegimeForZoom(zoom);
  if (regime === "landscape") return buildLandscape(portfolio, focusId);
  if (regime === "neighborhoods") return buildNeighborhoods(portfolio, focusId);
  return buildObjects(portfolio, focusId);
}

function buildLandscape(
  portfolio: WorkloadList,
  focusId: string,
): TopologyScene {
  const miningResults = portfolio.workloads.reduce(
    (total, workload) => total + workload.mining_results,
    0,
  );
  const reasoningSurfaces = portfolio.workloads.reduce(
    (total, workload) => total + workload.reasoning_surfaces,
    0,
  );
  const attention = portfolio.attention.rows.length;

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
        "drafts",
        "cluster:drafts",
        "REQUESTS",
        "Agentic handoffs",
        `${portfolio.catalog.draft_count} awaiting coding harness`,
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
      edge("drafts-workloads", "drafts", "workloads", "contains", "admits"),
      edge("context-portfolio", "context", "portfolio", "contains", "bounds"),
    ],
    omissions: [],
  };
}

function buildNeighborhoods(
  portfolio: WorkloadList,
  focusId: string,
): TopologyScene {
  const candidates: Array<WorkloadSummary | WorkloadDraft> = [
    ...portfolio.workloads,
    ...portfolio.drafts,
  ];
  const visibleCandidates = candidates.slice(0, NEIGHBORHOOD_LIMIT);
  const visibleAttention = portfolio.attention.rows.slice(
    0,
    NEIGHBORHOOD_LIMIT,
  );
  const nodes: TopologyNode[] = [];

  visibleCandidates.forEach((candidate, index) => {
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
    omissions.push(`${candidateOmissions} workload neighborhoods omitted`);
  if (attentionOmissions > 0)
    omissions.push(`${attentionOmissions} attention neighborhoods omitted`);

  return {
    regime: "neighborhoods",
    label: "CONTEXT NEIGHBORHOODS",
    detail: `${candidates.length} workload · ${portfolio.attention.rows.length} attention`,
    focus_id: focusId,
    regions: [
      {
        id: "workload-region",
        label: "WORKLOAD NEIGHBORHOODS",
        detail: `${candidates.length} bounded compute contexts`,
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

function buildObjects(
  portfolio: WorkloadList,
  requestedFocusId: string,
): TopologyScene {
  const focusId = resolveObjectFocus(portfolio, requestedFocusId);
  if (focusId.startsWith("workload:")) {
    const workloadId = focusId.slice("workload:".length);
    const workload = portfolio.workloads.find(
      (candidate) => candidate.workload.id === workloadId,
    );
    const draft = portfolio.drafts.find(
      (candidate) => candidate.request.workload_id === workloadId,
    );
    if (workload) return workloadObjectScene(portfolio, workload, focusId);
    if (draft) return draftObjectScene(portfolio, draft, focusId);
  }
  if (focusId.startsWith("attention:")) {
    const rowId = focusId.slice("attention:".length);
    const row = portfolio.attention.rows.find(
      (candidate) => candidate.row_id === rowId,
    );
    if (row) return attentionObjectScene(portfolio, row, focusId);
  }
  return portfolioObjectScene(portfolio, focusId);
}

function workloadObjectScene(
  portfolio: WorkloadList,
  workload: WorkloadSummary,
  focusId: string,
): TopologyScene {
  const attention = portfolio.attention.rows.filter(
    (row) => row.subject_id === workload.workload.id,
  );
  return {
    regime: "objects",
    label: "WORKLOAD OBJECTS",
    detail: workload.workload.id,
    focus_id: focusId,
    regions: [
      {
        id: "object-boundary",
        label: "BOUNDED COMPUTE GRAPH",
        detail: shortCoordinate(workload.workload.semantic_digest),
        x: 45,
        y: 55,
        width: 1110,
        height: 610,
        tone: qualificationTone(workload.qualification),
      },
    ],
    nodes: [
      node(
        "context-object",
        "cluster:context",
        "CONTEXT",
        workload.provenance?.origin ?? "compiled",
        workload.provenance?.source ?? "built-in conformance surface",
        160,
        355,
        205,
        "neutral",
      ),
      node(
        "workload-object",
        focusId,
        "WORKLOAD",
        workload.workload.id,
        `revision ${workload.workload.revision} · ${workload.qualification}`,
        395,
        355,
        230,
        qualificationTone(workload.qualification),
        workload.workload.id,
      ),
      node(
        "graph-object",
        focusId,
        "COMPUTE GRAPH",
        workload.candidate_graph.id,
        `revision ${workload.candidate_graph.revision} · ${shortCoordinate(workload.candidate_graph.semantic_digest)}`,
        650,
        205,
        225,
        "accent",
      ),
      node(
        "scenario-object",
        focusId,
        "SCENARIOS",
        `${workload.passed}/${workload.required} passing`,
        `${workload.failed} failing · ${workload.inconclusive} inconclusive`,
        650,
        505,
        225,
        workload.failed + workload.inconclusive > 0 ? "attention" : "healthy",
      ),
      node(
        "evidence-object",
        "cluster:evidence",
        "EVIDENCE",
        `${workload.mining_results} mined results`,
        `${workload.relation_deltas} deltas · ${workload.reasoning_surfaces} surfaces`,
        905,
        205,
        225,
        workload.incomplete_mining_results > 0 ? "blocked" : "neutral",
      ),
      node(
        "attention-object",
        attention[0] ? `attention:${attention[0].row_id}` : "cluster:attention",
        "ATTENTION",
        attention[0]?.action ?? "no directed row",
        attention[0]?.reason.replaceAll("_", " ") ?? "locally converged",
        905,
        505,
        225,
        attention.length > 0 ? "attention" : "healthy",
      ),
    ],
    edges: [
      edge(
        "context-workload-object",
        "context-object",
        "workload-object",
        "contains",
        "binds",
      ),
      edge(
        "workload-graph-object",
        "workload-object",
        "graph-object",
        "contains",
        "executes",
      ),
      edge(
        "graph-scenario-object",
        "graph-object",
        "scenario-object",
        "produces",
        "evaluated by",
      ),
      edge(
        "graph-evidence-object",
        "graph-object",
        "evidence-object",
        "produces",
        "mines",
      ),
      edge(
        "scenario-attention-object",
        "scenario-object",
        "attention-object",
        "produces",
        "diff directs",
      ),
      edge(
        "evidence-attention-object",
        "evidence-object",
        "attention-object",
        "observes",
        "supports",
      ),
    ],
    omissions:
      attention.length > 1
        ? [`${attention.length - 1} additional attention rows omitted`]
        : [],
  };
}

function draftObjectScene(
  portfolio: WorkloadList,
  draft: WorkloadDraft,
  focusId: string,
): TopologyScene {
  return {
    regime: "objects",
    label: "REQUEST OBJECTS",
    detail: draft.request.workload_id,
    focus_id: focusId,
    regions: [
      {
        id: "request-boundary",
        label: "AGENTIC HANDOFF",
        detail: shortCoordinate(draft.request.request_id),
        x: 85,
        y: 125,
        width: 1030,
        height: 470,
        tone: "blocked",
      },
    ],
    nodes: [
      node(
        "request-object",
        focusId,
        "REQUEST",
        draft.request.workload_id,
        draft.request.intent ?? draft.request.title,
        235,
        355,
        245,
        "accent",
        draft.request.workload_id,
      ),
      node(
        "harness-object",
        focusId,
        "CODING HARNESS",
        draft.request.proposer,
        "external agentic generation boundary",
        600,
        355,
        245,
        "attention",
      ),
      node(
        "package-object",
        focusId,
        "TARGET PACKAGE",
        draft.request.target_package,
        "graph + frozen scenario oracle missing",
        965,
        355,
        245,
        "blocked",
      ),
    ],
    edges: [
      edge(
        "request-harness-object",
        "request-object",
        "harness-object",
        "directs",
        "requests",
      ),
      edge(
        "harness-package-object",
        "harness-object",
        "package-object",
        "produces",
        "materializes",
      ),
    ],
    omissions: [],
  };
}

function attentionObjectScene(
  portfolio: WorkloadList,
  row: AttentionRow,
  focusId: string,
): TopologyScene {
  const subjectExists =
    portfolio.workloads.some(
      (workload) => workload.workload.id === row.subject_id,
    ) ||
    portfolio.drafts.some(
      (draft) => draft.request.workload_id === row.subject_id,
    );
  return {
    regime: "objects",
    label: "ATTENTION OBJECTS",
    detail: `${row.action} / ${row.readiness}`,
    focus_id: focusId,
    regions: [
      {
        id: "attention-boundary",
        label: "DIRECTED REASONING SURFACE",
        detail: shortCoordinate(row.row_id),
        x: 55,
        y: 75,
        width: 1090,
        height: 570,
        tone: row.readiness === "blocked" ? "blocked" : "attention",
      },
    ],
    nodes: [
      node(
        "snapshot-object",
        "cluster:portfolio",
        "SOURCE SNAPSHOT",
        shortCoordinate(portfolio.attention.source_snapshot_id),
        "retained portfolio observation",
        155,
        355,
        210,
        "neutral",
      ),
      node(
        "delta-object",
        focusId,
        "ATTENTION DELTA",
        row.reason.replaceAll("_", " "),
        `${row.action} · priority ${row.priority} · cost ${row.estimated_cost_units}`,
        425,
        355,
        245,
        row.readiness === "blocked" ? "blocked" : "attention",
      ),
      node(
        "subject-object",
        subjectExists ? `workload:${row.subject_id}` : focusId,
        row.subject_kind.toUpperCase(),
        row.subject_id,
        `${row.readiness} · ${row.evidence_ids.length} evidence bindings`,
        725,
        225,
        245,
        "accent",
        subjectExists ? row.subject_id : undefined,
      ),
      node(
        "evidence-binding-object",
        "cluster:evidence",
        "EVIDENCE BINDINGS",
        `${row.evidence_ids.length} exact references`,
        row.evidence_ids[0]
          ? shortCoordinate(row.evidence_ids[0])
          : "no retained evidence reference",
        1000,
        225,
        220,
        row.evidence_ids.length > 0 ? "healthy" : "blocked",
      ),
      node(
        "dependency-object",
        focusId,
        "DEPENDENCIES",
        `${row.dependency_ids.length} required coordinates`,
        row.dependency_ids[0]
          ? shortCoordinate(row.dependency_ids[0])
          : "ready without dependency",
        725,
        505,
        245,
        row.dependency_ids.length > 0 ? "attention" : "healthy",
      ),
    ],
    edges: [
      edge(
        "snapshot-delta-object",
        "snapshot-object",
        "delta-object",
        "produces",
        "derives",
      ),
      edge(
        "delta-subject-object",
        "delta-object",
        "subject-object",
        "directs",
        row.action,
      ),
      edge(
        "evidence-delta-object",
        "evidence-binding-object",
        "delta-object",
        "observes",
        "supports",
      ),
      edge(
        "dependency-delta-object",
        "dependency-object",
        "delta-object",
        "depends",
        "bounds",
      ),
    ],
    omissions: [
      ...(row.evidence_ids.length > 1
        ? [`${row.evidence_ids.length - 1} evidence references folded`]
        : []),
      ...(row.dependency_ids.length > 1
        ? [`${row.dependency_ids.length - 1} dependency references folded`]
        : []),
    ],
  };
}

function portfolioObjectScene(
  portfolio: WorkloadList,
  focusId: string,
): TopologyScene {
  const admitted = portfolio.catalog.admitted_count;
  const qualified = portfolio.workloads.filter(
    (workload) => workload.qualification === "qualified",
  ).length;
  return {
    regime: "objects",
    label: "PORTFOLIO OBJECTS",
    detail: shortCoordinate(portfolio.attention.source_snapshot_id),
    focus_id: focusId,
    regions: [
      {
        id: "portfolio-boundary",
        label: "CURRENT CONTEXT COORDINATE",
        detail: `${portfolio.catalog.workload_count} workload objects`,
        x: 55,
        y: 75,
        width: 1090,
        height: 570,
        tone: "neutral",
      },
    ],
    nodes: [
      node(
        "surface-object",
        "cluster:context",
        "SURFACES",
        `${portfolio.attention.summary.surfaces} declared`,
        `${portfolio.attention.summary.owned_surfaces} owned · ${portfolio.attention.summary.unowned_surfaces} unowned`,
        155,
        355,
        215,
        portfolio.attention.summary.unowned_surfaces > 0
          ? "attention"
          : "healthy",
      ),
      node(
        "catalog-object",
        "cluster:workloads",
        "CATALOG",
        `${admitted} admitted`,
        `${portfolio.catalog.draft_count} creation requests`,
        425,
        225,
        225,
        "accent",
      ),
      node(
        "qualification-object",
        "cluster:workloads",
        "QUALIFICATION",
        `${qualified}/${admitted} qualified`,
        "fresh deterministic scenario results",
        425,
        505,
        225,
        qualified === admitted && admitted > 0 ? "healthy" : "attention",
      ),
      node(
        "snapshot-evidence-object",
        "cluster:evidence",
        "SNAPSHOT",
        shortCoordinate(portfolio.attention.source_snapshot_id),
        "catalog + results + context + coverage",
        725,
        225,
        235,
        "neutral",
      ),
      node(
        "frontier-object",
        "cluster:attention",
        "FRONTIER",
        `${portfolio.attention.rows.length} directed rows`,
        shortCoordinate(portfolio.attention.attention_id),
        725,
        505,
        235,
        portfolio.attention.rows.length > 0 ? "attention" : "healthy",
      ),
      node(
        "bearing-object",
        "cluster:portfolio",
        "NEXT BEARING",
        nextBearing(portfolio),
        "derived from retained attention; no scheduler selection",
        1010,
        355,
        230,
        portfolio.attention.rows.length > 0 ? "accent" : "healthy",
      ),
    ],
    edges: [
      edge(
        "surface-catalog-object",
        "surface-object",
        "catalog-object",
        "contains",
        "bounds",
      ),
      edge(
        "catalog-qualification-object",
        "catalog-object",
        "qualification-object",
        "produces",
        "tests",
      ),
      edge(
        "catalog-snapshot-object",
        "catalog-object",
        "snapshot-evidence-object",
        "observes",
        "records",
      ),
      edge(
        "qualification-snapshot-object",
        "qualification-object",
        "snapshot-evidence-object",
        "produces",
        "retains",
      ),
      edge(
        "snapshot-frontier-object",
        "snapshot-evidence-object",
        "frontier-object",
        "produces",
        "diffs",
      ),
      edge(
        "frontier-bearing-object",
        "frontier-object",
        "bearing-object",
        "directs",
        "orients",
      ),
    ],
    omissions: [],
  };
}

function resolveObjectFocus(portfolio: WorkloadList, focusId: string): string {
  if (focusId.startsWith("workload:") || focusId.startsWith("attention:"))
    return focusId;
  if (focusId === "cluster:workloads" || focusId === "cluster:drafts") {
    const workloadId =
      portfolio.workloads[0]?.workload.id ??
      portfolio.drafts[0]?.request.workload_id;
    if (workloadId) return `workload:${workloadId}`;
  }
  if (focusId === "cluster:attention" && portfolio.attention.rows[0])
    return `attention:${portfolio.attention.rows[0].row_id}`;
  return focusId;
}

function nextBearing(portfolio: WorkloadList): string {
  const row = portfolio.attention.rows[0];
  return row
    ? `${row.action.toUpperCase()} ${row.subject_id}`
    : "NO UNRESOLVED ATTENTION";
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
