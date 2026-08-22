export const LANDSCAPE_WORKLOAD_SCHEMA = "rey.explorer-landscape-workloads.v2";

export function validateLandscapeWorkloadSuite(document) {
  if (document?.schema !== LANDSCAPE_WORKLOAD_SCHEMA)
    throw new Error(`unexpected Landscape workload schema ${document?.schema}`);
  if (
    typeof document.suite_id !== "string" ||
    document.suite_id.length === 0 ||
    !Array.isArray(document.target_viewports) ||
    document.target_viewports.length === 0 ||
    document.target_viewports.some((viewport) => !/^\d+x\d+$/.test(viewport)) ||
    !Array.isArray(document.workloads) ||
    document.workloads.length === 0
  )
    throw new Error("Landscape workload suite is malformed");
  const ids = new Set();
  for (const workload of document.workloads) {
    if (
      typeof workload?.id !== "string" ||
      workload.id.length === 0 ||
      ids.has(workload.id) ||
      typeof workload.purpose !== "string" ||
      workload.purpose.length === 0 ||
      !workload.requirements ||
      typeof workload.requirements !== "object"
    )
      throw new Error("Landscape workload definition is malformed");
    ids.add(workload.id);
  }
  return document;
}

export function landscapeWorkload(document, workloadId, viewport) {
  const suite = validateLandscapeWorkloadSuite(document);
  if (!suite.target_viewports.includes(viewport))
    throw new Error(
      `${viewport} is not a target viewport for ${suite.suite_id}`,
    );
  const workload = suite.workloads.find(({ id }) => id === workloadId);
  if (!workload) throw new Error(`unknown Landscape workload ${workloadId}`);
  return workload;
}

export function evaluateLandscapeCapture(
  capture,
  workload,
  request,
  lossFallbackObserved,
) {
  const renderer = capture?.renderer ?? {};
  const requirements = workload.requirements;
  const number = (name) => {
    const value = Number(renderer[name]);
    return Number.isFinite(value) ? value : Number.NaN;
  };
  const renderPasses = new Set(capture?.projection?.render_passes ?? []);
  const renderPassKinds = new Set(
    String(renderer.render_pass_kinds ?? "")
      .split(",")
      .filter(Boolean),
  );
  const omissions = (capture?.scene_omissions ?? []).map((omission) =>
    String(omission).toLowerCase(),
  );
  const checks = {
    landscape_stage: capture?.stage === "landscape",
    exact_scene_lineage:
      typeof capture?.scene_snapshot_id === "string" &&
      capture.scene_snapshot_id.length > 0 &&
      Array.isArray(capture?.source_revisions) &&
      capture.source_revisions.length > 0 &&
      typeof capture?.compilers === "string" &&
      capture.compilers.length > 0,
    terrain_field_present: number("source_valid_vertices") > 0,
    render_pass_set_bound:
      typeof renderer.render_pass_set_id === "string" &&
      renderer.render_pass_set_id !== "unbound",
    landscape_mosaic_bound:
      requirements.require_landscape_mosaic !== true ||
      (typeof renderer.landscape_mosaic_id === "string" &&
        renderer.landscape_mosaic_id !== "unbound" &&
        typeof renderer.landscape_composition_revision === "string" &&
        renderer.landscape_composition_revision !== "unbound" &&
        typeof renderer.landscape_primary_patch_id === "string" &&
        renderer.landscape_primary_patch_id !== "unbound"),
    resident_cpu_budget_respected:
      number("resident_cpu_bytes") <= number("resident_cpu_budget_bytes"),
    resident_gpu_budget_respected:
      number("resident_gpu_bytes") <= number("resident_gpu_budget_bytes"),
    screen_error_respected:
      requirements.maximum_screen_error_pixels === undefined ||
      number("terrain_maximum_screen_error_pixels") <=
        requirements.maximum_screen_error_pixels,
    tile_seams_respected:
      requirements.maximum_tile_seam_mismatches === undefined ||
      number("terrain_tile_seam_mismatches") <=
        requirements.maximum_tile_seam_mismatches,
    relief_partition_respected:
      requirements.maximum_relief_partition_mismatches === undefined ||
      number("terrain_relief_partition_mismatches") <=
        requirements.maximum_relief_partition_mismatches,
    no_data_leakage_respected:
      requirements.maximum_no_data_leak_triangles === undefined ||
      number("terrain_no_data_leak_triangles") <=
        requirements.maximum_no_data_leak_triangles,
    minimum_elevation_span:
      requirements.minimum_elevation_span === undefined ||
      number("source_elevation_span") >= requirements.minimum_elevation_span,
    maximum_elevation_span:
      requirements.maximum_elevation_span === undefined ||
      number("source_elevation_span") <= requirements.maximum_elevation_span,
    minimum_no_data_vertices:
      requirements.minimum_source_no_data_vertices === undefined ||
      number("source_no_data_vertices") >=
        requirements.minimum_source_no_data_vertices,
    minimum_render_pass_lines:
      requirements.minimum_render_pass_lines === undefined ||
      number("render_pass_line_count") >=
        requirements.minimum_render_pass_lines,
    minimum_render_pass_areas:
      requirements.minimum_render_pass_areas === undefined ||
      number("render_pass_area_count") >=
        requirements.minimum_render_pass_areas,
    minimum_label_candidates:
      requirements.minimum_label_candidates === undefined ||
      Number(capture?.labels?.total) >= requirements.minimum_label_candidates,
    required_render_passes:
      requirements.required_render_passes?.every((pass) =>
        renderPasses.has(pass),
      ) ?? true,
    required_render_pass_kinds:
      requirements.required_render_pass_kinds?.every((kind) =>
        renderPassKinds.has(kind),
      ) ?? true,
    required_scene_omissions:
      requirements.required_scene_omission_terms?.every((term) =>
        omissions.some((omission) =>
          omission.includes(String(term).toLowerCase()),
        ),
      ) ?? true,
    backend_loss:
      requirements.require_backend_loss !== true ||
      (request.loss !== "none" && lossFallbackObserved === true),
  };
  return {
    workload_id: workload.id,
    purpose: workload.purpose,
    passed: Object.values(checks).every(Boolean),
    checks,
    observed: {
      backend: renderer.backend ?? null,
      elevation_span: number("source_elevation_span"),
      label_candidates: Number(capture?.labels?.total),
      landscape_mosaic_id: renderer.landscape_mosaic_id ?? null,
      landscape_composition_revision:
        renderer.landscape_composition_revision ?? null,
      landscape_primary_patch_id: renderer.landscape_primary_patch_id ?? null,
      no_data_leak_triangles: number("terrain_no_data_leak_triangles"),
      no_data_vertices: number("source_no_data_vertices"),
      render_pass_kinds: [...renderPassKinds],
      render_pass_areas: number("render_pass_area_count"),
      render_pass_lines: number("render_pass_line_count"),
      screen_error_pixels: number("terrain_maximum_screen_error_pixels"),
      relief_partition_mismatches: number(
        "terrain_relief_partition_mismatches",
      ),
      tile_seam_mismatches: number("terrain_tile_seam_mismatches"),
      valid_vertices: number("source_valid_vertices"),
    },
  };
}
