import type {
  ExplorerGlobe,
  TerrainFieldSetInput,
  TerrainRenderPassSetInput,
} from "./types";

export function terrainFieldFixture(): TerrainFieldSetInput {
  const columns = 5;
  const rows = 4;
  const cells = columns * rows;
  const tint = new Float32Array(cells * 3);
  const normal = new Float32Array(cells * 3);
  const elevation = new Float32Array(cells);
  const curvature = new Float32Array(cells);
  const occlusion = new Float32Array(cells);
  const roughness = new Float32Array(cells);
  for (let index = 0; index < cells; index += 1) {
    const offset = index * 3;
    elevation[index] = Math.fround((index % columns) * 0.08);
    normal[offset] = 0;
    normal[offset + 1] = 1;
    normal[offset + 2] = 0;
    tint[offset] = 0.42;
    tint[offset + 1] = 0.5;
    tint[offset + 2] = 0.38;
    curvature[index] = 0;
    occlusion[index] = 0.92;
    roughness[index] = 0.84;
  }
  return {
    field_set_id: "terrain:fixture",
    field_cells: cells,
    field_bytes:
      elevation.byteLength +
      normal.byteLength +
      curvature.byteLength +
      tint.byteLength +
      occlusion.byteLength +
      roughness.byteLength +
      cells,
    elevation_scale: 18,
    grid: {
      columns,
      rows,
      bounds: { x: 100, y: 80, width: 1300, height: 840 },
    },
    validity: { values: new Uint8Array(cells).fill(1) },
    elevation: { values: elevation },
    normal: { values: normal },
    curvature: { values: curvature },
    material: { tint, occlusion, roughness },
  };
}

export function globeFixture(): ExplorerGlobe {
  return {
    schema: "rey.explore-orientation-globe.v1",
    posture: "orientation",
    globe_id: "orientation:fixture",
    source_revision: "working:fixture",
    compiler_revision: "orientation@1",
    coordinate_authority: "presentation only",
    clusters: [],
    regions: [],
    sectors: [
      {
        id: "sector:fixture",
        label: "SECTOR 4.3",
        west_degrees: -60,
        south_degrees: 10,
        east_degrees: -30,
        north_degrees: 30,
        crosses_antimeridian: false,
        tone: "neutral",
      },
    ],
    beacons: [
      {
        id: "workload-beacon:survey",
        focus_id: "beacon:survey",
        workload_id: "survey",
        label: "Survey context",
        detail: "WORKING",
        source: "sys/survey/workload.yaml",
        source_revision: "blake3:survey",
        producer: "codex@gpt-5",
        state: "working",
        mapping_role: "survey",
        next_step: "review and consent",
        longitude_degrees: 14,
        latitude_degrees: 6,
        tone: "attention",
      },
    ],
  };
}

export function terrainRenderPassFixture(): TerrainRenderPassSetInput {
  return {
    schema: "rey.terrain-render-pass-set.v1",
    pass_set_id: "terrain-passes:fixture",
    bounds: { x: 100, y: 80, width: 1300, height: 840 },
    passes: [
      {
        id: "validity_background",
        implementation_revision: "validity-pass:fixture",
        input_revision: "validity:fixture",
        authority: "evidence",
      },
      {
        id: "base_terrain",
        implementation_revision: "base-pass:fixture",
        input_revision: "material:fixture",
        authority: "derived",
      },
      {
        id: "height_normals_hillshade",
        implementation_revision: "hillshade-pass:fixture",
        input_revision: "normal:fixture",
        authority: "derived",
      },
      {
        id: "ambient_valley_occlusion",
        implementation_revision: "occlusion-pass:fixture",
        input_revision: "curvature:fixture",
        authority: "presentation",
      },
      {
        id: "contours",
        implementation_revision: "contour-pass:fixture",
        input_revision: "contour:fixture",
        authority: "derived",
      },
      {
        id: "features_labels_selection",
        implementation_revision: "feature-pass:fixture",
        input_revision: "feature:fixture",
        authority: "interface",
      },
    ],
    lines: [
      {
        id: "contour:fixture",
        pass_id: "contours",
        kind: "contour",
        source_revision: "contour-source:fixture",
        authority: "derived contour over admitted support",
        positions: Float32Array.from([200, 4, 240, 420, 8, 360]),
        color: 0xd8c99a,
        opacity: 0.46,
      },
    ],
    points: [
      {
        id: "selection:fixture",
        pass_id: "features_labels_selection",
        kind: "selection",
        source_revision: "selection-source:fixture",
        authority: "interface selection of retained identity",
        position: [360, 9, 300],
        color: 0xffd36e,
        radius: 6,
      },
    ],
    omissions: [],
  };
}
