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
  last_run_status: "passed" | "blocked" | null;
  last_test_result_id: string | null;
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
  schema: string;
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
