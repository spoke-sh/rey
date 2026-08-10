import {
  getKineticMaterialStyle,
  kineticGrammar,
  kineticThemeMaterials,
} from "@hifi/kinetic";
import {
  Link,
  Outlet,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
  useRouterState,
} from "@tanstack/react-router";
import {
  createContext,
  useContext,
  useEffect,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { AgentsPage } from "./agents";
import {
  loadCadence,
  loadEnvironment,
  loadPortfolio,
  type OperatorContext,
} from "./api";
import { CadencePage } from "./cadence";
import {
  scenarioPercent,
  shortDigest,
  sourceCommitUrl,
  workloadJourney,
  type WorkloadDraft,
  type WorkloadSummary,
} from "./domain";
import {
  currentApplications,
  environmentVariableDiff,
  type EnvironmentApplicationObservation,
  type EnvironmentObjectStatus,
} from "./environment";
import { ExplorePage } from "./explore";
import {
  parseExplorerCoordinate,
  resolveExplorerCoordinate,
} from "./explorer-coordinate";
import { startPassiveRevalidation } from "./passive";
import { activeSectionAt, SECTION_RAIL_ATTRIBUTE } from "./section-rail";
import { environmentStyles as styles } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";

const precision = kineticThemeMaterials.precision;
const precisionTheme = kineticGrammar.themes.find(
  (theme) => theme.name === "precision",
);
const PortfolioContext = createContext<OperatorContext | null>(null);

function usePassiveDocument<T>(initialDocument: T, load: () => Promise<T>) {
  const [document, setDocument] = useState(initialDocument);
  const [error, setError] = useState<Error | null>(null);
  useEffect(() => {
    setDocument(initialDocument);
    setError(null);
    return startPassiveRevalidation({
      intervalMs: 5_000,
      load,
      publish: setDocument,
      reportError: setError,
    });
  }, [initialDocument, load]);
  return { document, error };
}

function usePortfolio(): OperatorContext {
  const portfolio = useContext(PortfolioContext);
  if (!portfolio) throw new Error("portfolio context is unavailable");
  return portfolio;
}

function RootLayout() {
  const initialPortfolio = rootRoute.useLoaderData();
  const { document: portfolio } = usePassiveDocument(
    initialPortfolio,
    loadPortfolio,
  );
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const activeSection = useActiveRailSection(pathname);
  const rootStyle = {
    ...getKineticMaterialStyle(precision),
    "--rey-accent": precision.accentColor,
    "--rey-background": precision.backgroundColor,
    "--rey-foreground": precision.foregroundColor,
    "--rey-radius": `${precision.radius}px`,
  } as CSSProperties;
  return (
    <PortfolioContext.Provider value={portfolio}>
      <div
        className={sx(
          styles.environment,
          pathname.startsWith("/explore") && styles.environmentViewport,
        )}
        data-theme="precision"
        style={rootStyle}
      >
        <header
          className={sx(
            styles.topbar,
            pathname.startsWith("/explore") && styles.viewportFixed,
          )}
        >
          <Link className={sx(styles.focusable, styles.wordmark)} to="/explore">
            <span className={sx(styles.wordmarkMark)}>R</span>
            <span className={sx(styles.wordmarkText)}>
              <strong className={sx(styles.wordmarkStrong)}>REY</strong>
              <small className={sx(styles.micro, styles.wordmarkSmall)}>
                DIFF-DIRECTED RUNTIME
              </small>
            </span>
          </Link>
          <nav
            aria-label="Primary navigation"
            className={sx(styles.primaryNav)}
          >
            <Link
              activeProps={{ "aria-current": "page" }}
              className={sx(
                styles.focusable,
                styles.navLink,
                pathname.startsWith("/explore") && styles.navLinkActive,
              )}
              to="/explore"
            >
              Explore
            </Link>
            <Link
              activeProps={{ "aria-current": "page" }}
              className={sx(
                styles.focusable,
                styles.navLink,
                pathname.startsWith("/cadence") && styles.navLinkActive,
              )}
              to="/cadence"
            >
              Cadence
            </Link>
            <Link
              activeProps={{ "aria-current": "page" }}
              className={sx(
                styles.focusable,
                styles.navLink,
                pathname.startsWith("/agents") && styles.navLinkActive,
              )}
              to="/agents"
            >
              Agents
            </Link>
            <Link
              activeProps={{ "aria-current": "page" }}
              className={sx(
                styles.focusable,
                styles.navLink,
                pathname.startsWith("/workloads") && styles.navLinkActive,
              )}
              to="/workloads"
            >
              Workloads
            </Link>
            <Link
              activeProps={{ "aria-current": "page" }}
              className={sx(
                styles.focusable,
                styles.navLink,
                pathname.startsWith("/environment") && styles.navLinkActive,
              )}
              to="/environment"
            >
              Environment
            </Link>
          </nav>
        </header>

        <div
          className={sx(
            styles.micro,
            styles.coordinateRail,
            pathname.startsWith("/explore") && styles.viewportFixed,
          )}
          aria-hidden="true"
        >
          <span>00</span>
          <i className={sx(styles.railLine)} />
          <span>{activeSection ?? routeCoordinate(pathname)}</span>
          <i className={sx(styles.railLine)} />
          <span>{new Date().getUTCFullYear()}</span>
        </div>

        <Outlet />

        <footer
          className={sx(
            styles.micro,
            styles.footer,
            pathname.startsWith("/explore") && styles.viewportFixed,
          )}
        >
          <span>
            HIFI / {kineticGrammar.label.toUpperCase()} /{" "}
            {precisionTheme?.label.toUpperCase()}
          </span>
          <span>TICK → GRAPH → SCENARIO → DELTA → ATTENTION</span>
          <ImplementationLink
            repository={portfolio.ui_server.source_repository}
            revision={portfolio.ui_server.implementation_revision}
          />
        </footer>
      </div>
    </PortfolioContext.Provider>
  );
}

function EnvironmentPage() {
  const initialStatus = environmentRoute.useLoaderData();
  const { document: status, error: refreshError } = usePassiveDocument(
    initialStatus,
    loadEnvironment,
  );
  const projection = status.operator;
  const variableLines = environmentVariableDiff(projection.variables);
  const found = currentApplications(projection.applications, "available");
  const notFound = currentApplications(projection.applications, "unavailable");
  const errors = currentApplications(projection.applications, "error");
  const removed = projection.applications.filter(
    (application) => application.working === null && application.head !== null,
  );

  return (
    <main className={sx(styles.page, styles.environmentPage)}>
      <section
        className={sx(styles.environmentDiffSurface)}
        data-rey-section="01 / DIRECTED TEXT"
      >
        <div className={sx(styles.environmentPanelHeader)}>
          <div>
            <p className={sx(styles.micro, styles.sectionKicker)}>
              01 / DIRECTED TEXT
            </p>
            <h1 className={sx(styles.sectionTitle)}>Environment variables</h1>
          </div>
          <div className={sx(styles.environmentPanelMeta)}>
            <span className={sx(styles.micro, styles.muted)}>
              @@ {projection.source_label} → {projection.target_label} ·{" "}
              {status.state}
            </span>
            <span
              className={sx(
                styles.micro,
                refreshError
                  ? styles.toneWarning
                  : projection.complete
                    ? styles.stateGood
                    : styles.toneDanger,
              )}
              title={refreshError?.message}
            >
              {projection.mapping?.source_path ?? "NO MAP"} ·{" "}
              {projection.mapping?.schema ?? "UNMAPPED"} ·{" "}
              {refreshError
                ? "REVALIDATION DELAYED"
                : projection.complete
                  ? "COMPLETE"
                  : "INCOMPLETE"}
            </span>
            <span className={sx(styles.micro, styles.muted)}>
              {status.staged_delta.changes.length} STAGED ·{" "}
              {status.unstaged_delta.changes.length} WORKING
            </span>
          </div>
        </div>
        <div
          className={sx(styles.environmentDiffDocument)}
          role="table"
          aria-label="Environment variable diff"
        >
          {variableLines.length === 0 ? (
            <div className={sx(styles.environmentDiffEmpty)}>
              NO VARIABLES SELECTED BY THE ENVIRONMENT MAP
            </div>
          ) : (
            variableLines.map((line, index) => (
              <div
                className={sx(
                  styles.environmentDiffRow,
                  line.kind === "inserted" && styles.environmentDiffInserted,
                  line.kind === "deleted" && styles.environmentDiffDeleted,
                  line.kind === "context" && styles.environmentDiffContext,
                )}
                key={`${line.key}:${index}`}
                role="row"
              >
                <span className={sx(styles.environmentLineNumber)}>
                  {String(index + 1).padStart(2, "0")}
                </span>
                <span className={sx(styles.environmentDiffMarker)}>
                  {line.kind === "inserted"
                    ? "+"
                    : line.kind === "deleted"
                      ? "−"
                      : "·"}
                </span>
                <code className={sx(styles.environmentDiffCode)}>
                  {line.text}
                </code>
                <span className={sx(styles.environmentAdmissionTag)}>
                  {line.admission}
                </span>
              </div>
            ))
          )}
        </div>
      </section>

      <section
        className={sx(styles.environmentApplications)}
        data-rey-section="02 / BOUNDED SEARCH"
      >
        <div className={sx(styles.environmentPanelHeader)}>
          <div>
            <p className={sx(styles.micro, styles.sectionKicker)}>
              02 / BOUNDED SEARCH
            </p>
            <h2 className={sx(styles.sectionTitle)}>Applications</h2>
          </div>
          <span className={sx(styles.micro, styles.muted)}>
            {projection.summary.applications_searched} SEARCHED ·{" "}
            {projection.summary.applications_found} FOUND ·{" "}
            {projection.summary.applications_not_found} NOT FOUND
          </span>
        </div>
        <ApplicationGroup label="FOUND" applications={found} tone="found" />
        <ApplicationGroup
          label="SEARCHED, NOT FOUND"
          applications={notFound}
          tone="missing"
        />
        {errors.length > 0 ? (
          <ApplicationGroup
            label="OBSERVATION ERRORS"
            applications={errors}
            tone="error"
          />
        ) : null}
        {removed.length > 0 ? (
          <ApplicationGroup
            label="NO LONGER SEARCHED"
            applications={removed}
            tone="removed"
          />
        ) : null}
      </section>

      <section
        className={sx(styles.environmentTopology)}
        data-rey-section="03 / REFERENCE PLANE"
      >
        <div className={sx(styles.environmentPanelHeader)}>
          <div className={sx(styles.environmentReferenceHeading)}>
            <span className={sx(styles.sectionIndex)}>03</span>
            <div>
              <p className={sx(styles.micro, styles.sectionKicker)}>
                REFERENCE PLANE
              </p>
              <h2 className={sx(styles.sectionTitle)}>Inputs and topology</h2>
            </div>
          </div>
          <span className={sx(styles.micro, styles.muted)}>
            {projection.summary.inputs} INPUTS · {projection.summary.references}{" "}
            DECLARED EDGES
          </span>
        </div>
        <div className={sx(styles.environmentTopologyGrid)}>
          <div className={sx(styles.environmentInputList)}>
            {projection.inputs.map((input) => {
              const observation = input.working ?? input.head;
              if (!observation) return null;
              return (
                <div
                  className={sx(styles.environmentInputRow)}
                  key={input.object_id}
                >
                  <span
                    className={sx(
                      styles.environmentPresence,
                      observation.availability === "available"
                        ? styles.stateGood
                        : input.working === null
                          ? styles.toneDanger
                          : styles.toneWarning,
                    )}
                  >
                    {input.working === null
                      ? "−"
                      : observation.availability === "available"
                        ? "+"
                        : "?"}
                  </span>
                  <strong>{observation.path}</strong>
                  <span className={sx(styles.micro, styles.muted)}>
                    {observation.required ? "REQUIRED" : "OPTIONAL"} ·{" "}
                    {observation.byte_length ?? 0} BYTES ·{" "}
                    {input.changes.head_to_working.toUpperCase()}
                  </span>
                </div>
              );
            })}
          </div>
          <div className={sx(styles.environmentReferenceList)}>
            {projection.references.map((reference) => {
              const observation = reference.working ?? reference.head;
              if (!observation) return null;
              return (
                <div
                  className={sx(styles.environmentReferenceRow)}
                  key={reference.object_id}
                >
                  <span>{observation.from}</span>
                  <strong>
                    {observation.relation.replaceAll("_", " ")} /{" "}
                    {reference.changes.head_to_working.toUpperCase()}
                  </strong>
                  <span>{observation.to}</span>
                </div>
              );
            })}
          </div>
        </div>
      </section>
    </main>
  );
}

function ApplicationGroup({
  label,
  applications,
  tone,
}: {
  label: string;
  applications: EnvironmentObjectStatus<EnvironmentApplicationObservation>[];
  tone: "found" | "missing" | "error" | "removed";
}) {
  return (
    <div className={sx(styles.environmentApplicationGroup)}>
      <div className={sx(styles.environmentApplicationGroupHeader)}>
        <span className={sx(styles.micro)}>{label}</span>
        <strong>{String(applications.length).padStart(2, "0")}</strong>
      </div>
      {applications.length === 0 ? (
        <p className={sx(styles.micro, styles.muted)}>NONE</p>
      ) : (
        applications.map((application) => {
          const observation = application.working ?? application.head;
          if (!observation) return null;
          return (
            <div
              className={sx(styles.environmentApplicationRow)}
              key={application.object_id}
            >
              <span
                className={sx(
                  styles.environmentApplicationMarker,
                  tone === "found" && styles.stateGood,
                  tone === "missing" && styles.toneWarning,
                  (tone === "error" || tone === "removed") && styles.toneDanger,
                )}
              >
                {tone === "found"
                  ? "+"
                  : tone === "missing"
                    ? "?"
                    : tone === "removed"
                      ? "−"
                      : "!"}
              </span>
              <div className={sx(styles.environmentApplicationIdentity)}>
                <strong>{observation.name}</strong>
                <code>{observation.resolved_path ?? "NOT RESOLVED"}</code>
                <small className={sx(styles.micro, styles.muted)}>
                  {observation.searched_path_count} PATH ENTRIES ·{" "}
                  {application.changes.head_to_working.toUpperCase()}
                </small>
              </div>
              <span className={sx(styles.micro, styles.muted)}>
                {observation.required ? "REQUIRED" : "OPTIONAL"}
              </span>
              {observation.potential_capabilities.length > 0 ? (
                <p className={sx(styles.environmentCapabilityList)}>
                  {observation.potential_capabilities.join(" · ")}
                </p>
              ) : null}
            </div>
          );
        })
      )}
    </div>
  );
}

function WorkloadsPage() {
  const portfolio = usePortfolio();

  return (
    <main className={sx(styles.page)}>
      <section className={sx(styles.pageHeader)}>
        <div>
          <p className={sx(styles.micro, styles.eyebrow)}>
            CATALOG / {portfolio.catalog.root ?? "COMPILED"}
          </p>
          <h1 className={sx(styles.displayTitle, styles.pageTitle)}>
            WORKLOAD PORTFOLIO
          </h1>
        </div>
        <div className={sx(styles.catalogGauge)}>
          <span className={sx(styles.micro)}>ADMISSION</span>
          <strong className={sx(styles.catalogValue)}>
            {portfolio.catalog.admitted_count}
          </strong>
          <small className={sx(styles.micro, styles.muted)}>
            accepted / {portfolio.catalog.draft_count} awaiting harness
          </small>
        </div>
      </section>

      <section
        className={sx(styles.sectionSpacing)}
        aria-labelledby="admitted-heading"
        data-rey-section="01 / EXECUTABLE"
      >
        <SectionHeading
          index="01"
          kicker="EXECUTABLE"
          title="Admitted graphs"
          detail={`${portfolio.workloads.length} exact workload revisions`}
        />
        {portfolio.workloads.length === 0 ? (
          <EmptySurface>NO ADMITTED WORKLOAD PACKAGES</EmptySurface>
        ) : (
          <div className={sx(styles.twoColumnGrid)}>
            {portfolio.workloads.map((workload) => (
              <WorkloadCard key={workload.workload.id} workload={workload} />
            ))}
          </div>
        )}
      </section>

      <section
        className={sx(styles.sectionSpacing)}
        aria-labelledby="draft-heading"
        data-rey-section="02 / AGENTIC HANDOFF"
      >
        <SectionHeading
          index="02"
          kicker="AGENTIC HANDOFF"
          title="Creation requests"
          detail={`${portfolio.drafts.length} request-only catalog entries`}
        />
        {portfolio.drafts.length === 0 ? (
          <EmptySurface>NO WORKLOADS AWAITING CODING HARNESS</EmptySurface>
        ) : (
          <div className={sx(styles.twoColumnGrid)}>
            {portfolio.drafts.map((draft) => (
              <DraftCard draft={draft} key={draft.request.workload_id} />
            ))}
          </div>
        )}
      </section>
    </main>
  );
}

function WorkloadDetailPage() {
  const portfolio = usePortfolio();
  const { workloadId } = workloadDetailRoute.useParams();
  const workload = portfolio.workloads.find(
    (candidate) => candidate.workload.id === workloadId,
  );
  const draft = portfolio.drafts.find(
    (candidate) => candidate.request.workload_id === workloadId,
  );

  if (draft) {
    return (
      <main className={sx(styles.page)}>
        <BackLink />
        <p className={sx(styles.micro, styles.eyebrow)}>
          CREATION REQUEST / {shortDigest(draft.request.request_id)}
        </p>
        <h1 className={sx(styles.displayTitle, styles.pageTitle)}>
          {draft.request.workload_id}
        </h1>
        <div className={sx(styles.detailGrid)}>
          <DetailPanel label="JOURNEY" value="HYDRATE" accent />
          <DetailPanel label="ADMISSION" value="AWAITING CODING HARNESS" />
          <DetailPanel label="GRAPH" value="MISSING" />
          <DetailPanel label="SCENARIO ORACLE" value="NOT ADMITTED" />
        </div>
        <section
          className={sx(styles.sectionSpacing, styles.evidenceSurface)}
          data-rey-section="01 / REQUEST BINDING"
        >
          <h2 className={sx(styles.sectionTitle, styles.panelTitle)}>
            REQUEST BINDING
          </h2>
          <Definition label="Purpose" value={draft.request.title} />
          <Definition
            label="Intent"
            value={draft.request.intent ?? "not supplied"}
          />
          <Definition label="Request" value={draft.request.request_id} mono />
          <Definition label="Source" value={draft.source} mono />
          <Definition
            label="Target"
            value={draft.request.target_package}
            mono
          />
        </section>
      </main>
    );
  }

  if (!workload) return <NotFoundPage />;

  const percent = scenarioPercent(workload.passed, workload.required);
  return (
    <main className={sx(styles.page)}>
      <BackLink />
      <p className={sx(styles.micro, styles.eyebrow)}>
        WORKLOAD / REVISION {workload.workload.revision}
      </p>
      <h1 className={sx(styles.displayTitle, styles.pageTitle)}>
        {workload.workload.id}
      </h1>
      <p className={sx(styles.detailTitle)}>{workload.title}</p>
      <div className={sx(styles.detailGrid)}>
        <DetailPanel label="JOURNEY" value={workloadJourney(workload)} accent />
        <DetailPanel
          label="QUALIFICATION"
          value={workload.qualification.toUpperCase()}
        />
        <DetailPanel
          label="SCENARIOS"
          value={`${workload.passed}/${workload.required} · ${percent}%`}
        />
        <DetailPanel
          label="LAST RUN"
          value={(workload.last_run_status ?? "NOT RUN").toUpperCase()}
        />
      </div>
      <section
        className={sx(styles.sectionSpacing, styles.evidenceSurface)}
        data-rey-section="01 / LOCAL CONFORMANCE"
      >
        <div className={sx(styles.conformanceHeading)}>
          <span>LOCAL CONFORMANCE</span>
          <strong className={sx(styles.conformanceValue)}>{percent}%</strong>
        </div>
        <div
          className={sx(styles.progressTrack, styles.conformanceTrack)}
          aria-label={`${percent}% scenarios passing`}
        >
          <i
            className={sx(styles.progressFill, styles.conformanceFill)}
            style={{ width: `${percent}%` }}
          />
        </div>
        <div className={sx(styles.conformanceMeta)}>
          <span>{workload.passed} passing</span>
          <span>{workload.failed} failing</span>
          <span>{workload.inconclusive} inconclusive</span>
          <span>{workload.stale} stale</span>
          <span>{workload.optional} optional</span>
        </div>
      </section>
      <section
        className={sx(styles.sectionSpacing, styles.evidenceSurface)}
        data-rey-section="02 / EXACT BINDINGS"
      >
        <h2 className={sx(styles.sectionTitle, styles.panelTitle)}>
          EXACT BINDINGS
        </h2>
        <Definition
          label="Workload"
          value={workload.workload.semantic_digest}
          mono
        />
        <Definition
          label="Candidate graph"
          value={`${workload.candidate_graph.id}@${workload.candidate_graph.revision} · ${workload.candidate_graph.semantic_digest}`}
          mono
        />
        <Definition
          label="Package"
          value={workload.provenance?.source ?? "compiled"}
          mono
        />
        <Definition
          label="Package revision"
          value={workload.provenance?.source_digest ?? "compiled"}
          mono
        />
        <Definition
          label="Test evidence"
          value={workload.last_test_result_id ?? "none"}
          mono
        />
      </section>
      <section
        className={sx(
          styles.sectionSpacing,
          styles.evidenceSurface,
          styles.miningPanel,
        )}
        data-rey-section="03 / MINING / EVIDENCE"
      >
        <h2
          className={sx(
            styles.sectionTitle,
            styles.panelTitle,
            styles.miningTitle,
          )}
        >
          MINING / EVIDENCE
        </h2>
        <MetricCell label="OPERATIONS" value={workload.mining_operations} />
        <MetricCell label="RESULTS" value={workload.mining_results} />
        <MetricCell
          label="INCOMPLETE"
          value={workload.incomplete_mining_results}
        />
        <MetricCell label="DELTAS" value={workload.relation_deltas} />
        <MetricCell label="SURFACES" value={workload.reasoning_surfaces} />
      </section>
    </main>
  );
}

function WorkloadCard({ workload }: { workload: WorkloadSummary }) {
  const percent = scenarioPercent(workload.passed, workload.required);
  const progressStyle =
    workload.qualification === "failing" ||
    workload.qualification === "inconclusive"
      ? styles.progressFailure
      : workload.qualification === "stale"
        ? styles.progressStale
        : undefined;
  return (
    <Link
      className={sx(styles.focusable, styles.workloadCard)}
      params={{ workloadId: workload.workload.id }}
      to="/workloads/$workloadId"
    >
      <div className={sx(styles.micro, styles.cardIndex)}>
        <span className={sx(styles.cardJourney)}>
          {workloadJourney(workload)}
        </span>
        <i className={sx(styles.cardLine)} />
        <span>R{workload.workload.revision}</span>
      </div>
      <h3 className={sx(styles.cardTitle)}>{workload.workload.id}</h3>
      <p className={sx(styles.cardDescription)}>{workload.title}</p>
      <div className={sx(styles.cardProgress)}>
        <div className={sx(styles.progressTrack)}>
          <i
            className={sx(styles.progressFill, progressStyle)}
            style={{ width: `${percent}%` }}
          />
        </div>
        <strong className={sx(styles.progressPercent)}>{percent}%</strong>
      </div>
      <dl className={sx(styles.cardDefinitions)}>
        <div className={sx(styles.cardDefinition)}>
          <dt className={sx(styles.cardTerm)}>SCENARIOS</dt>
          <dd className={sx(styles.cardValue)}>
            {workload.passed}/{workload.required}
          </dd>
        </div>
        <div className={sx(styles.cardDefinition)}>
          <dt className={sx(styles.cardTerm)}>GRAPH</dt>
          <dd className={sx(styles.cardValue)}>
            {workload.candidate_graph.id}@{workload.candidate_graph.revision}
          </dd>
        </div>
        <div className={sx(styles.cardDefinition)}>
          <dt className={sx(styles.cardTerm)}>EVIDENCE</dt>
          <dd className={sx(styles.cardValue)}>
            {shortDigest(workload.last_test_result_id)}
          </dd>
        </div>
      </dl>
      <span className={sx(styles.controlLabel, styles.cardOpen)}>
        OPEN MECHANISM →
      </span>
    </Link>
  );
}

function DraftCard({ draft }: { draft: WorkloadDraft }) {
  return (
    <Link
      className={sx(styles.focusable, styles.workloadCard, styles.draftCard)}
      params={{ workloadId: draft.request.workload_id }}
      to="/workloads/$workloadId"
    >
      <div className={sx(styles.micro, styles.cardIndex)}>
        <span className={sx(styles.cardJourney)}>HYDRATE</span>
        <i className={sx(styles.cardLine)} />
        <span>DRAFT</span>
      </div>
      <h3 className={sx(styles.cardTitle)}>{draft.request.workload_id}</h3>
      <p className={sx(styles.cardDescription)}>
        {draft.request.intent ?? draft.request.title}
      </p>
      <div
        className={sx(styles.draftMechanism)}
        aria-label="Graph and scenario admission pending"
      >
        <span>REQUEST</span>
        <i className={sx(styles.draftLine)} />
        <span>GRAPH</span>
        <i className={sx(styles.draftLine)} />
        <span>ORACLE</span>
      </div>
      <dl className={sx(styles.cardDefinitions)}>
        <div className={sx(styles.cardDefinition)}>
          <dt className={sx(styles.cardTerm)}>ADMISSION</dt>
          <dd className={sx(styles.cardValue)}>AWAITING HARNESS</dd>
        </div>
        <div className={sx(styles.cardDefinition)}>
          <dt className={sx(styles.cardTerm)}>REQUEST</dt>
          <dd className={sx(styles.cardValue)}>
            {shortDigest(draft.request.request_id)}
          </dd>
        </div>
        <div className={sx(styles.cardDefinition)}>
          <dt className={sx(styles.cardTerm)}>TARGET</dt>
          <dd className={sx(styles.cardValue)}>
            {draft.request.target_package}
          </dd>
        </div>
      </dl>
      <span className={sx(styles.controlLabel, styles.cardOpen)}>
        INSPECT HANDOFF →
      </span>
    </Link>
  );
}

function MetricCell({
  label,
  value,
  tone,
}: {
  label: string;
  value: ReactNode;
  tone?: "danger" | "warning";
}) {
  return (
    <div className={sx(styles.metricCell)}>
      <span className={sx(styles.micro)}>{label}</span>
      <strong
        className={sx(
          styles.metricCellValue,
          tone === "danger" && styles.toneDanger,
          tone === "warning" && styles.toneWarning,
        )}
      >
        {value}
      </strong>
    </div>
  );
}

function SectionHeading({
  index,
  kicker,
  title,
  detail,
}: {
  index: string;
  kicker: string;
  title: string;
  detail: string;
}) {
  return (
    <header className={sx(styles.sectionHeading)}>
      <span className={sx(styles.sectionIndex)}>{index}</span>
      <div>
        <p className={sx(styles.micro, styles.sectionKicker)}>{kicker}</p>
        <h2 className={sx(styles.sectionTitle)}>{title}</h2>
      </div>
      <small className={sx(styles.micro, styles.sectionDetail)}>{detail}</small>
    </header>
  );
}

function DetailPanel({
  label,
  value,
  accent = false,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <article className={sx(styles.detailPanel)}>
      <span className={sx(styles.micro)}>{label}</span>
      <strong className={sx(styles.detailPanelValue, accent && styles.accent)}>
        {value}
      </strong>
    </article>
  );
}

function Definition({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className={sx(styles.definition)}>
      <span className={sx(styles.micro)}>{label}</span>
      <strong className={sx(styles.definitionValue, mono && styles.monoValue)}>
        {value}
      </strong>
    </div>
  );
}

function EmptySurface({ children }: { children: ReactNode }) {
  return (
    <div className={sx(styles.emptySurface)}>
      <i className={sx(styles.emptyLine)} />
      {children}
      <i className={sx(styles.emptyLine)} />
    </div>
  );
}

function ImplementationLink({
  repository,
  revision,
}: {
  repository: string;
  revision: string;
}) {
  const href = sourceCommitUrl(repository, revision);
  if (!href) return <span>SOURCE REVISION UNKNOWN</span>;
  return (
    <a
      className={sx(styles.focusable, styles.footerLink)}
      href={href}
      rel="noreferrer"
      target="_blank"
      title={`Open Rey commit ${revision}`}
    >
      {shortDigest(revision)}
    </a>
  );
}

function BackLink() {
  return (
    <Link
      className={sx(styles.focusable, styles.micro, styles.backLink)}
      to="/workloads"
    >
      ← WORKLOAD PORTFOLIO
    </Link>
  );
}

function PendingPage() {
  return (
    <main className={sx(styles.systemPage)}>
      <div className={sx(styles.loader)}>
        <i className={sx(styles.loaderSpinner)} />
        <span>CALIBRATING RUNTIME</span>
      </div>
    </main>
  );
}

function ErrorPage({ error }: { error: Error }) {
  return (
    <main className={sx(styles.systemPage)}>
      <p className={sx(styles.micro, styles.eyebrow)}>READ FAILURE</p>
      <h1 className={sx(styles.displayTitle, styles.systemTitle)}>
        RUNTIME PROJECTION UNAVAILABLE
      </h1>
      <pre className={sx(styles.systemError)}>{error.message}</pre>
    </main>
  );
}

function NotFoundPage() {
  return (
    <main className={sx(styles.systemPage)}>
      <p className={sx(styles.micro, styles.eyebrow)}>
        404 / UNRESOLVED COORDINATE
      </p>
      <h1 className={sx(styles.displayTitle, styles.systemTitle)}>
        NO SUCH SURFACE
      </h1>
      <Link
        className={sx(
          styles.focusable,
          styles.controlLabel,
          styles.mechanicalLink,
        )}
        to="/explore"
      >
        RETURN TO EXPLORE →
      </Link>
    </main>
  );
}

function ExploreRoutePage() {
  return <ExplorePage portfolio={usePortfolio()} />;
}

function ExploreCoordinateRoutePage() {
  const portfolio = usePortfolio();
  const { coordinate: segment, kind } = exploreCoordinateRoute.useParams();
  const coordinate = parseExplorerCoordinate(kind, segment);
  if (!coordinate) return <NotFoundPage />;
  return (
    <ExplorePage
      coordinate={resolveExplorerCoordinate(portfolio, coordinate)}
      portfolio={portfolio}
    />
  );
}

function CadenceRoutePage() {
  const initialCadence = cadenceRoute.useLoaderData();
  const { document: cadence } = usePassiveDocument(initialCadence, loadCadence);
  return <CadencePage cadence={cadence} />;
}

function AgentsRoutePage() {
  return <AgentsPage portfolio={usePortfolio()} />;
}

const rootRoute = createRootRoute({
  component: RootLayout,
  loader: loadPortfolio,
  pendingComponent: PendingPage,
  errorComponent: ({ error }) => <ErrorPage error={error} />,
  notFoundComponent: NotFoundPage,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/explore" });
  },
});

const exploreRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "explore",
  component: ExploreRoutePage,
});

const exploreCoordinateRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "explore/$kind/$coordinate",
  component: ExploreCoordinateRoutePage,
});

const cadenceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "cadence",
  loader: loadCadence,
  component: CadenceRoutePage,
});

const agentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "agents",
  component: AgentsRoutePage,
});

const environmentRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "environment",
  loader: loadEnvironment,
  component: EnvironmentPage,
});

const workloadsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "workloads",
  component: WorkloadsPage,
});

const workloadDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "workloads/$workloadId",
  component: WorkloadDetailPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  exploreRoute,
  exploreCoordinateRoute,
  cadenceRoute,
  agentsRoute,
  environmentRoute,
  workloadsRoute,
  workloadDetailRoute,
]);

export const router = createRouter({
  defaultPendingMinMs: 180,
  defaultPreload: "intent",
  routeTree,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

function routeCoordinate(pathname: string): string {
  if (pathname.startsWith("/explore")) return "EXPLORE / CONTEXT TOPOLOGY";
  if (pathname.startsWith("/cadence")) return "CADENCE / TICKS";
  if (pathname.startsWith("/agents")) return "AGENTS / PROVENANCE";
  if (pathname.startsWith("/environment")) return "ENVIRONMENT";
  if (pathname.startsWith("/workloads")) return "WORKLOADS";
  return "UNRESOLVED";
}

function useActiveRailSection(pathname: string): string | null {
  const [active, setActive] = useState<{
    pathname: string;
    section: string | null;
  }>({ pathname, section: null });

  useEffect(() => {
    let frame: number | null = null;
    const update = () => {
      frame = null;
      const sections = Array.from(
        document.querySelectorAll<HTMLElement>(`[${SECTION_RAIL_ATTRIBUTE}]`),
        (section) => ({
          label: section.dataset.reySection ?? "",
          top: section.getBoundingClientRect().top,
        }),
      ).filter((section) => section.label.length > 0);
      const section = activeSectionAt(sections, 105);
      setActive((current) =>
        current.pathname === pathname && current.section === section
          ? current
          : { pathname, section },
      );
    };
    const schedule = () => {
      if (frame === null) frame = window.requestAnimationFrame(update);
    };

    schedule();
    window.addEventListener("resize", schedule);
    window.addEventListener("scroll", schedule, { passive: true });
    return () => {
      if (frame !== null) window.cancelAnimationFrame(frame);
      window.removeEventListener("resize", schedule);
      window.removeEventListener("scroll", schedule);
    };
  }, [pathname]);

  return active.pathname === pathname ? active.section : null;
}
