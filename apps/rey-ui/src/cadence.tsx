import { Link } from "@tanstack/react-router";
import { shortDigest, sourceCommitUrl } from "./domain";
import { cadenceStyles as styles } from "./stylex/cadence.stylex";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";

export interface CadenceTick {
  id: string;
  kind: "git_commit" | "rey_admission";
  state: "observed" | "staged" | "committed";
  ordinal: string;
  title: string;
  detail: string;
  revision: string;
  parent_revisions: string[];
  occurred_at_unix: number | null;
}

export interface CadenceLane {
  id: string;
  label: string;
  clock: string;
  ordering: "newest_first";
  complete: boolean;
  ticks: CadenceTick[];
  omissions: string[];
}

export interface CadenceSchedule {
  id: string;
  label: string;
  source: string;
  interval_ms: number;
  activation: string;
  authority: "mounted_browser_projection";
  retention: "last_good_document";
}

export interface CadenceProjection {
  schema: "rey.ui-cadence.v1";
  ordering: "partial";
  source_repository: string | null;
  lanes: CadenceLane[];
  schedules: CadenceSchedule[];
  omissions: string[];
}

export function CadencePage({ cadence }: { cadence: CadenceProjection }) {
  return (
    <main className={sx(chrome.page, styles.page)}>
      <section className={sx(styles.section, styles.firstSection)}>
        <CadenceHeading
          detail={`${cadence.lanes.length} clocks · ${cadence.ordering} ordering`}
          index="01"
          kicker="RETAINED SEQUENCE"
          title="Tick lanes"
        />
        <div className={sx(styles.lanes)}>
          {cadence.lanes.map((lane) => (
            <CadenceLaneView
              key={lane.id}
              lane={lane}
              repository={cadence.source_repository}
            />
          ))}
        </div>
      </section>

      <section className={sx(styles.section)}>
        <CadenceHeading
          detail="read-only browser projection · no runtime admission"
          index="02"
          kicker="SCHEDULED SCANS"
          title="Mounted observation loops"
        />
        <div className={sx(styles.scheduleGrid)}>
          {cadence.schedules.map((schedule) => (
            <article className={sx(styles.scheduleCard)} key={schedule.id}>
              <div className={sx(styles.scheduleDial)}>
                <span>{formatInterval(schedule.interval_ms)}</span>
                <i />
              </div>
              <div className={sx(styles.scheduleBody)}>
                <span className={sx(chrome.micro, styles.scheduleId)}>
                  {schedule.id}
                </span>
                <h3>{schedule.label}</h3>
                <code>{schedule.source}</code>
                <dl className={sx(styles.scheduleDefinitions)}>
                  <div>
                    <dt>ACTIVATION</dt>
                    <dd>{schedule.activation.replaceAll("_", " ")}</dd>
                  </div>
                  <div>
                    <dt>AUTHORITY</dt>
                    <dd>{schedule.authority.replaceAll("_", " ")}</dd>
                  </div>
                  <div>
                    <dt>RETENTION</dt>
                    <dd>{schedule.retention.replaceAll("_", " ")}</dd>
                  </div>
                </dl>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className={sx(styles.section, styles.boundary)}>
        <CadenceHeading
          detail={`${cadence.omissions.length} declared ordering boundary`}
          index="03"
          kicker="REFERENCE PLANE"
          title="Ordering and omissions"
        />
        <div className={sx(styles.boundaryGrid)}>
          <p className={sx(styles.boundaryStatement)}>
            <strong>PARTIAL ORDER IS AUTHORITATIVE.</strong> Movement within a
            lane is sequenced. Movement between lanes is related only when
            retained evidence supplies an explicit edge.
          </p>
          <ul className={sx(styles.omissionList)}>
            {cadence.omissions.map((omission) => (
              <li key={omission}>{omission}</li>
            ))}
          </ul>
        </div>
      </section>
    </main>
  );
}

function CadenceLaneView({
  lane,
  repository,
}: {
  lane: CadenceLane;
  repository: string | null;
}) {
  return (
    <article className={sx(styles.lane)}>
      <header className={sx(styles.laneHeader)}>
        <div>
          <span className={sx(chrome.micro, styles.laneClock)}>
            {lane.clock.replaceAll("_", " ")}
          </span>
          <h2>{lane.label}</h2>
        </div>
        <span
          className={sx(
            chrome.micro,
            styles.completeness,
            lane.complete ? styles.complete : styles.bounded,
          )}
        >
          {lane.complete ? "COMPLETE" : "BOUNDED"}
        </span>
      </header>
      {lane.ticks.length === 0 ? (
        <div className={sx(chrome.micro, styles.emptyLane)}>
          NO TICKS OBSERVED ON THIS CLOCK
        </div>
      ) : (
        <ol className={sx(styles.tickList)}>
          {lane.ticks.map((tick, index) => (
            <li className={sx(styles.tick)} key={tick.id}>
              <div className={sx(styles.tickRail)}>
                <span>{String(index + 1).padStart(2, "0")}</span>
                <i />
              </div>
              <div className={sx(styles.tickBody)}>
                <div className={sx(styles.tickMeta)}>
                  <span className={sx(chrome.micro, styles.tickKind)}>
                    {tick.kind.replaceAll("_", " ")}
                  </span>
                  <span className={sx(chrome.micro)}>{tick.ordinal}</span>
                  <span className={sx(chrome.micro)}>{tick.state}</span>
                  <time className={sx(chrome.micro)}>
                    {formatCadenceTime(tick.occurred_at_unix)}
                  </time>
                </div>
                <h3>{tick.title}</h3>
                <p>{tick.detail}</p>
                <div className={sx(styles.tickBinding)}>
                  <code>{shortDigest(tick.revision)}</code>
                  <span>
                    {tick.parent_revisions.length} PARENT
                    {tick.parent_revisions.length === 1 ? "" : "S"}
                  </span>
                  <TickLink repository={repository} tick={tick} />
                </div>
              </div>
            </li>
          ))}
        </ol>
      )}
      {lane.omissions.length > 0 ? (
        <ul className={sx(styles.laneOmissions)}>
          {lane.omissions.map((omission) => (
            <li key={omission}>BOUND / {omission}</li>
          ))}
        </ul>
      ) : null}
    </article>
  );
}

function TickLink({
  repository,
  tick,
}: {
  repository: string | null;
  tick: CadenceTick;
}) {
  if (tick.kind === "git_commit") {
    const href = repository ? sourceCommitUrl(repository, tick.revision) : null;
    return href ? (
      <a href={href} rel="noreferrer" target="_blank">
        OPEN COMMIT ↗
      </a>
    ) : null;
  }
  return <Link to="/environment">OPEN ENVIRONMENT →</Link>;
}

function CadenceHeading({
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

export function formatCadenceTime(unixSeconds: number | null): string {
  if (unixSeconds === null) return "ORDER ONLY";
  return new Date(unixSeconds * 1_000)
    .toISOString()
    .replace("T", " ")
    .replace(".000Z", "Z");
}

function formatInterval(intervalMs: number): string {
  return intervalMs % 1_000 === 0
    ? `${intervalMs / 1_000}s`
    : `${intervalMs}ms`;
}
