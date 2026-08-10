import { Link } from "@tanstack/react-router";
import {
  deriveAgentIndex,
  scenarioPercent,
  type AgentSummary,
  type WorkloadList,
} from "./domain";
import {
  agentExplorerCoordinate,
  explorerCoordinateSegment,
} from "./explorer-coordinate";
import { agentsStyles as styles } from "./stylex/agents.stylex";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";

export function AgentsPage({ portfolio }: { portfolio: WorkloadList }) {
  const agents = deriveAgentIndex(portfolio);
  const workloadCount = agents.reduce(
    (total, agent) => total + agent.workload_ids.length,
    0,
  );
  const scenarioCount = agents.reduce(
    (total, agent) => total + agent.scenarios_required,
    0,
  );
  return (
    <main className={sx(chrome.page, styles.page)}>
      <header className={sx(styles.hero)}>
        <div>
          <p className={sx(chrome.micro, chrome.eyebrow)}>
            PROVENANCE / EXPLORER INDEX
          </p>
          <h1 className={sx(chrome.displayTitle, styles.title)}>AGENT INDEX</h1>
          <p className={sx(styles.introduction)}>
            A traditional registry over exact workload generator provenance.
            Locate an agent here, then enter its bounded neighborhood in the
            Explorer without losing revision identity.
          </p>
        </div>
        <dl className={sx(styles.summary)}>
          <AgentMetric label="IDENTIFIED" value={agents.length} />
          <AgentMetric label="ADMITTED OUTPUTS" value={workloadCount} />
          <AgentMetric label="SCENARIOS" value={scenarioCount} />
          <AgentMetric label="UNASSIGNED" value={portfolio.drafts.length} />
        </dl>
      </header>

      <section className={sx(styles.section)}>
        <AgentHeading
          detail={`${agents.length} exact producer + revision coordinates`}
          index="01"
          kicker="IDENTIFIED AGENTS"
          title="Generator provenance"
        />
        {agents.length === 0 ? (
          <div className={sx(chrome.micro, styles.empty)}>
            NO AGENTS IDENTIFIED BY ADMITTED WORKLOAD PROVENANCE
          </div>
        ) : (
          <div className={sx(styles.agentTable)} role="table">
            <div className={sx(chrome.micro, styles.tableHeader)} role="row">
              <span>IDENTITY</span>
              <span>ROLE / REVISION</span>
              <span>ADMITTED OUTPUTS</span>
              <span>CONFORMANCE</span>
              <span>LOCATION</span>
            </div>
            {agents.map((agent, index) => (
              <AgentRow agent={agent} index={index} key={agent.id} />
            ))}
          </div>
        )}
      </section>

      <section className={sx(styles.section)}>
        <AgentHeading
          detail={`${portfolio.drafts.length} request-only workload entries`}
          index="02"
          kicker="UNASSIGNED HANDOFFS"
          title="Agents still needed"
        />
        {portfolio.drafts.length === 0 ? (
          <div className={sx(chrome.micro, styles.empty)}>
            NO CREATION REQUESTS ARE WAITING FOR A CODING HARNESS
          </div>
        ) : (
          <div className={sx(styles.handoffGrid)}>
            {portfolio.drafts.map((draft) => (
              <article
                className={sx(styles.handoff)}
                key={draft.request.request_id}
              >
                <span className={sx(chrome.micro)}>
                  CODING HARNESS / UNASSIGNED
                </span>
                <h3>{draft.request.workload_id}</h3>
                <p>{draft.request.intent ?? draft.request.title}</p>
                <div className={sx(styles.handoffRoute)}>
                  <span>REQUEST</span>
                  <i />
                  <span>AGENT</span>
                  <i />
                  <span>PACKAGE</span>
                </div>
                <Link
                  params={{ workloadId: draft.request.workload_id }}
                  to="/workloads/$workloadId"
                >
                  INSPECT HANDOFF →
                </Link>
              </article>
            ))}
          </div>
        )}
      </section>
    </main>
  );
}

function AgentRow({ agent, index }: { agent: AgentSummary; index: number }) {
  const coordinate = agentExplorerCoordinate(agent);
  const percent = scenarioPercent(
    agent.scenarios_passed,
    agent.scenarios_required,
  );
  return (
    <article className={sx(styles.agentRow)} role="row">
      <div className={sx(styles.agentIdentity)}>
        <span className={sx(styles.agentOrdinal)}>
          {String(index + 1).padStart(2, "0")}
        </span>
        <div>
          <strong>{agent.producer}</strong>
          <code>{agent.id}</code>
        </div>
      </div>
      <div className={sx(styles.roleBinding)}>
        <span>{agent.kind.replaceAll("_", " ")}</span>
        <code>{agent.producer_revision}</code>
      </div>
      <div className={sx(styles.outputBinding)}>
        <strong>{agent.workload_ids.length}</strong>
        <span>{agent.workload_ids.join(" · ")}</span>
      </div>
      <div className={sx(styles.conformance)}>
        <div>
          <span>
            {agent.scenarios_passed}/{agent.scenarios_required}
          </span>
          <strong>{percent}%</strong>
        </div>
        <i>
          <b style={{ width: `${percent}%` }} />
        </i>
        <small>{agent.attention_rows} directed attention rows</small>
      </div>
      <Link
        className={sx(styles.locate)}
        params={{
          coordinate: explorerCoordinateSegment(coordinate),
          kind: coordinate.kind,
        }}
        to="/explore/$kind/$coordinate"
      >
        LOCATE IN EXPLORER →
      </Link>
    </article>
  );
}

function AgentMetric({ label, value }: { label: string; value: number }) {
  return (
    <div className={sx(styles.metric)}>
      <dt className={sx(chrome.micro)}>{label}</dt>
      <dd className={sx(styles.metricValue)}>{value}</dd>
    </div>
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
