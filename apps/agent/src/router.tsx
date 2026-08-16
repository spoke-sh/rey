import { getKineticMaterialStyle, kineticThemeMaterials } from "@hifi/kinetic";
import {
  Link,
  Outlet,
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  redirect,
  useRouterState,
} from "@tanstack/react-router";
import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
} from "react";
import {
  admitJournalEntry,
  loadAgentJournal,
  loadCadence,
  loadChannels,
  loadEnvironment,
  loadFeed,
  loadJournal,
  loadJournalSeed,
  loadOperatorShell,
  loadOperatorShellAfterRevision,
  loadPortfolio,
  loadPortfolioAfterRevision,
  loadWorkloadDeltaEvidence,
  loadWorkloadEvidence,
  loadWorkloadScenarioEvidence,
  writeChannelWorking,
  writeConversationMessage,
  writeObservation,
  type OperatorContext,
  type OperatorShell,
} from "./api";
import {
  conversationBrowserWriter,
  conversationParticipant,
  type ConversationTranscript,
} from "./conversations";
import type { ChannelMessage } from "./channels";
import { operatorMailboxRows, shortDigest } from "./domain";
import {
  environmentApplicationDiff,
  environmentVariableDiff,
  type EnvironmentApplicationDiffLine,
} from "./environment";
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
import {
  RegionalObjectEvidencePage,
  resolveRegionalObjectEvidence,
} from "./regional-object-evidence";
import { activeSectionAt, SECTION_RAIL_ATTRIBUTE } from "./section-rail";
import { environmentStyles as styles } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";

const AgentsPage = lazyRouteComponent(() => import("./agents"), "AgentsPage");
const CadencePage = lazyRouteComponent(
  () => import("./cadence"),
  "CadencePage",
);
const ExplorePage = lazyRouteComponent(
  () => import("./explore"),
  "ExplorePage",
);
const importWorkloads = () => import("./workloads");
const AdmittedWorkloadDetail = lazyRouteComponent(
  importWorkloads,
  "AdmittedWorkloadDetail",
);
const CandidateWorkloadDetail = lazyRouteComponent(
  importWorkloads,
  "CandidateWorkloadDetail",
);
const DraftWorkloadDetail = lazyRouteComponent(
  importWorkloads,
  "DraftWorkloadDetail",
);
const WorkloadsPage = lazyRouteComponent(importWorkloads, "WorkloadsPage");
const importWorkloadEvidence = () => import("./workload-evidence");
const DeltaEvidencePage = lazyRouteComponent(
  importWorkloadEvidence,
  "DeltaEvidencePage",
);
const ScenarioEvidencePage = lazyRouteComponent(
  importWorkloadEvidence,
  "ScenarioEvidencePage",
);

const precision = kineticThemeMaterials.precision;

export type CommunicationAxis = "mailbox" | "conversation";

export const PRIMARY_NAV_ITEMS = [
  { label: "Feed", to: "/feed", prefixes: ["/feed"] },
  { label: "Explore", to: "/explore", prefixes: ["/explore"] },
  { label: "Agents", to: "/agents", prefixes: ["/agents", "/journal"] },
  { label: "Cadence", to: "/cadence", prefixes: ["/cadence"] },
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
  renderer?: "reference" | "webgl2" | "webgpu";
  scale?: string;
} {
  const coordinate = search.coordinate;
  const renderer = search.renderer;
  const scale = search.scale;
  return {
    ...(typeof coordinate === "string" && coordinate.length <= 4_096
      ? { coordinate }
      : {}),
    ...(renderer === "reference" ||
    renderer === "webgl2" ||
    renderer === "webgpu"
      ? { renderer }
      : {}),
    ...(typeof scale === "string" && scale.length <= 64 ? { scale } : {}),
  };
}

export function browserRouterBasepath(location: {
  pathname: string;
  protocol: string;
}): string {
  if (location.protocol !== "file:") return "/";
  const separator = location.pathname.lastIndexOf("/");
  return separator <= 0 ? "/" : location.pathname.slice(0, separator);
}

export function journalSeedObservationIds(value: unknown): string[] {
  if (typeof value !== "string" || value.length > 1_200) return [];
  const ids = value.split(",");
  if (
    ids.length === 0 ||
    ids.length > 16 ||
    new Set(ids).size !== ids.length ||
    ids.some((id) => !/^blake3:[0-9a-f]{64}$/.test(id))
  ) {
    return [];
  }
  return ids;
}

export function normalizeJournalNewSearch(search: Record<string, unknown>): {
  observations?: string;
} {
  const ids = journalSeedObservationIds(search.observations);
  return ids.length > 0 ? { observations: ids.join(",") } : {};
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

function usePassivePortfolio(initialDocument: OperatorContext) {
  const [document, setDocument] = useState(initialDocument);
  const [error, setError] = useState<Error | null>(null);
  const retainedRevision = useRef(initialDocument.revalidation.revision);
  useEffect(() => {
    retainedRevision.current = initialDocument.revalidation.revision;
    setDocument(initialDocument);
    setError(null);
    return startPassiveRevalidation({
      intervalMs: initialDocument.revalidation.poll_after_ms,
      load: () => loadPortfolioAfterRevision(retainedRevision.current),
      publish: (next) => {
        if (!next) return;
        retainedRevision.current = next.revalidation.revision;
        setDocument(next);
      },
      reportError: setError,
    });
  }, [initialDocument]);
  return { document, error };
}

function usePassiveOperatorShell(initialDocument: OperatorShell) {
  const [document, setDocument] = useState(initialDocument);
  const [error, setError] = useState<Error | null>(null);
  const retainedRevision = useRef(initialDocument.revalidation.revision);
  useEffect(() => {
    retainedRevision.current = initialDocument.revalidation.revision;
    setDocument(initialDocument);
    setError(null);
    return startPassiveRevalidation({
      intervalMs: initialDocument.revalidation.poll_after_ms,
      load: () => loadOperatorShellAfterRevision(retainedRevision.current),
      publish: (next) => {
        if (!next) return;
        retainedRevision.current = next.revalidation.revision;
        setDocument(next);
      },
      reportError: setError,
    });
  }, [initialDocument]);
  return { document, error };
}

function RootLayout() {
  const initialShell = rootRoute.useLoaderData();
  const { document: shell, error: shellError } =
    usePassiveOperatorShell(initialShell);
  const [communicationAxis, setCommunicationAxis] =
    useState<CommunicationAxis | null>(null);
  const [mailboxPortfolio, setMailboxPortfolio] =
    useState<OperatorContext | null>(null);
  const [mailboxPending, setMailboxPending] = useState(false);
  const [mailboxError, setMailboxError] = useState<Error | null>(null);
  const mailboxRequest = useRef<Promise<OperatorContext> | null>(null);
  useEffect(() => {
    const needsInitialPortfolio =
      communicationAxis === "mailbox" && !mailboxPortfolio;
    const needsRefresh =
      mailboxPortfolio !== null &&
      mailboxPortfolio.revalidation.revision !== shell.revalidation.revision;
    if ((!needsInitialPortfolio && !needsRefresh) || mailboxRequest.current)
      return;
    setMailboxPending(true);
    setMailboxError(null);
    const request = loadPortfolio();
    mailboxRequest.current = request;
    void request
      .then(setMailboxPortfolio)
      .catch((error: unknown) => {
        setMailboxError(
          error instanceof Error ? error : new Error(String(error)),
        );
      })
      .finally(() => {
        mailboxRequest.current = null;
        setMailboxPending(false);
      });
  }, [communicationAxis, mailboxPortfolio, shell.revalidation.revision]);
  const mailboxCount =
    (mailboxPortfolio ? operatorMailboxRows(mailboxPortfolio).length : 0) +
    shell.channels.mailbox.messages.length;
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
    <>
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
            <strong className={sx(styles.wordmarkStrong)}>REY</strong>
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
          error={mailboxError ?? shellError}
          mailboxPending={mailboxPending}
          portfolio={mailboxPortfolio}
          shell={shell}
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
                (mailboxCount > 0 || mailboxError || shellError) &&
                  styles.mailboxCountActive,
              )}
            >
              {mailboxPending
                ? "…"
                : mailboxCount + (mailboxError || shellError ? 1 : 0)}
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
            repository={shell.ui_server.source_repository}
            revision={shell.ui_server.implementation_revision}
          />
        </footer>
      </div>
    </>
  );
}

function CommunicationPlane({
  axis,
  error,
  mailboxPending,
  onClose,
  portfolio,
  shell,
}: {
  axis: CommunicationAxis | null;
  error: Error | null;
  mailboxPending: boolean;
  onClose: () => void;
  portfolio: OperatorContext | null;
  shell: OperatorShell;
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
          <MailboxHistory
            error={error}
            pending={mailboxPending}
            portfolio={portfolio}
            shell={shell}
          />
        ) : (
          <ConversationSurface transcript={shell.conversation} />
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
  pending,
  portfolio,
  shell,
}: {
  error: Error | null;
  pending: boolean;
  portfolio: OperatorContext | null;
  shell: OperatorShell;
}) {
  const attentionMessages = portfolio ? operatorMailboxRows(portfolio) : [];
  const channelMessages = shell.channels.mailbox.messages;
  const channelBoundaryCount =
    shell.channels.mailbox.omissions.length > 0 ? 1 : 0;
  const messageCount =
    attentionMessages.length + channelMessages.length + channelBoundaryCount;
  return (
    <>
      <header className={sx(styles.communicationsHeader)}>
        <div>
          <p className={sx(styles.micro, styles.sectionKicker)}>
            HISTORY / CHANNELS + RUNTIME
          </p>
          <h2 className={sx(styles.sectionTitle)}>Mailbox history</h2>
        </div>
        <div className={sx(styles.communicationsCoordinate)}>
          <span className={sx(styles.micro, styles.muted)}>
            CHANNELS / {shortDigest(shell.channels.status.head.graph_id)} ·
            ATTENTION /{" "}
            {portfolio
              ? shortDigest(portfolio.attention.attention_id)
              : "LOADING"}
          </span>
          <span className={sx(styles.micro)}>
            {messageCount + (error ? 1 : 0)} ACTIVE · SOURCE ORDER
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
        {pending ? (
          <article className={sx(styles.communicationMessage)}>
            <span className={sx(styles.micro, styles.muted)}>
              PORTFOLIO / LOADING
            </span>
            <strong>Runtime attention is still projecting.</strong>
            <p>
              Channel history is available now. Workload attention will join it
              when the bounded portfolio projection completes.
            </p>
          </article>
        ) : null}
        {channelMessages.length > 0 ? (
          <MailboxBoundary
            detail="Current unread GitHub notifications and bounded comments are retained by a poll through the exact gh application admitted in Channel HEAD and environment HEAD. The CLI verifies one tick; the foreground Rey process supervises its committed cadence. GitHub provider order does not establish causal order with runtime attention."
            label={`GITHUB INBOX / ${shell.channels.mailbox.polls.length} POLL SOURCES`}
          />
        ) : null}
        {channelMessages.map((message) => (
          <GitHubMailboxMessage key={message.message_id} message={message} />
        ))}
        {shell.channels.mailbox.omissions.length > 0 ? (
          <MailboxBoundary
            detail={shell.channels.mailbox.omissions.join(" · ")}
            label="GITHUB INBOX / PARTIAL"
          />
        ) : null}
        {attentionMessages.length > 0 ? (
          <MailboxBoundary
            detail="Runtime attention rows retain scheduler readiness, priority, evidence, and dependency identity in portfolio order."
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
        {!error && !pending && portfolio && messageCount === 0 ? (
          <div className={sx(styles.communicationsQuiet)}>
            <span className={sx(styles.micro)}>NO NEWS</span>
            <strong>No mailbox entries in the current projection.</strong>
            <p>
              No retained Channel messages, runtime attention rows, or
              revalidation failures request operator attention. Observations are
              Feed items, not mail.
            </p>
          </div>
        ) : null}
      </div>
    </>
  );
}

export function GitHubMailboxMessage({ message }: { message: ChannelMessage }) {
  const source = message.source;
  if (source.kind === "local_admission") return null;
  const isNotification = source.kind === "git_hub_notification";
  const label = isNotification
    ? `GITHUB / NOTIFICATION / ${source.reason.toUpperCase()}`
    : source.kind === "git_hub_issue_comment"
      ? "GITHUB / PR COMMENT"
      : "GITHUB / REVIEW COMMENT";
  const title = isNotification
    ? source.repository
    : `${source.repository} #${source.pull_number} · @${source.author}`;
  const detail =
    source.kind === "git_hub_review_comment"
      ? `${source.path} · ${source.source_revision}`
      : source.source_revision;
  return (
    <article
      className={sx(styles.communicationMessage)}
      data-mailbox-source="channel"
    >
      <span className={sx(styles.micro, styles.communicationAction)}>
        {label}
      </span>
      <strong>
        <a
          className={sx(styles.focusable)}
          href={source.html_url}
          rel="noreferrer"
          target="_blank"
        >
          {title}
        </a>
      </strong>
      <p>{message.proposal.body}</p>
      <small className={sx(styles.micro, styles.muted)}>{detail}</small>
      <code title={message.message_id}>
        MESSAGE / {shortDigest(message.message_id)}
      </code>
    </article>
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

export function ConversationSurface({
  transcript: currentTranscript,
}: {
  transcript: ConversationTranscript;
}) {
  const [transcript, setTranscript] = useState(currentTranscript);
  const [message, setMessage] = useState("");
  const [pending, setPending] = useState(false);
  const [writeError, setWriteError] = useState<Error | null>(null);
  useEffect(() => {
    setTranscript(currentTranscript);
    setWriteError(null);
  }, [currentTranscript]);
  const session = transcript.session;
  const browserWriter = conversationBrowserWriter(transcript);
  const available = transcript.availability === "available" && session !== null;
  const body = message.trim();
  const bodyBytes = new TextEncoder().encode(body).length;
  const enabled = Boolean(
    available &&
    browserWriter &&
    body &&
    bodyBytes <= transcript.limits.max_message_bytes &&
    !writeError &&
    !pending,
  );
  const append = async (event: FormEvent) => {
    event.preventDefault();
    if (!enabled || !session) return;
    setPending(true);
    setWriteError(null);
    try {
      const result = await writeConversationMessage({
        schema: "rey.ui-conversation-message-write.v1",
        expected_log_id: transcript.log_id,
        session_id: session.session_id,
        body,
        reply_to: null,
      });
      setTranscript(result.transcript);
      setMessage("");
    } catch (cause) {
      setWriteError(cause instanceof Error ? cause : new Error(String(cause)));
    } finally {
      setPending(false);
    }
  };
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
          <span className={sx(styles.micro, styles.muted)}>
            SESSION / {session ? shortDigest(session.session_id) : "NONE"}
          </span>
          <span className={sx(styles.micro)}>
            TRANSPORT / {transcript.availability.toUpperCase()}
          </span>
        </div>
      </header>
      <div className={sx(styles.conversationBody)}>
        <div
          aria-live="polite"
          className={sx(styles.conversationThread)}
          role="log"
        >
          <ConversationContract transcript={transcript} />
          {transcript.messages.map((entry) => {
            const participant = conversationParticipant(
              transcript,
              entry.proposal.author_id,
            );
            return (
              <article
                className={sx(styles.conversationMessage)}
                data-conversation-message=""
                key={entry.message_id}
              >
                <span className={sx(styles.micro, styles.communicationAction)}>
                  C@{entry.sequence} ·{" "}
                  {participant?.kind.toUpperCase() ?? "UNKNOWN"}/{" "}
                  {entry.proposal.author_id} · SELF-ASSERTED
                </span>
                <strong>
                  {participant?.label ?? entry.proposal.author_id}
                </strong>
                <p>{entry.proposal.body}</p>
                <small className={sx(styles.micro, styles.muted)}>
                  DELIVERY / NOT ATTEMPTED · SOURCE / {entry.source.locator}
                </small>
                <code title={entry.message_id}>
                  MESSAGE / {shortDigest(entry.message_id)}
                </code>
              </article>
            );
          })}
          {available && transcript.messages.length === 0 ? (
            <div className={sx(styles.conversationBoundary)} role="status">
              <span className={sx(styles.micro)}>TRANSCRIPT / EMPTY</span>
              <strong>No messages retained in this exact session.</strong>
              <p>Nothing has been delivered or inferred.</p>
            </div>
          ) : null}
        </div>
        <form className={sx(styles.conversationComposer)} onSubmit={append}>
          <label
            className={sx(styles.micro, styles.conversationLabel)}
            htmlFor="rey-conversation-message"
          >
            MESSAGE
          </label>
          <textarea
            aria-describedby="rey-conversation-boundary"
            className={sx(
              styles.conversationInput,
              browserWriter && styles.conversationInputEnabled,
            )}
            disabled={!browserWriter || pending}
            id="rey-conversation-message"
            maxLength={transcript.limits.max_message_bytes}
            onChange={(event) => setMessage(event.target.value)}
            placeholder={
              browserWriter
                ? `Append as ${browserWriter.label} · self-asserted`
                : "Admit a session with a human browser writer to append"
            }
            rows={3}
            value={message}
          />
          <button
            className={sx(
              styles.conversationSend,
              enabled && styles.conversationSendEnabled,
            )}
            disabled={!enabled}
            type="submit"
          >
            {pending ? "APPENDING…" : "APPEND ↗"}
          </button>
          <small
            className={sx(
              styles.micro,
              styles.muted,
              styles.conversationBoundaryNote,
            )}
            id="rey-conversation-boundary"
          >
            {browserWriter
              ? `BOUND / LOCAL APPEND AS ${browserWriter.participant_id.toUpperCase()} · NO DELIVERY OR EXECUTION`
              : "BOUND / NO AVAILABLE BROWSER WRITER · SEND DISABLED"}
          </small>
          {writeError ? (
            <small
              className={sx(
                styles.micro,
                styles.toneWarning,
                styles.conversationBoundaryNote,
              )}
              role="alert"
            >
              APPEND REJECTED · {writeError.message}
            </small>
          ) : null}
        </form>
      </div>
    </>
  );
}

function ConversationContract({
  transcript,
}: {
  transcript: ConversationTranscript;
}) {
  const session = transcript.session;
  return (
    <div className={sx(styles.conversationBoundary)} role="status">
      <span
        className={sx(
          styles.micro,
          transcript.availability === "unavailable" && styles.toneWarning,
        )}
      >
        {transcript.availability === "available"
          ? "ADMITTED LOCAL TRANSCRIPT"
          : "NO ADMITTED CONVERSATION"}
      </span>
      <strong>
        {session?.proposal.title ?? "No conversation session is admitted."}
      </strong>
      <p>{transcript.availability_detail}</p>
      {session ? (
        <p className={sx(styles.micro, styles.muted)}>
          {session.proposal.transport.provider} /{" "}
          {session.proposal.transport.provider_revision} · PARTICIPANTS /{" "}
          {session.proposal.participants
            .map(
              (participant) =>
                `${participant.kind}:${participant.participant_id}`,
            )
            .join(" · ")}{" "}
          · WRITERS / {session.proposal.writer_ids.join(" · ")}
        </p>
      ) : null}
      <p className={sx(styles.micro, styles.muted)}>
        ORDERING / {transcript.ordering} · COVERAGE /{" "}
        {transcript.messages.length}/{transcript.total_messages} ·{" "}
        {transcript.completeness.toUpperCase()} · {transcript.omitted_messages}{" "}
        OMITTED
      </p>
      <p className={sx(styles.micro, styles.muted)}>
        RETENTION / {transcript.retention}
      </p>
      <p className={sx(styles.micro, styles.muted)}>
        READ / {transcript.read_authority} · BROWSER WRITE /{" "}
        {transcript.browser_write_authority}
      </p>
      <p className={sx(styles.micro, styles.muted)}>
        EFFECT / {transcript.effect_authority} · FAILURE /{" "}
        {transcript.failure_contract}
      </p>
      <code title={transcript.transcript_id}>
        TRANSCRIPT / {shortDigest(transcript.transcript_id)} · LOG /{" "}
        {shortDigest(transcript.log_id)}
      </code>
    </div>
  );
}

function EnvironmentPage() {
  const initialStatus = environmentRoute.useLoaderData();
  const { document: status } = usePassiveDocument(
    initialStatus,
    loadEnvironment,
  );
  const projection = status.operator;
  const variableLines = environmentVariableDiff(projection.variables);
  const searchedVariables = projection.variables.filter(
    (variable) => variable.working !== null,
  );
  const foundVariables = searchedVariables.filter(
    (variable) => variable.working?.availability === "available",
  ).length;
  const variablesNotFound = searchedVariables.filter(
    (variable) => variable.working?.availability === "unavailable",
  ).length;
  const applicationLines = environmentApplicationDiff(projection.applications);
  const foundApplicationLines = applicationLines.filter(
    (line) => line.observation.availability === "available",
  );
  const notFoundApplicationLines = applicationLines.filter(
    (line) => line.observation.availability !== "available",
  );
  const supported = projection.applications.filter(
    (application) => application.working !== null,
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
          <span className={sx(styles.micro, styles.muted)}>
            {searchedVariables.length} SEARCHED · {foundVariables} FOUND ·{" "}
            {variablesNotFound} NOT FOUND
          </span>
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
            {supported.length} SUPPORTED ·{" "}
            {projection.summary.applications_found} FOUND ·{" "}
            {projection.summary.applications_not_found} NOT FOUND
          </span>
        </div>
        <div
          className={sx(styles.environmentDiffDocument)}
          role="table"
          aria-label="Application diff"
        >
          {applicationLines.length === 0 ? (
            <div className={sx(styles.environmentDiffEmpty)}>
              NO APPLICATION SEARCH OUTCOMES
            </div>
          ) : (
            <>
              <ApplicationDiffGroup
                label="FOUND"
                lines={foundApplicationLines}
                outcome="found"
              />
              <ApplicationDiffGroup
                label="NOT FOUND"
                lines={notFoundApplicationLines}
                outcome="not-found"
              />
            </>
          )}
        </div>
      </section>
    </main>
  );
}

function ApplicationDiffGroup({
  label,
  lines,
  outcome,
}: {
  label: string;
  lines: EnvironmentApplicationDiffLine[];
  outcome: "found" | "not-found";
}) {
  return (
    <section
      className={sx(styles.environmentApplicationGroup)}
      aria-label={`${label} applications`}
      role="rowgroup"
    >
      <header className={sx(styles.environmentApplicationGroupHeader)}>
        <span className={sx(styles.micro)}>{label}</span>
        <strong>{String(lines.length).padStart(2, "0")}</strong>
      </header>
      {lines.length === 0 ? (
        <div className={sx(styles.environmentApplicationGroupEmpty)}>NONE</div>
      ) : (
        lines.map((line, index) => {
          const found = outcome === "found";
          const error = line.observation.availability === "error";
          const groups =
            line.observation.groups.length > 0
              ? line.observation.groups
              : ["ungrouped"];
          return (
            <div
              className={sx(
                styles.environmentDiffRow,
                found &&
                  line.kind === "inserted" &&
                  styles.environmentDiffInserted,
                found &&
                  line.kind === "deleted" &&
                  styles.environmentDiffDeleted,
                found &&
                  line.kind === "context" &&
                  styles.environmentDiffContext,
                !found && !error && styles.environmentApplicationNotFound,
                !found && error && styles.environmentApplicationError,
              )}
              key={`${line.key}:${index}`}
              role="row"
            >
              <span className={sx(styles.environmentLineNumber)}>
                {String(index + 1).padStart(2, "0")}
              </span>
              <span className={sx(styles.environmentDiffMarker)}>
                {found
                  ? line.kind === "inserted"
                    ? "+"
                    : line.kind === "deleted"
                      ? "−"
                      : "·"
                  : error
                    ? "!"
                    : "?"}
              </span>
              <div className={sx(styles.environmentApplicationDiffValue)}>
                <strong>{line.observation.name}</strong>
                <code className={sx(styles.environmentApplicationDiffPath)}>
                  {line.observation.resolved_path ?? "NOT RESOLVED"}
                </code>
                <small className={sx(styles.environmentApplicationDiffGroups)}>
                  {groups.join(", ")}
                </small>
              </div>
              {found ? (
                <span className={sx(styles.environmentAdmissionTag)}>
                  {line.admission}
                </span>
              ) : (
                <span className={sx(styles.environmentApplicationOutcome)}>
                  {error ? "ERROR" : "NOT FOUND"}
                </span>
              )}
            </div>
          );
        })
      )}
    </section>
  );
}

function WorkloadsRoutePage() {
  const initialPortfolio = workloadsRoute.useLoaderData();
  const { document: portfolio } = usePassivePortfolio(initialPortfolio);
  return <WorkloadsPage portfolio={portfolio} />;
}

function WorkloadDetailRoutePage() {
  const initial = workloadDetailRoute.useLoaderData();
  const { document: portfolio } = usePassivePortfolio(initial.portfolio);
  const workloadEvidence = initial.evidence;
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
  const evidence = workloadEvidence.workloads.find(
    (candidate) => candidate.workload_id === workloadId,
  );
  return <AdmittedWorkloadDetail evidence={evidence} workload={workload} />;
}

function WorkloadScenarioRoutePage() {
  return (
    <ScenarioEvidencePage evidence={workloadScenarioRoute.useLoaderData()} />
  );
}

function WorkloadDeltaRoutePage() {
  return <DeltaEvidencePage evidence={workloadDeltaRoute.useLoaderData()} />;
}

function RegionalObjectEvidenceRoutePage() {
  const portfolio = regionalObjectEvidenceRoute.useLoaderData();
  const { workloadId, sceneId, objectRevision } =
    regionalObjectEvidenceRoute.useParams();
  const evidence = resolveRegionalObjectEvidence(
    portfolio,
    workloadId,
    sceneId,
    objectRevision,
  );
  return evidence ? (
    <RegionalObjectEvidencePage evidence={evidence} />
  ) : (
    <NotFoundPage />
  );
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
  const initialPortfolio = exploreRoute.useLoaderData();
  const { document: portfolio } = usePassivePortfolio(initialPortfolio);
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
  const initial = feedRoute.useLoaderData();
  const { document: sources, publish } = usePassiveDocument(
    initial.sources,
    loadFeed,
  );
  const { document: portfolio } = usePassivePortfolio(initial.portfolio);
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
      onObservationCreate={
        portfolio.ui_server.observation_write_enabled
          ? async (write) => {
              const admission = await writeObservation(write);
              publish({ ...sources, observations: admission.frontier });
              return admission;
            }
          : undefined
      }
      portfolio={portfolio}
      sources={sources}
    />
  );
}

function CadenceRoutePage() {
  const initialCadence = cadenceRoute.useLoaderData();
  const { document: cadence } = usePassiveDocument(initialCadence, loadCadence);
  return <CadencePage cadence={cadence} />;
}

function AgentsRoutePage() {
  const initial = agentsRoute.useLoaderData();
  const { document } = usePassiveDocument(initial.journal, loadAgentJournal);
  const { document: portfolio } = usePassivePortfolio(initial.portfolio);
  return (
    <AgentsPage
      agent={portfolio.agent_process}
      journal={document.journal}
      opportunities={document.opportunities}
      portfolio={portfolio}
    />
  );
}

function JournalNewRoutePage() {
  const initial = journalNewRoute.useLoaderData();
  const { document: journal, publish } = usePassiveDocument(
    initial.journal,
    loadJournal,
  );
  const { document: portfolio } = usePassivePortfolio(initial.portfolio);
  const navigate = journalNewRoute.useNavigate();
  return (
    <JournalNewPage
      binding={defaultJournalBinding(portfolio)}
      seed={initial.seed}
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
  loader: () => loadOperatorShell(),
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
  path:
    typeof window !== "undefined" && window.location.protocol === "file:"
      ? "explore.html"
      : "explore",
  validateSearch: normalizeExplorerSearch,
  loader: loadPortfolio,
  component: ExploreRoutePage,
});

const feedRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "feed",
  validateSearch: normalizeFeedSearch,
  loader: async () => {
    const [sources, portfolio] = await Promise.all([
      loadFeed(),
      loadPortfolio(),
    ]);
    return { sources, portfolio };
  },
  component: FeedRoutePage,
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
  loader: async () => {
    const [journal, portfolio] = await Promise.all([
      loadAgentJournal(),
      loadPortfolio(),
    ]);
    return { journal, portfolio };
  },
  component: AgentsRoutePage,
});

const journalNewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "journal/new",
  validateSearch: normalizeJournalNewSearch,
  loaderDeps: ({ search }) => ({ observations: search.observations }),
  loader: async ({ deps }) => {
    const observationIds = journalSeedObservationIds(deps.observations);
    const [journal, seed, portfolio] = await Promise.all([
      loadJournal(),
      observationIds.length > 0
        ? loadJournalSeed(observationIds)
        : Promise.resolve(null),
      loadPortfolio(),
    ]);
    return { journal, seed, portfolio };
  },
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
  loader: loadPortfolio,
  component: WorkloadsRoutePage,
});

const workloadDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "workloads/$workloadId",
  loader: async () => {
    const [evidence, portfolio] = await Promise.all([
      loadWorkloadEvidence(),
      loadPortfolio(),
    ]);
    return { evidence, portfolio };
  },
  component: WorkloadDetailRoutePage,
});

const workloadScenarioRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "workloads/$workloadId/scenarios/$executionId",
  loader: ({ params }) =>
    loadWorkloadScenarioEvidence(params.workloadId, params.executionId),
  component: WorkloadScenarioRoutePage,
});

const workloadDeltaRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "workloads/$workloadId/deltas/$deltaId",
  loader: ({ params }) =>
    loadWorkloadDeltaEvidence(params.workloadId, params.deltaId),
  component: WorkloadDeltaRoutePage,
});

const regionalObjectEvidenceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "workloads/$workloadId/scenes/$sceneId/objects/$objectRevision",
  loader: loadPortfolio,
  component: RegionalObjectEvidenceRoutePage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  feedRoute,
  exploreRoute,
  cadenceRoute,
  agentsRoute,
  journalNewRoute,
  journalEntryRoute,
  environmentRoute,
  workloadsRoute,
  workloadDetailRoute,
  workloadScenarioRoute,
  workloadDeltaRoute,
  regionalObjectEvidenceRoute,
]);

export const router = createRouter({
  basepath:
    typeof window === "undefined"
      ? "/"
      : browserRouterBasepath(window.location),
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
