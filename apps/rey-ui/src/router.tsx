import {
  KineticSurface,
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
  useRouter,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect, type CSSProperties, type ReactNode } from "react";
import { loadPortfolio } from "./api";
import {
  derivePortfolioMetrics,
  scenarioPercent,
  shortDigest,
  sourceCommitUrl,
  workloadJourney,
  type AttentionRow,
  type WorkloadDraft,
  type WorkloadSummary,
} from "./domain";
import { ExplorePage } from "./explore";
import { environmentStyles as styles } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";

const precision = kineticThemeMaterials.precision;
const precisionTheme = kineticGrammar.themes.find(
  (theme) => theme.name === "precision",
);

function RootLayout() {
  const portfolio = rootRoute.useLoaderData();
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const router = useRouter();
  useEffect(() => {
    const interval = window.setInterval(() => {
      void router.invalidate();
    }, 5_000);
    return () => window.clearInterval(interval);
  }, [router]);
  const rootStyle = {
    ...getKineticMaterialStyle(precision),
    "--rey-accent": precision.accentColor,
    "--rey-background": precision.backgroundColor,
    "--rey-foreground": precision.foregroundColor,
    "--rey-radius": `${precision.radius}px`,
  } as CSSProperties;

  return (
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
        <nav aria-label="Primary navigation" className={sx(styles.primaryNav)}>
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
              pathname.startsWith("/environment") && styles.navLinkActive,
            )}
            to="/environment"
          >
            Environment
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
            Workloads{" "}
            <span className={sx(styles.navCount)}>
              {portfolio.catalog.workload_count}
            </span>
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
        <span>{routeCoordinate(pathname)}</span>
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
        <span>GRAPH → SCENARIO → DELTA → ATTENTION</span>
        <ImplementationLink
          repository={portfolio.ui_server.source_repository}
          revision={portfolio.ui_server.implementation_revision}
        />
      </footer>
    </div>
  );
}

function EnvironmentPage() {
  const portfolio = rootRoute.useLoaderData();
  const metrics = derivePortfolioMetrics(portfolio);
  const attentionTotal = portfolio.attention.rows.length;

  return (
    <main className={sx(styles.page)}>
      <section className={sx(styles.heroGrid)}>
        <div>
          <p className={sx(styles.micro, styles.eyebrow)}>
            ENVIRONMENT / PORTFOLIO CONTROL
          </p>
          <h1 className={sx(styles.displayTitle, styles.heroTitle)}>
            DELTA
            <span className={sx(styles.heroTitleAccent)}>DIRECTS WORK.</span>
          </h1>
          <p className={sx(styles.heroIntro)}>
            A live, bounded projection of workload qualification, scenario
            conformance, retained evidence, and the attention frontier.
          </p>
          <div className={sx(styles.heroLinks)}>
            <Link
              className={sx(
                styles.focusable,
                styles.controlLabel,
                styles.mechanicalLink,
              )}
              to="/workloads"
            >
              OPEN WORKLOAD PORTFOLIO{" "}
              <span className={sx(styles.mechanicalArrow)} aria-hidden="true">
                →
              </span>
            </Link>
            <span className={sx(styles.controlLabel)}>
              {portfolio.catalog.kind.replaceAll("_", " ").toUpperCase()}
            </span>
          </div>
        </div>

        <KineticSurface
          className={sx(styles.portfolioMachine)}
          theme="precision"
        >
          <div className={sx(styles.machineScale)} aria-hidden="true">
            {Array.from({ length: 13 }, (_, index) => (
              <i className={sx(styles.machineScaleTick)} key={index} />
            ))}
          </div>
          <div className={sx(styles.machineReadout)}>
            <span className={sx(styles.micro)}>
              ATTENTION / READY + BLOCKED
            </span>
            <output className={sx(styles.machineOutput)}>
              {String(attentionTotal).padStart(3, "0")}
            </output>
            <small className={sx(styles.micro)}>
              {portfolio.attention.rows.length === 0
                ? "CONVERGED SURFACE"
                : "UNRESOLVED SURFACE"}
            </small>
          </div>
          <div className={sx(styles.machineStatusGrid)}>
            <MetricCell
              label="QUALIFIED"
              value={`${metrics.qualified}/${metrics.admitted}`}
            />
            <MetricCell label="DRAFT" value={metrics.drafts} />
            <MetricCell label="FAILING" value={metrics.failing} tone="danger" />
            <MetricCell label="STALE" value={metrics.stale} tone="warning" />
          </div>
          <div className={sx(styles.micro, styles.machinePlate)}>
            <span>m / {precision.mass.toFixed(2)} kg</span>
            <span>k / {precision.stiffness.toFixed(0)} N·m</span>
            <span>ζ / {precision.damping.toFixed(0)}</span>
          </div>
        </KineticSurface>
      </section>

      <section
        className={sx(styles.metricStrip)}
        aria-label="Portfolio metrics"
      >
        <MetricPanel
          index="01"
          label="INVENTORY"
          primary={metrics.total}
          secondary={`${metrics.admitted} admitted · ${metrics.drafts} draft`}
        />
        <MetricPanel
          index="02"
          label="SCENARIOS"
          primary={`${metrics.scenariosPassed}/${metrics.scenariosRequired}`}
          secondary="required branches passing"
        />
        <MetricPanel
          index="03"
          label="RUNS"
          primary={metrics.runsPassed}
          secondary={`${metrics.runsBlocked} blocked · ${metrics.runsPending} pending`}
        />
        <MetricPanel
          index="04"
          label="COVERAGE"
          primary={`${portfolio.attention.summary.owned_surfaces}/${portfolio.attention.summary.surfaces}`}
          secondary={`${portfolio.attention.summary.unowned_surfaces} unowned surfaces`}
        />
      </section>

      <section
        className={sx(styles.sectionSpacing)}
        aria-labelledby="attention-heading"
      >
        <SectionHeading
          index="01"
          kicker="OUTER LOOP"
          title="Attention frontier"
          detail={`${portfolio.attention.rows.length} retained rows / priority ordered`}
        />
        {portfolio.attention.rows.length === 0 ? (
          <EmptySurface>NO UNRESOLVED PORTFOLIO ATTENTION</EmptySurface>
        ) : (
          <div
            className={sx(styles.attentionTable)}
            role="table"
            aria-label="Attention frontier"
          >
            <div
              className={sx(styles.attentionGrid, styles.attentionHeader)}
              role="row"
            >
              <span role="columnheader">ACTION</span>
              <span role="columnheader">SUBJECT</span>
              <span role="columnheader">REASON</span>
              <span role="columnheader">READINESS</span>
              <span role="columnheader">P / C</span>
            </div>
            {portfolio.attention.rows.map((row) => (
              <AttentionLine key={row.row_id} row={row} />
            ))}
          </div>
        )}
      </section>

      <section
        className={sx(styles.sectionSpacing)}
        aria-labelledby="workload-preview-heading"
      >
        <SectionHeading
          index="02"
          kicker="INNER LOOPS"
          title="Workload mechanisms"
          detail={`${portfolio.catalog.admitted_count} admitted / ${portfolio.catalog.draft_count} awaiting harness`}
        />
        <div className={sx(styles.twoColumnGrid)}>
          {portfolio.workloads.slice(0, 4).map((workload) => (
            <WorkloadCard key={workload.workload.id} workload={workload} />
          ))}
          {portfolio.drafts.slice(0, 2).map((draft) => (
            <DraftCard draft={draft} key={draft.request.workload_id} />
          ))}
        </div>
      </section>
    </main>
  );
}

function WorkloadsPage() {
  const portfolio = rootRoute.useLoaderData();

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
  const portfolio = rootRoute.useLoaderData();
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
        <section className={sx(styles.sectionSpacing, styles.evidenceSurface)}>
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
      <section className={sx(styles.sectionSpacing, styles.evidenceSurface)}>
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
      <section className={sx(styles.sectionSpacing, styles.evidenceSurface)}>
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

function AttentionLine({ row }: { row: AttentionRow }) {
  return (
    <div
      className={sx(
        styles.attentionGrid,
        styles.attentionRow,
        row.readiness === "excluded" && styles.readinessExcluded,
      )}
      role="row"
    >
      <strong
        className={sx(styles.attentionText, styles.attentionAction)}
        role="cell"
      >
        {row.action.toUpperCase()}
      </strong>
      <span
        className={sx(styles.attentionText, styles.attentionSubject)}
        role="cell"
      >
        {row.subject_id}
      </span>
      <span
        className={sx(styles.attentionText, styles.attentionReason)}
        role="cell"
      >
        {row.reason.replaceAll("_", " ")}
      </span>
      <span className={sx(styles.attentionText)} role="cell">
        <i
          className={sx(
            styles.statusDot,
            row.readiness === "blocked" && styles.statusDotBlocked,
          )}
        />{" "}
        {row.readiness.toUpperCase()}
      </span>
      <code className={sx(styles.attentionText)} role="cell">
        {row.priority} / {row.estimated_cost_units}
      </code>
    </div>
  );
}

function MetricPanel({
  index,
  label,
  primary,
  secondary,
}: {
  index: string;
  label: string;
  primary: ReactNode;
  secondary: string;
}) {
  return (
    <article className={sx(styles.metricPanel)}>
      <span className={sx(styles.micro)}>
        {index} / {label}
      </span>
      <strong className={sx(styles.metricPanelValue)}>{primary}</strong>
      <small
        className={sx(styles.micro, styles.muted, styles.metricPanelDetail)}
      >
        {secondary}
      </small>
    </article>
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
        <span>CALIBRATING PORTFOLIO</span>
      </div>
    </main>
  );
}

function ErrorPage({ error }: { error: Error }) {
  return (
    <main className={sx(styles.systemPage)}>
      <p className={sx(styles.micro, styles.eyebrow)}>READ FAILURE</p>
      <h1 className={sx(styles.displayTitle, styles.systemTitle)}>
        PORTFOLIO UNAVAILABLE
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
  component: () => <ExplorePage portfolio={rootRoute.useLoaderData()} />,
});

const environmentRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "environment",
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
  if (pathname.startsWith("/environment")) return "ENVIRONMENT";
  if (pathname.startsWith("/workloads")) return "WORKLOADS";
  return "UNRESOLVED";
}
