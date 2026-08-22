import { describe, expect, it } from "vitest";
import {
  expectedLandscapeAssessmentRows,
  LANDSCAPE_REVIEW_SCHEMA,
  landscapeAssessmentRowKey,
  summarizeLandscapeAssessment,
  validateLandscapePerceptualReview,
} from "./explorer-landscape-assessment.mjs";

const suite = {
  suite_id: "landscape-suite@fixture",
  required_backends: ["reference", "webgl2", "webgpu"],
  target_viewports: ["1920x1080", "3840x2160"],
  perceptual_criteria: ["composition", "hillshade_continuity"],
  workloads: [{ id: "steep-relief" }, { id: "explicit-holes" }],
};

const row = {
  workload_id: "steep-relief",
  viewport: "1920x1080",
  consumer_reference_id: "consumer-map",
  manifests: {
    reference: "reference/manifest.json",
    webgl2: "webgl2/manifest.json",
    webgpu: "webgpu/manifest.json",
  },
  parity_manifest: "parity/manifest.json",
  criteria: [
    { criterion: "composition", result: "pass", note: "balanced" },
    {
      criterion: "hillshade_continuity",
      result: "minor",
      note: "one low-contrast shoulder",
    },
  ],
};

const review = {
  schema: LANDSCAPE_REVIEW_SCHEMA,
  suite_id: suite.suite_id,
  authority: "self-asserted operator review",
  operator: { label: "fixture operator" },
  consumer_references: [
    {
      id: "consumer-map",
      file: "consumer.png",
      sha256: `sha256:${"a".repeat(64)}`,
      description: "operator-supplied map reference",
    },
  ],
  rows: [row],
};

describe("Landscape fidelity assessment", () => {
  it("enumerates every workload and viewport row", () => {
    expect(expectedLandscapeAssessmentRows(suite)).toHaveLength(4);
    expect(landscapeAssessmentRowKey("steep-relief", "1920x1080")).toBe(
      "steep-relief@1920x1080",
    );
  });

  it("validates exact backend, reference, and perceptual bindings", () => {
    expect(validateLandscapePerceptualReview(review, suite)).toBe(review);
    expect(() =>
      validateLandscapePerceptualReview(
        {
          ...review,
          rows: [
            {
              ...row,
              criteria: row.criteria.slice(0, 1),
            },
          ],
        },
        suite,
      ),
    ).toThrow("incomplete criteria");
  });

  it("keeps missing rows open while allowing explicitly retained minor results", () => {
    expect(
      summarizeLandscapeAssessment([{ ...row, bindings_valid: true }], suite),
    ).toMatchObject({
      complete: false,
      evaluated_rows: 1,
      expected_rows: 4,
      invalid_rows: [],
      major_results: [],
      minor_results: [
        {
          criterion: "hillshade_continuity",
          workload_id: "steep-relief",
        },
      ],
    });
  });

  it("leaves a fully covered assessment open when any result is major", () => {
    const rows = expectedLandscapeAssessmentRows(suite).map(
      ({ viewport, workload_id }, index) => ({
        ...row,
        workload_id,
        viewport,
        bindings_valid: true,
        criteria:
          index === 0
            ? [
                {
                  criterion: "composition",
                  result: "major",
                  note: "scalar mud remains",
                },
                row.criteria[1],
              ]
            : row.criteria,
      }),
    );
    expect(summarizeLandscapeAssessment(rows, suite)).toMatchObject({
      complete: false,
      missing_rows: [],
      major_results: [
        {
          criterion: "composition",
          note: "scalar mud remains",
          workload_id: "steep-relief",
          viewport: "1920x1080",
        },
      ],
    });
  });

  it("retains a present row with changed lineage as invalid rather than missing", () => {
    const result = summarizeLandscapeAssessment(
      [{ ...row, bindings_valid: false }],
      {
        ...suite,
        target_viewports: ["1920x1080"],
        workloads: [{ id: "steep-relief" }],
      },
    );
    expect(result).toMatchObject({
      complete: false,
      evaluated_rows: 1,
      invalid_rows: [{ viewport: "1920x1080", workload_id: "steep-relief" }],
      missing_rows: [],
    });
  });
});
