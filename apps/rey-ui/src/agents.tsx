import { Link } from "@tanstack/react-router";
import {
  scenarioPercent,
  shortDigest,
  workloadJourney,
  type AttentionAction,
  type AttentionReadiness,
  type WorkloadList,
} from "./domain";
import { agentsStyles as styles } from "./stylex/agents.stylex";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";
import {
  JournalCreateLink,
  JournalEntries,
  type JournalOpportunity,
  type JournalOpportunitySurface,
  type JournalProjection,
} from "./journal";

export type JournalOperation = "AUTHOR" | "REFINE" | "RESOLVE" | "TEST";

export interface DerivedJournalEntry {
  id: string;
  author: "rey";
  author_kind: "system";
  origin: "derived";
  subject_id: string;
  subject_kind: "surface" | "workload";
  operation: JournalOperation;
  profile: string;
  reason: string;
  source: "ATTENTION" | "REQUEST" | "REQUEST + ATTENTION";
  readiness: AttentionReadiness;
  priority: number;
  estimated_cost_units: number;
  evidence_count: number;
  dependency_count: number;
  workload_id: string | null;
}

export interface WorkInsight {
  id: string;
  workload_id: string;
  title: string;
  kind: "ADMITTED" | "REQUEST";
  revision: string;
  observed_operation: "ADMISSION" | "REQUEST" | "RUN" | "TEST";
  result: string;
  journey: string;
  scenarios_passed: number;
  scenarios_required: number;
  artifact_summary: string;
  attention_rows: number;
  evidence_id: string | null;
}

export function deriveJournalEntries(
  portfolio: WorkloadList,
): DerivedJournalEntry[] {
  const attentionBySubject = new Map(
    portfolio.attention.rows
      .filter((row) => row.readiness !== "excluded")
      .map((row) => [`${row.subject_kind}:${row.subject_id}`, row]),
  );
  const entries: DerivedJournalEntry[] = [];

  for (const draft of portfolio.drafts) {
    const key = `workload:${draft.request.workload_id}`;
    const attention = attentionBySubject.get(key);
    if (attention) attentionBySubject.delete(key);
    entries.push({
      id: draft.request.request_id,
      author: "rey",
      author_kind: "system",
      origin: "derived",
      subject_id: draft.request.workload_id,
      subject_kind: "workload",
      operation: "AUTHOR",
      profile: "CODING HARNESS",
      reason: draft.request.intent ?? draft.request.title,
      source: attention ? "REQUEST + ATTENTION" : "REQUEST",
      readiness: attention?.readiness ?? "ready",
      priority: attention?.priority ?? 0,
      estimated_cost_units: attention?.estimated_cost_units ?? 0,
      evidence_count: attention?.evidence_ids.length ?? 0,
      dependency_count: attention?.dependency_ids.length ?? 0,
      workload_id: draft.request.workload_id,
    });
  }

  for (const attention of attentionBySubject.values()) {
    const operation = journalOperation(attention.action);
    entries.push({
      id: attention.row_id,
      author: "rey",
      author_kind: "system",
      origin: "derived",
      subject_id: attention.subject_id,
      subject_kind: attention.subject_kind,
      operation,
      profile: journalProfile(operation),
      reason: attention.reason,
      source: "ATTENTION",
      readiness: attention.readiness,
      priority: attention.priority,
      estimated_cost_units: attention.estimated_cost_units,
      evidence_count: attention.evidence_ids.length,
      dependency_count: attention.dependency_ids.length,
      workload_id:
        attention.subject_kind === "workload" ? attention.subject_id : null,
    });
  }

  return entries.sort(
    (left, right) =>
      readinessOrder(left.readiness) - readinessOrder(right.readiness) ||
      right.priority - left.priority ||
      left.estimated_cost_units - right.estimated_cost_units ||
      left.subject_id.localeCompare(right.subject_id),
  );
}

export function deriveWorkInsights(portfolio: WorkloadList): WorkInsight[] {
  const admitted = portfolio.workloads.map((workload): WorkInsight => {
    const observedOperation = workload.last_run_status
      ? "RUN"
      : workload.last_test_result_id
        ? "TEST"
        : "ADMISSION";
    return {
      id: `workload:${workload.workload.id}`,
      workload_id: workload.workload.id,
      title: workload.title,
      kind: "ADMITTED",
      revision: `R${workload.workload.revision}`,
      observed_operation: observedOperation,
      result:
        observedOperation === "RUN"
          ? (workload.last_run_status ?? "unknown").toUpperCase()
          : workload.qualification.toUpperCase(),
      journey: workloadJourney(workload),
      scenarios_passed: workload.passed,
      scenarios_required: workload.required,
      artifact_summary: `${workload.mining_results} mining · ${workload.relation_deltas} deltas · ${workload.reasoning_surfaces} surfaces`,
      attention_rows: workload.attention_rows,
      evidence_id: workload.last_test_result_id,
    };
  });
  const requested = portfolio.drafts.map((draft): WorkInsight => ({
    id: `request:${draft.request.request_id}`,
    workload_id: draft.request.workload_id,
    title: draft.request.intent ?? draft.request.title,
    kind: "REQUEST",
    revision: "DRAFT",
    observed_operation: "REQUEST",
    result: "AWAITING AUTHOR",
    journey: "AUTHOR",
    scenarios_passed: 0,
    scenarios_required: 0,
    artifact_summary: "graph pending · scenario oracle pending",
    attention_rows: portfolio.attention.rows.filter(
      (row) =>
        row.subject_kind === "workload" &&
        row.subject_id === draft.request.workload_id &&
        row.readiness !== "excluded",
    ).length,
    evidence_id: draft.request.request_id,
  }));
  return [...admitted, ...requested];
}

function journalOperation(action: AttentionAction): JournalOperation {
  switch (action) {
    case "create":
      return "AUTHOR";
    case "refine":
      return "REFINE";
    case "retest":
      return "TEST";
    case "block":
    case "policy_excluded":
      return "RESOLVE";
  }
}

function journalProfile(operation: JournalOperation): string {
  switch (operation) {
    case "AUTHOR":
    case "REFINE":
      return "CODING HARNESS";
    case "TEST":
      return "QUALIFICATION RUNNER";
    case "RESOLVE":
      return "SURVEY / OPERATOR";
  }
}

function readinessOrder(readiness: AttentionReadiness): number {
  if (readiness === "ready") return 0;
  if (readiness === "blocked") return 1;
  return 2;
}

export function AgentsPage({
  journal,
  opportunities,
  portfolio,
}: {
  journal: JournalProjection;
  opportunities: JournalOpportunitySurface;
  portfolio: WorkloadList;
}) {
  const systemEntries = deriveJournalEntries(portfolio);
  const insights = deriveWorkInsights(portfolio);
  const ready = systemEntries.filter(
    (entry) => entry.readiness === "ready",
  ).length;
  const totalEntries = journal.log.entries.length + systemEntries.length;

  return (
    <main className={sx(chrome.page, styles.page)}>
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / JOURNAL"
      >
        <AgentHeading
          detail={`${totalEntries} entries · ${ready} ready · ${journal.log.entries.length} retained authored`}
          index="01"
          kicker="JOURNAL"
          title="What should happen next"
        />
        {journal.log.entries.length > 0 ? (
          <JournalEntries compact entries={journal.log.entries} />
        ) : null}
        {opportunities.rows.length > 0 ? (
          <AuthoredOpportunities surface={opportunities} />
        ) : null}
        {totalEntries === 0 ? (
          <div className={sx(chrome.micro, styles.empty)}>
            NO AGENT WORK RECOMMENDED BY CURRENT EVIDENCE
          </div>
        ) : null}
        {systemEntries.length > 0 ? (
          <div className={sx(styles.table)} role="table">
            <div className={sx(chrome.micro, styles.journalHeader)} role="row">
              <span>ENTRY / SUBJECT</span>
              <span>OPERATION</span>
              <span>WHY NOW</span>
              <span>BOUNDS</span>
              <span>READINESS</span>
              <span>LOCATION</span>
            </div>
            {systemEntries.map((entry, index) => (
              <JournalRow entry={entry} index={index} key={entry.id} />
            ))}
          </div>
        ) : null}
        <JournalCreateLink />
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / WORK LEDGER"
      >
        <AgentHeading
          detail={`${portfolio.workloads.length} admitted · ${portfolio.drafts.length} requested · current bounded portfolio`}
          index="02"
          kicker="WORK LEDGER"
          title="Observed work"
        />
        {insights.length === 0 ? (
          <div className={sx(chrome.micro, styles.empty)}>
            NO WORK HAS ENTERED THE CURRENT PORTFOLIO
          </div>
        ) : (
          <div className={sx(styles.table)} role="table">
            <div className={sx(chrome.micro, styles.ledgerHeader)} role="row">
              <span>SUBJECT</span>
              <span>LAST OBSERVED</span>
              <span>RESULT</span>
              <span>ARTIFACT OUTPUT</span>
              <span>SYSTEM READ</span>
              <span>LOCATION</span>
            </div>
            {insights.map((insight, index) => (
              <InsightRow index={index} insight={insight} key={insight.id} />
            ))}
          </div>
        )}
      </section>
    </main>
  );
}

function AuthoredOpportunities({
  surface,
}: {
  surface: JournalOpportunitySurface;
}) {
  return (
    <div
      className={sx(styles.table)}
      data-journal-opportunity-surface={surface.surface_id}
      role="table"
      title={`${surface.source_log_id} · ${surface.ordering}`}
    >
      <div className={sx(chrome.micro, styles.journalHeader)} role="row">
        <span>ENTRY / CELL</span>
        <span>OPERATION</span>
        <span>DESIRED DELTA</span>
        <span>CITATIONS</span>
        <span>READINESS</span>
        <span>LOCATION</span>
      </div>
      {surface.rows.map((opportunity, index) => (
        <AuthoredOpportunityRow
          index={index}
          key={opportunity.opportunity_id}
          opportunity={opportunity}
        />
      ))}
      <div className={sx(chrome.micro, styles.opportunityBoundary)}>
        AUTHORED OPPORTUNITIES · {surface.completeness.toUpperCase()} ·{" "}
        {surface.summary.omitted} OMITTED · NO ASSIGNMENT OR EXECUTION · RUNTIME
        WORK REQUIRES A VERIFIED SELECTED READY CREATE ATTENTION ROW AND
        WORKLOAD ADMISSION
      </div>
    </div>
  );
}

function AuthoredOpportunityRow({
  index,
  opportunity,
}: {
  index: number;
  opportunity: JournalOpportunity;
}) {
  return (
    <article className={sx(styles.journalRow)} role="row">
      <div className={sx(styles.identity)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div className={sx(styles.identityDetail)}>
          <strong>
            J@{opportunity.entry_sequence}#{opportunity.block_id}
          </strong>
          <code title={opportunity.opportunity_id}>
            {shortDigest(opportunity.opportunity_id)}
          </code>
          <span className={sx(chrome.micro)}>
            {opportunity.author.kind} / {opportunity.author.id} / self-asserted
          </span>
        </div>
      </div>
      <div className={sx(styles.operation)}>
        <strong>{opportunity.operation.toUpperCase()}</strong>
        <span className={sx(chrome.micro)}>AUTHORED ACTION CELL</span>
      </div>
      <div className={sx(styles.reason)}>
        <p className={sx(styles.reasonText)}>{opportunity.desired_delta}</p>
        <code title={opportunity.entry_id}>
          {shortDigest(opportunity.entry_id)}
        </code>
      </div>
      <div className={sx(styles.bounds)}>
        <strong>
          {opportunity.evidence_ids.length} evidence ·{" "}
          {opportunity.dependency_ids.length} dependencies
        </strong>
        <span className={sx(chrome.micro)}>EXACT AUTHORED CITATIONS</span>
      </div>
      <div className={sx(styles.readiness)}>
        <strong>AUTHORED ONLY</strong>
        <span className={sx(chrome.micro)}>AUTHORITY / NONE</span>
      </div>
      <a
        className={sx(styles.locate)}
        href={`${opportunity.document_path}#${opportunity.fragment}`}
      >
        OPEN EXACT CELL →
      </a>
    </article>
  );
}

function JournalRow({
  entry,
  index,
}: {
  entry: DerivedJournalEntry;
  index: number;
}) {
  return (
    <article className={sx(styles.journalRow)} role="row">
      <div className={sx(styles.identity)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div className={sx(styles.identityDetail)}>
          <strong>{entry.subject_id}</strong>
          <code title={entry.id}>{shortDigest(entry.id)}</code>
          <span className={sx(chrome.micro)}>{entry.subject_kind}</span>
        </div>
      </div>
      <div className={sx(styles.operation)}>
        <strong>{entry.operation}</strong>
        <span className={sx(chrome.micro)}>{entry.profile}</span>
      </div>
      <div className={sx(styles.reason)}>
        <p className={sx(styles.reasonText)}>{entry.reason}</p>
        <span className={sx(chrome.micro)}>
          {entry.author} / {entry.origin} / {entry.source}
        </span>
      </div>
      <div className={sx(styles.bounds)}>
        <strong>
          P{entry.priority} · C{entry.estimated_cost_units}
        </strong>
        <span>
          {entry.evidence_count} evidence · {entry.dependency_count}{" "}
          dependencies
        </span>
      </div>
      <div className={sx(styles.readiness)}>
        <strong>{entry.readiness.toUpperCase()}</strong>
        <span className={sx(chrome.micro)}>ASSIGNMENT / OPEN</span>
      </div>
      {entry.workload_id ? (
        <TaskLink workloadId={entry.workload_id} />
      ) : (
        <span className={sx(chrome.micro, styles.unlocated)}>
          LOCATOR / PENDING
        </span>
      )}
    </article>
  );
}

function InsightRow({
  insight,
  index,
}: {
  insight: WorkInsight;
  index: number;
}) {
  const percent = scenarioPercent(
    insight.scenarios_passed,
    insight.scenarios_required,
  );
  return (
    <article className={sx(styles.ledgerRow)} role="row">
      <div className={sx(styles.identity)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div className={sx(styles.identityDetail)}>
          <strong>{insight.workload_id}</strong>
          <p className={sx(styles.identityDescription)}>{insight.title}</p>
          <span className={sx(chrome.micro)}>
            {insight.kind} · {insight.revision}
          </span>
        </div>
      </div>
      <div className={sx(styles.operation)}>
        <strong>{insight.observed_operation}</strong>
        <code title={insight.evidence_id ?? undefined}>
          {shortDigest(insight.evidence_id)}
        </code>
      </div>
      <div className={sx(styles.result)}>
        <div className={sx(styles.resultSummary)}>
          <strong>{insight.result}</strong>
          <span>{percent}%</span>
        </div>
        <i className={sx(styles.resultTrack)}>
          <b
            className={sx(styles.resultFill)}
            style={{ width: `${percent}%` }}
          />
        </i>
        <small className={sx(styles.resultMeta)}>
          {insight.scenarios_passed}/{insight.scenarios_required} scenarios
        </small>
      </div>
      <div className={sx(styles.artifacts)}>
        <strong>{insight.artifact_summary}</strong>
        <span className={sx(chrome.micro)}>EXACT CURRENT REVISION</span>
      </div>
      <div className={sx(styles.systemRead)}>
        <strong>{insight.journey}</strong>
        <span>{insight.attention_rows} attention rows</span>
      </div>
      <TaskLink workloadId={insight.workload_id} />
    </article>
  );
}

function TaskLink({ workloadId }: { workloadId: string }) {
  return (
    <Link
      className={sx(styles.locate)}
      params={{ workloadId }}
      to="/workloads/$workloadId"
    >
      INSPECT EVIDENCE →
    </Link>
  );
}

function AgentHeading({
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
