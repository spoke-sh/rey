import { describe, expect, it } from "vitest";
import type { WorkloadList } from "../../domain";
import { DEFAULT_LENS_ZOOM } from "./camera";
import { compileSceneSnapshot, LastGoodSceneCompiler } from "./scene";
import { PORTFOLIO_SCENE_PROJECTION_REVISION } from "../projection/portfolio-scene";

const emptyPortfolio = {
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
    attention_id: "attention:empty",
    source_snapshot_id: "portfolio:empty",
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
} satisfies WorkloadList;

describe("reference scene compiler", () => {
  it("creates an immutable snapshot whose identity excludes zoom within one regime", () => {
    const first = compileSceneSnapshot(
      emptyPortfolio,
      DEFAULT_LENS_ZOOM,
      "cluster:portfolio",
    );
    const second = compileSceneSnapshot(
      emptyPortfolio,
      DEFAULT_LENS_ZOOM + 0.02,
      "cluster:portfolio",
    );

    expect(first.snapshot_id).toBe(second.snapshot_id);
    expect(Object.isFrozen(first)).toBe(true);
    expect(Object.isFrozen(first.scene)).toBe(true);
    expect(Object.isFrozen(first.scene.nodes)).toBe(true);
    expect(first.source_revisions).toContain("attention:empty");
    expect(first.compiler_revisions).toContain(
      PORTFOLIO_SCENE_PROJECTION_REVISION,
    );
  });

  it("changes the view snapshot identity when focus changes", () => {
    const first = compileSceneSnapshot(
      emptyPortfolio,
      DEFAULT_LENS_ZOOM,
      "cluster:portfolio",
    );
    const focused = compileSceneSnapshot(
      emptyPortfolio,
      DEFAULT_LENS_ZOOM,
      "cluster:other",
    );
    expect(first.snapshot_id).not.toBe(focused.snapshot_id);
  });

  it("retains the last-good immutable scene when a later projection fails", () => {
    const compiler = new LastGoodSceneCompiler();
    const admitted = compiler.compile(
      emptyPortfolio,
      DEFAULT_LENS_ZOOM,
      "cluster:portfolio",
    );
    const rejected = compiler.compile(
      {} as WorkloadList,
      DEFAULT_LENS_ZOOM,
      "cluster:portfolio",
    );

    expect(admitted.retained_last_good).toBe(false);
    expect(rejected.retained_last_good).toBe(true);
    expect(rejected.snapshot).toBe(admitted.snapshot);
    expect(rejected.error?.message).toBeTruthy();
  });

  it("replaces retained fallback after a valid scene compiles", () => {
    const compiler = new LastGoodSceneCompiler();
    const original = compiler.compile(
      emptyPortfolio,
      DEFAULT_LENS_ZOOM,
      "cluster:portfolio",
    );
    compiler.compile(
      {} as WorkloadList,
      DEFAULT_LENS_ZOOM,
      "cluster:portfolio",
    );
    const recovered = compiler.compile(
      emptyPortfolio,
      DEFAULT_LENS_ZOOM,
      "cluster:recovered",
    );

    expect(recovered.retained_last_good).toBe(false);
    expect(recovered.error).toBeNull();
    expect(recovered.snapshot.snapshot_id).not.toBe(
      original.snapshot.snapshot_id,
    );
  });
});
