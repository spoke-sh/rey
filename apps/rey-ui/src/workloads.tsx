import { KineticDenseTable, type KineticDenseTableColumn } from "@hifi/kinetic";
import { useState } from "react";
import { admitWorkloadFiles } from "./api";
import {
  derivePortfolioMetrics,
  scenarioPercent,
  shortDigest,
  workloadJourney,
  type WorkloadDraft,
  type WorkloadList,
  type WorkloadPackageSnapshot,
  type WorkloadRevisionStatus,
  type WorkloadSummary,
} from "./domain";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";
import { workloadsStyles as styles } from "./stylex/workloads.stylex";
import {
  WorkloadEvidenceIndexSection,
  type WorkloadEvidenceIndex,
} from "./workload-evidence";

const admittedColumns: readonly KineticDenseTableColumn<WorkloadSummary>[] = [
  {
    id: "workload",
    header: "WORKLOAD / REVISION",
    rowHeader: true,
    width: "22%",
    render: (workload, index) => (
      <div className={sx(styles.identity)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div className={sx(styles.identityDetail)}>
          <strong>{workload.workload.id}</strong>
          <p className={sx(styles.description)}>{workload.title}</p>
          <span className={sx(chrome.micro)}>
            R{workload.workload.revision} ·{" "}
            {workload.provenance?.origin.replaceAll("_", " ") ?? "local"}
          </span>
        </div>
      </div>
    ),
  },
  {
    id: "journey",
    header: "JOURNEY / STATE",
    width: "14%",
    render: (workload) => (
      <div className={sx(styles.cellStack)}>
        <strong>{workloadJourney(workload)}</strong>
        <span className={sx(chrome.micro)}>
          {workload.qualification.toUpperCase()} ·{" "}
          {workload.freshness.toUpperCase()}
        </span>
        <span className={sx(styles.secondary)}>
          RUN / {workload.last_run_status ?? "not run"}
        </span>
      </div>
    ),
  },
  {
    id: "conformance",
    header: "LOCAL CONFORMANCE",
    width: "18%",
    render: (workload) => <ConformanceCell workload={workload} />,
  },
  {
    id: "graph",
    header: "GRAPH / EVIDENCE",
    width: "20%",
    render: (workload) => (
      <div className={sx(styles.cellStack)}>
        <strong>
          {workload.candidate_graph.id}@{workload.candidate_graph.revision}
        </strong>
        <code title={workload.candidate_graph.semantic_digest}>
          GRAPH / {shortDigest(workload.candidate_graph.semantic_digest)}
        </code>
        <code title={workload.last_test_result_id ?? undefined}>
          TEST / {shortDigest(workload.last_test_result_id)}
        </code>
      </div>
    ),
  },
  {
    id: "mining",
    header: "MINING / ATTENTION",
    width: "16%",
    render: (workload) => (
      <div className={sx(styles.cellStack)}>
        <strong>
          {workload.mining_results}/{workload.mining_operations} RESULTS
        </strong>
        <span>
          {workload.relation_deltas} deltas · {workload.reasoning_surfaces}{" "}
          surfaces
        </span>
        <span className={sx(chrome.micro)}>
          {workload.incomplete_mining_results} incomplete ·{" "}
          {workload.attention_rows} attention
        </span>
      </div>
    ),
  },
  {
    align: "right",
    id: "location",
    header: "LOCATION",
    width: "10%",
    render: (workload) => (
      <WorkloadLink id={workload.workload.id}>INSPECT EVIDENCE →</WorkloadLink>
    ),
  },
];

const draftColumns: readonly KineticDenseTableColumn<WorkloadDraft>[] = [
  {
    id: "workload",
    header: "WORKLOAD / REQUEST",
    rowHeader: true,
    width: "21%",
    render: (draft, index) => (
      <div className={sx(styles.identity)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div className={sx(styles.identityDetail)}>
          <strong>{draft.request.workload_id}</strong>
          <span className={sx(chrome.micro)}>DRAFT · CODING HARNESS</span>
          <code title={draft.request.request_id}>
            {shortDigest(draft.request.request_id)}
          </code>
        </div>
      </div>
    ),
  },
  {
    id: "intent",
    header: "INTENT",
    width: "27%",
    render: (draft) => (
      <div className={sx(styles.cellStack)}>
        <strong>{draft.request.title}</strong>
        <p className={sx(styles.description)}>
          {draft.request.intent ?? "No additional intent supplied"}
        </p>
      </div>
    ),
  },
  {
    id: "admission",
    header: "ADMISSION",
    width: "16%",
    render: () => (
      <div className={sx(styles.cellStack)}>
        <strong>AWAITING HARNESS</strong>
        <span>graph missing</span>
        <span className={sx(chrome.micro)}>ORACLE / NOT ADMITTED</span>
      </div>
    ),
  },
  {
    id: "target",
    header: "TARGET PACKAGE",
    width: "18%",
    render: (draft) => (
      <code className={sx(styles.breakable)}>
        {draft.request.target_package}
      </code>
    ),
  },
  {
    id: "source",
    header: "REQUEST SOURCE",
    width: "12%",
    render: (draft) => (
      <div className={sx(styles.cellStack)}>
        <code className={sx(styles.breakable)}>{draft.source}</code>
        <code title={draft.source_digest}>
          {shortDigest(draft.source_digest)}
        </code>
      </div>
    ),
  },
  {
    align: "right",
    id: "location",
    header: "LOCATION",
    width: "10%",
    render: (draft) => (
      <WorkloadLink id={draft.request.workload_id}>
        INSPECT HANDOFF →
      </WorkloadLink>
    ),
  },
];

const runtimeColumns: readonly KineticDenseTableColumn<WorkloadSummary>[] = [
  {
    id: "workload",
    header: "WORKLOAD / REVISION",
    rowHeader: true,
    width: "23%",
    render: (workload) => (
      <div className={sx(styles.cellStack)}>
        <strong>{workload.workload.id}</strong>
        <span>{workload.title}</span>
        <code title={workload.workload.semantic_digest}>
          R{workload.workload.revision} ·{" "}
          {shortDigest(workload.workload.semantic_digest)}
        </code>
      </div>
    ),
  },
  {
    id: "journey",
    header: "JOURNEY / STATE",
    width: "16%",
    render: (workload) => (
      <div className={sx(styles.cellStack)}>
        <strong>{workloadJourney(workload)}</strong>
        <span className={sx(chrome.micro)}>
          {workload.qualification.toUpperCase()} ·{" "}
          {workload.freshness.toUpperCase()}
        </span>
      </div>
    ),
  },
  {
    id: "conformance",
    header: "LOCAL CONFORMANCE",
    width: "25%",
    render: (workload) => <ConformanceCell workload={workload} />,
  },
  {
    id: "outcomes",
    header: "SCENARIO OUTCOMES",
    width: "21%",
    render: (workload) => (
      <div className={sx(styles.outcomes)}>
        <strong>{workload.passed} PASSING</strong>
        <span>{workload.failed} failing</span>
        <span>{workload.inconclusive} inconclusive</span>
        <span>{workload.stale} stale</span>
        <span>{workload.optional} optional</span>
      </div>
    ),
  },
  {
    id: "run",
    header: "RUN / ATTENTION",
    width: "15%",
    render: (workload) => (
      <div className={sx(styles.cellStack)}>
        <strong>{(workload.last_run_status ?? "NOT RUN").toUpperCase()}</strong>
        <span>{workload.attention_rows} attention rows</span>
        <span className={sx(chrome.micro)}>
          {workload.evaluated}/{workload.required} EVALUATED
        </span>
      </div>
    ),
  },
];

interface BindingRow {
  contract: string;
  identity: string;
  locator: string;
  object: string;
  revision: string;
}

const bindingColumns: readonly KineticDenseTableColumn<BindingRow>[] = [
  {
    id: "object",
    header: "OBJECT",
    rowHeader: true,
    width: "15%",
    render: (row) => <strong>{row.object}</strong>,
  },
  {
    id: "locator",
    header: "LOCATOR",
    width: "30%",
    render: (row) => (
      <code className={sx(styles.breakable)}>{row.locator}</code>
    ),
  },
  {
    id: "revision",
    header: "REVISION",
    width: "12%",
    render: (row) => <strong>{row.revision}</strong>,
  },
  {
    id: "identity",
    header: "CONTENT IDENTITY",
    width: "28%",
    render: (row) => (
      <code className={sx(styles.breakable)}>{row.identity}</code>
    ),
  },
  {
    id: "contract",
    header: "CONTRACT",
    width: "15%",
    render: (row) => <span className={sx(chrome.micro)}>{row.contract}</span>,
  },
];

const miningColumns: readonly KineticDenseTableColumn<WorkloadSummary>[] = [
  metricColumn("operations", "OPERATIONS", (workload) =>
    String(workload.mining_operations),
  ),
  metricColumn("results", "RESULTS", (workload) =>
    String(workload.mining_results),
  ),
  metricColumn("incomplete", "INCOMPLETE", (workload) =>
    String(workload.incomplete_mining_results),
  ),
  metricColumn("deltas", "DELTAS", (workload) =>
    String(workload.relation_deltas),
  ),
  metricColumn("surfaces", "SURFACES", (workload) =>
    String(workload.reasoning_surfaces),
  ),
  metricColumn("attention", "ATTENTION", (workload) =>
    String(workload.attention_rows),
  ),
];

interface RequestBindingRow {
  contract: string;
  field: string;
  value: string;
}

const requestBindingColumns: readonly KineticDenseTableColumn<RequestBindingRow>[] =
  [
    {
      id: "field",
      header: "FIELD",
      rowHeader: true,
      width: "18%",
      render: (row) => <strong>{row.field}</strong>,
    },
    {
      id: "value",
      header: "EXACT VALUE",
      width: "62%",
      render: (row) => (
        <code className={sx(styles.breakable)}>{row.value}</code>
      ),
    },
    {
      id: "contract",
      header: "CONTRACT",
      width: "20%",
      render: (row) => <span className={sx(chrome.micro)}>{row.contract}</span>,
    },
  ];

const requestPostureColumns: readonly KineticDenseTableColumn<WorkloadDraft>[] =
  [
    {
      id: "workload",
      header: "WORKLOAD / REQUEST",
      rowHeader: true,
      width: "28%",
      render: (draft) => (
        <div className={sx(styles.cellStack)}>
          <strong>{draft.request.workload_id}</strong>
          <code>{draft.request.request_id}</code>
        </div>
      ),
    },
    {
      id: "journey",
      header: "JOURNEY",
      width: "14%",
      render: () => <strong>HYDRATE</strong>,
    },
    {
      id: "admission",
      header: "ADMISSION",
      width: "22%",
      render: () => (
        <div className={sx(styles.cellStack)}>
          <strong>AWAITING CODING HARNESS</strong>
          <span className={sx(chrome.micro)}>PROPOSAL / RETAINED</span>
        </div>
      ),
    },
    {
      id: "graph",
      header: "GRAPH",
      width: "18%",
      render: () => <strong>MISSING</strong>,
    },
    {
      id: "oracle",
      header: "SCENARIO ORACLE",
      width: "18%",
      render: () => <strong>NOT ADMITTED</strong>,
    },
  ];

export interface AdmissionCandidateRow {
  package: WorkloadPackageSnapshot;
  plane: "INDEX" | "WORKING";
  ready: boolean;
}

export function admissionCandidates(
  revision: WorkloadRevisionStatus | undefined,
): AdmissionCandidateRow[] {
  if (!revision) return [];
  const rows: AdmissionCandidateRow[] = (revision.index?.packages ?? []).map(
    (candidate) => ({
      package: candidate,
      plane: "INDEX" as const,
      ready: revision.commit_ready,
    }),
  );
  for (const change of revision.unstaged.changes) {
    const candidate = revision.working.packages.find(
      (item) => item.workload_id === change.workload_id,
    );
    if (candidate)
      rows.push({ package: candidate, plane: "WORKING", ready: false });
  }
  return rows;
}

const candidateColumns: readonly KineticDenseTableColumn<AdmissionCandidateRow>[] =
  [
    {
      id: "workload",
      header: "WORKLOAD / REVISION",
      rowHeader: true,
      width: "25%",
      render: (row, index) => (
        <div className={sx(styles.identity)}>
          <span className={sx(styles.ordinal)}>
            {String(index + 1).padStart(2, "0")}
          </span>
          <div className={sx(styles.identityDetail)}>
            <strong>{row.package.workload_id}</strong>
            <p className={sx(styles.description)}>{row.package.title}</p>
            <span className={sx(chrome.micro)}>
              R{row.package.workload_revision} / {row.plane}
            </span>
          </div>
        </div>
      ),
    },
    {
      id: "admission",
      header: "ADMISSION",
      width: "18%",
      render: (row) => (
        <div className={sx(styles.cellStack)}>
          <strong>
            {row.ready
              ? "AWAITING HUMAN APPROVAL"
              : row.plane === "WORKING"
                ? "READY FOR FILE REVIEW"
                : "INDEX"}
          </strong>
          <span className={sx(chrome.micro)}>
            {row.ready
              ? "QUALIFIED / FROZEN"
              : row.plane === "WORKING"
                ? "QUALIFIES ON ADMISSION"
                : "NOT ADMITTED"}
          </span>
          <span className={sx(styles.secondary)}>
            {row.package.generation.kind.replaceAll("_", " ")} /{" "}
            {row.package.generation.producer}@
            {row.package.generation.producer_revision}
          </span>
        </div>
      ),
    },
    {
      id: "package",
      header: "EXACT PACKAGE",
      width: "25%",
      render: (row) => (
        <div className={sx(styles.cellStack)}>
          <code title={row.package.source_digest}>
            {shortDigest(row.package.source_digest)}
          </code>
          <span className={sx(styles.secondary)}>{row.package.source}</span>
        </div>
      ),
    },
    {
      id: "graph",
      header: "GRAPH",
      width: "16%",
      render: (row) => (
        <code title={row.package.graph.semantic_digest}>
          {row.package.graph.id}@{row.package.graph.revision}
        </code>
      ),
    },
    {
      id: "oracle",
      header: "SCENARIO ORACLE",
      width: "13%",
      render: (row) => (
        <code title={row.package.scenario_suite.semantic_digest}>
          {row.package.scenario_suite.id}@{row.package.scenario_suite.revision}
        </code>
      ),
    },
    {
      align: "right",
      id: "location",
      header: "LOCATION",
      width: "9%",
      render: (row) => (
        <WorkloadLink id={row.package.workload_id}>REVIEW →</WorkloadLink>
      ),
    },
  ];

export function WorkloadsPage({ portfolio }: { portfolio: WorkloadList }) {
  const metrics = derivePortfolioMetrics(portfolio);
  const candidates = admissionCandidates(portfolio.revision);
  const attentionRows = portfolio.workloads.reduce(
    (total, workload) => total + workload.attention_rows,
    0,
  );
  return (
    <main className={sx(chrome.page, styles.page)}>
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / ADMISSION"
      >
        <WorkloadHeading
          detail={`${candidates.length} incoming · ${portfolio.revision?.index?.packages.length ?? 0} staged · ${portfolio.revision?.commit_ready ? "approval ready" : candidates.length > 0 ? "file review ready" : "quiet"}`}
          index="01"
          kicker="ADMISSION"
          title="Incoming workload revisions"
        />
        <KineticDenseTable
          ariaLabel="Incoming workload revisions"
          className={sx(styles.table)}
          columns={candidateColumns}
          emptyState="NO WORKLOAD REVISION IS WAITING FOR ADMISSION"
          getRowClassName={(row) =>
            sx(styles.row, row.plane === "WORKING" && styles.draftRow)
          }
          getRowKey={(row) => `${row.plane}:${row.package.workload_id}`}
          minWidth={1100}
          rows={candidates}
          theme="precision"
        />
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / EXECUTABLE"
      >
        <WorkloadHeading
          detail={`${metrics.admitted} admitted · ${metrics.qualified} qualified · ${metrics.failing} failing · ${metrics.stale} stale · ${attentionRows} attention`}
          index="02"
          kicker="EXECUTABLE"
          title="Admitted workload HEAD"
        />
        <KineticDenseTable
          ariaLabel="Admitted workload revisions"
          className={sx(styles.table)}
          columns={admittedColumns}
          emptyState="NO WORKLOAD PACKAGES HAVE BEEN ADMITTED"
          getRowClassName={(workload) =>
            sx(
              styles.row,
              (workload.qualification === "failing" ||
                workload.qualification === "inconclusive") &&
                styles.rowFailure,
              workload.qualification === "stale" && styles.rowStale,
            )
          }
          getRowKey={(workload) => workload.workload.id}
          minWidth={1180}
          rows={portfolio.workloads}
          theme="precision"
        />
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="03 / AGENTIC HANDOFF"
      >
        <WorkloadHeading
          detail={`${metrics.drafts} requested · ${portfolio.attention.summary.create} create attention · ${portfolio.catalog.root ?? "workspace catalog"}`}
          index="03"
          kicker="AGENTIC HANDOFF"
          title="Creation requests"
        />
        <KineticDenseTable
          ariaLabel="Workload creation requests"
          className={sx(styles.table)}
          columns={draftColumns}
          emptyState="NO WORKLOADS AWAITING CODING HARNESS"
          getRowClassName={() => sx(styles.row, styles.draftRow)}
          getRowKey={(draft) => draft.request.workload_id}
          minWidth={1080}
          rows={portfolio.drafts}
          theme="precision"
        />
      </section>
    </main>
  );
}

function WorkloadAdmissionControl({
  revision,
}: {
  revision: WorkloadRevisionStatus;
}) {
  const [message, setMessage] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const working = revision.working;
  const hasPendingFiles =
    working.packages.length > 0 &&
    revision.head?.snapshot.snapshot_revision !== working.snapshot_revision;
  const enabled = hasPendingFiles && message.trim().length > 0;

  const approve = async () => {
    if (!enabled) return;
    setSubmitting(true);
    setError(null);
    try {
      await admitWorkloadFiles({
        message: message.trim(),
        expected_head: revision.head?.commit_id ?? "EMPTY",
        expected_working: working.snapshot_revision,
      });
      window.location.reload();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setSubmitting(false);
    }
  };

  return (
    <aside
      aria-label="Exact workload snapshot approval"
      className={sx(styles.admissionControl)}
    >
      <div className={sx(styles.admissionControlCopy)}>
        <span className={sx(chrome.micro)}>EXACT SNAPSHOT APPROVAL</span>
        <strong>
          {working.packages.length} FILE PACKAGE
          {working.packages.length === 1 ? "" : "S"}
        </strong>
        <p>{revision.admission_boundary}</p>
        <code title={working.snapshot_revision}>
          WORKING / {shortDigest(working.snapshot_revision)}
        </code>
      </div>
      <div className={sx(styles.admissionControlAction)}>
        <input
          aria-label="Workload approval message"
          className={sx(styles.admissionMessage)}
          disabled={submitting || !hasPendingFiles}
          maxLength={4096}
          onChange={(event) => setMessage(event.target.value)}
          placeholder="Why are you admitting this workload revision?"
          value={message}
        />
        <button
          className={sx(styles.admissionApprove)}
          disabled={!enabled || submitting}
          onClick={() => void approve()}
          type="button"
        >
          {submitting ? "QUALIFYING & ADMITTING…" : "ADMIT EXACT FILE SNAPSHOT"}
        </button>
        {error ? <p role="alert">{error}</p> : null}
      </div>
    </aside>
  );
}

export function CandidateWorkloadDetail({
  candidate,
  revision,
}: {
  candidate: WorkloadPackageSnapshot;
  revision: WorkloadRevisionStatus;
}) {
  const indexed = revision.index?.packages.some(
    (item) =>
      item.workload_id === candidate.workload_id &&
      item.source_digest === candidate.source_digest,
  );
  const row: AdmissionCandidateRow = {
    package: candidate,
    plane: indexed ? "INDEX" : "WORKING",
    ready:
      (indexed === true && revision.commit_ready) ||
      (indexed === false &&
        revision.head?.snapshot.snapshot_revision !==
          revision.working.snapshot_revision),
  };
  return (
    <main className={sx(chrome.page, styles.page)}>
      <PortfolioLink />
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / ADMISSION POSTURE"
      >
        <WorkloadHeading
          detail={`${row.plane} · ${row.ready ? (row.plane === "WORKING" ? "ready for exact file admission" : "qualified and awaiting approval") : "not admitted"}`}
          index="01"
          kicker="ADMISSION POSTURE"
          title={candidate.workload_id}
        />
        <KineticDenseTable
          ariaLabel={`${candidate.workload_id} admission posture`}
          className={sx(styles.table)}
          columns={candidateColumns}
          getRowKey={(item) => `${item.plane}:${item.package.workload_id}`}
          minWidth={1100}
          rows={[row]}
          theme="precision"
        />
        <WorkloadAdmissionControl revision={revision} />
      </section>
    </main>
  );
}

export function AdmittedWorkloadDetail({
  evidence,
  workload,
}: {
  evidence?: WorkloadEvidenceIndex;
  workload: WorkloadSummary;
}) {
  const bindings: BindingRow[] = [
    {
      object: "WORKLOAD",
      locator: workload.workload.id,
      revision: `R${workload.workload.revision}`,
      identity: workload.workload.semantic_digest,
      contract: "ADMITTED",
    },
    {
      object: "CANDIDATE GRAPH",
      locator: workload.candidate_graph.id,
      revision: `R${workload.candidate_graph.revision}`,
      identity: workload.candidate_graph.semantic_digest,
      contract: "CANDIDATE",
    },
    {
      object: "PACKAGE",
      locator: workload.provenance?.source ?? "compiled",
      revision: "PACKAGE",
      identity: workload.provenance?.source_digest ?? "compiled",
      contract: workload.provenance
        ? `${workload.provenance.admission.state} / ${workload.provenance.admission.scenario_oracle}`
        : "BUILT IN",
    },
    {
      object: "TEST RESULT",
      locator: "local result index",
      revision: "LATEST",
      identity: workload.last_test_result_id ?? "none",
      contract: workload.last_test_result_id ? "RETAINED" : "ABSENT",
    },
  ];
  return (
    <main className={sx(chrome.page, styles.page)}>
      <PortfolioLink />
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / RUNTIME POSTURE"
      >
        <WorkloadHeading
          detail={`${workload.title} · exact current revision`}
          index="01"
          kicker="RUNTIME POSTURE"
          title={workload.workload.id}
        />
        <KineticDenseTable
          ariaLabel={`${workload.workload.id} runtime posture`}
          className={sx(styles.table)}
          columns={runtimeColumns}
          getRowKey={(row) => row.workload.id}
          minWidth={1050}
          rows={[workload]}
          theme="precision"
        />
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / EXACT BINDINGS"
      >
        <WorkloadHeading
          detail="source, graph, package, and retained test identity"
          index="02"
          kicker="REFERENCE PLANE"
          title="Exact bindings"
        />
        <KineticDenseTable
          ariaLabel={`${workload.workload.id} exact bindings`}
          className={sx(styles.table)}
          columns={bindingColumns}
          getRowKey={(row) => row.object}
          minWidth={960}
          rows={bindings}
          theme="precision"
        />
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="03 / MINING / EVIDENCE"
      >
        <WorkloadHeading
          detail="bounded output retained by the current workload revision"
          index="03"
          kicker="MINING / EVIDENCE"
          title="Artifact output"
        />
        <KineticDenseTable
          ariaLabel={`${workload.workload.id} mining evidence`}
          className={sx(styles.table, styles.metricTable)}
          columns={miningColumns}
          getRowKey={(row) => row.workload.id}
          minWidth={760}
          rows={[workload]}
          theme="precision"
        />
      </section>
      {evidence ? <WorkloadEvidenceIndexSection evidence={evidence} /> : null}
    </main>
  );
}

export function DraftWorkloadDetail({ draft }: { draft: WorkloadDraft }) {
  const bindings: RequestBindingRow[] = [
    {
      field: "PURPOSE",
      value: draft.request.title,
      contract: "AUTHORED",
    },
    {
      field: "INTENT",
      value: draft.request.intent ?? "not supplied",
      contract: draft.request.intent ? "AUTHORED" : "ABSENT",
    },
    {
      field: "REQUEST",
      value: draft.request.request_id,
      contract: "CONTENT IDENTIFIED",
    },
    { field: "SOURCE", value: draft.source, contract: "WORKSPACE BOUND" },
    {
      field: "SOURCE IDENTITY",
      value: draft.source_digest,
      contract: "EXACT",
    },
    {
      field: "TARGET PACKAGE",
      value: draft.request.target_package,
      contract: "EXPECTED OUTPUT",
    },
  ];
  return (
    <main className={sx(chrome.page, styles.page)}>
      <PortfolioLink />
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / REQUEST POSTURE"
      >
        <WorkloadHeading
          detail={`${draft.request.title} · retained creation request`}
          index="01"
          kicker="REQUEST POSTURE"
          title={draft.request.workload_id}
        />
        <KineticDenseTable
          ariaLabel={`${draft.request.workload_id} request posture`}
          className={sx(styles.table)}
          columns={requestPostureColumns}
          getRowClassName={() => sx(styles.draftRow)}
          getRowKey={(row) => row.request.workload_id}
          minWidth={920}
          rows={[draft]}
          theme="precision"
        />
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / REQUEST BINDINGS"
      >
        <WorkloadHeading
          detail="the harness must satisfy these retained request coordinates"
          index="02"
          kicker="AGENTIC HANDOFF"
          title="Request bindings"
        />
        <KineticDenseTable
          ariaLabel={`${draft.request.workload_id} request bindings`}
          className={sx(styles.table)}
          columns={requestBindingColumns}
          getRowKey={(row) => row.field}
          minWidth={820}
          rows={bindings}
          theme="precision"
        />
      </section>
    </main>
  );
}

function ConformanceCell({ workload }: { workload: WorkloadSummary }) {
  const percent = scenarioPercent(workload.passed, workload.required);
  return (
    <div className={sx(styles.conformance)}>
      <div className={sx(styles.conformanceSummary)}>
        <strong>{workload.qualification.toUpperCase()}</strong>
        <span>{percent}%</span>
      </div>
      <i className={sx(styles.progressTrack)}>
        <b
          className={sx(
            styles.progressFill,
            (workload.qualification === "failing" ||
              workload.qualification === "inconclusive") &&
              styles.progressFailure,
            workload.qualification === "stale" && styles.progressStale,
          )}
          style={{ width: `${percent}%` }}
        />
      </i>
      <span>
        {workload.passed}/{workload.required} passing
      </span>
      <span className={sx(chrome.micro)}>
        {workload.failed} failed · {workload.inconclusive} inconclusive ·{" "}
        {workload.stale} stale
      </span>
    </div>
  );
}

function WorkloadLink({ children, id }: { children: string; id: string }) {
  return (
    <a
      className={sx(styles.location)}
      href={`/workloads/${encodeURIComponent(id)}`}
    >
      {children}
    </a>
  );
}

function PortfolioLink() {
  return (
    <a className={sx(styles.portfolioLink)} href="/workloads">
      ← WORKLOAD PORTFOLIO
    </a>
  );
}

function metricColumn(
  id: string,
  header: string,
  value: (workload: WorkloadSummary) => string,
): KineticDenseTableColumn<WorkloadSummary> {
  return {
    align: "center",
    header,
    id,
    render: (workload) => (
      <strong className={sx(styles.metricValue)}>{value(workload)}</strong>
    ),
    width: "16.66%",
  };
}

function WorkloadHeading({
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
