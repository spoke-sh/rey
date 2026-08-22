#!/usr/bin/env node

import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  LANDSCAPE_ASSESSMENT_SCHEMA,
  landscapeAssessmentRowKey,
  summarizeLandscapeAssessment,
  validateLandscapePerceptualReview,
} from "./explorer-landscape-assessment.mjs";
import { validateLandscapeWorkloadSuite } from "./explorer-landscape-qualification.mjs";

const AUTHORITY =
  "retained local operator assessment over exact browser captures and consumer-map references; not terrain evidence, source truth, verified operator identity, or proof authority";
const REPOSITORY_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const SUITE_PATH = join(
  REPOSITORY_ROOT,
  "apps/agent/qualification/explorer-landscape-workloads.json",
);
const DEFAULT_OUTPUT_ROOT = join(
  REPOSITORY_ROOT,
  ".rey/qualification/explorer",
);

function usage() {
  return `Bind and retain one explicit Landscape fidelity assessment.

Usage:
  pnpm qualify:explorer-landscape -- --review PATH [--output-dir PATH]

The review names each workload/viewport row, its reference/WebGL2/WebGPU
voyage manifests, rendered-parity manifest, operator-supplied consumer-map
reference, and pass/minor/major result for every suite criterion. Missing rows
and major results produce an INCOMPLETE retained report.`;
}

function parseArguments(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--") continue;
    if (token === "--help" || token === "-h") return { help: true };
    if (!token?.startsWith("--"))
      throw new Error(`unexpected argument ${token}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--"))
      throw new Error(`${token} requires a value`);
    values.set(token.slice(2), value);
    index += 1;
  }
  const review = values.get("review");
  if (!review) throw new Error("--review is required");
  return {
    review: isAbsolute(review) ? review : resolve(process.cwd(), review),
    outputRoot: values.get("output-dir")
      ? isAbsolute(values.get("output-dir"))
        ? values.get("output-dir")
        : resolve(process.cwd(), values.get("output-dir"))
      : DEFAULT_OUTPUT_ROOT,
  };
}

function sha256(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function timestamp() {
  return new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
}

function inputPath(reviewPath, path) {
  return isAbsolute(path) ? path : resolve(dirname(reviewPath), path);
}

async function loadJson(path, label) {
  const bytes = await readFile(path);
  let document;
  try {
    document = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new Error(`${label} is not valid JSON: ${path}`);
  }
  return { bytes, document, path, sha256: sha256(bytes) };
}

async function bindConsumerReference(reference, reviewPath) {
  const path = inputPath(reviewPath, reference.file);
  const bytes = await readFile(path);
  const observed = sha256(bytes);
  if (observed !== reference.sha256)
    throw new Error(
      `consumer reference ${reference.id} digest ${observed} does not match ${reference.sha256}`,
    );
  return {
    description: reference.description,
    file: path,
    id: reference.id,
    sha256: observed,
  };
}

async function bindVoyage(backend, row, reviewPath, suite) {
  const input = await loadJson(
    inputPath(reviewPath, row.manifests[backend]),
    `${backend} voyage`,
  );
  const manifest = input.document;
  if (manifest.schema !== "rey.explorer-qualification-voyage.v1")
    throw new Error(
      `${backend} voyage has unexpected schema ${manifest.schema}`,
    );
  if (manifest.voyage?.complete !== true)
    throw new Error(`${backend} voyage is incomplete`);
  const request = manifest.request ?? {};
  if (
    request.backend !== backend ||
    request.landscape_workload !== row.workload_id ||
    `${request.width}x${request.height}` !== row.viewport ||
    !suite.required_backends.includes(backend)
  )
    throw new Error(
      `${backend} voyage does not bind ${row.workload_id}@${row.viewport}`,
    );
  if (
    manifest.inputs?.landscape_workload?.workload_id !== row.workload_id ||
    manifest.landscape_workload?.passed !== true
  )
    throw new Error(`${backend} voyage did not pass its Landscape workload`);
  const capture = manifest.captures?.find(({ stage }) => stage === "landscape");
  if (!capture?.screenshot?.file || !capture.screenshot.sha256)
    throw new Error(`${backend} voyage has no retained Landscape screenshot`);
  const screenshotPath = resolve(dirname(input.path), capture.screenshot.file);
  const screenshotBytes = await readFile(screenshotPath);
  const screenshotDigest = sha256(screenshotBytes);
  if (screenshotDigest !== capture.screenshot.sha256)
    throw new Error(`${backend} Landscape screenshot digest changed`);
  return {
    backend,
    landscape_capture: {
      file: screenshotPath,
      sha256: screenshotDigest,
    },
    manifest_path: input.path,
    manifest_sha256: input.sha256,
    scene_snapshot_id: capture.scene_snapshot_id,
    terrain: {
      composition_revision:
        capture.renderer?.landscape_composition_revision ?? null,
      height_hierarchies:
        capture.renderer?.landscape_height_hierarchies ?? null,
      mosaic_id: capture.renderer?.landscape_mosaic_id ?? null,
      pyramid_envelopes: capture.renderer?.landscape_pyramid_envelopes ?? null,
      relief_pyramids: capture.renderer?.landscape_relief_pyramids ?? null,
      relief_revision: capture.renderer?.landscape_relief_revision ?? null,
      terrain_source_key: capture.renderer?.terrain_source_key ?? null,
    },
    transport_posture: manifest.voyage.transport_posture,
    voyage_name: manifest.voyage.name,
  };
}

async function bindParity(row, voyages, reviewPath) {
  const input = await loadJson(
    inputPath(reviewPath, row.parity_manifest),
    "rendered parity manifest",
  );
  const parity = input.document;
  if (
    parity.schema !== "rey.explorer-rendered-parity.v1" ||
    parity.complete !== true
  )
    throw new Error("rendered parity manifest is incomplete or unsupported");
  if (
    `${parity.request?.width}x${parity.request?.height}` !== row.viewport ||
    parity.input_binding?.landscape_workload?.workload_id !== row.workload_id
  )
    throw new Error(
      `rendered parity does not bind ${row.workload_id}@${row.viewport}`,
    );
  for (const voyage of voyages) {
    if (
      parity.inputs?.[voyage.backend]?.manifest_sha256 !==
      voyage.manifest_sha256
    )
      throw new Error(
        `rendered parity does not bind the exact ${voyage.backend} voyage`,
      );
  }
  return {
    manifest_path: input.path,
    manifest_sha256: input.sha256,
    maximum_accelerated_normalized_rmse:
      parity.assertions?.accelerated_maximum_normalized_rmse ?? null,
    semantic_evidence_equal: parity.assertions?.semantic_evidence_equal ?? null,
  };
}

async function run(options) {
  const [suiteInput, reviewInput] = await Promise.all([
    loadJson(SUITE_PATH, "Landscape workload suite"),
    loadJson(options.review, "Landscape perceptual review"),
  ]);
  const suite = validateLandscapeWorkloadSuite(suiteInput.document);
  const review = validateLandscapePerceptualReview(reviewInput.document, suite);
  const references = await Promise.all(
    review.consumer_references.map((reference) =>
      bindConsumerReference(reference, options.review),
    ),
  );
  const referenceById = new Map(
    references.map((reference) => [reference.id, reference]),
  );
  const evaluatedRows = [];
  for (const row of review.rows) {
    const retainedRow = {
      consumer_reference: referenceById.get(row.consumer_reference_id),
      criteria: row.criteria,
      key: landscapeAssessmentRowKey(row.workload_id, row.viewport),
      viewport: row.viewport,
      workload_id: row.workload_id,
    };
    try {
      const voyages = await Promise.all(
        suite.required_backends.map((backend) =>
          bindVoyage(backend, row, options.review, suite),
        ),
      );
      const parity = await bindParity(row, voyages, options.review);
      evaluatedRows.push({
        ...retainedRow,
        bindings_valid: true,
        parity,
        voyages: Object.fromEntries(
          voyages.map((voyage) => [voyage.backend, voyage]),
        ),
      });
    } catch (error) {
      evaluatedRows.push({
        ...retainedRow,
        binding_error: error instanceof Error ? error.message : String(error),
        bindings_valid: false,
        parity: null,
        voyages: null,
      });
    }
  }
  const coverage = summarizeLandscapeAssessment(evaluatedRows, suite);
  const outputDirectory = join(
    options.outputRoot,
    `${timestamp()}-landscape-fidelity-assessment`,
  );
  await mkdir(outputDirectory, { recursive: true });
  const report = {
    schema: LANDSCAPE_ASSESSMENT_SCHEMA,
    authority: AUTHORITY,
    complete: coverage.complete,
    suite: {
      path: suiteInput.path,
      sha256: suiteInput.sha256,
      suite_id: suite.suite_id,
    },
    review: {
      authority: review.authority,
      operator: review.operator,
      path: reviewInput.path,
      sha256: reviewInput.sha256,
    },
    consumer_references: references,
    coverage,
    rows: evaluatedRows,
    omissions: [
      "perceptual ratings and operator labels are self-asserted human judgments",
      "consumer-map references are comparison inputs, not admitted terrain evidence or licenses",
      ...(coverage.missing_rows.length > 0
        ? [
            `${coverage.missing_rows.length} required workload/viewport rows are missing`,
          ]
        : []),
      ...(coverage.invalid_rows.length > 0
        ? [
            `${coverage.invalid_rows.length} retained rows have invalid or changed bindings`,
          ]
        : []),
      ...(coverage.major_results.length > 0
        ? [
            `${coverage.major_results.length} major perceptual results remain open`,
          ]
        : []),
    ],
  };
  const outputPath = join(outputDirectory, "manifest.json");
  const bytes = Buffer.from(`${JSON.stringify(report, null, 2)}\n`);
  await writeFile(outputPath, bytes);
  process.stdout.write(
    `${coverage.complete ? "PASS" : "INCOMPLETE"} Landscape fidelity assessment\n` +
      `${coverage.evaluated_rows}/${coverage.expected_rows} rows; ` +
      `${coverage.invalid_rows.length} invalid; ${coverage.major_results.length} major; ` +
      `${coverage.minor_results.length} minor\n` +
      `${outputPath}\n${sha256(bytes)}\n`,
  );
  if (!coverage.complete) process.exitCode = 1;
}

try {
  const options = parseArguments(process.argv.slice(2));
  if (options.help) process.stdout.write(`${usage()}\n`);
  else await run(options);
} catch (error) {
  process.stderr.write(
    `${error instanceof Error ? error.message : String(error)}\n\n${usage()}\n`,
  );
  process.exitCode = 2;
}
