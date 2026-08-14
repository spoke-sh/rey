import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { WorkloadList } from "./domain";
import {
  defaultJournalBinding,
  formatJournalProse,
  journalBlockFragment,
  journalDraftDelta,
  journalEntrySlug,
  JournalDocumentPage,
  JournalEntries,
  JournalNewPage,
  parseJournalProse,
  resolveJournalEntry,
  type JournalEntryProposal,
  type JournalSeed,
  type RetainedJournalEntry,
} from "./journal";

describe("collaboration Journal", () => {
  it("binds new entries to the exact current portfolio coordinate", () => {
    expect(defaultJournalBinding(portfolio())).toEqual({
      coordinate: "rey+local://portfolio/current?revision=blake3%3Asource",
      scale: 0.26,
      source_revision: "blake3:source",
    });
  });

  it("renders the full typed notebook grammar without executing declarations", () => {
    const markup = renderToStaticMarkup(
      createElement(JournalEntries, { entries: [entry()] }),
    );

    for (const evidence of [
      "PROSE",
      "EXPLORE MAP",
      "SQL QUERY",
      "READ ONLY · DECLARATION",
      "select * from coverage",
      "FRAME SNAPSHOT",
      "2 rows · COMPLETE",
      "DIRECTED DIFF",
      "DIFFERENT",
      "NEXT ACTION",
      "REFINE",
      "OPEN MAP →",
      'href="#block-query"',
    ]) {
      expect(markup).toContain(evidence);
    }
    expect(markup).toContain(
      'href="/explore?coordinate=rey%2Blocal%3A%2F%2Fportfolio%2Fcurrent%3Frevision%3Dblake3%253Asource&amp;scale=0.68"',
    );
    expect(markup).toContain('data-journal-layout="broadsheet"');
    expect(markup).toContain("grid-column:span 8");
  });

  it("parses the bounded prose shorthand deterministically", () => {
    const source =
      "# Bearing\n\nA paragraph.\n\n- Mine context\n\n> Verify it\n\n```\nselect 1\n```";
    const document = parseJournalProse(source);
    expect(document.map((node) => node.kind)).toEqual([
      "heading",
      "paragraph",
      "bullet",
      "quote",
      "code",
    ]);
    expect(formatJournalProse(document)).toBe(source);
  });

  it("directs a working revision from exact retained content", () => {
    const retained = entry();
    const proposal: JournalEntryProposal = {
      schema: "rey.journal-entry-proposal.v2",
      title: retained.title,
      author: retained.author,
      binding: retained.binding,
      supersedes: retained.entry_id,
      layout: retained.layout,
      blocks: retained.blocks,
    };
    expect(journalDraftDelta(retained, proposal)).toMatchObject({
      changed: false,
      inserted: 0,
      modified: 0,
      removed: 0,
    });
    proposal.blocks = proposal.blocks.map((block) =>
      block.id === "action" && block.kind === "action"
        ? { ...block, desired_delta: "Mine and verify the remaining rows." }
        : block,
    );
    expect(journalDraftDelta(retained, proposal)).toMatchObject({
      changed: true,
      inserted: 0,
      modified: 1,
      removed: 0,
    });
  });

  it("uses one live broadsheet surface for new and retained routes", () => {
    const onAdmit = async () => entry();
    const onClose = () => undefined;
    const newMarkup = renderToStaticMarkup(
      createElement(JournalNewPage, {
        binding: defaultJournalBinding(portfolio()),
        onAdmit,
        onClose,
      }),
    );
    const retainedMarkup = renderToStaticMarkup(
      createElement(JournalDocumentPage, {
        entry: entry(),
        log: {
          schema: "rey.journal-log.v2",
          log_id: "blake3:log",
          entries: [entry()],
        },
        onAdmit,
        onClose,
      }),
    );
    for (const markup of [newMarkup, retainedMarkup]) {
      expect(markup).toContain('data-journal-surface="broadsheet"');
      expect(markup).toContain("Reason on the context map");
      expect(markup).toContain("WORKING Δ");
      expect(markup).toContain("+ ADD TYPED CELL");
    }
  });

  it("opens an exact observation seed as an unretained editable proposal", () => {
    const seeded = seed();
    const markup = renderToStaticMarkup(
      createElement(JournalNewPage, {
        binding: defaultJournalBinding(portfolio()),
        seed: seeded,
        onAdmit: async () => entry(),
        onClose: () => undefined,
      }),
    );

    expect(markup).toContain("1 EXACT OBSERVATIONS · UNRETAINED SEED");
    expect(markup).toContain("SEED / seed");
    expect(markup).toContain("PROPOSAL ONLY · REVIEW BEFORE RECORDING");
    expect(markup).toContain("Catch up on 1 unresolved observation");
    expect(markup).toContain("A retained delta remains unresolved.");
    expect(markup).toContain('value="operator"');
    expect(markup).toContain("RECORD ENTRY");
  });

  it("links immutable supersession edges on the retained plane", () => {
    const root = entry();
    const revision: RetainedJournalEntry = {
      ...entry(),
      entry_id: "blake3:revision",
      sequence: 2,
      title: "Verify the remaining surface",
      supersedes: root.entry_id,
    };
    const log = {
      schema: "rey.journal-log.v2" as const,
      log_id: "blake3:log",
      entries: [root, revision],
    };
    const props = {
      log,
      onAdmit: async () => revision,
      onClose: () => undefined,
    };
    const rootMarkup = renderToStaticMarkup(
      createElement(JournalDocumentPage, { ...props, entry: root }),
    );
    const revisionMarkup = renderToStaticMarkup(
      createElement(JournalDocumentPage, { ...props, entry: revision }),
    );
    expect(rootMarkup).toContain(
      `href="/journal/${journalEntrySlug(revision)}"`,
    );
    expect(revisionMarkup).toContain(
      `href="/journal/${journalEntrySlug(root)}"`,
    );
  });

  it("gives retained entries exact slugs and block-level deep links", () => {
    const retained = entry();
    const slug = "j1-mine-the-remaining-surface--blake3-entry";
    expect(journalEntrySlug(retained)).toBe(slug);
    expect(
      resolveJournalEntry(
        {
          schema: "rey.journal-log.v2",
          log_id: "blake3:log",
          entries: [retained],
        },
        slug,
      ),
    ).toBe(retained);
    expect(journalBlockFragment("coverage/query")).toBe("block-coverage/query");

    const markup = renderToStaticMarkup(
      createElement(JournalEntries, { compact: true, entries: [retained] }),
    );
    expect(markup).toContain(`href="/journal/${slug}"`);
    expect(markup).toContain(`data-journal-slug="${slug}"`);
  });
});

function entry(): RetainedJournalEntry {
  const coordinate = "rey+local://portfolio/current?revision=blake3%3Asource";
  return {
    schema: "rey.journal-entry.v2",
    entry_id: "blake3:entry",
    sequence: 1,
    admitted_at: "2026-08-10T21:00:00Z",
    title: "Mine the remaining surface",
    author: { kind: "agent", id: "codex" },
    binding: { coordinate, scale: 0.68, source_revision: "blake3:source" },
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
        {
          id: "evidence",
          cells: [
            { block_id: "query", span: 6 },
            { block_id: "frame", span: 6 },
          ],
        },
        {
          id: "bearing",
          cells: [
            { block_id: "diff", span: 8 },
            { block_id: "action", span: 4 },
          ],
        },
      ],
    },
    blocks: [
      {
        kind: "prose",
        id: "context",
        document: [{ kind: "paragraph", text: "Two sources remain." }],
      },
      {
        kind: "explore",
        id: "map",
        coordinate,
        scale: 0.68,
        source_revision: "blake3:source",
        caption: "Current portfolio",
      },
      {
        kind: "query",
        id: "query",
        language: "sql",
        provider: "local",
        mode: "read_only",
        statement: "select * from coverage",
        parameters: {},
      },
      {
        kind: "frame",
        id: "frame",
        source_block_id: "query",
        snapshot_id: "blake3:frame",
        columns: [{ name: "surface", data_type: "Utf8" }],
        preview_rows: [{ surface: "alpha" }, { surface: null }],
        row_count: 2,
        truncated: false,
      },
      {
        kind: "diff",
        id: "diff",
        source: "frame://before",
        target: "frame://after",
        direction: "expected_to_observed",
        assessment: "different",
        summary: "Two rows remain.",
      },
      {
        kind: "action",
        id: "action",
        operation: "refine",
        desired_delta: "Reduce remaining rows to zero.",
        evidence_ids: ["blake3:frame"],
        dependency_ids: [],
      },
    ],
  };
}

function portfolio(): WorkloadList {
  return {
    schema: "rey.workload-list.v1",
    semantic_atlas: null,
    semantic_atlas_history: [],
    semantic_atlas_deltas: [],
    catalog: {
      schema: "rey.workload-catalog.v1",
      kind: "workspace_packages",
      root: "workloads",
      workload_count: 0,
      admitted_count: 0,
      draft_count: 0,
    },
    workloads: [],
    drafts: [],
    attention: {
      schema: "rey.workload-attention.v1",
      attention_id: "blake3:attention",
      source_snapshot_id: "blake3:source",
      rows: [],
      summary: {
        refine: 0,
        retest: 0,
        create: 0,
        blocked: 0,
        policy_excluded: 0,
        workloads: 0,
        surfaces: 0,
        owned_surfaces: 0,
        unowned_surfaces: 0,
      },
    },
  };
}

function seed(): JournalSeed {
  const coordinate =
    "rey+local://document/observation-frontier?revision=blake3%3Alog";
  return {
    schema: "rey.journal-seed.v1",
    seed_id: "blake3:seed",
    source_log_id: "blake3:log",
    observation_ids: ["blake3:observation"],
    proposal: {
      schema: "rey.journal-entry-proposal.v2",
      title: "Catch up on 1 unresolved observation",
      author: { kind: "human", id: "operator" },
      binding: { coordinate, scale: 1, source_revision: "blake3:log" },
      supersedes: null,
      layout: {
        kind: "broadsheet",
        columns: 12,
        bands: [
          {
            id: "observation-1",
            cells: [
              { block_id: "observation-1-context", span: 8 },
              { block_id: "observation-1-source", span: 4 },
            ],
          },
        ],
      },
      blocks: [
        {
          kind: "prose",
          id: "observation-1-context",
          document: [
            {
              kind: "paragraph",
              text: "A retained delta remains unresolved.",
            },
          ],
        },
        {
          kind: "explore",
          id: "observation-1-source",
          coordinate:
            "rey+local://document/observation-blake3%3Aobservation?revision=blake3%3Aobservation",
          scale: 1,
          source_revision: "blake3:observation",
          caption: "Exact observation",
        },
      ],
    },
  };
}
