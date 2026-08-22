#!/usr/bin/env node

import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCHEMA = "rey.explorer-performance-qualification.v1";
const VOYAGE_SCHEMA = "rey.explorer-qualification-voyage.v1";
const BUDGET_SCHEMA = "rey.explorer-performance-budget.v1";
const REQUIRED_INTERACTIONS = [
  "world_to_atlas",
  "atlas_to_county",
  "county_to_neighborhoods",
  "neighborhoods_to_objects",
  "objects_to_evidence",
];
const REPOSITORY_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const DEFAULT_BUDGET = join(
  REPOSITORY_ROOT,
  "apps/agent/qualification/explorer-performance-budget.json",
);

function usage() {
  return `Retain one named Explorer performance qualification over a complete voyage matrix.

Usage:
  pnpm qualify:explorer-performance -- --machine-name NAME --manifest PATH [--manifest PATH ...] [options]

Required:
  --machine-name NAME   Human-readable name for this exact reference machine
  --manifest PATH       Voyage manifest; repeat for every required backend/viewport

Options:
  --budget PATH         Versioned budget document (default: qualification/explorer-performance-budget.json)
  --output-dir PATH     Retained output root (default: .rey/qualification/explorer)
  --help                Show this help

The result bounds observable CPU, resource, interaction, and browser-presentation
measurements. It does not measure GPU execution or establish a frame-rate claim.`;
}

function parseArguments(argv) {
  const manifests = [];
  let budget = DEFAULT_BUDGET;
  let machineName;
  let outputRoot = join(REPOSITORY_ROOT, ".rey/qualification/explorer");
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--") continue;
    if (token === "--help" || token === "-h") return { help: true };
    const value = argv[index + 1];
    if (!value || value.startsWith("--"))
      throw new Error(`${token} requires a value`);
    if (token === "--manifest")
      manifests.push(
        isAbsolute(value) ? value : resolve(REPOSITORY_ROOT, value),
      );
    else if (token === "--budget")
      budget = isAbsolute(value) ? value : resolve(REPOSITORY_ROOT, value);
    else if (token === "--machine-name") machineName = value.trim();
    else if (token === "--output-dir")
      outputRoot = isAbsolute(value) ? value : resolve(REPOSITORY_ROOT, value);
    else throw new Error(`unexpected argument ${token}`);
    index += 1;
  }
  if (!machineName) throw new Error("--machine-name is required");
  if (manifests.length === 0)
    throw new Error("at least one --manifest is required");
  return { budget, machineName, manifests, outputRoot };
}

function timestamp() {
  return new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
}

function sha256(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function finiteNumber(value, description) {
  const number = Number(value);
  if (!Number.isFinite(number) || number < 0)
    throw new Error(`${description} is not a finite non-negative number`);
  return number;
}

function maximum(values) {
  return values.length === 0 ? 0 : Math.max(...values);
}

function machineIdentity(machine) {
  return JSON.stringify({
    architecture: machine.architecture,
    browser: machine.browser,
    browser_version: machine.browser_version,
    cpu_model: machine.cpu_model,
    hostname: machine.hostname,
    operating_system: machine.operating_system,
    release: machine.release,
  });
}

function summarizeVoyage(document, path) {
  if (document.schema !== VOYAGE_SCHEMA)
    throw new Error(`${path} has unexpected schema ${document.schema}`);
  if (document.voyage?.complete !== true)
    throw new Error(`${path} is not a complete voyage`);
  if (
    document.request?.landscape_workload &&
    document.landscape_workload?.passed !== true
  )
    throw new Error(`${path} did not pass its named Landscape workload`);
  const captures = document.captures ?? [];
  if (captures.length === 0) throw new Error(`${path} has no captures`);
  const interactions = document.interactions ?? [];
  const missingInteractions = REQUIRED_INTERACTIONS.filter(
    (name) => !interactions.some((interaction) => interaction.name === name),
  );
  if (missingInteractions.length > 0)
    throw new Error(
      `${path} lacks required interactions: ${missingInteractions.join(", ")}`,
    );
  const measurements = {
    voyage_elapsed_ms: finiteNumber(
      document.voyage.elapsed_ms,
      `${path} voyage elapsed time`,
    ),
    scene_compilation_ms: maximum(
      captures.map((capture) =>
        finiteNumber(capture.scene_compilation_ms, `${path} scene compilation`),
      ),
    ),
    field_evaluation_ms: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.field_evaluation_ms,
          `${path} field evaluation`,
        ),
      ),
    ),
    geometry_compilation_ms: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.geometry_compilation_ms,
          `${path} geometry compilation`,
        ),
      ),
    ),
    render_submission_ms: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.submission_ms,
          `${path} render submission`,
        ),
      ),
    ),
    draw_calls: maximum(
      captures.map((capture) =>
        finiteNumber(capture.renderer?.draw_calls, `${path} draw calls`),
      ),
    ),
    gpu_upload_bytes: maximum(
      captures.map((capture) =>
        finiteNumber(capture.renderer?.gpu_bytes, `${path} GPU upload bytes`),
      ),
    ),
    resident_cpu_bytes: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.resident_cpu_bytes,
          `${path} resident CPU bytes`,
        ),
      ),
    ),
    resident_gpu_bytes: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.resident_gpu_bytes,
          `${path} resident GPU bytes`,
        ),
      ),
    ),
    render_pass_lines: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.render_pass_line_count,
          `${path} render-pass lines`,
        ),
      ),
    ),
    render_pass_areas: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.render_pass_area_count,
          `${path} render-pass areas`,
        ),
      ),
    ),
    render_pass_line_batches: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.render_pass_line_batch_count,
          `${path} render-pass line batches`,
        ),
      ),
    ),
    terrain_screen_error_pixels: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.terrain_maximum_screen_error_pixels,
          `${path} terrain screen error`,
        ),
      ),
    ),
    terrain_tile_seam_mismatches: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.terrain_tile_seam_mismatches,
          `${path} terrain tile seam mismatches`,
        ),
      ),
    ),
    terrain_relief_partition_mismatches: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.terrain_relief_partition_mismatches,
          `${path} terrain relief partition mismatches`,
        ),
      ),
    ),
    terrain_no_data_leak_triangles: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.renderer?.terrain_no_data_leak_triangles,
          `${path} terrain no-data leakage`,
        ),
      ),
    ),
    label_candidates: maximum(
      captures.map((capture) =>
        finiteNumber(capture.labels?.total, `${path} label candidates`),
      ),
    ),
    js_heap_used_bytes: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.performance_metrics?.js_heap_used_bytes,
          `${path} JavaScript heap`,
        ),
      ),
    ),
    frame_cadence_median_ms: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.frame_cadence?.median_ms,
          `${path} frame cadence median`,
        ),
      ),
    ),
    frame_cadence_p95_ms: maximum(
      captures.map((capture) =>
        finiteNumber(
          capture.frame_cadence?.p95_ms,
          `${path} frame cadence p95`,
        ),
      ),
    ),
    interaction_convergence_ms: maximum(
      interactions.map((interaction) =>
        finiteNumber(interaction.elapsed_ms, `${path} interaction convergence`),
      ),
    ),
  };
  const first = captures[0];
  return {
    backend: document.request.backend,
    input_identity: JSON.stringify({
      admitted_region: document.inputs.admitted_region,
      atlas_revision: document.inputs.atlas_revision,
      landscape_workload: document.inputs.landscape_workload ?? null,
      workload_revision: document.inputs.workload_revision,
    }),
    machine_identity: machineIdentity(document.machine),
    manifest: path,
    measurements,
    scene_source_identity: JSON.stringify(
      captures.map((capture) => ({
        focus_id: capture.focus_id,
        scene_snapshot_id: capture.scene_snapshot_id,
        source_revisions: capture.source_revisions,
        stage: capture.stage,
      })),
    ),
    transport: document.request.transport,
    landscape_workload: document.request.landscape_workload ?? null,
    viewport: `${document.request.width}x${document.request.height}`,
  };
}

async function qualify(options) {
  const budgetBytes = await readFile(options.budget);
  const budget = JSON.parse(budgetBytes);
  if (budget.schema !== BUDGET_SCHEMA)
    throw new Error(`unexpected budget schema ${budget.schema}`);
  const loaded = await Promise.all(
    options.manifests.map(async (path) => {
      const bytes = await readFile(path);
      return {
        bytes,
        document: JSON.parse(bytes),
        path,
        sha256: sha256(bytes),
      };
    }),
  );
  const voyages = loaded.map(({ document, path }) =>
    summarizeVoyage(document, path),
  );
  const expectedMatrix = budget.required_viewports.flatMap((viewport) =>
    budget.required_backends.map((backend) => `${backend}:${viewport}`),
  );
  const observedMatrix = voyages
    .map(({ backend, viewport }) => `${backend}:${viewport}`)
    .sort();
  if (new Set(observedMatrix).size !== observedMatrix.length)
    throw new Error("the performance matrix contains a duplicate voyage");
  const missing = expectedMatrix.filter(
    (entry) => !observedMatrix.includes(entry),
  );
  const extra = observedMatrix.filter(
    (entry) => !expectedMatrix.includes(entry),
  );
  if (missing.length > 0 || extra.length > 0)
    throw new Error(
      `performance matrix mismatch (missing: ${missing.join(", ") || "none"}; extra: ${extra.join(", ") || "none"})`,
    );
  if (
    new Set(voyages.map(({ machine_identity }) => machine_identity)).size !== 1
  )
    throw new Error("voyages were not captured on one exact machine/browser");
  if (new Set(voyages.map(({ transport }) => transport)).size !== 1)
    throw new Error("voyages do not share one transport posture");
  if (
    new Set(voyages.map(({ landscape_workload }) => landscape_workload))
      .size !== 1
  )
    throw new Error("voyages do not share one named Landscape workload");
  for (const viewport of budget.required_viewports) {
    const members = voyages.filter((voyage) => voyage.viewport === viewport);
    if (new Set(members.map(({ input_identity }) => input_identity)).size !== 1)
      throw new Error(
        `${viewport} voyages do not bind the same admitted input`,
      );
    if (
      new Set(members.map(({ scene_source_identity }) => scene_source_identity))
        .size !== 1
    )
      throw new Error(
        `${viewport} voyages do not bind the same scene/source lineage`,
      );
  }

  const evaluations = voyages.flatMap((voyage) =>
    Object.entries(budget.ceilings).map(([metric, ceiling]) => {
      const observed = finiteNumber(
        voyage.measurements[metric],
        `${voyage.backend}:${voyage.viewport} ${metric}`,
      );
      return {
        backend: voyage.backend,
        ceiling,
        metric,
        observed,
        passed: observed <= ceiling,
        viewport: voyage.viewport,
      };
    }),
  );
  const acceleratedAllocation = voyages
    .filter(({ backend }) => backend !== "reference")
    .every(
      ({ measurements }) =>
        measurements.draw_calls > 0 && measurements.gpu_upload_bytes > 0,
    );
  const passed =
    evaluations.every(({ passed: evaluationPassed }) => evaluationPassed) &&
    acceleratedAllocation;
  const sourceOmissions = [
    ...new Set(loaded.flatMap(({ document }) => document.omissions ?? [])),
  ];
  const outputDirectory = join(
    options.outputRoot,
    `${timestamp()}-performance-${options.machineName.replaceAll(/[^a-zA-Z0-9_-]+/g, "-")}`,
  );
  await mkdir(outputDirectory, { recursive: true });
  const manifest = {
    schema: SCHEMA,
    authority:
      "retained local performance qualification over observable CPU, resource, interaction, and browser-presentation measurements; not semantic evidence, GPU execution timing, a frame-rate claim, action authority, or proof authority",
    qualification: {
      machine_name: options.machineName,
      passed,
      transport: voyages[0].transport,
    },
    budget: {
      budget_id: budget.budget_id,
      path: options.budget,
      sha256: sha256(budgetBytes),
      ceilings: budget.ceilings,
    },
    machine: loaded[0].document.machine,
    assertions: {
      accelerated_draw_and_upload_present: acceleratedAllocation,
      complete_backend_viewport_matrix: true,
      exact_machine_browser_match: true,
      exact_viewport_input_and_scene_lineage_match: true,
      one_landscape_workload: true,
      one_transport_posture: true,
    },
    evaluations,
    voyages: loaded.map(({ document, path, sha256: digest }, index) => ({
      backend: document.request.backend,
      manifest: path,
      measurements: voyages[index].measurements,
      sha256: digest,
      viewport: voyages[index].viewport,
    })),
    omissions: [
      ...sourceOmissions,
      "requestAnimationFrame samples browser presentation cadence, not GPU execution duration",
      "render submission measures the synchronous CPU call boundary, not GPU completion",
      ...(voyages[0].transport === "fulfilled"
        ? [
            "fulfilled transport does not qualify direct browser networking or passive live revalidation",
          ]
        : []),
    ],
  };
  const serialized = `${JSON.stringify(manifest, null, 2)}\n`;
  const manifestPath = join(outputDirectory, "manifest.json");
  await writeFile(manifestPath, serialized);
  process.stdout.write(
    `${passed ? "PASS" : "FAIL"} ${budget.budget_id} / ${options.machineName}\n${manifestPath}\n${sha256(serialized)}\n`,
  );
  if (!passed) process.exitCode = 1;
}

try {
  const options = parseArguments(process.argv.slice(2));
  if (options.help) process.stdout.write(`${usage()}\n`);
  else await qualify(options);
} catch (error) {
  process.stderr.write(
    `${error instanceof Error ? error.message : String(error)}\n\n${usage()}\n`,
  );
  process.exitCode = 2;
}
