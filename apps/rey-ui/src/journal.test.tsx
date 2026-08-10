import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { WorkloadList } from "./domain";
import {
  defaultJournalBinding,
  JournalEntries,
  type RetainedJournalEntry,
} from "./journal";

describe("collaboration Journal", () => {
  it("binds new entries to the exact current portfolio coordinate", () => {
    expect(defaultJournalBinding(portfolio())).toEqual({
      coordinate:
        "/explore/portfolio/current;at=blake3%3Asource;lens=landscape",
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
    ]) {
      expect(markup).toContain(evidence);
    }
  });
});

function entry(): RetainedJournalEntry {
  const coordinate =
    "/explore/portfolio/current;at=blake3%3Asource;lens=landscape";
  return {
    schema: "rey.journal-entry.v1",
    entry_id: "blake3:entry",
    sequence: 1,
    admitted_at: "2026-08-10T21:00:00Z",
    title: "Mine the remaining surface",
    author: { kind: "agent", id: "codex" },
    binding: { coordinate, source_revision: "blake3:source" },
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
