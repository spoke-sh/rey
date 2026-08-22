export const LANDSCAPE_REVIEW_SCHEMA =
  "rey.explorer-landscape-perceptual-review.v1";
export const LANDSCAPE_ASSESSMENT_SCHEMA =
  "rey.explorer-landscape-fidelity-assessment.v1";
export const PERCEPTUAL_RESULTS = Object.freeze(["pass", "minor", "major"]);

export function landscapeAssessmentRowKey(workloadId, viewport) {
  return `${workloadId}@${viewport}`;
}

export function expectedLandscapeAssessmentRows(suite) {
  return suite.workloads.flatMap(({ id }) =>
    suite.target_viewports.map((viewport) => ({
      key: landscapeAssessmentRowKey(id, viewport),
      viewport,
      workload_id: id,
    })),
  );
}

export function validateLandscapePerceptualReview(review, suite) {
  if (review?.schema !== LANDSCAPE_REVIEW_SCHEMA)
    throw new Error(`unexpected Landscape review schema ${review?.schema}`);
  if (review.suite_id !== suite.suite_id)
    throw new Error(
      `Landscape review suite ${review.suite_id} does not match ${suite.suite_id}`,
    );
  if (
    typeof review.authority !== "string" ||
    review.authority.length === 0 ||
    typeof review.operator?.label !== "string" ||
    review.operator.label.length === 0 ||
    !Array.isArray(review.consumer_references) ||
    review.consumer_references.length === 0 ||
    !Array.isArray(review.rows)
  )
    throw new Error("Landscape perceptual review is malformed");

  const referenceIds = new Set();
  for (const reference of review.consumer_references) {
    if (
      typeof reference?.id !== "string" ||
      reference.id.length === 0 ||
      referenceIds.has(reference.id) ||
      typeof reference.file !== "string" ||
      reference.file.length === 0 ||
      !/^sha256:[a-f0-9]{64}$/.test(reference.sha256 ?? "") ||
      typeof reference.description !== "string" ||
      reference.description.length === 0
    )
      throw new Error("Landscape consumer reference binding is malformed");
    referenceIds.add(reference.id);
  }

  const workloadIds = new Set(suite.workloads.map(({ id }) => id));
  const rowKeys = new Set();
  for (const row of review.rows) {
    const key = landscapeAssessmentRowKey(row?.workload_id, row?.viewport);
    if (
      !workloadIds.has(row?.workload_id) ||
      !suite.target_viewports.includes(row?.viewport) ||
      rowKeys.has(key) ||
      !referenceIds.has(row?.consumer_reference_id) ||
      typeof row.parity_manifest !== "string" ||
      row.parity_manifest.length === 0 ||
      !row.manifests ||
      typeof row.manifests !== "object" ||
      Object.keys(row.manifests).sort().join(",") !==
        [...suite.required_backends].sort().join(",") ||
      suite.required_backends.some(
        (backend) =>
          typeof row.manifests[backend] !== "string" ||
          row.manifests[backend].length === 0,
      ) ||
      !Array.isArray(row.criteria)
    )
      throw new Error(`Landscape perceptual review row ${key} is malformed`);
    rowKeys.add(key);

    const criterionIds = row.criteria.map(({ criterion }) => criterion);
    if (
      new Set(criterionIds).size !== criterionIds.length ||
      criterionIds.length !== suite.perceptual_criteria.length ||
      suite.perceptual_criteria.some(
        (criterion) => !criterionIds.includes(criterion),
      ) ||
      row.criteria.some(
        ({ criterion, result, note }) =>
          typeof criterion !== "string" ||
          !PERCEPTUAL_RESULTS.includes(result) ||
          typeof note !== "string" ||
          note.length === 0,
      )
    )
      throw new Error(
        `Landscape perceptual review row ${key} has incomplete criteria`,
      );
  }
  return review;
}

export function summarizeLandscapeAssessment(evaluatedRows, suite) {
  const expected = expectedLandscapeAssessmentRows(suite);
  const evaluatedByKey = new Map(
    evaluatedRows.map((row) => [
      landscapeAssessmentRowKey(row.workload_id, row.viewport),
      row,
    ]),
  );
  const missingRows = expected.filter(({ key }) => !evaluatedByKey.has(key));
  const invalidRows = evaluatedRows.filter(
    ({ bindings_valid }) => bindings_valid !== true,
  );
  const majorResults = evaluatedRows.flatMap((row) =>
    row.criteria
      .filter(({ result }) => result === "major")
      .map(({ criterion, note }) => ({
        criterion,
        note,
        viewport: row.viewport,
        workload_id: row.workload_id,
      })),
  );
  const minorResults = evaluatedRows.flatMap((row) =>
    row.criteria
      .filter(({ result }) => result === "minor")
      .map(({ criterion, note }) => ({
        criterion,
        note,
        viewport: row.viewport,
        workload_id: row.workload_id,
      })),
  );
  return {
    complete:
      missingRows.length === 0 &&
      invalidRows.length === 0 &&
      majorResults.length === 0,
    expected_rows: expected.length,
    evaluated_rows: evaluatedRows.length,
    invalid_rows: invalidRows.map(({ viewport, workload_id }) => ({
      viewport,
      workload_id,
    })),
    major_results: majorResults,
    minor_results: minorResults,
    missing_rows: missingRows.map(({ viewport, workload_id }) => ({
      viewport,
      workload_id,
    })),
  };
}
