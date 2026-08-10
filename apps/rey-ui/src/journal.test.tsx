import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { WorkloadList } from "./domain";
import {
  defaultJournalBinding,
  journalBlockFragment,
  journalEntrySlug,
  JournalEntries,
  resolveJournalEntry,
  type RetainedJournalEntry,
} from "./journal";

describe("collaboration Journal", () => {
  it("binds new entries to the exact current portfolio coordinate", () => {
    expect(defaultJournalBinding(portfolio())).toEqual({
      coordinate: "rey+local://portfolio/current?revision=blake3%3Asource",
      scale: 0.68,
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
        provider: "spoke",
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
    schema: "rey.workload-list.v5",
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
