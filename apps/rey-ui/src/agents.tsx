import { Link } from "@tanstack/react-router";
import type {
  EnvironmentApplicationObservation,
  EnvironmentStatus,
} from "./environment";
import type {
  AttentionAction,
  AttentionReadiness,
  WorkloadList,
} from "./domain";
import { shortDigest } from "./domain";
import { agentsStyles as styles } from "./stylex/agents.stylex";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";

export type CollaborationOperation =
  "AUTHOR" | "REFINE" | "RESOLVE" | "SURVEY" | "TEST";

export interface CollaborationTask {
  id: string;
  subject_id: string;
  subject_kind: "surface" | "workload";
  objective: string;
  operation: CollaborationOperation;
  readiness: AttentionReadiness;
  priority: number;
  evidence_count: number;
  dependency_count: number;
  workload_id: string | null;
}

export interface AgentRuntime {
  id: string;
  application: EnvironmentApplicationObservation;
}

const workflowLanes = [
  {
    label: "CONTEXT",
    operations: ["DISCOVER", "REASON", "SURVEY", "PROCESS"],
  },
  {
    label: "WORKLOAD",
    operations: ["ORIENT", "AUTHOR", "TEST", "REFINE", "RUN"],
  },
] as const;

export function deriveAgentRuntimes(status: EnvironmentStatus): AgentRuntime[] {
  return status.operator.applications
    .flatMap((application) => {
      const observation = application.working;
      return observation?.potential_capabilities.some((capability) =>
        capability.startsWith("agent.runtime."),
      )
        ? [{ id: application.object_id, application: observation }]
        : [];
    })
    .sort((left, right) =>
      left.application.name.localeCompare(right.application.name),
    );
}

export function deriveCollaborationTasks(
  portfolio: WorkloadList,
): CollaborationTask[] {
  const attentionBySubject = new Map(
    portfolio.attention.rows
      .filter((row) => row.readiness !== "excluded")
      .map((row) => [`${row.subject_kind}:${row.subject_id}`, row]),
  );
  const tasks: CollaborationTask[] = [];

  for (const draft of portfolio.drafts) {
    const key = `workload:${draft.request.workload_id}`;
    const attention = attentionBySubject.get(key);
    if (attention) attentionBySubject.delete(key);
    tasks.push({
      id: draft.request.request_id,
      subject_id: draft.request.workload_id,
      subject_kind: "workload",
      objective: draft.request.intent ?? draft.request.title,
      operation: "AUTHOR",
      readiness: attention?.readiness ?? "ready",
      priority: attention?.priority ?? 0,
      evidence_count: attention?.evidence_ids.length ?? 0,
      dependency_count: attention?.dependency_ids.length ?? 0,
      workload_id: draft.request.workload_id,
    });
  }

  for (const attention of attentionBySubject.values()) {
    tasks.push({
      id: attention.row_id,
      subject_id: attention.subject_id,
      subject_kind: attention.subject_kind,
      objective: attention.reason,
      operation: attentionOperation(attention.action),
      readiness: attention.readiness,
      priority: attention.priority,
      evidence_count: attention.evidence_ids.length,
      dependency_count: attention.dependency_ids.length,
      workload_id:
        attention.subject_kind === "workload" ? attention.subject_id : null,
    });
  }

  return tasks.sort(
    (left, right) =>
      readinessOrder(left.readiness) - readinessOrder(right.readiness) ||
      right.priority - left.priority ||
      left.subject_id.localeCompare(right.subject_id),
  );
}

function attentionOperation(action: AttentionAction): CollaborationOperation {
  switch (action) {
    case "create":
      return "AUTHOR";
    case "refine":
      return "REFINE";
    case "retest":
      return "TEST";
    case "block":
      return "RESOLVE";
    case "policy_excluded":
      return "SURVEY";
  }
}

function readinessOrder(readiness: AttentionReadiness): number {
  if (readiness === "ready") return 0;
  if (readiness === "blocked") return 1;
  return 2;
}

export function AgentsPage({
  environment,
  portfolio,
  refreshError,
}: {
  environment: EnvironmentStatus;
  portfolio: WorkloadList;
  refreshError: Error | null;
}) {
  const tasks = deriveCollaborationTasks(portfolio);
  const runtimes = deriveAgentRuntimes(environment);
  const found = runtimes.filter(
    (runtime) => runtime.application.availability === "available",
  );

  return (
    <main className={sx(chrome.page, styles.page)}>
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / TASK PLANE"
      >
        <AgentHeading
          detail={`${tasks.length} current frontier tasks · derived, not retained`}
          index="01"
          kicker="TASK PLANE"
          title="Bounded collaboration"
        />
        <div className={sx(styles.workflowGrammar)}>
          <div className={sx(styles.workflowGrammarLabel)}>
            <span className={sx(chrome.micro)}>OPERATION GRAMMAR</span>
            <strong>Tasks enter one bounded operation.</strong>
            <p>
              Workflows organize artifact movement. Journeys are derived from
              operation state; Rey does not retain a second pile of journey
              objects.
            </p>
          </div>
          <div className={sx(styles.workflowLanes)}>
            {workflowLanes.map((lane) => (
              <div className={sx(styles.workflowLane)} key={lane.label}>
                <span className={sx(chrome.micro)}>{lane.label}</span>
                <div className={sx(styles.workflowOperations)}>
                  {lane.operations.map((operation, index) => (
                    <span className={sx(styles.workflowStep)} key={operation}>
                      <strong>{operation}</strong>
                      {index < lane.operations.length - 1 ? (
                        <i className={sx(styles.workflowArrow)}>→</i>
                      ) : null}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
        {tasks.length === 0 ? (
          <div className={sx(chrome.micro, styles.empty)}>
            NO BOUNDED COLLABORATION TASKS ON THE CURRENT FRONTIER
          </div>
        ) : (
          <div className={sx(styles.taskTable)} role="table">
            <div
              className={sx(chrome.micro, styles.taskTableHeader)}
              role="row"
            >
              <span>TASK / SUBJECT</span>
              <span>CURRENT OPERATION</span>
              <span>ARTIFACT BOUND</span>
              <span>ASSIGNMENT</span>
              <span>LOCATION</span>
            </div>
            {tasks.map((task, index) => (
              <TaskRow index={index} key={task.id} task={task} />
            ))}
          </div>
        )}
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / AGENT RUNTIMES"
      >
        <AgentHeading
          detail={`${runtimes.length} desired · ${found.length} found · ${runtimes.length - found.length} unresolved`}
          index="02"
          kicker="AGENT RUNTIMES"
          title="Collaboration applications"
        />
        <div className={sx(styles.runtimeBoundary)}>
          <span className={sx(chrome.micro)}>DISCOVERY BOUNDARY</span>
          <strong>FOUND ≠ ADMITTED TO ACT</strong>
          <p>
            Rey searches its process-owned inventory through bounded PATH
            resolution without starting agent CLIs. Assignment, execution, and
            an Explorer locator remain separate admissions.
          </p>
          {refreshError ? (
            <small className={sx(chrome.micro, styles.runtimeError)}>
              REVALIDATION DELAYED · {refreshError.message}
            </small>
          ) : null}
        </div>
        <div className={sx(styles.runtimeGrid)}>
          {runtimes.map((runtime, index) => (
            <RuntimeCard index={index} key={runtime.id} runtime={runtime} />
          ))}
        </div>
      </section>
    </main>
  );
}

function TaskRow({ task, index }: { task: CollaborationTask; index: number }) {
  const artifactCount = task.evidence_count + task.dependency_count;
  return (
    <article className={sx(styles.taskRow)} role="row">
      <div className={sx(styles.taskIdentity)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div>
          <strong>{task.subject_id}</strong>
          <code title={task.id}>{shortDigest(task.id)}</code>
          <p className={sx(styles.taskObjective)}>{task.objective}</p>
        </div>
      </div>
      <div className={sx(styles.operationBinding)}>
        <strong>{task.operation}</strong>
        <span className={sx(chrome.micro)}>{task.subject_kind} task</span>
      </div>
      <div className={sx(styles.artifactBinding)}>
        <strong>{artifactCount}</strong>
        <span>
          {task.evidence_count} evidence · {task.dependency_count} dependencies
        </span>
      </div>
      <div className={sx(styles.assignment)}>
        <strong>{task.readiness.toUpperCase()}</strong>
        <span className={sx(chrome.micro)}>AGENT / UNASSIGNED</span>
      </div>
      {task.workload_id ? (
        <Link
          className={sx(styles.locate)}
          params={{ workloadId: task.workload_id }}
          to="/workloads/$workloadId"
        >
          OPEN TASK SUBJECT →
        </Link>
      ) : (
        <span className={sx(chrome.micro, styles.unlocated)}>
          LOCATOR / PENDING
        </span>
      )}
    </article>
  );
}

function RuntimeCard({
  runtime,
  index,
}: {
  runtime: AgentRuntime;
  index: number;
}) {
  const { application } = runtime;
  const status = application.availability;
  return (
    <article className={sx(styles.runtimeCard)}>
      <header className={sx(styles.runtimeCardHeader)}>
        <span className={sx(styles.ordinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div className={sx(styles.runtimeIdentity)}>
          <h3 className={sx(styles.runtimeTitle)}>{application.name}</h3>
          <p className={sx(styles.runtimePurpose)}>{application.purpose}</p>
        </div>
        <span
          className={sx(
            chrome.micro,
            styles.runtimeStatus,
            status === "available" && styles.runtimeFound,
            status === "unavailable" && styles.runtimeMissing,
            status === "error" && styles.runtimeError,
          )}
        >
          {status === "available"
            ? "FOUND"
            : status === "unavailable"
              ? "NOT FOUND"
              : "PROBE ERROR"}
        </span>
      </header>
      <dl className={sx(styles.runtimeDefinitions)}>
        <div className={sx(styles.runtimeDefinition)}>
          <dt className={sx(styles.runtimeTerm)}>SEARCH</dt>
          <dd className={sx(styles.runtimeValue)}>
            {application.resolved_path ?? "PATH / UNRESOLVED"}
          </dd>
        </div>
        <div className={sx(styles.runtimeDefinition)}>
          <dt className={sx(styles.runtimeTerm)}>POTENTIAL</dt>
          <dd className={sx(styles.runtimeValue)}>
            {application.potential_capabilities.join(" · ")}
          </dd>
        </div>
        <div className={sx(styles.runtimeDefinition)}>
          <dt className={sx(styles.runtimeTerm)}>AUTHORITY</dt>
          <dd className={sx(styles.runtimeValue)}>DISCOVERED / NOT ADMITTED</dd>
        </div>
      </dl>
      <footer className={sx(styles.runtimeFooter)}>
        <span className={sx(chrome.micro)}>
          {application.searched_path_count} PATH ENTRIES SEARCHED
        </span>
        <span className={sx(chrome.micro, styles.unlocated)}>
          EXPLORER LOCATOR / PENDING SURVEY
        </span>
      </footer>
    </article>
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
