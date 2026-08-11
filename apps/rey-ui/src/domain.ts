export type Qualification =
  "untested" | "qualified" | "failing" | "inconclusive" | "stale";
export type Freshness = "untested" | "fresh" | "stale";
export type AttentionAction =
  "refine" | "retest" | "create" | "block" | "policy_excluded";
export type AttentionReadiness = "ready" | "blocked" | "excluded";

export interface ContractIdentity {
  id: string;
  revision: number;
  semantic_digest: string;
}

export interface CatalogDescriptor {
  schema: string;
  kind: "workspace_packages" | "built_in_conformance";
  root: string | null;
  workload_count: number;
  admitted_count: number;
  draft_count: number;
}

export interface GeneratorProvenance {
  kind: "coding_harness" | "rule" | "human";
  producer: string;
  producer_revision: string;
}

export interface WorkloadProvenance {
  origin: "workspace_package" | "built_in_conformance" | "built_in_system";
  source: string;
  source_digest: string | null;
  generation: GeneratorProvenance | null;
  admission: {
    state: "proposed" | "accepted" | "rejected";
    scenario_oracle: "mutable" | "frozen";
  };
}

export interface WorkloadSummary {
  provenance: WorkloadProvenance | null;
  workload: ContractIdentity;
  title: string;
  candidate_graph: ContractIdentity;
  freshness: Freshness;
  qualification: Qualification;
  required: number;
  passed: number;
  failed: number;
  inconclusive: number;
  evaluated: number;
  stale: number;
  optional: number;
  mining_operations: number;
  mining_results: number;
  incomplete_mining_results: number;
  relation_deltas: number;
  reasoning_surfaces: number;
  attention_rows: number;
  topography_results: number;
  topography_revision: string | null;
  topography_coverage: TopographyCoverage | null;
  topography_frontier_rows: number;
  topography_patch: TopographyPatch | null;
  topography_projection: ProjectionPacket | null;
  last_run_status: "passed" | "blocked" | null;
  last_test_result_id: string | null;
}

export type TopographyRegionState =
  | "surveyed"
  | "surveyed_empty"
  | "unexplored"
  | "omitted"
  | "stale"
  | "unsupported"
  | "frontier";

export type LocatorResolutionStatus =
  | "resolved"
  | "missing"
  | "stale"
  | "unsupported"
  | "unauthorized"
  | "malformed"
  | "truncated";

export interface CoordinateBinding {
  schema: string;
  binding_id: string;
  profile: "local_standalone" | "spoke_public";
  provider: ContractIdentity;
  coordinate: string;
  identity_class: "immutable" | "revision_bound" | "mutable";
  source_revision: string;
  retention: string;
}

export interface TopographyCoverage {
  requested_seeds: number;
  surveyed_seeds: number;
  surveyed_empty_seeds: number;
  missing_seeds: number;
  omitted_seeds: number;
  candidates: number;
  unique_candidates: number;
  resolved_candidates: number;
  unresolved_candidates: number;
}

export type ProjectionObjectKind = "anchor" | "frontier";
export type ProjectionFieldKind = "scalar" | "vector" | "mask";
export type ProjectionLayerAuthority = "evidence" | "derived" | "presentation";
export type ProjectionTerrainRegime =
  "world" | "atlas" | "landscape" | "neighborhoods" | "objects" | "evidence";

export interface ProjectionFieldLevel {
  level_id: string;
  columns: number;
  rows: number;
  cells: number;
  bytes_per_cell: number;
  total_bytes: number;
  sample_stride: number;
  regimes: ProjectionTerrainRegime[];
  detail_authority: string;
}

export interface ProjectionPacket {
  schema: "rey.projection-packet.v2";
  packet_id: string;
  source_patch_id: string;
  source_topography_revision: string;
  projection_basis: {
    contract: ContractIdentity;
    input_dimensions: string[];
    output_dimensions: string[];
    parameters: Record<string, string>;
    normalization: string;
    random_seed: number | null;
    distance_semantics: string;
    neighborhood_semantics: string;
    distortion: string;
    stable_coordinate_rule: string;
  };
  scene_compiler: ContractIdentity;
  extent: { width: number; height: number; unit: string };
  field_pyramid: {
    schema: "rey.terrain-field-pyramid.v1";
    levels: ProjectionFieldLevel[];
    total_cells: number;
    total_bytes: number;
    stable_coordinate_rule: string;
  };
  objects: Array<{
    object_id: string;
    source_id: string;
    kind: ProjectionObjectKind;
    anchor_kind: "workspace" | "file" | "document" | "external_resource" | null;
    frontier_status: LocatorResolutionStatus | null;
    coordinate: string | null;
    label: string;
    detail: string;
    source_revision: string;
  }>;
  validity: Array<{
    region_id: string;
    coordinate: string;
    state: TopographyRegionState;
    detail: string;
    source_revision: string;
  }>;
  field_channels: Array<{
    id: string;
    kind: ProjectionFieldKind;
    semantics: string;
    units: string;
    normalization: string;
    source_revision: string;
    implementation: ContractIdentity;
  }>;
  layers: Array<{
    id: string;
    authority: ProjectionLayerAuthority;
    semantics: string;
    source_revision: string;
  }>;
  excluded_source_relationships: number;
  limits: {
    max_anchor_objects: number;
    max_frontier_objects: number;
    max_validity_regions: number;
    max_field_channels: number;
    max_field_levels: number;
    max_layers: number;
    max_omissions: number;
    max_field_cells: number;
    max_field_bytes: number;
    max_total_field_cells: number;
    max_total_field_bytes: number;
    max_contours: number;
    max_natural_features: number;
    max_labels: number;
  };
  complete: boolean;
  degradation: Array<{
    kind: string;
    omitted_count: number;
    reason: string;
  }>;
  omissions: Array<{
    kind: string;
    subject: string;
    omitted_count: number;
    reason: string;
  }>;
  lineage: Array<{ kind: string; identity: string; revision: string }>;
}

export interface TopographyPatch {
  schema: "rey.topography-patch.v1";
  patch_id: string;
  topography_revision: string;
  prior_topography_revision: string;
  workload: ContractIdentity;
  graph: ContractIdentity;
  scenario: ContractIdentity | null;
  campaign_id: string;
  execution_id: string;
  operation: ContractIdentity;
  implementation: ContractIdentity;
  provider: ContractIdentity;
  capability_snapshot_id: string;
  complete: boolean;
  seeds: Array<{
    path: string;
    state:
      "surveyed" | "surveyed_empty" | "missing" | "omitted" | "unsupported";
    source_revision: string | null;
    coordinate: CoordinateBinding | null;
    candidate_count: number;
    detail: string;
  }>;
  candidates: Array<{
    candidate_id: string;
    seed_coordinate: string;
    seed_revision: string;
    raw: string;
    relationship: string;
    duplicate: boolean;
  }>;
  resolutions: Array<{
    resolution_id: string;
    candidate: string;
    status: LocatorResolutionStatus;
    coordinate: CoordinateBinding | null;
    source_revision: string;
    complete: boolean;
    detail: string;
  }>;
  anchors: Array<{
    anchor_id: string;
    coordinate: CoordinateBinding;
    kind: "workspace" | "file" | "document" | "external_resource";
    label: string;
    source_revision: string;
  }>;
  edges: Array<{
    edge_id: string;
    source_coordinate: string;
    target_coordinate: string;
    kind: "contains" | "references";
    locator: string;
    evidence_revision: string;
  }>;
  regions: Array<{
    region_id: string;
    coordinate: string;
    state: TopographyRegionState;
    surveyed_seeds: number;
    candidate_count: number;
    detail: string;
  }>;
  coverage: TopographyCoverage;
  frontier: Array<{
    row_id: string;
    source_coordinate: string;
    locator: string;
    status: LocatorResolutionStatus;
    reason: string;
  }>;
  omissions: Array<{
    kind: string;
    subject: string;
    omitted_count: number;
    reason: string;
  }>;
  lineage: Array<{ kind: string; identity: string; revision: string }>;
  delta: {
    delta_id: string;
    source_revision: string;
    target_revision: string;
    inserted: number;
    deleted: number;
    modified: number;
  };
}

export interface WorkloadDraft {
  request: {
    request_id: string;
    workload_id: string;
    title: string;
    intent: string | null;
    proposer: "coding_harness";
    target_package: string;
  };
  source: string;
  source_digest: string;
}

export interface AttentionRow {
  row_id: string;
  action: AttentionAction;
  subject_kind: "workload" | "surface";
  subject_id: string;
  reason: string;
  readiness: AttentionReadiness;
  evidence_ids: string[];
  dependency_ids: string[];
  priority: number;
  estimated_cost_units: number;
}

export interface AttentionSummary {
  refine: number;
  retest: number;
  create: number;
  blocked: number;
  policy_excluded: number;
  workloads: number;
  surfaces: number;
  owned_surfaces: number;
  unowned_surfaces: number;
}

export interface WorkloadList {
  schema: "rey.workload-list.v7";
  catalog: CatalogDescriptor;
  workloads: WorkloadSummary[];
  drafts: WorkloadDraft[];
  attention: {
    schema: string;
    attention_id: string;
    source_snapshot_id: string;
    rows: AttentionRow[];
    summary: AttentionSummary;
  };
}

export interface AgentSummary {
  id: string;
  kind: GeneratorProvenance["kind"];
  producer: string;
  producer_revision: string;
  workload_ids: string[];
  package_sources: string[];
  scenarios_passed: number;
  scenarios_required: number;
  attention_rows: number;
}

export interface PortfolioMetrics {
  total: number;
  admitted: number;
  drafts: number;
  qualified: number;
  failing: number;
  stale: number;
  scenariosPassed: number;
  scenariosRequired: number;
  runsPassed: number;
  runsBlocked: number;
  runsPending: number;
}

export function operatorMailboxRows(portfolio: WorkloadList): AttentionRow[] {
  return portfolio.attention.rows.filter((row) => row.readiness !== "excluded");
}

export function derivePortfolioMetrics(
  portfolio: WorkloadList,
): PortfolioMetrics {
  const metrics: PortfolioMetrics = {
    total: portfolio.catalog.workload_count,
    admitted: portfolio.catalog.admitted_count,
    drafts: portfolio.catalog.draft_count,
    qualified: 0,
    failing: 0,
    stale: 0,
    scenariosPassed: 0,
    scenariosRequired: 0,
    runsPassed: 0,
    runsBlocked: 0,
    runsPending: 0,
  };

  for (const workload of portfolio.workloads) {
    if (workload.qualification === "qualified") metrics.qualified += 1;
    if (
      workload.qualification === "failing" ||
      workload.qualification === "inconclusive"
    ) {
      metrics.failing += 1;
    }
    if (workload.qualification === "stale") metrics.stale += 1;
    metrics.scenariosPassed += workload.passed;
    metrics.scenariosRequired += workload.required;
    if (workload.last_run_status === "passed") metrics.runsPassed += 1;
    else if (workload.last_run_status === "blocked") metrics.runsBlocked += 1;
    else metrics.runsPending += 1;
  }

  return metrics;
}

export function agentIdentity(
  kind: GeneratorProvenance["kind"],
  producer: string,
  producerRevision: string,
): string {
  return `${kind}:${producer}@${producerRevision}`;
}

export function deriveAgentIndex(portfolio: WorkloadList): AgentSummary[] {
  const agents = new Map<string, AgentSummary>();
  for (const workload of portfolio.workloads) {
    const generation = workload.provenance?.generation;
    if (!generation) continue;
    const id = agentIdentity(
      generation.kind,
      generation.producer,
      generation.producer_revision,
    );
    const existing = agents.get(id) ?? {
      id,
      kind: generation.kind,
      producer: generation.producer,
      producer_revision: generation.producer_revision,
      workload_ids: [],
      package_sources: [],
      scenarios_passed: 0,
      scenarios_required: 0,
      attention_rows: 0,
    };
    existing.workload_ids.push(workload.workload.id);
    if (workload.provenance?.source) {
      existing.package_sources.push(workload.provenance.source);
    }
    existing.scenarios_passed += workload.passed;
    existing.scenarios_required += workload.required;
    existing.attention_rows += workload.attention_rows;
    agents.set(id, existing);
  }
  return [...agents.values()]
    .map((agent) => ({
      ...agent,
      package_sources: [...new Set(agent.package_sources)].sort(),
      workload_ids: [...new Set(agent.workload_ids)].sort(),
    }))
    .sort(
      (left, right) =>
        left.producer.localeCompare(right.producer) ||
        left.producer_revision.localeCompare(right.producer_revision) ||
        left.kind.localeCompare(right.kind),
    );
}

export function workloadJourney(workload: WorkloadSummary): string {
  switch (workload.qualification) {
    case "untested":
      return "TEST";
    case "failing":
      return "REVISE GRAPH";
    case "inconclusive":
      return "RESTORE EVIDENCE";
    case "stale":
      return "RETEST";
    case "qualified":
      return workload.last_run_status === "passed"
        ? "RUN COMPLETE"
        : "RUN READY";
  }
}

export function scenarioPercent(passed: number, required: number): number {
  if (required === 0) return 0;
  return Math.round((Math.max(0, passed) * 100) / required);
}

export function shortDigest(digest: string | null | undefined): string {
  if (!digest) return "none";
  const value = digest.startsWith("blake3:") ? digest.slice(7) : digest;
  return value.length > 16 ? `${value.slice(0, 8)}…${value.slice(-6)}` : value;
}

export function sourceCommitUrl(
  repository: string,
  revision: string,
): string | null {
  if (!/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/i.test(revision)) return null;
  return `${repository.replace(/\/$/, "")}/commit/${revision}`;
}
