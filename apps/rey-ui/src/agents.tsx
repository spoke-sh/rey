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

export type RecommendedOperation = "AUTHOR" | "REFINE" | "RESOLVE" | "TEST";

export interface SystemRecommendation {
  id: string;
  subject_id: string;
  subject_kind: "surface" | "workload";
  operation: RecommendedOperation;
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

export function deriveSystemRecommendations(
  portfolio: WorkloadList,
): SystemRecommendation[] {
  const attentionBySubject = new Map(
    portfolio.attention.rows
      .filter((row) => row.readiness !== "excluded")
      .map((row) => [`${row.subject_kind}:${row.subject_id}`, row]),
  );
  const recommendations: SystemRecommendation[] = [];

  for (const draft of portfolio.drafts) {
    const key = `workload:${draft.request.workload_id}`;
    const attention = attentionBySubject.get(key);
    if (attention) attentionBySubject.delete(key);
    recommendations.push({
      id: draft.request.request_id,
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
    const operation = recommendationOperation(attention.action);
    recommendations.push({
      id: attention.row_id,
      subject_id: attention.subject_id,
      subject_kind: attention.subject_kind,
      operation,
      profile: recommendedProfile(operation),
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

  return recommendations.sort(
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

function recommendationOperation(
  action: AttentionAction,
): RecommendedOperation {
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

function recommendedProfile(operation: RecommendedOperation): string {
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

export function AgentsPage({ portfolio }: { portfolio: WorkloadList }) {
  const recommendations = deriveSystemRecommendations(portfolio);
  const insights = deriveWorkInsights(portfolio);
  const ready = recommendations.filter(
    (recommendation) => recommendation.readiness === "ready",
  ).length;

  return (
    <main className={sx(chrome.page, styles.page)}>
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / SYSTEM RECOMMENDATIONS"
      >
        <AgentHeading
          detail={`${ready} ready · ${recommendations.length - ready} blocked · typed frontier evidence`}
          index="01"
          kicker="SYSTEM RECOMMENDATIONS"
          title="What should happen next"
        />
        <div className={sx(styles.recommendationBoundary)}>
          <span className={sx(chrome.micro)}>RECOMMENDATION BASIS</span>
          <strong>REQUEST + ATTENTION + DELTA</strong>
          <p>
            Rey ranks unresolved work from retained evidence. It recommends an
            operation and capability profile; it does not fabricate an agent
            assignment or let a proposer declare success.
          </p>
        </div>
        {recommendations.length === 0 ? (
          <div className={sx(chrome.micro, styles.empty)}>
            NO AGENT WORK RECOMMENDED BY CURRENT EVIDENCE
          </div>
        ) : (
          <div className={sx(styles.table)} role="table">
            <div
              className={sx(chrome.micro, styles.recommendationHeader)}
              role="row"
            >
              <span>RANK / SUBJECT</span>
              <span>RECOMMENDATION</span>
              <span>WHY NOW</span>
              <span>BOUNDS</span>
              <span>READINESS</span>
              <span>LOCATION</span>
            </div>
            {recommendations.map((recommendation, index) => (
              <RecommendationRow
                index={index}
                key={recommendation.id}
                recommendation={recommendation}
              />
            ))}
          </div>
        )}
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
        <div className={sx(styles.ledgerBoundary)}>
          <span className={sx(chrome.micro)}>EVIDENCE BOUNDARY</span>
          <strong>RETAINED RESULTS / NOT LIVE AGENT TELEMETRY</strong>
          <p>
            Rows summarize admitted revisions, tests, runs, mined artifacts, and
            current attention. Rey does not yet claim who is actively working or
            stream process activity.
          </p>
        </div>
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

function RecommendationRow({
  recommendation,
  index,
}: {
  recommendation: SystemRecommendation;
  index: number;
}) {
  return (
    <article className={sx(styles.recommendationRow)} role="row">
      <div className={sx(styles.identity)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div className={sx(styles.identityDetail)}>
          <strong>{recommendation.subject_id}</strong>
          <code title={recommendation.id}>
            {shortDigest(recommendation.id)}
          </code>
          <span className={sx(chrome.micro)}>
            {recommendation.subject_kind}
          </span>
        </div>
      </div>
      <div className={sx(styles.operation)}>
        <strong>{recommendation.operation}</strong>
        <span className={sx(chrome.micro)}>{recommendation.profile}</span>
      </div>
      <div className={sx(styles.reason)}>
        <p className={sx(styles.reasonText)}>{recommendation.reason}</p>
        <span className={sx(chrome.micro)}>{recommendation.source}</span>
      </div>
      <div className={sx(styles.bounds)}>
        <strong>
          P{recommendation.priority} · C{recommendation.estimated_cost_units}
        </strong>
        <span>
          {recommendation.evidence_count} evidence ·{" "}
          {recommendation.dependency_count} dependencies
        </span>
      </div>
      <div className={sx(styles.readiness)}>
        <strong>{recommendation.readiness.toUpperCase()}</strong>
        <span className={sx(chrome.micro)}>ASSIGNMENT / OPEN</span>
      </div>
      {recommendation.workload_id ? (
        <TaskLink workloadId={recommendation.workload_id} />
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
