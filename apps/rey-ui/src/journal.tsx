import { useMemo, useState, type FormEvent } from "react";
import { shortDigest, type WorkloadList } from "./domain";
import {
  explorerCoordinateUri,
  explorerViewPath,
  parseExplorerCoordinate,
} from "./explorer-coordinate";
import { DEFAULT_LENS_ZOOM } from "./topology";
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
  scale: number;
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

export interface JournalLayoutCell {
  block_id: string;
  span: number;
}

export interface JournalLayoutBand {
  id: string;
  cells: JournalLayoutCell[];
}

export interface JournalLayout {
  kind: "broadsheet";
  columns: 12;
  bands: JournalLayoutBand[];
}

export type JournalBlock =
  | { kind: "prose"; id: string; document: JournalProseNode[] }
  | {
      kind: "explore";
      id: string;
      coordinate: string;
      scale: number;
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
  schema: "rey.journal-entry-proposal.v2";
  title: string;
  author: JournalAuthor;
  binding: JournalBinding;
  supersedes?: string | null;
  layout: JournalLayout;
  blocks: JournalBlock[];
}

export interface RetainedJournalEntry {
  schema: "rey.journal-entry.v2";
  entry_id: string;
  sequence: number;
  admitted_at: string;
  title: string;
  author: JournalAuthor;
  binding: JournalBinding;
  supersedes: string | null;
  layout: JournalLayout;
  blocks: JournalBlock[];
}

export interface JournalLog {
  schema: "rey.journal-log.v2";
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
  schema: "rey.journal-admission.v2";
  admitted: boolean;
  entry: RetainedJournalEntry;
  log: JournalLog;
}

export type JournalCellDelta = "inserted" | "modified" | "unchanged";

export interface JournalDraftDelta {
  changed: boolean;
  inserted: number;
  modified: number;
  removed: number;
  layout_changed: boolean;
  metadata_changed: boolean;
  cells: Record<string, JournalCellDelta>;
}

export function defaultJournalBinding(portfolio: WorkloadList): JournalBinding {
  const sourceRevision = portfolio.attention.source_snapshot_id;
  return {
    coordinate: explorerCoordinateUri({
      scheme: "rey+local",
      kind: "portfolio",
      identity: "current",
      revision: sourceRevision,
    }),
    scale: DEFAULT_LENS_ZOOM,
    source_revision: sourceRevision,
  };
}

export function journalBindingPath(binding: JournalBinding): string {
  const coordinate = parseExplorerCoordinate(binding.coordinate);
  return coordinate
    ? explorerViewPath({ coordinate, scale: binding.scale })
    : "/explore";
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

export function journalBlockSpan(
  layout: JournalLayout,
  blockId: string,
): number | null {
  for (const band of layout.bands) {
    const cell = band.cells.find((candidate) => candidate.block_id === blockId);
    if (cell) return cell.span;
  }
  return null;
}

function journalBlockPlacement(
  layout: JournalLayout,
  blockId: string,
): string | null {
  for (const [bandIndex, band] of layout.bands.entries()) {
    const cellIndex = band.cells.findIndex(
      (candidate) => candidate.block_id === blockId,
    );
    if (cellIndex >= 0) {
      return `${bandIndex}:${cellIndex}:${band.cells[cellIndex]!.span}`;
    }
  }
  return null;
}

export function journalDraftDelta(
  base: RetainedJournalEntry | null,
  proposal: JournalEntryProposal,
): JournalDraftDelta {
  const cells: Record<string, JournalCellDelta> = {};
  let inserted = 0;
  let modified = 0;
  const baseBlocks = new Map(
    (base?.blocks ?? []).map((block) => [block.id, block]),
  );
  for (const block of proposal.blocks) {
    const prior = baseBlocks.get(block.id);
    if (!prior) {
      cells[block.id] = "inserted";
      inserted += 1;
      continue;
    }
    const contentChanged = JSON.stringify(prior) !== JSON.stringify(block);
    const placementChanged =
      journalBlockPlacement(base!.layout, block.id) !==
      journalBlockPlacement(proposal.layout, block.id);
    if (contentChanged || placementChanged) {
      cells[block.id] = "modified";
      modified += 1;
    } else {
      cells[block.id] = "unchanged";
    }
  }
  const removed = (base?.blocks ?? []).filter(
    (block) => !proposal.blocks.some((candidate) => candidate.id === block.id),
  ).length;
  const layoutChanged =
    base !== null &&
    JSON.stringify(base.layout) !== JSON.stringify(proposal.layout);
  const metadataChanged =
    base !== null &&
    (base.title !== proposal.title ||
      base.author.id !== proposal.author.id ||
      JSON.stringify(base.binding) !== JSON.stringify(proposal.binding));
  return {
    changed:
      base === null ||
      inserted > 0 ||
      modified > 0 ||
      removed > 0 ||
      layoutChanged ||
      metadataChanged,
    inserted,
    modified,
    removed,
    layout_changed: layoutChanged,
    metadata_changed: metadataChanged,
    cells,
  };
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
      <JournalBroadsheet entry={entry} />
      <footer className={sx(styles.entryBinding)}>
        <span className={sx(chrome.micro)}>EXACT EXPLORE BINDING</span>
        <code>{entry.binding.source_revision}</code>
        <a
          className={sx(styles.exploreLink)}
          href={journalBindingPath(entry.binding)}
        >
          OPEN MAP →
        </a>
      </footer>
    </article>
  );
}

function JournalBroadsheet({ entry }: { entry: RetainedJournalEntry }) {
  const blocks = new Map(entry.blocks.map((block) => [block.id, block]));
  return (
    <div className={sx(styles.broadsheet)} data-journal-layout="broadsheet">
      {entry.layout.bands.map((band) => (
        <div className={sx(styles.broadsheetBand)} key={band.id}>
          {band.cells.map((cell) => {
            const block = blocks.get(cell.block_id);
            return block ? (
              <div
                className={sx(styles.broadsheetCell)}
                key={cell.block_id}
                style={{ gridColumn: `span ${cell.span}` }}
              >
                <JournalBlockView block={block} />
              </div>
            ) : null;
          })}
        </div>
      ))}
    </div>
  );
}

export function JournalBlockView({ block }: { block: JournalBlock }) {
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
        <a className={sx(styles.exploreLink)} href={journalBindingPath(block)}>
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

type AuthorableBlockKind = "prose" | "query" | "diff" | "action";

function cloneBlock<T extends JournalBlock>(block: T): T {
  return JSON.parse(JSON.stringify(block)) as T;
}

function defaultJournalProposal(
  binding: JournalBinding,
  base: RetainedJournalEntry | null,
): JournalEntryProposal {
  if (base) {
    return {
      schema: "rey.journal-entry-proposal.v2",
      title: base.title,
      author: {
        kind: "human",
        id: base.author.kind === "human" ? base.author.id : "operator",
      },
      binding: { ...base.binding },
      supersedes: base.entry_id,
      layout: JSON.parse(JSON.stringify(base.layout)) as JournalLayout,
      blocks: base.blocks.map(cloneBlock),
    };
  }
  return {
    schema: "rey.journal-entry-proposal.v2",
    title: "",
    author: { kind: "human", id: "operator" },
    binding: { ...binding },
    supersedes: null,
    layout: {
      kind: "broadsheet",
      columns: 12,
      bands: [
        {
          id: "lead",
          cells: [
            { block_id: "context", span: 8 },
            { block_id: "map", span: 4 },
          ],
        },
      ],
    },
    blocks: [
      { kind: "prose", id: "context", document: [] },
      {
        kind: "explore",
        id: "map",
        coordinate: binding.coordinate,
        scale: binding.scale,
        source_revision: binding.source_revision,
        caption: "Context at admission",
      },
    ],
  };
}

export function parseJournalProse(value: string): JournalProseNode[] {
  const units: string[] = [];
  let lines: string[] = [];
  let fenced = false;
  const flush = () => {
    const unit = lines.join("\n").trim();
    if (unit) units.push(unit);
    lines = [];
  };
  for (const line of value.split("\n")) {
    if (line.trim() === "```") {
      lines.push("```");
      fenced = !fenced;
    } else if (!fenced && line.trim() === "") {
      flush();
    } else {
      lines.push(line);
    }
  }
  flush();
  return units
    .map((text) => text.trim())
    .map((text): JournalProseNode => {
      if (text.startsWith("# "))
        return { kind: "heading", text: text.slice(2) };
      if (text.startsWith("- ")) return { kind: "bullet", text: text.slice(2) };
      if (text.startsWith("> ")) return { kind: "quote", text: text.slice(2) };
      if (text.startsWith("```\n") && text.endsWith("```")) {
        return { kind: "code", text: text.slice(4, -3).trimEnd() };
      }
      return { kind: "paragraph", text };
    });
}

export function formatJournalProse(document: JournalProseNode[]): string {
  return document
    .map((node) => {
      if (node.kind === "heading") return `# ${node.text}`;
      if (node.kind === "bullet") return `- ${node.text}`;
      if (node.kind === "quote") return `> ${node.text}`;
      if (node.kind === "code") return `\`\`\`\n${node.text}\n\`\`\``;
      return node.text;
    })
    .join("\n\n");
}

function nextIdentifier(
  proposal: JournalEntryProposal,
  prefix: string,
): string {
  const used = new Set([
    ...proposal.blocks.map((block) => block.id),
    ...proposal.layout.bands.map((band) => band.id),
  ]);
  let index = 1;
  while (used.has(`${prefix}-${index}`)) index += 1;
  return `${prefix}-${index}`;
}

function newBlock(
  proposal: JournalEntryProposal,
  kind: AuthorableBlockKind,
): JournalBlock {
  const id = nextIdentifier(proposal, kind);
  if (kind === "prose") return { kind, id, document: [] };
  if (kind === "query") {
    return {
      kind,
      id,
      language: "sql",
      provider: "local",
      mode: "read_only",
      statement: "",
      parameters: {},
    };
  }
  if (kind === "diff") {
    return {
      kind,
      id,
      source: "journal://source",
      target: "journal://target",
      direction: "expected_to_observed",
      assessment: "inconclusive",
      summary: "",
    };
  }
  return {
    kind,
    id,
    operation: "mine",
    desired_delta: "",
    evidence_ids: [],
    dependency_ids: [],
  };
}

function proposalIsAdmissible(proposal: JournalEntryProposal): boolean {
  if (!proposal.title.trim() || !proposal.author.id.trim()) return false;
  if (proposal.blocks.length === 0) return false;
  return proposal.blocks.every((block) => {
    if (block.kind === "prose") {
      return (
        block.document.length > 0 &&
        block.document.every((node) => node.text.trim().length > 0)
      );
    }
    if (block.kind === "query") {
      return Boolean(block.provider.trim() && block.statement.trim());
    }
    if (block.kind === "diff") {
      return Boolean(
        block.source.trim() &&
        block.target.trim() &&
        block.direction.trim() &&
        block.summary.trim(),
      );
    }
    if (block.kind === "action") {
      return Boolean(block.operation.trim() && block.desired_delta.trim());
    }
    return true;
  });
}

function parseJournalReferences(value: string): string[] {
  return [
    ...new Set(
      value
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean),
    ),
  ];
}

export function JournalComposer({
  base = null,
  binding,
  onAdmit,
  onClose,
}: {
  base?: RetainedJournalEntry | null;
  binding: JournalBinding;
  onAdmit: (proposal: JournalEntryProposal) => Promise<RetainedJournalEntry>;
  onClose: () => void;
}) {
  const [proposal, setProposal] = useState(() =>
    defaultJournalProposal(binding, base),
  );
  const [addKind, setAddKind] = useState<AuthorableBlockKind>("prose");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const delta = useMemo(
    () => journalDraftDelta(base, proposal),
    [base, proposal],
  );

  const updateBlock = (next: JournalBlock) => {
    setProposal((current) => ({
      ...current,
      blocks: current.blocks.map((block) =>
        block.id === next.id ? next : block,
      ),
    }));
  };

  const addBlock = () => {
    setProposal((current) => {
      const block = newBlock(current, addKind);
      return {
        ...current,
        blocks: [...current.blocks, block],
        layout: {
          ...current.layout,
          bands: [
            ...current.layout.bands,
            {
              id: nextIdentifier(current, "band"),
              cells: [{ block_id: block.id, span: 12 }],
            },
          ],
        },
      };
    });
  };

  const removeBlock = (blockId: string) => {
    setProposal((current) => {
      const removed = new Set([blockId]);
      for (const block of current.blocks) {
        if (block.kind === "frame" && block.source_block_id === blockId) {
          removed.add(block.id);
        }
      }
      return {
        ...current,
        blocks: current.blocks.filter((block) => !removed.has(block.id)),
        layout: {
          ...current.layout,
          bands: current.layout.bands
            .map((band) => ({
              ...band,
              cells: band.cells.filter((cell) => !removed.has(cell.block_id)),
            }))
            .filter((band) => band.cells.length > 0),
        },
      };
    });
  };

  const setSpan = (blockId: string, span: number) => {
    setProposal((current) => ({
      ...current,
      layout: {
        ...current.layout,
        bands: current.layout.bands.map((band) => {
          if (!band.cells.some((cell) => cell.block_id === blockId))
            return band;
          const other = band.cells
            .filter((cell) => cell.block_id !== blockId)
            .reduce((total, cell) => total + cell.span, 0);
          return other + span <= 12
            ? {
                ...band,
                cells: band.cells.map((cell) =>
                  cell.block_id === blockId ? { ...cell, span } : cell,
                ),
              }
            : band;
        }),
      },
    }));
  };

  const joinPreviousBand = (blockId: string) => {
    setProposal((current) => {
      const sourceIndex = current.layout.bands.findIndex((band) =>
        band.cells.some((cell) => cell.block_id === blockId),
      );
      if (sourceIndex <= 0) return current;
      const source = current.layout.bands[sourceIndex]!;
      const sourceCell = source.cells.find(
        (cell) => cell.block_id === blockId,
      )!;
      if (source.cells[0]?.block_id !== blockId) return current;
      const target = current.layout.bands[sourceIndex - 1]!;
      const occupied = target.cells.reduce(
        (total, cell) => total + cell.span,
        0,
      );
      if (occupied + sourceCell.span > 12) return current;
      const bands = current.layout.bands.map((band) => ({
        ...band,
        cells: [...band.cells],
      }));
      bands[sourceIndex - 1]!.cells.push(sourceCell);
      bands[sourceIndex]!.cells = bands[sourceIndex]!.cells.filter(
        (cell) => cell.block_id !== blockId,
      );
      return {
        ...current,
        layout: {
          ...current.layout,
          bands: bands.filter((band) => band.cells.length > 0),
        },
      };
    });
  };

  const breakBand = (blockId: string) => {
    setProposal((current) => {
      const bandIndex = current.layout.bands.findIndex((band) =>
        band.cells.some((cell) => cell.block_id === blockId),
      );
      if (bandIndex < 0) return current;
      const band = current.layout.bands[bandIndex]!;
      const cellIndex = band.cells.findIndex(
        (cell) => cell.block_id === blockId,
      );
      if (cellIndex <= 0) return current;
      const bands = current.layout.bands.map((candidate) => ({
        ...candidate,
        cells: [...candidate.cells],
      }));
      const trailing = bands[bandIndex]!.cells.splice(cellIndex);
      bands.splice(bandIndex + 1, 0, {
        id: nextIdentifier(current, "band"),
        cells: trailing,
      });
      return { ...current, layout: { ...current.layout, bands } };
    });
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await onAdmit({
        ...proposal,
        title: proposal.title.trim(),
        author: { ...proposal.author, id: proposal.author.id.trim() },
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSubmitting(false);
    }
  };

  const canSubmit =
    proposalIsAdmissible(proposal) && delta.changed && !submitting;
  const blocks = new Map(proposal.blocks.map((block) => [block.id, block]));

  return (
    <form
      className={sx(styles.entry, styles.liveDocument)}
      data-journal-mode={base ? "revision" : "new"}
      data-journal-surface="broadsheet"
      onSubmit={(event) => void submit(event)}
    >
      <header className={sx(styles.entryHeader, styles.liveHeader)}>
        <div className={sx(styles.entryOrdinal)}>
          {base ? `J@${base.sequence}` : "J@NEW"}
        </div>
        <div className={sx(styles.entryIdentity)}>
          <label className={sx(styles.inlineLabel)}>
            <span className={sx(chrome.micro)}>HUMAN / SELF-ASSERTED</span>
            <input
              aria-label="Revision author"
              className={sx(styles.inlineMetaControl)}
              maxLength={128}
              onChange={(event) =>
                setProposal((current) => ({
                  ...current,
                  author: { ...current.author, id: event.target.value },
                }))
              }
              value={proposal.author.id}
            />
          </label>
          <input
            aria-label="Journal title"
            className={sx(styles.titleControl)}
            maxLength={240}
            onChange={(event) =>
              setProposal((current) => ({
                ...current,
                title: event.target.value,
              }))
            }
            placeholder="Untitled bearing"
            value={proposal.title}
          />
        </div>
        <div
          className={sx(styles.deltaSummary)}
          data-draft-changed={delta.changed}
        >
          <span className={sx(chrome.micro)}>WORKING Δ</span>
          <strong>
            +{delta.inserted} ~{delta.modified} −{delta.removed} · L
            {delta.layout_changed ? 1 : 0} M{delta.metadata_changed ? 1 : 0}
          </strong>
          <code>{base ? shortDigest(base.entry_id) : "unretained"}</code>
        </div>
      </header>

      <div className={sx(styles.broadsheet)} data-journal-layout="broadsheet">
        {proposal.layout.bands.map((band, bandIndex) => (
          <div className={sx(styles.broadsheetBand)} key={band.id}>
            {band.cells.map((cell) => {
              const block = blocks.get(cell.block_id);
              if (!block) return null;
              const occupied = band.cells.reduce(
                (total, candidate) => total + candidate.span,
                0,
              );
              return (
                <div
                  className={sx(styles.broadsheetCell)}
                  data-cell-delta={delta.cells[block.id]}
                  key={block.id}
                  style={{ gridColumn: `span ${cell.span}` }}
                >
                  <div className={sx(styles.cellTools)}>
                    <span className={sx(chrome.micro)}>{block.id}</span>
                    <span className={sx(styles.deltaBadge)}>
                      {delta.cells[block.id] ?? "unchanged"}
                    </span>
                    <label>
                      <span className={sx(styles.visuallyHidden)}>
                        Cell width
                      </span>
                      <select
                        aria-label={`${block.id} cell width`}
                        className={sx(styles.cellSelect)}
                        onChange={(event) =>
                          setSpan(block.id, Number(event.target.value))
                        }
                        value={cell.span}
                      >
                        {Array.from({ length: 12 }, (_, index) => index + 1)
                          .filter((span) => occupied - cell.span + span <= 12)
                          .map((span) => (
                            <option key={span} value={span}>
                              {span}/12
                            </option>
                          ))}
                      </select>
                    </label>
                    {bandIndex > 0 ? (
                      <button
                        className={sx(styles.cellToolButton)}
                        onClick={() => joinPreviousBand(block.id)}
                        type="button"
                      >
                        JOIN ↑
                      </button>
                    ) : null}
                    {band.cells[0]?.block_id !== block.id ? (
                      <button
                        className={sx(styles.cellToolButton)}
                        onClick={() => breakBand(block.id)}
                        type="button"
                      >
                        BREAK ↵
                      </button>
                    ) : null}
                    <button
                      className={sx(styles.cellToolButton)}
                      disabled={proposal.blocks.length === 1}
                      onClick={() => removeBlock(block.id)}
                      type="button"
                    >
                      REMOVE
                    </button>
                  </div>
                  <JournalEditableBlock block={block} onChange={updateBlock} />
                </div>
              );
            })}
          </div>
        ))}
      </div>

      <div className={sx(styles.addCellBar)}>
        <select
          aria-label="New cell type"
          className={sx(styles.cellSelect)}
          onChange={(event) =>
            setAddKind(event.target.value as AuthorableBlockKind)
          }
          value={addKind}
        >
          <option value="prose">Prose</option>
          <option value="query">Read-only query</option>
          <option value="diff">Directed diff</option>
          <option value="action">Action opportunity</option>
        </select>
        <button className={sx(styles.addCell)} onClick={addBlock} type="button">
          + ADD TYPED CELL
        </button>
        <span className={sx(chrome.micro)}>
          CELLS FLOW IN READING ORDER · 12-COLUMN BOUNDED BROADSHEET
        </span>
      </div>

      <footer className={sx(styles.entryBinding, styles.liveFooter)}>
        <div className={sx(styles.bindingIdentity)}>
          <span className={sx(chrome.micro)}>EXACT EXPLORE BINDING</span>
          <code>{proposal.binding.source_revision}</code>
          <a
            className={sx(styles.exploreLink)}
            href={journalBindingPath(proposal.binding)}
          >
            OPEN MAP →
          </a>
        </div>
        <div className={sx(styles.revisionActions)}>
          <button
            className={sx(styles.textButton)}
            onClick={onClose}
            type="button"
          >
            CLOSE
          </button>
          <button
            className={sx(styles.submitButton)}
            disabled={!canSubmit}
            type="submit"
          >
            {submitting
              ? "RECORDING…"
              : base
                ? "RECORD REVISION"
                : "RECORD ENTRY"}
          </button>
        </div>
      </footer>
      {error ? <p className={sx(styles.composerError)}>{error}</p> : null}
      <p className={sx(styles.authorityNote, chrome.micro)}>
        RECORDING APPENDS AN IMMUTABLE REVISION · QUERY AND ACTION CELLS DO NOT
        EXECUTE
      </p>
    </form>
  );
}

function JournalEditableBlock({
  block,
  onChange,
}: {
  block: JournalBlock;
  onChange: (block: JournalBlock) => void;
}) {
  if (block.kind === "prose") {
    return (
      <section
        className={sx(styles.block, styles.proseBlock)}
        id={journalBlockFragment(block.id)}
      >
        <BlockHeader id={block.id} kind="PROSE" />
        <textarea
          aria-label={`${block.id} prose`}
          className={sx(styles.liveText, styles.proseLiveText)}
          maxLength={65_536}
          onChange={(event) =>
            onChange({
              ...block,
              document: parseJournalProse(event.target.value),
            })
          }
          placeholder="# Heading\n\nParagraph\n\n- Point\n\n> Quotation"
          rows={7}
          value={formatJournalProse(block.document)}
        />
      </section>
    );
  }
  if (block.kind === "query") {
    return (
      <section
        className={sx(styles.block, styles.queryBlock)}
        id={journalBlockFragment(block.id)}
      >
        <BlockHeader
          id={block.id}
          kind={`${block.language.toUpperCase()} QUERY`}
        />
        <div className={sx(styles.queryMeta)}>
          <input
            aria-label={`${block.id} provider`}
            className={sx(styles.darkInlineControl)}
            maxLength={128}
            onChange={(event) =>
              onChange({ ...block, provider: event.target.value })
            }
            placeholder="provider"
            value={block.provider}
          />
          <strong>READ ONLY · DECLARATION</strong>
          <span>{Object.keys(block.parameters).length} parameters</span>
        </div>
        <textarea
          aria-label={`${block.id} statement`}
          className={sx(styles.codeSurface, styles.liveCode)}
          maxLength={32_768}
          onChange={(event) =>
            onChange({ ...block, statement: event.target.value })
          }
          placeholder="select …"
          rows={7}
          value={block.statement}
        />
      </section>
    );
  }
  if (block.kind === "diff") {
    return (
      <section
        className={sx(styles.block, styles.diffBlock)}
        id={journalBlockFragment(block.id)}
      >
        <BlockHeader id={block.id} kind="DIRECTED DIFF" />
        <div className={sx(styles.inlineFields)}>
          <input
            aria-label={`${block.id} source`}
            className={sx(styles.inlineControl)}
            onChange={(event) =>
              onChange({ ...block, source: event.target.value })
            }
            value={block.source}
          />
          <span>→</span>
          <input
            aria-label={`${block.id} target`}
            className={sx(styles.inlineControl)}
            onChange={(event) =>
              onChange({ ...block, target: event.target.value })
            }
            value={block.target}
          />
        </div>
        <select
          aria-label={`${block.id} assessment`}
          className={sx(styles.cellSelect)}
          onChange={(event) =>
            onChange({
              ...block,
              assessment: event.target.value as
                "equal" | "different" | "inconclusive",
            })
          }
          value={block.assessment}
        >
          <option value="inconclusive">Inconclusive</option>
          <option value="different">Different</option>
          <option value="equal">Equal</option>
        </select>
        <textarea
          aria-label={`${block.id} summary`}
          className={sx(styles.liveText)}
          maxLength={65_536}
          onChange={(event) =>
            onChange({ ...block, summary: event.target.value })
          }
          placeholder="Describe expected → observed."
          rows={5}
          value={block.summary}
        />
      </section>
    );
  }
  if (block.kind === "action") {
    return (
      <section
        className={sx(styles.block, styles.actionBlock)}
        id={journalBlockFragment(block.id)}
      >
        <BlockHeader id={block.id} kind="ACTION OPPORTUNITY" />
        <input
          aria-label={`${block.id} operation`}
          className={sx(styles.operationControl)}
          maxLength={128}
          onChange={(event) =>
            onChange({ ...block, operation: event.target.value })
          }
          placeholder="mine / build / verify / …"
          value={block.operation}
        />
        <textarea
          aria-label={`${block.id} desired delta`}
          className={sx(styles.liveText)}
          maxLength={65_536}
          onChange={(event) =>
            onChange({ ...block, desired_delta: event.target.value })
          }
          placeholder="State the exact desired delta."
          rows={5}
          value={block.desired_delta}
        />
        <div className={sx(styles.referenceEditors)}>
          <label className={sx(styles.referenceEditor)}>
            <span className={sx(chrome.micro)}>
              EVIDENCE LOCATORS / ONE PER LINE
            </span>
            <textarea
              aria-label={`${block.id} evidence locators`}
              className={sx(styles.liveText, styles.referenceText)}
              maxLength={131_072}
              onChange={(event) =>
                onChange({
                  ...block,
                  evidence_ids: parseJournalReferences(event.target.value),
                })
              }
              rows={3}
              value={block.evidence_ids.join("\n")}
            />
          </label>
          <label className={sx(styles.referenceEditor)}>
            <span className={sx(chrome.micro)}>
              DEPENDENCY LOCATORS / ONE PER LINE
            </span>
            <textarea
              aria-label={`${block.id} dependency locators`}
              className={sx(styles.liveText, styles.referenceText)}
              maxLength={131_072}
              onChange={(event) =>
                onChange({
                  ...block,
                  dependency_ids: parseJournalReferences(event.target.value),
                })
              }
              rows={3}
              value={block.dependency_ids.join("\n")}
            />
          </label>
        </div>
        <span className={sx(chrome.micro)}>
          {block.evidence_ids.length} EVIDENCE · {block.dependency_ids.length}{" "}
          DEPENDENCIES · NOT ADMITTED FOR EXECUTION
        </span>
      </section>
    );
  }
  return <JournalBlockView block={block} />;
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
      <section data-rey-section="JOURNAL / BROADSHEET">
        <JournalRouteHeading
          detail="LIVE WORKING Δ · UNAUTHENTICATED ADMISSION"
          index="J@NEW"
          kicker="JOURNAL / BROADSHEET"
          title="Reason on the context map"
        />
        <nav
          aria-label="Journal entry actions"
          className={sx(styles.documentNav)}
        >
          <a className={sx(styles.exploreLink)} href="/agents">
            ← JOURNAL INDEX
          </a>
          <div className={sx(styles.revisionTrail)}>
            <span className={sx(chrome.micro)}>BASE / NONE</span>
            <strong>WORKING</strong>
          </div>
          <span className={sx(chrome.micro)}>UNRETAINED</span>
        </nav>
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
  log,
  onAdmit,
  onClose,
}: {
  entry: RetainedJournalEntry;
  log: JournalLog;
  onAdmit: (proposal: JournalEntryProposal) => Promise<RetainedJournalEntry>;
  onClose: () => void;
}) {
  const previous = entry.supersedes
    ? (log.entries.find(
        (candidate) => candidate.entry_id === entry.supersedes,
      ) ?? null)
    : null;
  const revisions = log.entries.filter(
    (candidate) => candidate.supersedes === entry.entry_id,
  );
  return (
    <main className={sx(chrome.page, styles.page)}>
      <section data-rey-section="JOURNAL / BROADSHEET">
        <JournalRouteHeading
          detail={`${entry.blocks.length} CELLS · EDITS APPEND A REVISION`}
          index={`J@${entry.sequence}`}
          kicker="JOURNAL / BROADSHEET"
          title="Reason on the context map"
        />
        <nav
          aria-label="Journal entry actions"
          className={sx(styles.documentNav)}
        >
          <a className={sx(styles.exploreLink)} href="/agents">
            ← JOURNAL INDEX
          </a>
          <div className={sx(styles.revisionTrail)}>
            {previous ? (
              <a
                className={sx(styles.exploreLink)}
                href={`/journal/${journalEntrySlug(previous)}`}
              >
                ← J@{previous.sequence}
              </a>
            ) : (
              <span className={sx(chrome.micro)}>ROOT</span>
            )}
            <strong>J@{entry.sequence}</strong>
            {revisions.slice(0, 3).map((revision) => (
              <a
                className={sx(styles.exploreLink)}
                href={`/journal/${journalEntrySlug(revision)}`}
                key={revision.entry_id}
              >
                J@{revision.sequence} →
              </a>
            ))}
            {revisions.length > 3 ? (
              <span className={sx(chrome.micro)}>
                +{revisions.length - 3} BRANCHES
              </span>
            ) : null}
          </div>
          <a className={sx(styles.exploreLink)} href="/journal/new">
            NEW ENTRY →
          </a>
        </nav>
        <JournalComposer
          base={entry}
          binding={entry.binding}
          key={entry.entry_id}
          onAdmit={onAdmit}
          onClose={onClose}
        />
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
