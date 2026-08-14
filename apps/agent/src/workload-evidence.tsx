import { KineticDenseTable, type KineticDenseTableColumn } from "@hifi/kinetic";
import type { ContractIdentity, WorkloadProvenance } from "./domain";
import { OBJECT_LENS_ZOOM } from "./explore/engine/camera";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";
import { workloadsStyles as styles } from "./stylex/workloads.stylex";

export type EvidenceFreshness = "fresh" | "stale";
export type DeltaAssessment = "equal" | "different" | "inconclusive";
export type ScenarioEvaluation = "passed" | "failed" | "inconclusive";
export type DirectedDeltaKind =
  "scenario_output" | "source_matches" | "topography_patch";

interface WorkloadLimits {
  max_scenarios: number;
  max_outputs_per_scenario: number;
  max_owned_surfaces: number;
  max_git_dependencies: number;
  max_required_capabilities: number;
  max_string_bytes: number;
  scenario_delta: ScenarioDeltaLimits;
}

interface ScenarioDeltaLimits {
  max_value_bytes: number;
  max_lines: number;
  max_alignment_cells: number;
  max_changes: number;
  max_string_bytes: number;
}

interface EvidenceCurrent {
  workload: ContractIdentity;
  graph: ContractIdentity;
  scenario_suite: ContractIdentity;
  evaluator: ContractIdentity;
  source: WorkloadProvenance;
  limits: WorkloadLimits;
}

interface EvidenceResultReference {
  result_id: string;
  campaign_id: string;
  workload: ContractIdentity;
  graph: ContractIdentity;
  scenario_suite: ContractIdentity;
  evaluator: ContractIdentity;
  status: "passed" | "failed" | "inconclusive";
  stop_reason: string;
}

export interface DeltaReference {
  kind: DirectedDeltaKind;
  delta_id: string;
  label: string;
  assessment: DeltaAssessment | null;
  route: string;
}

export interface ScenarioReference {
  scenario: ContractIdentity;
  required: boolean;
  execution_id: string;
  evaluation: ScenarioEvaluation;
  route: string;
  deltas: DeltaReference[];
}

export interface WorkloadEvidenceIndex {
  schema: "rey.ui-workload-evidence-index.v1";
  authority: string;
  workload_id: string;
  availability: "retained" | "absent";
  freshness: EvidenceFreshness | null;
  source_binding:
    | "exact_current"
    | "current_source_not_bound_to_retained_result"
    | "no_retained_result";
  current: EvidenceCurrent;
  result: EvidenceResultReference | null;
  scenarios: ScenarioReference[];
}

export interface WorkloadEvidenceCatalog {
  schema: "rey.ui-workload-evidence-catalog.v1";
  authority: string;
  workloads: WorkloadEvidenceIndex[];
}

interface TextLine {
  kind: "context" | "delete" | "insert";
  source_line: number | null;
  target_line: number | null;
  text: string;
}

interface TextHunk {
  source_start_line: number;
  source_line_count: number;
  target_start_line: number;
  target_line_count: number;
  lines: TextLine[];
}

interface TextDelta {
  schema: "rey.text-delta.v1";
  delta_id: string;
  inputs: {
    source_artifact_id: string;
    target_artifact_id: string;
    source_label: string;
    target_label: string;
    comparator: ContractIdentity;
    encoding: string;
    segmentation: string;
  };
  assessment: DeltaAssessment;
  source_line_count: number;
  target_line_count: number;
  source_final_newline: boolean;
  target_final_newline: boolean;
  hunks: TextHunk[];
  limits: {
    max_input_bytes: number;
    max_lines: number;
    max_alignment_cells: number;
    max_changes: number;
    max_string_bytes: number;
  };
}

interface ScenarioOutputDelta {
  schema: "rey.scenario-output-delta.v1";
  delta_id: string;
  inputs: {
    workload: ContractIdentity;
    graph: ContractIdentity;
    scenario: ContractIdentity;
    output_id: string;
    comparator: ContractIdentity;
  };
  value_type: "utf8";
  expected: string;
  observed: string;
  assessment: DeltaAssessment;
  text_delta: TextDelta;
  limits: ScenarioDeltaLimits;
}

interface ObservedSourceMatch {
  path_display: string;
  source_artifact_id: string;
  match_id: string;
  start_line: number;
  start_byte_in_line: number;
  end_line: number;
  end_byte_in_line: number;
  matched_text: string;
  context_artifact_id: string;
  context_text: string;
  context_ref: string;
}

interface SourceMatchDelta {
  schema: "rey.source-match-delta.v1";
  delta_id: string;
  inputs: {
    workload: ContractIdentity;
    graph: ContractIdentity;
    scenario: ContractIdentity;
    comparator: ContractIdentity;
    binding_id: string;
    mining_request_id: string;
    mining_result_id: string;
  };
  expected_relation_id: string;
  observed_relation_id: string | null;
  completeness: string;
  assessment: DeltaAssessment;
  summary: {
    expected_rows: number;
    observed_rows: number;
    equal_rows: number;
    inserted: number;
    deleted: number;
    modified: number;
  };
  observed: ObservedSourceMatch[];
  changes: Array<{
    kind: string;
    changed_fields: string[];
    expected: { matched_text: string } | null;
    observed: ObservedSourceMatch | null;
  }>;
  limits: {
    max_expected_rows: number;
    max_observed_rows: number;
    max_changes: number;
    max_string_bytes: number;
  };
}

interface MiningScenarioEvidence {
  relation_delta: SourceMatchDelta;
  execution: {
    corpus: { binding_id: string; root_id: string; total_bytes: number };
    request: {
      request_id: string;
      capability_snapshot_id: string;
      effective_limits: Record<string, number>;
    };
    evidence: {
      result: {
        result_id: string;
        operation: ContractIdentity;
        provider: ContractIdentity;
        capability_snapshot_id: string;
        completeness: string;
        omissions: Array<{
          kind: string;
          subject_id: string | null;
          omitted_count: number;
          reason: string;
        }>;
        consumption: {
          files: number;
          matches: number;
          rows: number;
          bytes_read: number;
        };
        lineage: Array<{
          kind: string;
          identity: ContractIdentity;
          execution_id: string | null;
        }>;
      };
    };
  };
}

interface TopographyPatch {
  workload: ContractIdentity;
  graph: ContractIdentity;
  scenario: ContractIdentity | null;
  patch_id: string;
  topography_revision: string;
  prior_topography_revision: string;
  campaign_id: string;
  execution_id: string;
  operation: ContractIdentity;
  implementation: ContractIdentity;
  provider: ContractIdentity;
  capability_snapshot_id: string;
  complete: boolean;
  coverage: {
    requested_seeds: number;
    surveyed_seeds: number;
    surveyed_empty_seeds: number;
    missing_seeds: number;
    omitted_seeds: number;
    candidates: number;
    unique_candidates: number;
    resolved_candidates: number;
    unresolved_candidates: number;
  };
  seeds: Array<{
    path: string;
    state: string;
    source_revision: string | null;
    candidate_count: number;
    detail: string;
  }>;
  resolutions: Array<{
    resolution_id: string;
    candidate: string;
    status: string;
    coordinate: {
      binding_id: string;
      coordinate: string;
      source_revision: string;
      retention: string;
    } | null;
    provider: ContractIdentity;
    source_revision: string;
    capability_snapshot_id: string;
    limits: Record<string, number>;
    complete: boolean;
    detail: string;
  }>;
  anchors: Array<{
    anchor_id: string;
    coordinate: {
      binding_id: string;
      coordinate: string;
      source_revision: string;
      retention: string;
    };
    kind: string;
    label: string;
    source_revision: string;
  }>;
  edges: Array<{
    edge_id: string;
    source_coordinate: string;
    target_coordinate: string;
    kind: string;
    locator: string;
    evidence_revision: string;
  }>;
  regions: Array<{
    region_id: string;
    coordinate: string;
    state: string;
    surveyed_seeds: number;
    candidate_count: number;
    detail: string;
  }>;
  frontier: Array<{
    row_id: string;
    locator: string;
    status: string;
    reason: string;
  }>;
  omissions: Array<{
    kind: string;
    subject: string;
    omitted_count: number;
    reason: string;
  }>;
  lineage: Array<{ kind: string; identity: string; revision: string }>;
  limits: Record<string, number>;
  delta: {
    schema: "rey.topography-patch-delta.v1";
    delta_id: string;
    source_revision: string;
    target_revision: string;
    inserted: number;
    deleted: number;
    modified: number;
    changes: Array<{
      object_kind: string;
      object_id: string;
      kind: string;
      before: string | null;
      after: string | null;
    }>;
  };
}

export interface ScenarioResult {
  scenario: ContractIdentity;
  required: boolean;
  execution_id: string;
  evaluation: ScenarioEvaluation;
  deltas: ScenarioOutputDelta[];
  mining: MiningScenarioEvidence[];
  topography: TopographyPatch[];
  attention: unknown[];
}

export interface WorkloadScenarioEvidence {
  schema: "rey.ui-workload-scenario-evidence.v1";
  authority: string;
  freshness: EvidenceFreshness;
  source_binding: WorkloadEvidenceIndex["source_binding"];
  current: EvidenceCurrent;
  result: EvidenceResultReference;
  scenario: ScenarioResult;
  deltas: DeltaReference[];
}

type DirectedDeltaEvidence =
  | { kind: "scenario_output"; delta: ScenarioOutputDelta }
  | { kind: "source_matches"; evidence: MiningScenarioEvidence }
  | { kind: "topography_patch"; patch: TopographyPatch };

export interface WorkloadDeltaEvidence {
  schema: "rey.ui-workload-delta-evidence.v1";
  authority: string;
  freshness: EvidenceFreshness;
  source_binding: WorkloadEvidenceIndex["source_binding"];
  current: EvidenceCurrent;
  result: EvidenceResultReference;
  scenario: ContractIdentity;
  scenario_execution_id: string;
  scenario_route: string;
  delta_id: string;
  evidence: DirectedDeltaEvidence;
}

const scenarioColumns: readonly KineticDenseTableColumn<ScenarioReference>[] = [
  {
    id: "scenario",
    header: "SCENARIO / EXECUTION",
    rowHeader: true,
    width: "42%",
    render: (row) => (
      <div className={sx(styles.cellStack)}>
        <strong>
          {row.scenario.id}@{row.scenario.revision}
        </strong>
        <code className={sx(styles.breakable)}>{row.execution_id}</code>
      </div>
    ),
  },
  {
    id: "evaluation",
    header: "EVALUATION / ROLE",
    width: "18%",
    render: (row) => (
      <div className={sx(styles.cellStack)}>
        <strong>{row.evaluation.toUpperCase()}</strong>
        <span className={sx(chrome.micro)}>
          {row.required ? "REQUIRED" : "OPTIONAL"}
        </span>
      </div>
    ),
  },
  {
    id: "deltas",
    header: "DIRECTED DELTAS",
    width: "18%",
    render: (row) => (
      <div className={sx(styles.cellStack)}>
        <strong>{row.deltas.length}</strong>
        <span>{row.deltas.map((delta) => delta.label).join(" · ")}</span>
      </div>
    ),
  },
  {
    align: "right",
    id: "route",
    header: "EXACT ROUTE",
    width: "22%",
    render: (row) => (
      <EvidenceLink href={row.route}>OPEN SCENARIO →</EvidenceLink>
    ),
  },
];

export function WorkloadEvidenceIndexSection({
  evidence,
}: {
  evidence: WorkloadEvidenceIndex;
}) {
  return (
    <section
      className={sx(styles.section)}
      data-rey-section="04 / SCENARIO EVIDENCE"
    >
      <EvidenceHeading
        detail={`${evidence.availability} · ${evidence.freshness ?? "not evaluated"} · ${evidence.scenarios.length} scenarios`}
        index="04"
        kicker="PLAIN → -V → -VV"
        title="Retained scenario evidence"
      />
      <KineticDenseTable
        ariaLabel={`${evidence.workload_id} retained scenario evidence`}
        className={sx(styles.table)}
        columns={scenarioColumns}
        emptyState="NO RETAINED SCENARIO RESULT"
        getRowKey={(row) => row.execution_id}
        minWidth={920}
        rows={evidence.scenarios}
        theme="precision"
      />
      <p className={sx(chrome.micro, styles.description)}>
        {evidence.authority} · source binding{" "}
        {evidence.source_binding.replaceAll("_", " ")}
      </p>
    </section>
  );
}

export function ScenarioEvidencePage({
  evidence,
}: {
  evidence: WorkloadScenarioEvidence;
}) {
  const unresolved = evidence.scenario.deltas.filter(
    (delta) => delta.assessment !== "equal",
  );
  return (
    <main className={sx(chrome.page, styles.page)}>
      <EvidenceLink
        href={`/workloads/${encodeURIComponent(evidence.current.workload.id)}`}
      >
        ← WORKLOAD EVIDENCE
      </EvidenceLink>

      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / PLAIN"
      >
        <EvidenceHeading
          detail={`${evidence.scenario.required ? "required" : "optional"} · ${evidence.freshness}`}
          index="01"
          kicker="PLAIN / OUTCOME"
          title={`${evidence.scenario.evaluation.toUpperCase()} · ${evidence.scenario.scenario.id}`}
        />
        <div className={sx(styles.evidenceSummary)}>
          <EvidenceMetric
            label="EVALUATION"
            value={evidence.scenario.evaluation.toUpperCase()}
          />
          <EvidenceMetric
            label="ROLE"
            value={evidence.scenario.required ? "REQUIRED" : "OPTIONAL"}
          />
          <EvidenceMetric
            label="ASSERTIONS"
            value={String(
              evidence.scenario.deltas.length +
                evidence.scenario.mining.length * 2 +
                evidence.scenario.topography.length,
            )}
          />
          <EvidenceMetric
            label="UNRESOLVED OUTPUTS"
            value={String(unresolved.length)}
          />
        </div>
        {unresolved.length > 0 ? (
          <div className={sx(styles.evidenceList)}>
            {unresolved.map((delta) => (
              <OutputAssertion delta={delta} key={delta.delta_id} link />
            ))}
          </div>
        ) : (
          <p className={sx(styles.evidenceEmpty)}>
            Passing assertions are folded at the plain layer. The retained
            result remains exact below.
          </p>
        )}
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / -V ASSERTIONS"
      >
        <EvidenceHeading
          detail="every compact EXPECTED → ACTUAL assertion"
          index="02"
          kicker="-V / ASSERTIONS"
          title="Observed comparisons"
        />
        <div className={sx(styles.evidenceList)}>
          {evidence.scenario.deltas.map((delta) => (
            <OutputAssertion delta={delta} key={delta.delta_id} link />
          ))}
          {evidence.scenario.mining.map((mining) => (
            <MiningAssertion
              evidence={mining}
              key={mining.relation_delta.delta_id}
            />
          ))}
          {evidence.scenario.topography.map((patch) => (
            <TopographyAssertion key={patch.patch_id} patch={patch} />
          ))}
        </div>
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="03 / -VV EXACT EVIDENCE"
      >
        <EvidenceHeading
          detail="identity, source revision, omissions, limits, and lineage"
          index="03"
          kicker="-VV / EXACT EVIDENCE"
          title="Verified retained bindings"
        />
        <ExactRows rows={scenarioExactRows(evidence)} />
        {evidence.scenario.deltas.map((delta) => (
          <OutputExactEvidence delta={delta} key={delta.delta_id} />
        ))}
        {evidence.scenario.mining.map((mining) => (
          <MiningExactEvidence
            evidence={mining}
            key={mining.relation_delta.delta_id}
          />
        ))}
        {evidence.scenario.topography.map((patch) => (
          <TopographyExactEvidence key={patch.patch_id} patch={patch} />
        ))}
        <p className={sx(chrome.micro, styles.description)}>
          {evidence.authority}
        </p>
      </section>
    </main>
  );
}

function OutputExactEvidence({ delta }: { delta: ScenarioOutputDelta }) {
  return (
    <div className={sx(styles.evidenceList)}>
      <ExactRows
        rows={[
          ["scenario delta schema", delta.schema],
          ["scenario delta", delta.delta_id],
          ["output", delta.inputs.output_id],
          ["comparator", contract(delta.inputs.comparator)],
          ["text delta schema", delta.text_delta.schema],
          ["text delta", delta.text_delta.delta_id],
          ["source artifact", delta.text_delta.inputs.source_artifact_id],
          ["target artifact", delta.text_delta.inputs.target_artifact_id],
          ["scenario limits", limits(delta.limits)],
          ["text limits", limits(delta.text_delta.limits)],
        ]}
      />
      <EvidenceLink href={deltaRoute(delta)}>OPEN EXACT DELTA →</EvidenceLink>
    </div>
  );
}

export function DeltaEvidencePage({
  evidence,
}: {
  evidence: WorkloadDeltaEvidence;
}) {
  const title =
    evidence.evidence.kind === "scenario_output"
      ? `output.${evidence.evidence.delta.inputs.output_id}`
      : evidence.evidence.kind === "source_matches"
        ? "source.matches"
        : "topography.patch";
  return (
    <main className={sx(chrome.page, styles.page)}>
      <EvidenceLink href={evidence.scenario_route}>
        ← SCENARIO EVIDENCE
      </EvidenceLink>
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / PLAIN"
      >
        <EvidenceHeading
          detail={`${evidence.evidence.kind.replaceAll("_", " ")} · ${evidence.freshness}`}
          index="01"
          kicker="PLAIN / DIRECTED DELTA"
          title={title}
        />
        <DeltaPlain evidence={evidence.evidence} />
      </section>
      <section
        className={sx(styles.section)}
        data-rey-section="02 / -V PROJECTION"
      >
        <EvidenceHeading
          detail="authoritative direction and retained changes"
          index="02"
          kicker="-V / PROJECTION"
          title="Source → target"
        />
        <DeltaProjection evidence={evidence.evidence} />
      </section>
      <section
        className={sx(styles.section)}
        data-rey-section="03 / -VV EXACT EVIDENCE"
      >
        <EvidenceHeading
          detail="identities, revisions, bounds, omissions, and lineage"
          index="03"
          kicker="-VV / EXACT EVIDENCE"
          title="Verified retained bindings"
        />
        <ExactRows rows={deltaEnvelopeRows(evidence)} />
        <DeltaExact evidence={evidence.evidence} />
        <p className={sx(chrome.micro, styles.description)}>
          {evidence.authority}
        </p>
      </section>
    </main>
  );
}

function DeltaPlain({ evidence }: { evidence: DirectedDeltaEvidence }) {
  if (evidence.kind === "scenario_output") {
    return <OutputAssertion delta={evidence.delta} />;
  }
  if (evidence.kind === "source_matches") {
    return <MiningAssertion evidence={evidence.evidence} />;
  }
  return <TopographyAssertion patch={evidence.patch} />;
}

function DeltaProjection({ evidence }: { evidence: DirectedDeltaEvidence }) {
  if (evidence.kind === "scenario_output") {
    const delta = evidence.delta;
    return (
      <div className={sx(styles.evidenceList)}>
        <div className={sx(styles.evidenceValues)}>
          <EvidenceValue label="EXPECTED" value={delta.expected} />
          <EvidenceValue label="ACTUAL" value={delta.observed} />
        </div>
        <TextPatch delta={delta.text_delta} />
      </div>
    );
  }
  if (evidence.kind === "source_matches") {
    const delta = evidence.evidence.relation_delta;
    return (
      <div className={sx(styles.evidenceList)}>
        {delta.changes.map((change, index) => (
          <article
            className={sx(styles.evidenceCard)}
            key={`${change.kind}:${index}`}
          >
            <strong>
              {change.kind.toUpperCase()} ·{" "}
              {change.changed_fields.join(", ") || "whole row"}
            </strong>
            <div className={sx(styles.evidenceValues)}>
              <EvidenceValue
                label="EXPECTED"
                value={change.expected?.matched_text ?? "∅"}
              />
              <EvidenceValue
                label="ACTUAL"
                value={change.observed?.matched_text ?? "∅"}
              />
            </div>
          </article>
        ))}
        {delta.observed.map((row) => (
          <SourceMatchEvidence key={row.match_id} row={row} />
        ))}
      </div>
    );
  }
  const delta = evidence.patch.delta;
  return (
    <div className={sx(styles.evidenceList)}>
      {delta.changes.map((change) => (
        <article
          className={sx(styles.evidenceCard)}
          key={`${change.object_kind}:${change.object_id}`}
        >
          <strong>
            {change.kind.toUpperCase()} · {change.object_kind}
          </strong>
          <code className={sx(styles.breakable)}>{change.object_id}</code>
          <div className={sx(styles.evidenceValues)}>
            <EvidenceValue label="SOURCE" value={change.before ?? "∅"} />
            <EvidenceValue label="TARGET" value={change.after ?? "∅"} />
          </div>
        </article>
      ))}
    </div>
  );
}

function DeltaExact({ evidence }: { evidence: DirectedDeltaEvidence }) {
  if (evidence.kind === "scenario_output") {
    const delta = evidence.delta;
    return (
      <ExactRows
        rows={[
          ["scenario delta schema", delta.schema],
          ["scenario delta", delta.delta_id],
          ["text delta schema", delta.text_delta.schema],
          ["text delta", delta.text_delta.delta_id],
          ["source artifact", delta.text_delta.inputs.source_artifact_id],
          ["target artifact", delta.text_delta.inputs.target_artifact_id],
          ["comparator", contract(delta.inputs.comparator)],
          [
            "encoding",
            `${delta.text_delta.inputs.encoding} · ${delta.text_delta.inputs.segmentation}`,
          ],
          ["scenario limits", limits(delta.limits)],
          ["text limits", limits(delta.text_delta.limits)],
        ]}
      />
    );
  }
  if (evidence.kind === "source_matches") {
    return <MiningExactEvidence evidence={evidence.evidence} />;
  }
  return <TopographyExactEvidence patch={evidence.patch} />;
}

function OutputAssertion({
  delta,
  link = false,
}: {
  delta: ScenarioOutputDelta;
  link?: boolean;
}) {
  return (
    <article className={sx(styles.evidenceCard)}>
      <div className={sx(styles.evidenceCardHeader)}>
        <strong>
          {delta.assessment === "equal"
            ? "="
            : delta.assessment === "different"
              ? "!"
              : "?"}{" "}
          output.{delta.inputs.output_id} · {delta.assessment.toUpperCase()}
        </strong>
        {link ? (
          <EvidenceLink href={deltaRoute(delta)}>OPEN DELTA →</EvidenceLink>
        ) : null}
      </div>
      <div className={sx(styles.evidenceValues)}>
        <EvidenceValue label="EXPECTED" value={delta.expected} />
        <EvidenceValue label="ACTUAL" value={delta.observed} />
      </div>
      {delta.assessment !== "equal" ? (
        <TextPatch delta={delta.text_delta} />
      ) : null}
    </article>
  );
}

function MiningAssertion({ evidence }: { evidence: MiningScenarioEvidence }) {
  const delta = evidence.relation_delta;
  const result = evidence.execution.evidence.result;
  return (
    <article className={sx(styles.evidenceCard)}>
      <div className={sx(styles.evidenceCardHeader)}>
        <strong>source.matches · {delta.assessment.toUpperCase()}</strong>
        <EvidenceLink
          href={deltaRouteFromInputs(delta.inputs.workload.id, delta.delta_id)}
        >
          OPEN DELTA →
        </EvidenceLink>
      </div>
      <span>
        EXPECTED {delta.summary.expected_rows} rows → ACTUAL{" "}
        {delta.summary.observed_rows} rows · +{delta.summary.inserted} −
        {delta.summary.deleted} ~{delta.summary.modified}
      </span>
      <span className={sx(chrome.micro)}>
        COMPLETENESS / {result.completeness.toUpperCase()} ·{" "}
        {result.consumption.files} files · {result.consumption.matches} matches
        · {result.consumption.bytes_read} bytes
      </span>
      {result.omissions.map((omission) => (
        <span
          className={sx(styles.omission)}
          key={`${omission.kind}:${omission.subject_id ?? "all"}`}
        >
          OMISSION · {omission.kind} · {omission.omitted_count} ·{" "}
          {omission.reason}
        </span>
      ))}
    </article>
  );
}

function TopographyAssertion({ patch }: { patch: TopographyPatch }) {
  return (
    <article className={sx(styles.evidenceCard)}>
      <div className={sx(styles.evidenceCardHeader)}>
        <strong>
          topography.complete · {patch.complete ? "COMPLETE" : "INCONCLUSIVE"}
        </strong>
        <EvidenceLink
          href={deltaRouteFromInputs(patch.workload.id, patch.delta.delta_id)}
        >
          OPEN DELTA →
        </EvidenceLink>
      </div>
      <span>
        {patch.delta.source_revision} → {patch.delta.target_revision}
      </span>
      <span>
        seeds {patch.coverage.surveyed_seeds}/{patch.coverage.requested_seeds} ·
        candidates {patch.coverage.resolved_candidates}/
        {patch.coverage.unique_candidates} resolved · +{patch.delta.inserted} −
        {patch.delta.deleted} ~{patch.delta.modified}
      </span>
      {patch.omissions.map((omission) => (
        <span
          className={sx(styles.omission)}
          key={`${omission.kind}:${omission.subject}`}
        >
          OMISSION · {omission.kind} · {omission.subject} ·{" "}
          {omission.omitted_count} · {omission.reason}
        </span>
      ))}
    </article>
  );
}

function MiningExactEvidence({
  evidence,
}: {
  evidence: MiningScenarioEvidence;
}) {
  const result = evidence.execution.evidence.result;
  return (
    <div className={sx(styles.evidenceList)}>
      <ExactRows
        rows={[
          ["relation", evidence.relation_delta.delta_id],
          ["expected relation", evidence.relation_delta.expected_relation_id],
          [
            "observed relation",
            evidence.relation_delta.observed_relation_id ??
              "typed empty / absent",
          ],
          ["operation", contract(result.operation)],
          ["provider", contract(result.provider)],
          ["capability", result.capability_snapshot_id],
          ["corpus", evidence.execution.corpus.binding_id],
          ["corpus root", evidence.execution.corpus.root_id],
          ["request", evidence.execution.request.request_id],
          ["result", result.result_id],
          ["relation limits", limits(evidence.relation_delta.limits)],
          [
            "mining limits",
            limits(evidence.execution.request.effective_limits),
          ],
        ]}
      />
      {evidence.relation_delta.observed.map((row) => (
        <SourceMatchEvidence key={row.match_id} row={row} />
      ))}
      {result.omissions.map((omission) => (
        <article
          className={sx(styles.evidenceCard, styles.omission)}
          key={`${omission.kind}:${omission.subject_id ?? "all"}`}
        >
          <strong>OMISSION / {omission.kind.toUpperCase()}</strong>
          <span>
            {omission.omitted_count} omitted · {omission.reason}
          </span>
        </article>
      ))}
      {result.lineage.map((lineage, index) => (
        <article
          className={sx(styles.evidenceCard)}
          key={`${lineage.kind}:${index}`}
        >
          <strong>LINEAGE / {lineage.kind.toUpperCase()}</strong>
          <code>{contract(lineage.identity)}</code>
          <code>{lineage.execution_id ?? "no execution identity"}</code>
        </article>
      ))}
    </div>
  );
}

function TopographyExactEvidence({ patch }: { patch: TopographyPatch }) {
  return (
    <div className={sx(styles.evidenceList)}>
      <ExactRows
        rows={[
          ["patch", patch.patch_id],
          ["topography", patch.topography_revision],
          ["prior", patch.prior_topography_revision],
          ["delta", patch.delta.delta_id],
          ["campaign", patch.campaign_id],
          ["execution", patch.execution_id],
          ["operation", contract(patch.operation)],
          ["implementation", contract(patch.implementation)],
          ["provider", contract(patch.provider)],
          ["capability", patch.capability_snapshot_id],
          ["limits", limits(patch.limits)],
        ]}
      />
      {patch.seeds.map((seed) => (
        <article className={sx(styles.evidenceCard)} key={seed.path}>
          <strong>
            SEED / {seed.state.toUpperCase()} · {seed.path}
          </strong>
          <span>
            {seed.candidate_count} candidates · {seed.detail}
          </span>
          <code>{seed.source_revision ?? "source revision unavailable"}</code>
        </article>
      ))}
      {patch.resolutions.map((resolution) => (
        <article
          className={sx(styles.evidenceCard)}
          key={resolution.resolution_id}
        >
          <strong>
            LOCATOR / {resolution.status.toUpperCase()} · {resolution.candidate}
          </strong>
          {resolution.coordinate ? (
            <a
              className={sx(styles.sourceLink)}
              href={coordinateRoute(resolution.coordinate.coordinate)}
            >
              {resolution.coordinate.coordinate} →
            </a>
          ) : (
            <span>{resolution.detail}</span>
          )}
          <code>
            source {resolution.source_revision} · binding{" "}
            {resolution.coordinate?.binding_id ?? "none"}
          </code>
          <code>limits {limits(resolution.limits)}</code>
        </article>
      ))}
      {patch.anchors.map((anchor) => (
        <article className={sx(styles.evidenceCard)} key={anchor.anchor_id}>
          <strong>
            ANCHOR / {anchor.kind.toUpperCase()} · {anchor.label}
          </strong>
          <a
            className={sx(styles.sourceLink)}
            href={coordinateRoute(anchor.coordinate.coordinate)}
          >
            {anchor.coordinate.coordinate} →
          </a>
          <code>
            source {anchor.source_revision} · binding{" "}
            {anchor.coordinate.binding_id}
          </code>
        </article>
      ))}
      {patch.edges.map((edge) => (
        <article className={sx(styles.evidenceCard)} key={edge.edge_id}>
          <strong>
            EDGE / {edge.kind.toUpperCase()} · {edge.locator}
          </strong>
          <code>
            {edge.source_coordinate} → {edge.target_coordinate}
          </code>
          <code>evidence {edge.evidence_revision}</code>
        </article>
      ))}
      {patch.regions.map((region) => (
        <article className={sx(styles.evidenceCard)} key={region.region_id}>
          <strong>REGION / {region.state.toUpperCase()}</strong>
          <a
            className={sx(styles.sourceLink)}
            href={coordinateRoute(region.coordinate)}
          >
            {region.coordinate} →
          </a>
          <span>
            {region.surveyed_seeds} seeds · {region.candidate_count} candidates
            · {region.detail}
          </span>
        </article>
      ))}
      {patch.frontier.map((row) => (
        <article className={sx(styles.evidenceCard)} key={row.row_id}>
          <strong>FRONTIER / {row.status.toUpperCase()}</strong>
          <code>{row.locator}</code>
          <span>{row.reason}</span>
        </article>
      ))}
      {patch.omissions.map((omission) => (
        <article
          className={sx(styles.evidenceCard, styles.omission)}
          key={`${omission.kind}:${omission.subject}`}
        >
          <strong>OMISSION / {omission.kind.toUpperCase()}</strong>
          <span>
            {omission.subject} · {omission.omitted_count} · {omission.reason}
          </span>
        </article>
      ))}
      {patch.lineage.map((lineage, index) => (
        <article
          className={sx(styles.evidenceCard)}
          key={`${lineage.kind}:${index}`}
        >
          <strong>LINEAGE / {lineage.kind.toUpperCase()}</strong>
          <code>{lineage.identity}</code>
          <code>{lineage.revision}</code>
        </article>
      ))}
    </div>
  );
}

function SourceMatchEvidence({ row }: { row: ObservedSourceMatch }) {
  return (
    <article className={sx(styles.evidenceCard)}>
      <div className={sx(styles.evidenceCardHeader)}>
        <strong>
          {row.path_display}:{row.start_line}:{row.start_byte_in_line}-
          {row.end_byte_in_line}
        </strong>
        <a className={sx(styles.sourceLink)} href={row.context_ref}>
          EXACT SOURCE CONTEXT →
        </a>
      </div>
      <code>{row.matched_text}</code>
      <pre className={sx(styles.patch)}>{row.context_text}</pre>
      <code className={sx(styles.breakable)}>
        source {row.source_artifact_id} · match {row.match_id} · context{" "}
        {row.context_artifact_id}
      </code>
    </article>
  );
}

function TextPatch({ delta }: { delta: TextDelta }) {
  if (delta.hunks.length === 0)
    return (
      <pre className={sx(styles.patch)}>
        NO CHANGED LINES · {delta.assessment.toUpperCase()}
      </pre>
    );
  return (
    <pre className={sx(styles.patch)}>
      {delta.hunks
        .flatMap((hunk) => [
          `@@ -${hunk.source_start_line},${hunk.source_line_count} +${hunk.target_start_line},${hunk.target_line_count} @@\n`,
          ...hunk.lines.map(
            (line) =>
              `${line.kind === "delete" ? "-" : line.kind === "insert" ? "+" : " "} ${line.text.replace(/\n$/, "")}\n`,
          ),
        ])
        .join("")}
    </pre>
  );
}

function ExactRows({ rows }: { rows: Array<[string, string]> }) {
  return (
    <dl className={sx(styles.exactRows)}>
      {rows.map(([label, value], index) => (
        <div className={sx(styles.exactRow)} key={`${label}:${index}`}>
          <dt className={sx(chrome.micro)}>{label.toUpperCase()}</dt>
          <dd className={sx(styles.exactValue)}>
            <code>{value}</code>
          </dd>
        </div>
      ))}
    </dl>
  );
}

function EvidenceMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className={sx(styles.evidenceMetric)}>
      <span className={sx(chrome.micro)}>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function EvidenceValue({ label, value }: { label: string; value: string }) {
  return (
    <pre className={sx(styles.evidenceValue)}>
      <span className={sx(chrome.micro)}>{label}</span>
      {value}
    </pre>
  );
}

function EvidenceLink({ children, href }: { children: string; href: string }) {
  return (
    <a className={sx(styles.location)} href={href}>
      {children}
    </a>
  );
}

function EvidenceHeading({
  detail,
  index,
  kicker,
  title,
}: {
  detail: string;
  index: string;
  kicker: string;
  title: string;
}) {
  return (
    <header className={sx(styles.sectionHeading)}>
      <span className={sx(styles.sectionIndex)}>{index}</span>
      <div>
        <p className={sx(chrome.micro, styles.kicker)}>{kicker}</p>
        <h2>{title}</h2>
      </div>
      <small className={sx(chrome.micro)}>{detail}</small>
    </header>
  );
}

function scenarioExactRows(
  evidence: WorkloadScenarioEvidence,
): Array<[string, string]> {
  return [
    ["workload", contract(evidence.result.workload)],
    ["graph", contract(evidence.result.graph)],
    ["suite", contract(evidence.result.scenario_suite)],
    ["evaluator", contract(evidence.result.evaluator)],
    ["scenario", contract(evidence.scenario.scenario)],
    ["execution", evidence.scenario.execution_id],
    ["test result", evidence.result.result_id],
    ["campaign", evidence.result.campaign_id],
    ["source", evidence.current.source.source],
    [
      "source revision",
      evidence.current.source.source_digest ?? "not content identified",
    ],
    ["source binding", evidence.source_binding.replaceAll("_", " ")],
    ["current workload limits", limits(evidence.current.limits)],
  ];
}

function deltaEnvelopeRows(
  evidence: WorkloadDeltaEvidence,
): Array<[string, string]> {
  return [
    ["workload", contract(evidence.result.workload)],
    ["graph", contract(evidence.result.graph)],
    ["suite", contract(evidence.result.scenario_suite)],
    ["evaluator", contract(evidence.result.evaluator)],
    ["scenario", contract(evidence.scenario)],
    ["execution", evidence.scenario_execution_id],
    ["delta", evidence.delta_id],
    ["test result", evidence.result.result_id],
    ["campaign", evidence.result.campaign_id],
    ["source", evidence.current.source.source],
    [
      "source revision",
      evidence.current.source.source_digest ?? "not content identified",
    ],
    ["source binding", evidence.source_binding.replaceAll("_", " ")],
  ];
}

function contract(identity: ContractIdentity): string {
  return `${identity.id}@${identity.revision} · ${identity.semantic_digest}`;
}

function limits(
  value: Record<string, number> | ScenarioDeltaLimits | WorkloadLimits,
): string {
  return Object.entries(value)
    .flatMap(([key, item]) =>
      typeof item === "object"
        ? Object.entries(item).map(
            ([nestedKey, nestedValue]) => `${key}.${nestedKey}=${nestedValue}`,
          )
        : [`${key}=${item}`],
    )
    .join(" · ");
}

function deltaRoute(delta: ScenarioOutputDelta): string {
  return deltaRouteFromInputs(delta.inputs.workload.id, delta.delta_id);
}

function deltaRouteFromInputs(workloadId: string, deltaId: string): string {
  return `/workloads/${encodeURIComponent(workloadId)}/deltas/${encodeURIComponent(deltaId)}`;
}

function coordinateRoute(coordinate: string): string {
  return `/explore?coordinate=${encodeURIComponent(coordinate)}&scale=${OBJECT_LENS_ZOOM}`;
}
