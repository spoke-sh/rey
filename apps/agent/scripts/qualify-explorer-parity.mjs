#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCHEMA = "rey.explorer-rendered-parity.v1";
const AUTHORITY =
  "retained local rendered-output qualification only; pixel difference is a perceptual regression signal, not semantic evidence, source truth, GPU execution timing, or proof authority";
const REPOSITORY_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const DEFAULT_OUTPUT_ROOT = join(
  REPOSITORY_ROOT,
  ".rey/qualification/explorer",
);
const BACKENDS = ["reference", "webgl2", "webgpu"];
const ACCELERATED_PARITY_LIMIT = 0.02;

function usage() {
  return `Compare one bound set of retained Explorer voyage captures.

Usage:
  pnpm qualify:explorer-parity -- \\
    --reference PATH/manifest.json \\
    --webgl2 PATH/manifest.json \\
    --webgpu PATH/manifest.json [--output-dir PATH]

ImageMagick compare must be available. Output stays beneath ignored
.rey/qualification/explorer by default.`;
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
  const manifests = Object.fromEntries(
    BACKENDS.map((backend) => {
      const path = values.get(backend);
      if (!path) throw new Error(`--${backend} is required`);
      return [
        backend,
        isAbsolute(path) ? path : resolve(REPOSITORY_ROOT, path),
      ];
    }),
  );
  return {
    manifests,
    outputRoot: values.get("output-dir")
      ? isAbsolute(values.get("output-dir"))
        ? values.get("output-dir")
        : resolve(REPOSITORY_ROOT, values.get("output-dir"))
      : DEFAULT_OUTPUT_ROOT,
  };
}

function digest(buffer) {
  return `sha256:${createHash("sha256").update(buffer).digest("hex")}`;
}

function timestamp() {
  return new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
}

function stableEvidence(capture) {
  return {
    exact_evidence_uris: capture.exact_evidence_links
      .map(({ uri }) => uri)
      .sort((left, right) => left.localeCompare(right)),
    focus_id: capture.focus_id,
    labels: capture.labels,
    projection: {
      regime: capture.projection.regime,
      render_graph_id: capture.projection.render_graph_id,
      render_passes: capture.projection.render_passes,
    },
    scene_snapshot_id: capture.scene_snapshot_id,
    scene_omissions: capture.scene_omissions,
    source_revisions: capture.source_revisions,
    stage: capture.stage,
    terrain: {
      no_data_leak_triangles: capture.renderer?.terrain_no_data_leak_triangles,
      render_pass_kinds: capture.renderer?.render_pass_kinds,
      render_pass_area_count: capture.renderer?.render_pass_area_count,
      render_pass_line_batch_count:
        capture.renderer?.render_pass_line_batch_count,
      render_pass_line_count: capture.renderer?.render_pass_line_count,
      render_pass_point_count: capture.renderer?.render_pass_point_count,
      render_pass_set_id: capture.renderer?.render_pass_set_id,
      source_elevation_maximum: capture.renderer?.source_elevation_maximum,
      source_elevation_minimum: capture.renderer?.source_elevation_minimum,
      source_elevation_span: capture.renderer?.source_elevation_span,
      source_no_data_vertices: capture.renderer?.source_no_data_vertices,
      source_valid_vertices: capture.renderer?.source_valid_vertices,
      relief_partition_mismatches:
        capture.renderer?.terrain_relief_partition_mismatches,
      tile_seam_mismatches: capture.renderer?.terrain_tile_seam_mismatches,
    },
  };
}

function normalizedRmse(left, right) {
  const result = spawnSync(
    "compare",
    ["-metric", "RMSE", left, right, "null:"],
    {
      encoding: "utf8",
    },
  );
  if (result.error) throw result.error;
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(
      `ImageMagick compare failed (${result.status}): ${(result.stderr || result.stdout).trim()}`,
    );
  }
  const match = /\(([0-9.eE+-]+)\)/.exec(result.stderr || result.stdout);
  if (!match)
    throw new Error("ImageMagick compare did not emit normalized RMSE");
  return Number(match[1]);
}

async function loadManifest(backend, path) {
  const bytes = await readFile(path);
  const manifest = JSON.parse(bytes.toString("utf8"));
  if (manifest.schema !== "rey.explorer-qualification-voyage.v1")
    throw new Error(
      `${backend} input has unexpected schema ${manifest.schema}`,
    );
  if (manifest.request.backend !== backend)
    throw new Error(
      `${backend} input reports requested backend ${manifest.request.backend}`,
    );
  if (!manifest.voyage.complete)
    throw new Error(`${backend} input voyage is incomplete`);
  return { bytes, manifest, path };
}

async function run(options) {
  const compareVersion = spawnSync("compare", ["-version"], {
    encoding: "utf8",
  });
  if (compareVersion.error) throw compareVersion.error;
  if (compareVersion.status !== 0)
    throw new Error("ImageMagick compare is unavailable");

  const inputs = Object.fromEntries(
    await Promise.all(
      BACKENDS.map(async (backend) => [
        backend,
        await loadManifest(backend, options.manifests[backend]),
      ]),
    ),
  );
  const referenceRequest = inputs.reference.manifest.request;
  for (const backend of BACKENDS.slice(1)) {
    const request = inputs[backend].manifest.request;
    for (const key of [
      "device_pixel_ratio",
      "height",
      "landscape_workload",
      "region",
      "width",
    ]) {
      if (request[key] !== referenceRequest[key])
        throw new Error(`${backend} request does not match reference ${key}`);
    }
  }
  const inputBinding = {
    atlas_revision: inputs.reference.manifest.inputs.atlas_revision,
    attention_snapshot_id:
      inputs.reference.manifest.inputs.attention_snapshot_id,
    admitted_scene_id:
      inputs.reference.manifest.inputs.admitted_region.source_scene_id,
    landscape_workload:
      inputs.reference.manifest.inputs.landscape_workload ?? null,
  };
  for (const backend of BACKENDS.slice(1)) {
    const candidate = inputs[backend].manifest.inputs;
    if (
      candidate.atlas_revision !== inputBinding.atlas_revision ||
      candidate.attention_snapshot_id !== inputBinding.attention_snapshot_id ||
      candidate.admitted_region.source_scene_id !==
        inputBinding.admitted_scene_id ||
      JSON.stringify(candidate.landscape_workload ?? null) !==
        JSON.stringify(inputBinding.landscape_workload)
    ) {
      throw new Error(
        `${backend} voyage does not bind the same admitted inputs`,
      );
    }
  }

  const referenceEvidence =
    inputs.reference.manifest.captures.map(stableEvidence);
  const semanticParity = Object.fromEntries(
    BACKENDS.slice(1).map((backend) => [
      backend,
      JSON.stringify(inputs[backend].manifest.captures.map(stableEvidence)) ===
        JSON.stringify(referenceEvidence),
    ]),
  );
  const comparisons = [];
  for (const [leftBackend, rightBackend] of [
    ["reference", "webgl2"],
    ["reference", "webgpu"],
    ["webgl2", "webgpu"],
  ]) {
    const left = inputs[leftBackend];
    const right = inputs[rightBackend];
    for (const leftCapture of left.manifest.captures) {
      const rightCapture = right.manifest.captures.find(
        ({ stage }) => stage === leftCapture.stage,
      );
      if (!rightCapture)
        throw new Error(
          `${rightBackend} is missing stage ${leftCapture.stage}`,
        );
      const leftImage = join(dirname(left.path), leftCapture.screenshot.file);
      const rightImage = join(
        dirname(right.path),
        rightCapture.screenshot.file,
      );
      comparisons.push({
        left_backend: leftBackend,
        right_backend: rightBackend,
        stage: leftCapture.stage,
        left_screenshot: leftCapture.screenshot,
        right_screenshot: rightCapture.screenshot,
        normalized_rmse: normalizedRmse(leftImage, rightImage),
      });
    }
  }
  const acceleratedComparisons = comparisons.filter(
    ({ left_backend, right_backend }) =>
      left_backend === "webgl2" && right_backend === "webgpu",
  );
  const maximumAcceleratedRmse = Math.max(
    ...acceleratedComparisons.map(({ normalized_rmse }) => normalized_rmse),
  );
  const complete =
    Object.values(semanticParity).every(Boolean) &&
    maximumAcceleratedRmse <= ACCELERATED_PARITY_LIMIT;
  const name = `rendered-parity-${referenceRequest.width}x${referenceRequest.height}`;
  const outputDirectory = join(options.outputRoot, `${timestamp()}-${name}`);
  await mkdir(outputDirectory, { recursive: true });
  const manifest = {
    schema: SCHEMA,
    authority: AUTHORITY,
    name,
    complete,
    request: {
      width: referenceRequest.width,
      height: referenceRequest.height,
      device_pixel_ratio: referenceRequest.device_pixel_ratio,
      region: referenceRequest.region,
    },
    input_binding: inputBinding,
    inputs: Object.fromEntries(
      BACKENDS.map((backend) => [
        backend,
        {
          manifest_path: inputs[backend].path,
          manifest_sha256: digest(inputs[backend].bytes),
          voyage_name: inputs[backend].manifest.voyage.name,
          transport_posture: inputs[backend].manifest.voyage.transport_posture,
        },
      ]),
    ),
    assertions: {
      semantic_evidence_equal: semanticParity,
      accelerated_maximum_normalized_rmse: maximumAcceleratedRmse,
      accelerated_normalized_rmse_limit: ACCELERATED_PARITY_LIMIT,
      accelerated_output_within_limit:
        maximumAcceleratedRmse <= ACCELERATED_PARITY_LIMIT,
    },
    comparisons,
    tool: compareVersion.stdout.split("\n")[0],
    omissions: [
      "reference and accelerated materials intentionally differ; no reference-to-accelerated pixel threshold asserts semantic equality",
      "pixel comparison does not assess GPU execution, frame rate, direct browser transport, or source truth",
    ],
  };
  const outputPath = join(outputDirectory, "manifest.json");
  const serialized = Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(outputPath, serialized);
  process.stdout.write(
    `${complete ? "PASS" : "INCOMPLETE"} ${name}\n${outputPath}\n${digest(serialized)}\n`,
  );
  if (!complete) process.exitCode = 1;
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
