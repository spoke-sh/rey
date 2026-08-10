import { useState, type FormEvent } from "react";
import { shortDigest, type WorkloadList } from "./domain";
import { explorerCoordinatePath } from "./explorer-coordinate";
import { journalStyles as styles } from "./stylex/journal.stylex";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";

export type JournalAuthorKind = "human" | "agent" | "system";

export interface JournalAuthor {
  kind: JournalAuthorKind;
  id: string;
}

export interface JournalBinding {
  coordinate: string;
  source_revision: string;
}

export interface JournalProseNode {
  kind: "heading" | "paragraph" | "bullet" | "quote" | "code";
  text: string;
}

export interface JournalFrameColumn {
  name: string;
  data_type: string;
}

export type JournalBlock =
  | { kind: "prose"; id: string; document: JournalProseNode[] }
  | {
      kind: "explore";
      id: string;
      coordinate: string;
      source_revision: string;
      caption: string | null;
    }
  | {
      kind: "query";
      id: string;
      language: string;
      provider: string;
      mode: "read_only";
      statement: string;
      parameters: Record<string, string>;
    }
  | {
      kind: "frame";
      id: string;
      source_block_id: string;
      snapshot_id: string;
      columns: JournalFrameColumn[];
      preview_rows: Array<Record<string, string | null>>;
      row_count: number;
      truncated: boolean;
    }
  | {
      kind: "diff";
      id: string;
      source: string;
      target: string;
      direction: string;
      assessment: "equal" | "different" | "inconclusive";
      summary: string;
    }
  | {
      kind: "action";
      id: string;
      operation: string;
      desired_delta: string;
      evidence_ids: string[];
      dependency_ids: string[];
    };

export interface JournalEntryProposal {
  schema: "rey.journal-entry-proposal.v1";
  title: string;
  author: JournalAuthor;
  binding: JournalBinding;
  supersedes?: string | null;
  blocks: JournalBlock[];
}

export interface RetainedJournalEntry {
  schema: "rey.journal-entry.v1";
  entry_id: string;
  sequence: number;
  admitted_at: string;
  title: string;
  author: JournalAuthor;
  binding: JournalBinding;
  supersedes: string | null;
  blocks: JournalBlock[];
}

export interface JournalLog {
  schema: "rey.journal-log.v1";
  log_id: string;
  entries: RetainedJournalEntry[];
}

export interface JournalProjection {
  schema: "rey.ui-journal.v2";
  write_enabled: boolean;
  authority: "unauthenticated_journal_admission";
  log: JournalLog;
}

export interface JournalAdmission {
  schema: "rey.journal-admission.v1";
  admitted: boolean;
  entry: RetainedJournalEntry;
  log: JournalLog;
}

export function defaultJournalBinding(portfolio: WorkloadList): JournalBinding {
  const sourceRevision = portfolio.attention.source_snapshot_id;
  return {
    coordinate: explorerCoordinatePath({
      kind: "portfolio",
      identity: "current",
      lens: "landscape",
      at: sourceRevision,
    }),
    source_revision: sourceRevision,
  };
}

export function journalEntrySlug(entry: RetainedJournalEntry): string {
  const title = entry.title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 80)
    .replace(/-$/, "");
  const identity = entry.entry_id
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return `j${entry.sequence}-${title || "entry"}--${identity}`;
}

export function resolveJournalEntry(
  log: JournalLog,
  slug: string,
): RetainedJournalEntry | null {
  return log.entries.find((entry) => journalEntrySlug(entry) === slug) ?? null;
}

export function journalBlockFragment(blockId: string): string {
  return `block-${blockId}`;
}

export function JournalEntries({
  compact = false,
  entries,
}: {
  compact?: boolean;
  entries: RetainedJournalEntry[];
}) {
  return (
    <div className={sx(styles.entries)}>
      {[...entries].reverse().map((entry) => (
        <JournalEntryDocument
          compact={compact}
          entry={entry}
          key={entry.entry_id}
        />
      ))}
    </div>
  );
}

export function JournalEntryDocument({
  compact = false,
  entry,
}: {
  compact?: boolean;
  entry: RetainedJournalEntry;
}) {
  const slug = journalEntrySlug(entry);
  if (compact) {
    return (
      <article className={sx(styles.entry)} data-journal-slug={slug}>
        <a
          className={sx(styles.entryHeader, styles.entryLink)}
          href={`/journal/${slug}`}
        >
          <div className={sx(styles.entryOrdinal)}>J@{entry.sequence}</div>
          <div className={sx(styles.entryIdentity)}>
            <span className={sx(chrome.micro)}>
              {entry.author.kind} / {entry.author.id}
            </span>
            <h3>{entry.title}</h3>
            <code title={entry.entry_id}>{shortDigest(entry.entry_id)}</code>
          </div>
          <div className={sx(styles.entrySelection)}>
            <time className={sx(chrome.micro)}>{entry.admitted_at}</time>
            <strong>OPEN ENTRY →</strong>
          </div>
        </a>
      </article>
    );
  }
  return (
    <article
      className={sx(styles.entry)}
      data-journal-slug={slug}
      id={`journal-${slug}`}
    >
      <header className={sx(styles.entryHeader)}>
        <div className={sx(styles.entryOrdinal)}>J@{entry.sequence}</div>
        <div className={sx(styles.entryIdentity)}>
          <span className={sx(chrome.micro)}>
            {entry.author.kind} / {entry.author.id}
          </span>
          <h3>{entry.title}</h3>
          <code title={entry.entry_id}>{shortDigest(entry.entry_id)}</code>
        </div>
        <time className={sx(chrome.micro)}>{entry.admitted_at}</time>
      </header>
      <div className={sx(styles.blockStack)}>
        {entry.blocks.map((block) => (
          <JournalBlockView block={block} key={block.id} />
        ))}
      </div>
      <footer className={sx(styles.entryBinding)}>
        <span className={sx(chrome.micro)}>EXACT EXPLORE BINDING</span>
        <code>{entry.binding.source_revision}</code>
        <a className={sx(styles.exploreLink)} href={entry.binding.coordinate}>
          OPEN MAP →
        </a>
      </footer>
    </article>
  );
}

function JournalBlockView({ block }: { block: JournalBlock }) {
  const fragment = journalBlockFragment(block.id);
  if (block.kind === "prose") {
    return (
      <section className={sx(styles.block, styles.proseBlock)} id={fragment}>
        <BlockHeader id={block.id} kind="PROSE" />
        <div className={sx(styles.proseDocument)}>
          {block.document.map((node, index) => (
            <ProseNode key={`${block.id}:${index}`} node={node} />
          ))}
        </div>
      </section>
    );
  }
  if (block.kind === "explore") {
    return (
      <section className={sx(styles.block, styles.exploreBlock)} id={fragment}>
        <BlockHeader id={block.id} kind="EXPLORE MAP" />
        <strong>{block.caption ?? "Bound context topology"}</strong>
        <code className={sx(styles.coordinate)}>{block.coordinate}</code>
        <a className={sx(styles.exploreLink)} href={block.coordinate}>
          ENTER LENS →
        </a>
      </section>
    );
  }
  if (block.kind === "query") {
    return (
      <section className={sx(styles.block, styles.queryBlock)} id={fragment}>
        <BlockHeader
          id={block.id}
          kind={`${block.language.toUpperCase()} QUERY`}
        />
        <div className={sx(styles.queryMeta)}>
          <span>{block.provider}</span>
          <strong>READ ONLY · DECLARATION</strong>
          <span>{Object.keys(block.parameters).length} parameters</span>
        </div>
        <pre className={sx(styles.codeSurface)}>{block.statement}</pre>
      </section>
    );
  }
  if (block.kind === "frame") {
    return (
      <section className={sx(styles.block)} id={fragment}>
        <BlockHeader id={block.id} kind="FRAME SNAPSHOT" />
        <div className={sx(styles.frameMeta)}>
          <code>{shortDigest(block.snapshot_id)}</code>
          <span>
            {block.row_count} rows · {block.truncated ? "BOUNDED" : "COMPLETE"}
          </span>
        </div>
        <div className={sx(styles.frameScroll)}>
          <table className={sx(styles.frameTable)}>
            <thead>
              <tr>
                {block.columns.map((column) => (
                  <th key={column.name}>
                    {column.name}
                    <small>{column.data_type}</small>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.preview_rows.map((row, index) => (
                <tr key={`${block.id}:${index}`}>
                  {block.columns.map((column) => (
                    <td key={column.name}>{row[column.name] ?? "NULL"}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    );
  }
  if (block.kind === "diff") {
    return (
      <section className={sx(styles.block, styles.diffBlock)} id={fragment}>
        <BlockHeader id={block.id} kind="DIRECTED DIFF" />
        <strong>{block.assessment.toUpperCase()}</strong>
        <p>{block.summary}</p>
        <code>
          {block.source} → {block.target}
        </code>
      </section>
    );
  }
  return (
    <section className={sx(styles.block, styles.actionBlock)} id={fragment}>
      <BlockHeader id={block.id} kind="NEXT ACTION" />
      <strong>{block.operation.toUpperCase()}</strong>
      <p>{block.desired_delta}</p>
      <span className={sx(chrome.micro)}>
        {block.evidence_ids.length} EVIDENCE · {block.dependency_ids.length}{" "}
        DEPENDENCIES
      </span>
    </section>
  );
}

function BlockHeader({ id, kind }: { id: string; kind: string }) {
  const fragment = journalBlockFragment(id);
  return (
    <header className={sx(styles.blockHeader)}>
      <span className={sx(chrome.micro)}>{kind}</span>
      <a
        aria-label={`Link to ${id}`}
        className={sx(styles.blockPermalink)}
        href={`#${encodeURIComponent(fragment)}`}
      >
        <code>{id} #</code>
      </a>
    </header>
  );
}

function ProseNode({ node }: { node: JournalProseNode }) {
  if (node.kind === "heading") return <h4>{node.text}</h4>;
  if (node.kind === "bullet") return <p>— {node.text}</p>;
  if (node.kind === "quote") return <blockquote>{node.text}</blockquote>;
  if (node.kind === "code") return <pre>{node.text}</pre>;
  return <p>{node.text}</p>;
}

export function JournalCreateLink() {
  return (
    <a
      className={sx(styles.composeButton)}
      data-journal-admission="available"
      href="/journal/new"
    >
      <strong>WRITE A JOURNAL ENTRY</strong>
      <span className={sx(chrome.micro)}>HUMAN + AGENT · EXPLORE-BOUND</span>
      <span className={sx(chrome.micro)}>
        UNAUTHENTICATED · VALIDATED DOCUMENT ADMISSION
      </span>
    </a>
  );
}

export function JournalComposer({
  binding,
  onAdmit,
  onClose,
}: {
  binding: JournalBinding;
  onAdmit: (proposal: JournalEntryProposal) => Promise<RetainedJournalEntry>;
  onClose: () => void;
}) {
  const [author, setAuthor] = useState("operator");
  const [title, setTitle] = useState("");
  const [prose, setProse] = useState("");
  const [queryOpen, setQueryOpen] = useState(false);
  const [provider, setProvider] = useState("");
  const [statement, setStatement] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    const blocks: JournalBlock[] = [
      {
        kind: "prose",
        id: "context",
        document: prose
          .split(/\n\s*\n/)
          .map((text) => text.trim())
          .filter(Boolean)
          .map((text) => ({ kind: "paragraph", text })),
      },
      {
        kind: "explore",
        id: "map",
        coordinate: binding.coordinate,
        source_revision: binding.source_revision,
        caption: "Context at admission",
      },
    ];
    if (queryOpen && statement.trim()) {
      blocks.push({
        kind: "query",
        id: "query",
        language: "sql",
        provider: provider.trim(),
        mode: "read_only",
        statement: statement.trim(),
        parameters: {},
      });
    }
    try {
      await onAdmit({
        schema: "rey.journal-entry-proposal.v1",
        title: title.trim(),
        author: { kind: "human", id: author.trim() },
        binding,
        blocks,
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form
      className={sx(styles.composer)}
      onSubmit={(event) => void submit(event)}
    >
      <header className={sx(styles.composerHeader)}>
        <div>
          <span className={sx(chrome.micro)}>NEW JOURNAL ENTRY</span>
          <h3>Compose on the context map</h3>
        </div>
        <button
          className={sx(styles.textButton)}
          onClick={onClose}
          type="button"
        >
          CLOSE
        </button>
      </header>
      <div className={sx(styles.composerMeta)}>
        <label className={sx(styles.field)}>
          <span className={sx(chrome.micro)}>AUTHOR LABEL / SELF-ASSERTED</span>
          <input
            className={sx(styles.control)}
            onChange={(event) => setAuthor(event.target.value)}
            required
            value={author}
          />
        </label>
        <label className={sx(styles.field)}>
          <span className={sx(chrome.micro)}>TITLE</span>
          <input
            className={sx(styles.control)}
            onChange={(event) => setTitle(event.target.value)}
            required
            value={title}
          />
        </label>
      </div>
      <label className={sx(styles.editorCell)}>
        <span className={sx(chrome.micro)}>PROSE / NOTEBOOK TEXT CELL</span>
        <textarea
          className={sx(styles.control, styles.textControl)}
          onChange={(event) => setProse(event.target.value)}
          placeholder="Write the context, observation, or direction. Separate paragraphs with a blank line."
          required
          rows={8}
          value={prose}
        />
      </label>
      <div className={sx(styles.bindingCell)}>
        <span className={sx(chrome.micro)}>EXPLORE MAP / EXACT BINDING</span>
        <code>{binding.coordinate}</code>
      </div>
      {queryOpen ? (
        <div className={sx(styles.queryComposer)}>
          <label className={sx(styles.field)}>
            <span className={sx(chrome.micro)}>QUERY PROVIDER</span>
            <input
              className={sx(styles.control, styles.queryControl)}
              onChange={(event) => setProvider(event.target.value)}
              placeholder="Exact provider id"
              required
              value={provider}
            />
          </label>
          <label className={sx(styles.field)}>
            <span className={sx(chrome.micro)}>
              SQL / READ-ONLY DECLARATION
            </span>
            <textarea
              className={sx(
                styles.control,
                styles.textControl,
                styles.queryControl,
              )}
              onChange={(event) => setStatement(event.target.value)}
              placeholder="select …"
              required
              rows={5}
              value={statement}
            />
          </label>
        </div>
      ) : (
        <button
          className={sx(styles.addCell)}
          onClick={() => setQueryOpen(true)}
          type="button"
        >
          + ADD READ-ONLY SQL CELL
        </button>
      )}
      {error ? <p className={sx(styles.composerError)}>{error}</p> : null}
      <footer className={sx(styles.composerFooter)}>
        <span className={sx(chrome.micro)}>
          ADMISSION RETAINS THE ENTRY · QUERY CELLS DO NOT EXECUTE
        </span>
        <button
          className={sx(styles.submitButton)}
          disabled={submitting}
          type="submit"
        >
          {submitting ? "ADMITTING…" : "ADMIT JOURNAL ENTRY"}
        </button>
      </footer>
    </form>
  );
}

export function JournalNewPage({
  binding,
  onAdmit,
  onClose,
}: {
  binding: JournalBinding;
  onAdmit: (proposal: JournalEntryProposal) => Promise<RetainedJournalEntry>;
  onClose: () => void;
}) {
  return (
    <main className={sx(chrome.page, styles.page)}>
      <section data-rey-section="01 / NEW ENTRY">
        <JournalRouteHeading
          detail="UNAUTHENTICATED · VALIDATED DOCUMENT ADMISSION"
          index="01"
          kicker="JOURNAL / NEW"
          title="Write on the context map"
        />
        <JournalComposer
          binding={binding}
          onAdmit={onAdmit}
          onClose={onClose}
        />
      </section>
    </main>
  );
}

export function JournalDocumentPage({
  entry,
}: {
  entry: RetainedJournalEntry;
}) {
  return (
    <main className={sx(chrome.page, styles.page)}>
      <section data-rey-section={`J@${entry.sequence} / JOURNAL ENTRY`}>
        <JournalRouteHeading
          detail={`${entry.blocks.length} BLOCKS · EXACT RETAINED DOCUMENT`}
          index={`J@${entry.sequence}`}
          kicker="JOURNAL / ENTRY"
          title={entry.title}
        />
        <nav
          aria-label="Journal entry actions"
          className={sx(styles.documentNav)}
        >
          <a className={sx(styles.exploreLink)} href="/agents">
            ← JOURNAL INDEX
          </a>
          <a className={sx(styles.exploreLink)} href="/journal/new">
            NEW ENTRY →
          </a>
        </nav>
        <JournalEntryDocument entry={entry} />
      </section>
    </main>
  );
}

function JournalRouteHeading({
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
    <header className={sx(styles.routeHeading)}>
      <span className={sx(styles.routeIndex)}>{index}</span>
      <div>
        <p className={sx(chrome.micro, styles.routeKicker)}>{kicker}</p>
        <h1>{title}</h1>
      </div>
      <small className={sx(chrome.micro)}>{detail}</small>
    </header>
  );
}
