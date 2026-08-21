import { Link } from "@tanstack/react-router";
import { shortDigest, sourceBranchUrl } from "./domain";
import { GitCommitLink } from "./git-commit-link";
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
  publication: "pushed" | "local" | "unknown" | null;
}

export interface CadenceRepositoryState {
  id: string;
  working_tree_state: "clean" | "dirty";
  staged_entries: number;
  unstaged_entries: number;
  untracked_entries: number;
  conflicted_entries: number;
  push_state:
    | "pushed"
    | "unpushed"
    | "behind"
    | "diverged"
    | "no_upstream"
    | "detached"
    | "unborn"
    | "unknown";
  branch: string | null;
  head_revision: string | null;
  upstream: string | null;
  upstream_revision: string | null;
  ahead: number | null;
  behind: number | null;
  comparison_basis: "local_tracking_ref";
  complete: boolean;
  scope: "tracked_changes_and_untracked_files";
  omissions: string[];
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
  repository_state: CadenceRepositoryState | null;
  lanes: CadenceLane[];
  schedules: CadenceSchedule[];
  omissions: string[];
}

export function CadencePage({ cadence }: { cadence: CadenceProjection }) {
  return (
    <main className={sx(chrome.page, styles.page)}>
      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / REPOSITORY STATE"
      >
        <CadenceHeading
          detail="local observation · no remote transport"
          index="01"
          kicker="REPOSITORY STATE"
          title="Working tree and remote"
        />
        <RepositoryStateView
          gitAdmissionRequired={cadence.lanes.some(
            (lane) => lane.id === "git:unavailable",
          )}
          repository={cadence.source_repository}
          state={cadence.repository_state}
        />
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / RETAINED SEQUENCE"
      >
        <CadenceHeading
          detail={`${cadence.lanes.length} clocks · ${cadence.ordering} ordering`}
          index="02"
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

      <section
        className={sx(styles.section)}
        data-rey-section="03 / SCHEDULED SCANS"
      >
        <CadenceHeading
          detail="read-only browser projection · no runtime admission"
          index="03"
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

      <section
        className={sx(styles.section, styles.boundary)}
        data-rey-section="04 / REFERENCE PLANE"
      >
        <CadenceHeading
          detail={`${cadence.omissions.length} declared ordering boundary`}
          index="04"
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

function RepositoryStateView({
  gitAdmissionRequired,
  repository,
  state,
}: {
  gitAdmissionRequired: boolean;
  repository: string | null;
  state: CadenceRepositoryState | null;
}) {
  if (state === null) {
    return (
      <div className={sx(chrome.micro, styles.repositoryAbsent)}>
        {gitAdmissionRequired
          ? "GIT ADMISSION REQUIRED · Working-tree and local-upstream state cannot be read until an available git executable is committed in Environment HEAD. Ambient PATH presence does not enable this observation."
          : "REPOSITORY STATE NOT OBSERVED"}
      </div>
    );
  }
  const branchHref =
    repository && state.branch
      ? sourceBranchUrl(repository, state.branch)
      : null;
  const upstreamBranch = state.upstream?.split("/").slice(1).join("/") || null;
  const upstreamHref =
    repository && upstreamBranch
      ? sourceBranchUrl(repository, upstreamBranch)
      : null;
  return (
    <div className={sx(styles.repositoryState)}>
      <article className={sx(styles.stateInstrument)}>
        <header className={sx(styles.stateInstrumentHeader)}>
          <div>
            <span className={sx(chrome.micro)}>WORKING TREE</span>
            <strong className={sx(styles.stateTitle)}>
              {state.working_tree_state.toUpperCase()}
            </strong>
          </div>
          <span
            className={sx(
              chrome.micro,
              styles.stateSignal,
              state.working_tree_state === "clean"
                ? styles.signalClear
                : styles.signalAttention,
            )}
          >
            {state.working_tree_state === "clean"
              ? "NO ATTENTION"
              : "ATTENTION"}
          </span>
        </header>
        <div className={sx(styles.stateMeasures)}>
          <StateMeasure label="STAGED" value={state.staged_entries} />
          <StateMeasure label="UNSTAGED" value={state.unstaged_entries} />
          <StateMeasure label="UNTRACKED" value={state.untracked_entries} />
          <StateMeasure label="CONFLICTED" value={state.conflicted_entries} />
        </div>
      </article>

      <article className={sx(styles.stateInstrument)}>
        <header className={sx(styles.stateInstrumentHeader)}>
          <div>
            <span className={sx(chrome.micro)}>PUSH RELATION</span>
            <strong className={sx(styles.stateTitle)}>
              {state.push_state.replaceAll("_", " ")}
            </strong>
          </div>
          <span className={sx(chrome.micro, styles.comparisonBasis)}>
            LOCAL REF
          </span>
        </header>
        <div className={sx(styles.refPath)}>
          <div className={sx(styles.refEndpoint)}>
            <span className={sx(chrome.micro)}>BRANCH</span>
            {branchHref ? (
              <a
                className={sx(
                  chrome.focusable,
                  styles.refValue,
                  styles.refLink,
                )}
                href={branchHref}
                rel="noreferrer"
                target="_blank"
              >
                {state.branch}
              </a>
            ) : (
              <strong className={sx(styles.refValue)}>
                {state.branch ?? "DETACHED"}
              </strong>
            )}
          </div>
          <i aria-hidden="true" className={sx(styles.refArrow)}>
            →
          </i>
          <div className={sx(styles.refEndpoint, styles.refEndpointRemote)}>
            <span className={sx(chrome.micro)}>UPSTREAM</span>
            {upstreamHref ? (
              <a
                className={sx(
                  chrome.focusable,
                  styles.refValue,
                  styles.refLink,
                )}
                href={upstreamHref}
                rel="noreferrer"
                target="_blank"
              >
                {state.upstream}
              </a>
            ) : (
              <strong className={sx(styles.refValue)}>
                {state.upstream ?? "NOT CONFIGURED"}
              </strong>
            )}
          </div>
        </div>
        <div className={sx(styles.publicationMeasures)}>
          <StateMeasure label="AHEAD" value={state.ahead} />
          <StateMeasure label="BEHIND" value={state.behind} />
          {repository === null ? (
            <div className={sx(styles.repositoryUnbound)}>
              <span className={sx(chrome.micro)}>COMMIT LINKS</span>
              <strong>REPOSITORY UNBOUND</strong>
            </div>
          ) : (
            <>
              <RepositoryRevision
                label="HEAD"
                repository={repository}
                revision={state.head_revision}
              />
              <RepositoryRevision
                label="UPSTREAM"
                repository={repository}
                revision={state.upstream_revision}
              />
            </>
          )}
        </div>
      </article>
    </div>
  );
}

function StateMeasure({
  label,
  value,
}: {
  label: string;
  value: number | null;
}) {
  return (
    <div className={sx(styles.stateMeasure)}>
      <span className={sx(chrome.micro)}>{label}</span>
      <strong className={sx(styles.measureValue)}>{value ?? "—"}</strong>
    </div>
  );
}

function RepositoryRevision({
  label,
  repository,
  revision,
}: {
  label: string;
  repository: string | null;
  revision: string | null;
}) {
  return (
    <div className={sx(styles.repositoryRevision)}>
      <span className={sx(chrome.micro)}>{label}</span>
      {revision === null ? (
        <strong className={sx(styles.refValue)}>—</strong>
      ) : (
        <GitCommitLink
          className={sx(
            chrome.focusable,
            styles.refValue,
            styles.refLink,
          )}
          fallback="GIT COMMIT / REPOSITORY UNBOUND"
          repository={repository}
          revision={revision}
        >
          <code>{shortDigest(revision)}</code>
        </GitCommitLink>
      )}
    </div>
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
                  {tick.publication === null ? null : (
                    <span
                      className={sx(
                        chrome.micro,
                        styles.publication,
                        tick.publication === "pushed"
                          ? styles.publicationPushed
                          : tick.publication === "local"
                            ? styles.publicationLocal
                            : styles.publicationUnknown,
                      )}
                    >
                      {tick.publication}
                    </span>
                  )}
                  <time className={sx(chrome.micro)}>
                    {formatCadenceTime(tick.occurred_at_unix)}
                  </time>
                </div>
                <h3>{tick.title}</h3>
                <p>{tick.detail}</p>
                <div className={sx(styles.tickBinding)}>
                  <TickRevision repository={repository} tick={tick} />
                  <span>
                    {tick.parent_revisions.length} PARENT
                    {tick.parent_revisions.length === 1 ? "" : "S"}
                  </span>
                  <TickLink tick={tick} />
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

function TickLink({ tick }: { tick: CadenceTick }) {
  if (tick.kind === "git_commit") return null;
  return <Link to="/environment">OPEN ENVIRONMENT →</Link>;
}

function TickRevision({
  repository,
  tick,
}: {
  repository: string | null;
  tick: CadenceTick;
}) {
  if (tick.kind !== "git_commit") {
    return <code>{shortDigest(tick.revision)}</code>;
  }
  return (
    <GitCommitLink
      className={sx(chrome.focusable, styles.commitLink)}
      fallback="GIT COMMIT / REPOSITORY UNBOUND"
      repository={repository}
      revision={tick.revision}
    >
      <code>{shortDigest(tick.revision)}</code>
    </GitCommitLink>
  );
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
