import { getKineticMaterialStyle, kineticThemeMaterials } from "@hifi/kinetic";
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
} from "react";
import { AgentsPage } from "./agents";
import {
  admitJournalEntry,
  loadCadence,
  loadChannels,
  loadEnvironment,
  loadFeed,
  loadJournal,
  loadPortfolio,
  writeChannelWorking,
  type OperatorContext,
} from "./api";
import { CadencePage } from "./cadence";
import { ChannelsPage } from "./channels";
import { operatorMailboxRows, shortDigest } from "./domain";
import {
  operatorObservationMailboxRows,
  type ObservationMailboxRow,
} from "./observations";
import {
  currentApplications,
  environmentVariableDiff,
  type EnvironmentApplicationObservation,
  type EnvironmentObjectStatus,
} from "./environment";
import { ExplorePage } from "./explore";
import {
  channelWorkingWriteForFeedLayout,
  FeedPage,
  resolveFeedLayout,
  serializeFeedStreams,
} from "./feed";
import { GitCommitLink } from "./git-commit-link";
import {
  defaultJournalBinding,
  JournalDocumentPage,
  JournalNewPage,
  journalEntrySlug,
  resolveJournalEntry,
} from "./journal";
import { parseExplorerView, resolveExplorerView } from "./explorer-coordinate";
import { startPassiveRevalidation } from "./passive";
import { activeSectionAt, SECTION_RAIL_ATTRIBUTE } from "./section-rail";
import { environmentStyles as styles } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";
import {
  AdmittedWorkloadDetail,
  CandidateWorkloadDetail,
  DraftWorkloadDetail,
  WorkloadsPage,
} from "./workloads";

const precision = kineticThemeMaterials.precision;
const PortfolioContext = createContext<OperatorContext | null>(null);

export type CommunicationAxis = "mailbox" | "conversation";

export const PRIMARY_NAV_ITEMS = [
  { label: "Feed", to: "/feed", prefixes: ["/feed"] },
  { label: "Explore", to: "/explore", prefixes: ["/explore"] },
  { label: "Agents", to: "/agents", prefixes: ["/agents", "/journal"] },
  { label: "Cadence", to: "/cadence", prefixes: ["/cadence"] },
  { label: "Channels", to: "/channels", prefixes: ["/channels"] },
  { label: "Workloads", to: "/workloads", prefixes: ["/workloads"] },
  { label: "Environment", to: "/environment", prefixes: ["/environment"] },
] as const;

export function isViewportLockedPath(pathname: string): boolean {
  return pathname.startsWith("/explore") || pathname.startsWith("/feed");
}

export function normalizeFeedSearch(search: Record<string, unknown>): {
  streams?: string;
} {
  const streams = search.streams;
  return typeof streams === "string" && streams.length <= 4_096
    ? { streams }
    : {};
}

export function normalizeExplorerSearch(search: Record<string, unknown>): {
  coordinate?: string;
  scale?: string;
} {
  const coordinate = search.coordinate;
  const scale = search.scale;
  return {
    ...(typeof coordinate === "string" && coordinate.length <= 4_096
      ? { coordinate }
      : {}),
    ...(typeof scale === "string" && scale.length <= 64 ? { scale } : {}),
  };
}

export function activateCommunicationAxis(
  current: CommunicationAxis | null,
  requested: CommunicationAxis,
): CommunicationAxis | null {
  return current === requested ? null : requested;
}

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
  return { document, error, publish: setDocument };
}

function usePortfolio(): OperatorContext {
  const portfolio = useContext(PortfolioContext);
  if (!portfolio) throw new Error("portfolio context is unavailable");
  return portfolio;
}

function RootLayout() {
  const initialPortfolio = rootRoute.useLoaderData();
  const { document: portfolio, error: portfolioError } = usePassiveDocument(
    initialPortfolio,
    loadPortfolio,
  );
  const [communicationAxis, setCommunicationAxis] =
    useState<CommunicationAxis | null>(null);
  const mailboxCount =
    operatorMailboxRows(portfolio).length +
    operatorObservationMailboxRows(portfolio.observations).length;
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const viewportLocked = isViewportLockedPath(pathname);
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
          viewportLocked && styles.environmentViewport,
        )}
        data-theme="precision"
        style={rootStyle}
      >
        <header
          className={sx(styles.topbar, viewportLocked && styles.viewportFixed)}
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
            {PRIMARY_NAV_ITEMS.map((item) => {
              const active = item.prefixes.some((prefix) =>
                pathname.startsWith(prefix),
              );
              return (
                <Link
                  aria-current={active ? "page" : undefined}
                  className={sx(
                    styles.focusable,
                    styles.navLink,
                    active && styles.navLinkActive,
                  )}
                  key={item.to}
                  to={item.to}
                >
                  {item.label}
                </Link>
              );
            })}
          </nav>
        </header>

        <div
          className={sx(
            styles.micro,
            styles.coordinateRail,
            viewportLocked && styles.viewportFixed,
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

        <CommunicationPlane
          axis={communicationAxis}
          error={portfolioError}
          portfolio={portfolio}
          onClose={() => setCommunicationAxis(null)}
        />

        <footer
          className={sx(
            styles.micro,
            styles.footer,
            viewportLocked && styles.viewportFixed,
          )}
        >
          <button
            aria-controls="rey-communications"
            aria-expanded={communicationAxis === "mailbox"}
            aria-label={
              communicationAxis === "mailbox"
                ? "Close mailbox history"
                : "Open mailbox history"
            }
            className={sx(
              styles.focusable,
              styles.footerButton,
              styles.footerMailbox,
            )}
            onClick={() =>
              setCommunicationAxis((axis) =>
                activateCommunicationAxis(axis, "mailbox"),
              )
            }
            type="button"
          >
            <span>MAILBOX</span>
            <span
              className={sx(
                styles.mailboxCount,
                (mailboxCount > 0 || portfolioError) &&
                  styles.mailboxCountActive,
              )}
            >
              {mailboxCount + (portfolioError ? 1 : 0)}
            </span>
          </button>
          <button
            aria-controls="rey-communications"
            aria-expanded={communicationAxis === "conversation"}
            aria-label={
              communicationAxis === "conversation"
                ? "Close Rey conversation"
                : "Open Rey conversation"
            }
            className={sx(
              styles.focusable,
              styles.footerButton,
              styles.communicationToggle,
            )}
            onClick={() =>
              setCommunicationAxis((axis) =>
                activateCommunicationAxis(axis, "conversation"),
              )
            }
            type="button"
          >
            {communicationAxis === "conversation" ? "⌄ ⌄ ⌄" : "⌃ ⌃ ⌃"}
          </button>
          <ImplementationLink
            repository={portfolio.ui_server.source_repository}
            revision={portfolio.ui_server.implementation_revision}
          />
        </footer>
      </div>
    </PortfolioContext.Provider>
  );
}

function CommunicationPlane({
  axis,
  error,
  onClose,
  portfolio,
}: {
  axis: CommunicationAxis | null;
  error: Error | null;
  onClose: () => void;
  portfolio: OperatorContext;
}) {
  const [lastAxis, setLastAxis] = useState<CommunicationAxis>("mailbox");
  const open = axis !== null;
  const visibleAxis = axis ?? lastAxis;
  useEffect(() => {
    if (axis) setLastAxis(axis);
  }, [axis]);
  useEffect(() => {
    if (!open) return;
    const closeOnEscape = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose, open]);

  return (
    <>
      <CommunicationBackdrop onClose={onClose} open={open} />
      <aside
        aria-hidden={!open}
        aria-label={
          visibleAxis === "mailbox"
            ? "Mailbox history"
            : "Rey agent operator conversation"
        }
        className={sx(
          styles.communicationsPanel,
          open && styles.communicationsPanelOpen,
        )}
        data-communication-axis={visibleAxis}
        id="rey-communications"
      >
        {visibleAxis === "mailbox" ? (
          <MailboxHistory error={error} portfolio={portfolio} />
        ) : (
          <ConversationSurface />
        )}
      </aside>
    </>
  );
}

export function CommunicationBackdrop({
  onClose,
  open,
}: {
  onClose: () => void;
  open: boolean;
}) {
  return (
    <div
      aria-hidden="true"
      className={sx(
        styles.communicationsBackdrop,
        open && styles.communicationsBackdropOpen,
      )}
      data-communication-backdrop=""
      onClick={onClose}
    />
  );
}

function MailboxHistory({
  error,
  portfolio,
}: {
  error: Error | null;
  portfolio: OperatorContext;
}) {
  const attentionMessages = operatorMailboxRows(portfolio);
  const observationMessages = operatorObservationMailboxRows(
    portfolio.observations,
  );
  const messageCount = attentionMessages.length + observationMessages.length;
  return (
    <>
      <header className={sx(styles.communicationsHeader)}>
        <div>
          <p className={sx(styles.micro, styles.sectionKicker)}>
            HISTORY / RUNTIME + COLLABORATION
          </p>
          <h2 className={sx(styles.sectionTitle)}>Mailbox history</h2>
        </div>
        <div className={sx(styles.communicationsCoordinate)}>
          <span className={sx(styles.micro, styles.muted)}>
            ATTENTION / {shortDigest(portfolio.attention.attention_id)} ·
            OBSERVATIONS / {shortDigest(portfolio.observations.frontier_id)}
          </span>
          <span className={sx(styles.micro)}>
            {messageCount + (error ? 1 : 0)} ACTIVE · SOURCE-ORDERED
          </span>
        </div>
      </header>
      <div className={sx(styles.communicationsBody)} aria-live="polite">
        {error ? (
          <article className={sx(styles.communicationMessage)}>
            <span className={sx(styles.micro, styles.toneWarning)}>
              REVALIDATION DELAYED
            </span>
            <strong>Portfolio projection</strong>
            <p>{error.message}</p>
          </article>
        ) : null}
        {observationMessages.length > 0 ? (
          <MailboxBoundary
            detail="Unresolved observations retain O@sequence order. They do not carry unread, priority, assignment, action, or proof state."
            label={`OBSERVATION FRONTIER / ${portfolio.observations.ordering.replaceAll("_", " ").toUpperCase()}`}
          />
        ) : null}
        {observationMessages.map((message) => (
          <ObservationMailboxMessage key={message.row_id} message={message} />
        ))}
        {attentionMessages.length > 0 ? (
          <MailboxBoundary
            detail="Runtime attention rows retain scheduler readiness and priority in their own typed projection. No order across mailbox sources is claimed."
            label="RUNTIME ATTENTION / CURRENT PROJECTION"
          />
        ) : null}
        {attentionMessages.map((message) => (
          <article
            className={sx(styles.communicationMessage)}
            key={message.row_id}
          >
            <span className={sx(styles.micro, styles.communicationAction)}>
              {message.action} / {message.readiness}
            </span>
            <strong>
              {message.subject_kind} / {message.subject_id}
            </strong>
            <p>{message.reason}</p>
            <small className={sx(styles.micro, styles.muted)}>
              PRIORITY {message.priority} · {message.evidence_ids.length}{" "}
              EVIDENCE · {message.dependency_ids.length} DEPENDENCIES
            </small>
          </article>
        ))}
        {!error && messageCount === 0 ? (
          <div className={sx(styles.communicationsQuiet)}>
            <span className={sx(styles.micro)}>NO NEWS</span>
            <strong>No mailbox entries in the current projection.</strong>
            <p>
              No unresolved observations or runtime attention rows request
              operator attention.
            </p>
          </div>
        ) : null}
      </div>
    </>
  );
}

function MailboxBoundary({ detail, label }: { detail: string; label: string }) {
  return (
    <article className={sx(styles.communicationMessage)}>
      <span className={sx(styles.micro, styles.muted)}>SOURCE BOUNDARY</span>
      <strong>{label}</strong>
      <p>{detail}</p>
    </article>
  );
}

function ObservationMailboxMessage({
  message,
}: {
  message: ObservationMailboxRow;
}) {
  return (
    <article
      className={sx(styles.communicationMessage)}
      data-mailbox-source="observation"
    >
      <span className={sx(styles.micro, styles.communicationAction)}>
        {message.position} / {message.kind.toUpperCase()} / UNRESOLVED
      </span>
      <strong>{message.subject_locator}</strong>
      <p>{message.body}</p>
      <small className={sx(styles.micro, styles.muted)}>
        {message.author.kind.toUpperCase()} / {message.author.id} /
        SELF-ASSERTED · {message.completeness.toUpperCase()} ·{" "}
        {message.evidence_count} EVIDENCE · {message.omission_count} OMISSIONS ·{" "}
        {message.channel_ids.length} CHANNELS
      </small>
      <code title={message.observation_id}>
        OBSERVATION / {shortDigest(message.observation_id)}
      </code>
    </article>
  );
}

export function ConversationSurface() {
  return (
    <>
      <header className={sx(styles.communicationsHeader)}>
        <div>
          <p className={sx(styles.micro, styles.sectionKicker)}>
            REY / AGENT / OPERATOR
          </p>
          <h2 className={sx(styles.sectionTitle)}>Conversation</h2>
        </div>
        <div className={sx(styles.communicationsCoordinate)}>
          <span className={sx(styles.micro, styles.muted)}>SESSION / NONE</span>
          <span className={sx(styles.micro)}>TRANSPORT / UNAVAILABLE</span>
        </div>
      </header>
      <div className={sx(styles.conversationBody)}>
        <div
          aria-live="polite"
          className={sx(styles.conversationThread)}
          role="log"
        >
          <div className={sx(styles.conversationBoundary)} role="status">
            <span className={sx(styles.micro, styles.toneWarning)}>
              NO ADMITTED CONVERSATION
            </span>
            <strong>No Rey or agent session is connected.</strong>
            <p>
              This server has no conversation admission path. A transport,
              participant identity, retention contract, and message contract
              must be bound before communication can begin.
            </p>
          </div>
        </div>
        <form
          className={sx(styles.conversationComposer)}
          onSubmit={(event) => event.preventDefault()}
        >
          <label
            className={sx(styles.micro, styles.conversationLabel)}
            htmlFor="rey-conversation-message"
          >
            MESSAGE
          </label>
          <textarea
            aria-describedby="rey-conversation-boundary"
            className={sx(styles.conversationInput)}
            disabled
            id="rey-conversation-message"
            placeholder="Connect an admitted Rey / agent session to send a message"
            rows={3}
          />
          <button
            className={sx(styles.conversationSend)}
            disabled
            type="submit"
          >
            SEND ↗
          </button>
          <small
            className={sx(
              styles.micro,
              styles.muted,
              styles.conversationBoundaryNote,
            )}
            id="rey-conversation-boundary"
          >
            BOUND / NO TRANSPORT · NO RETENTION · NO WRITE AUTHORITY
          </small>
        </form>
      </div>
    </>
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
  const desired = projection.applications.filter(
    (application) => application.working !== null,
  );
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
              {projection.mapping?.source_path ?? "PROCESS SEEDS"} ·{" "}
              {projection.mapping?.schema ?? "REY.DISCOVERY-SEEDS.V1"} ·{" "}
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
            {status.ignored ? (
              <span
                className={sx(styles.micro, styles.toneWarning)}
                title={`${status.ignored.source} · ${status.ignored.source_digest}`}
              >
                {status.ignored.source} · {status.ignored.rules.length} RULES ·{" "}
                {status.ignored.ignored} OMITTED
              </span>
            ) : null}
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
            {desired.length} DESIRED · {projection.summary.applications_found}{" "}
            FOUND · {projection.summary.applications_not_found} NOT FOUND
          </span>
        </div>
        <ApplicationInventory
          applications={desired}
          inventoryId={
            projection.application_inventory.working?.inventory_id ?? null
          }
          sourcePath={
            projection.application_inventory.working?.source_path ?? null
          }
        />
        <div className={sx(styles.environmentSearchRecord)}>
          <div>
            <span className={sx(styles.micro)}>SEARCH RECORD</span>
            <strong>WORKING</strong>
          </div>
          <code title={status.working_snapshot.semantic_digest}>
            {shortDigest(status.working_snapshot.semantic_digest)}
          </code>
          <span className={sx(styles.micro, styles.muted)}>
            DECLARED ADAPTERS · BOUNDED PATH RESOLUTION · FIXED IDENTITY PROBES
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

function ApplicationInventory({
  applications,
  inventoryId,
  sourcePath,
}: {
  applications: EnvironmentObjectStatus<EnvironmentApplicationObservation>[];
  inventoryId: string | null;
  sourcePath: string | null;
}) {
  return (
    <div className={sx(styles.environmentApplicationGroup)}>
      <div className={sx(styles.environmentApplicationGroupHeader)}>
        <span className={sx(styles.micro)}>DESIRED INVENTORY</span>
        <strong>{String(applications.length).padStart(2, "0")}</strong>
      </div>
      <div className={sx(styles.environmentInventoryRecord)}>
        <span className={sx(styles.micro, styles.muted)}>RECORD</span>
        <code title={inventoryId ?? undefined}>
          {sourcePath ?? "NO MAP"} @ {shortDigest(inventoryId)}
        </code>
      </div>
      {applications.length === 0 ? (
        <p className={sx(styles.micro, styles.muted)}>NONE</p>
      ) : (
        applications.map((application) => {
          const observation = application.working;
          if (!observation) return null;
          return (
            <div
              className={sx(styles.environmentApplicationRow)}
              key={application.object_id}
            >
              <span className={sx(styles.environmentApplicationMarker)}>→</span>
              <div className={sx(styles.environmentApplicationIdentity)}>
                <strong>{application.object_id}</strong>
                <code>{observation.name}</code>
                <small className={sx(styles.micro, styles.muted)}>
                  {observation.purpose ?? "PURPOSE NOT RECORDED"}
                </small>
              </div>
              <span className={sx(styles.micro, styles.muted)}>
                {observation.required ? "REQUIRED" : "OPTIONAL"}
              </span>
              <p className={sx(styles.environmentCapabilityList)}>
                {observation.potential_capabilities.length > 0
                  ? observation.potential_capabilities.join(" · ")
                  : "NO DESIRED CAPABILITIES"}
              </p>
            </div>
          );
        })
      )}
    </div>
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
                {tone === "found"
                  ? "RESOLVED"
                  : tone === "missing"
                    ? "UNRESOLVED"
                    : tone === "removed"
                      ? "REMOVED"
                      : "ERROR"}
              </span>
            </div>
          );
        })
      )}
    </div>
  );
}

function WorkloadsRoutePage() {
  return <WorkloadsPage portfolio={usePortfolio()} />;
}

function WorkloadDetailRoutePage() {
  const portfolio = usePortfolio();
  const { workloadId } = workloadDetailRoute.useParams();
  const workload = portfolio.workloads.find(
    (candidate) => candidate.workload.id === workloadId,
  );
  const draft = portfolio.drafts.find(
    (candidate) => candidate.request.workload_id === workloadId,
  );
  const revision = portfolio.revision;
  const stagedCandidate = revision?.staged.changes.some(
    (change) => change.workload_id === workloadId,
  )
    ? revision?.index?.packages.find((item) => item.workload_id === workloadId)
    : undefined;
  const workingCandidate = revision?.unstaged.changes.some(
    (change) => change.workload_id === workloadId,
  )
    ? revision?.working.packages.find((item) => item.workload_id === workloadId)
    : undefined;
  const candidate = stagedCandidate ?? workingCandidate;

  if (candidate && revision)
    return (
      <CandidateWorkloadDetail candidate={candidate} revision={revision} />
    );

  if (draft) return <DraftWorkloadDetail draft={draft} />;

  if (!workload) return <NotFoundPage />;
  return <AdmittedWorkloadDetail workload={workload} />;
}

function ImplementationLink({
  repository,
  revision,
}: {
  repository: string | null;
  revision: string;
}) {
  return (
    <GitCommitLink
      className={sx(styles.focusable, styles.footerLink)}
      fallback="SOURCE REVISION UNKNOWN"
      repository={repository}
      revision={revision}
      title={`Open Rey commit ${revision}`}
    />
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
  const portfolio = usePortfolio();
  const search = exploreRoute.useSearch();
  if (!search.coordinate && !search.scale) {
    return <ExplorePage portfolio={portfolio} />;
  }
  if (!search.coordinate || !search.scale) return <NotFoundPage />;
  const view = parseExplorerView(search.coordinate, search.scale);
  if (!view) return <NotFoundPage />;
  return (
    <ExplorePage
      coordinate={resolveExplorerView(portfolio, view)}
      portfolio={portfolio}
    />
  );
}

function FeedRoutePage() {
  const initialSources = feedRoute.useLoaderData();
  const { document: sources, publish } = usePassiveDocument(
    initialSources,
    loadFeed,
  );
  const search = feedRoute.useSearch();
  const navigate = feedRoute.useNavigate();
  const layout = resolveFeedLayout(search.streams ?? null, sources.channels);
  return (
    <FeedPage
      layout={layout}
      onAdopted={() => {
        void navigate({ replace: true, search: {} });
      }}
      onConfigurationChange={(streams) => {
        const encoded = serializeFeedStreams(streams);
        void navigate({
          replace: true,
          search: { streams: encoded },
        });
      }}
      onLayoutWrite={async (streams) => {
        try {
          const result = await writeChannelWorking(
            channelWorkingWriteForFeedLayout(sources.channels, streams),
          );
          const channels = await loadChannels();
          publish({ ...sources, channels });
          return { result, projection: channels, error: null };
        } catch (error) {
          const channels = await loadChannels();
          publish({ ...sources, channels });
          return {
            result: null,
            projection: channels,
            error: error instanceof Error ? error : new Error(String(error)),
          };
        }
      }}
      portfolio={usePortfolio()}
      sources={sources}
    />
  );
}

function CadenceRoutePage() {
  const initialCadence = cadenceRoute.useLoaderData();
  const { document: cadence } = usePassiveDocument(initialCadence, loadCadence);
  return <CadencePage cadence={cadence} />;
}

function ChannelsRoutePage() {
  const initialChannels = channelsRoute.useLoaderData();
  const {
    document: projection,
    error,
    publish,
  } = usePassiveDocument(initialChannels, loadChannels);
  return (
    <ChannelsPage
      onWrite={async (write) => {
        await writeChannelWorking(write);
        const current = await loadChannels();
        publish(current);
        return current;
      }}
      projection={projection}
      refreshError={error}
    />
  );
}

function AgentsRoutePage() {
  const initialJournal = agentsRoute.useLoaderData();
  const { document: journal } = usePassiveDocument(initialJournal, loadJournal);
  return <AgentsPage journal={journal} portfolio={usePortfolio()} />;
}

function JournalNewRoutePage() {
  const initialJournal = journalNewRoute.useLoaderData();
  const { document: journal, publish } = usePassiveDocument(
    initialJournal,
    loadJournal,
  );
  const navigate = journalNewRoute.useNavigate();
  return (
    <JournalNewPage
      binding={defaultJournalBinding(usePortfolio())}
      onAdmit={async (proposal) => {
        const admission = await admitJournalEntry(proposal);
        publish({ ...journal, log: admission.log });
        await navigate({
          params: { slug: journalEntrySlug(admission.entry) },
          to: "/journal/$slug",
        });
        return admission.entry;
      }}
      onClose={() => void navigate({ to: "/agents" })}
    />
  );
}

function JournalEntryRoutePage() {
  const initialJournal = journalEntryRoute.useLoaderData();
  const { document: journal, publish } = usePassiveDocument(
    initialJournal,
    loadJournal,
  );
  const navigate = journalEntryRoute.useNavigate();
  const { slug } = journalEntryRoute.useParams();
  const entry = resolveJournalEntry(journal.log, slug);
  return entry ? (
    <JournalDocumentPage
      entry={entry}
      log={journal.log}
      onAdmit={async (proposal) => {
        const admission = await admitJournalEntry(proposal);
        publish({ ...journal, log: admission.log });
        await navigate({
          params: { slug: journalEntrySlug(admission.entry) },
          to: "/journal/$slug",
        });
        return admission.entry;
      }}
      onClose={() => void navigate({ to: "/agents" })}
    />
  ) : (
    <NotFoundPage />
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
  validateSearch: normalizeExplorerSearch,
  component: ExploreRoutePage,
});

const feedRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "feed",
  validateSearch: normalizeFeedSearch,
  loader: loadFeed,
  component: FeedRoutePage,
});

const cadenceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "cadence",
  loader: loadCadence,
  component: CadenceRoutePage,
});

const channelsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "channels",
  loader: loadChannels,
  component: ChannelsRoutePage,
});

const agentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "agents",
  loader: loadJournal,
  component: AgentsRoutePage,
});

const journalNewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "journal/new",
  loader: loadJournal,
  component: JournalNewRoutePage,
});

const journalEntryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "journal/$slug",
  loader: loadJournal,
  component: JournalEntryRoutePage,
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
  component: WorkloadsRoutePage,
});

const workloadDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "workloads/$workloadId",
  component: WorkloadDetailRoutePage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  feedRoute,
  exploreRoute,
  cadenceRoute,
  channelsRoute,
  agentsRoute,
  journalNewRoute,
  journalEntryRoute,
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
  if (pathname.startsWith("/feed")) return "FEED / SIGNALS · ADMISSION · FLOW";
  if (pathname.startsWith("/explore")) return "EXPLORE / CONTEXT TOPOLOGY";
  if (pathname.startsWith("/cadence")) return "CADENCE / TICKS";
  if (pathname.startsWith("/channels")) return "CHANNELS / OPERATOR INDEX";
  if (pathname.startsWith("/agents")) return "AGENTS";
  if (pathname.startsWith("/journal")) return "JOURNAL";
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
