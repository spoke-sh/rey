import type { ExplorerGlobe, TerrainFieldSetInput } from "./types";

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
