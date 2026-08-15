import { MeshStandardNodeMaterial } from "three/src/Three.WebGPU.js";
import {
  add,
  attribute,
  clamp,
  float,
  max,
  mul,
  negate,
  normalWorld,
  sub,
  vec3,
} from "three/src/nodes/TSL.js";
import type { TerrainCameraView, TerrainFieldSetInput } from "./types";

export const CONTINUOUS_RELIEF_MATERIAL_REVISION =
  "rey.terrain.tsl-continuous-relief@1";
export const MAX_ACCELERATED_TERRAIN_GPU_BYTES = 64 * 1024 * 1024;
export const TERRAIN_MESH_PARITY_REVISION =
  "rey.terrain.cpu-mesh-upload-parity@1";

export interface CompiledContinuousRelief {
  material_revision: string;
  meshes: readonly {
    field_set_id: string;
    data: TerrainMeshData;
  }[];
  statistics: {
    field_sets: number;
    vertices: number;
    triangles: number;
    field_bytes: number;
    gpu_bytes: number;
    gpu_budget_bytes: number;
    parity_revision: string;
    parity_samples: number;
    geometry_compilation_ms: number;
  };
}

export interface TerrainCameraProjection {
  bottom: number;
  center_x: number;
  center_y: number;
  far: number;
  left: number;
  right: number;
  top: number;
  position: readonly [number, number, number];
  rotation: readonly [number, number, number];
}

export interface TerrainMeshData {
  positions: Float32Array;
  normals: Float32Array;
  tint: Float32Array;
  occlusion: Float32Array;
  roughness: Float32Array;
  curvature: Float32Array;
  indices: Uint32Array;
}

export function terrainMeshByteLength(mesh: TerrainMeshData): number {
  return (
    mesh.positions.byteLength +
    mesh.normals.byteLength +
    mesh.tint.byteLength +
    mesh.occlusion.byteLength +
    mesh.roughness.byteLength +
    mesh.curvature.byteLength +
    mesh.indices.byteLength
  );
}

export function verifyTerrainMeshParity(
  fields: TerrainFieldSetInput,
  mesh: TerrainMeshData,
): number {
  if (
    mesh.positions.length !== fields.field_cells * 3 ||
    mesh.normals.length !== fields.field_cells * 3 ||
    mesh.tint.length !== fields.field_cells * 3 ||
    mesh.occlusion.length !== fields.field_cells ||
    mesh.roughness.length !== fields.field_cells ||
    mesh.curvature.length !== fields.field_cells
  )
    throw new Error("accelerated terrain mesh shape diverges from CPU fields");
  for (let row = 0; row < fields.grid.rows; row += 1) {
    for (let column = 0; column < fields.grid.columns; column += 1) {
      const index = row * fields.grid.columns + column;
      const offset = index * 3;
      const point = fieldPoint(fields.grid, column, row);
      const expected = [
        point.x,
        fields.elevation.values[index]! * fields.elevation_scale,
        point.y,
        fields.normal.values[offset]!,
        fields.normal.values[offset + 2]!,
        fields.normal.values[offset + 1]!,
        fields.material.tint[offset]!,
        fields.material.tint[offset + 1]!,
        fields.material.tint[offset + 2]!,
        fields.material.occlusion[index]!,
        fields.material.roughness[index]!,
        fields.curvature.values[index]!,
      ].map(Math.fround);
      const actual = [
        mesh.positions[offset],
        mesh.positions[offset + 1],
        mesh.positions[offset + 2],
        mesh.normals[offset],
        mesh.normals[offset + 1],
        mesh.normals[offset + 2],
        mesh.tint[offset],
        mesh.tint[offset + 1],
        mesh.tint[offset + 2],
        mesh.occlusion[index],
        mesh.roughness[index],
        mesh.curvature[index],
      ];
      if (actual.some((value, component) => value !== expected[component]))
        throw new Error(
          `accelerated terrain mesh diverges from CPU fields at sample ${index}`,
        );
    }
  }
  for (const index of mesh.indices)
    if (fields.validity.values[index] === 0)
      throw new Error("accelerated terrain mesh indexes invalid CPU support");
  return fields.field_cells;
}

export function buildTerrainMeshData(
  fields: TerrainFieldSetInput,
): TerrainMeshData {
  const { grid } = fields;
  const positions = new Float32Array(fields.field_cells * 3);
  const normals = new Float32Array(fields.normal.values.length);
  for (let row = 0; row < grid.rows; row += 1) {
    for (let column = 0; column < grid.columns; column += 1) {
      const index = row * grid.columns + column;
      const offset = index * 3;
      const point = fieldPoint(grid, column, row);
      positions[offset] = point.x;
      positions[offset + 1] =
        fields.elevation.values[index]! * fields.elevation_scale;
      positions[offset + 2] = point.y;
      normals[offset] = fields.normal.values[offset]!;
      normals[offset + 1] = fields.normal.values[offset + 2]!;
      normals[offset + 2] = fields.normal.values[offset + 1]!;
    }
  }

  const indices = terrainTriangleIndices(fields);
  return {
    positions,
    normals,
    tint: fields.material.tint.slice(),
    occlusion: fields.material.occlusion.slice(),
    roughness: fields.material.roughness.slice(),
    curvature: fields.curvature.values.slice(),
    indices,
  };
}

export function terrainTriangleIndices(
  fields: Pick<TerrainFieldSetInput, "grid" | "validity">,
): Uint32Array {
  const { grid } = fields;
  const indices: number[] = [];
  const appendTriangle = (first: number, second: number, third: number) => {
    if (
      fields.validity.values[first] !== 0 &&
      fields.validity.values[second] !== 0 &&
      fields.validity.values[third] !== 0
    )
      indices.push(first, second, third);
  };
  for (let row = 0; row < grid.rows - 1; row += 1) {
    for (let column = 0; column < grid.columns - 1; column += 1) {
      const topLeft = row * grid.columns + column;
      const topRight = topLeft + 1;
      const bottomLeft = topLeft + grid.columns;
      const bottomRight = bottomLeft + 1;
      const descending = [
        [topLeft, bottomLeft, bottomRight],
        [topLeft, bottomRight, topRight],
      ] as const;
      const ascending = [
        [topLeft, bottomLeft, topRight],
        [topRight, bottomLeft, bottomRight],
      ] as const;
      const score = (triangles: typeof descending) =>
        triangles.filter((triangle) =>
          triangle.every((index) => fields.validity.values[index] !== 0),
        ).length;
      const descendingScore = score(descending);
      const ascendingScore = score(ascending);
      const triangles =
        descendingScore === ascendingScore
          ? (row + column) % 2 === 0
            ? descending
            : ascending
          : descendingScore > ascendingScore
            ? descending
            : ascending;
      for (const triangle of triangles)
        appendTriangle(triangle[0], triangle[1], triangle[2]);
    }
  }
  return Uint32Array.from(indices);
}

export function compileContinuousRelief(
  fields: readonly TerrainFieldSetInput[],
  gpuBudgetBytes = MAX_ACCELERATED_TERRAIN_GPU_BYTES,
): CompiledContinuousRelief {
  const compilationStarted = measurementNow();
  if (!Number.isSafeInteger(gpuBudgetBytes) || gpuBudgetBytes < 1)
    throw new Error("accelerated terrain GPU budget is invalid");
  const meshData = fields.map(buildTerrainMeshData);
  const paritySamples = fields.reduce(
    (total, fieldSet, index) =>
      total + verifyTerrainMeshParity(fieldSet, meshData[index]!),
    0,
  );
  const gpuBytes = meshData.reduce(
    (total, data) => total + terrainMeshByteLength(data),
    0,
  );
  if (gpuBytes > gpuBudgetBytes)
    throw new Error(
      `accelerated terrain upload ${gpuBytes} exceeds GPU budget ${gpuBudgetBytes}`,
    );
  let vertices = 0;
  let triangles = 0;
  let fieldBytes = 0;
  for (const [index, fieldSet] of fields.entries()) {
    const data = meshData[index]!;
    vertices += fieldSet.field_cells;
    triangles += data.indices.length / 3;
    fieldBytes += fieldSet.field_bytes;
  }

  return {
    material_revision: CONTINUOUS_RELIEF_MATERIAL_REVISION,
    meshes: Object.freeze(
      fields.map((fieldSet, index) =>
        Object.freeze({
          field_set_id: fieldSet.field_set_id,
          data: meshData[index]!,
        }),
      ),
    ),
    statistics: Object.freeze({
      field_sets: fields.length,
      vertices,
      triangles,
      field_bytes: fieldBytes,
      gpu_bytes: gpuBytes,
      gpu_budget_bytes: gpuBudgetBytes,
      parity_revision: TERRAIN_MESH_PARITY_REVISION,
      parity_samples: paritySamples,
      geometry_compilation_ms: measurementNow() - compilationStarted,
    }),
  };
}

function measurementNow(): number {
  return globalThis.performance?.now() ?? Date.now();
}

function fieldPoint(
  grid: TerrainFieldSetInput["grid"],
  column: number,
  row: number,
) {
  if (
    !Number.isInteger(column) ||
    !Number.isInteger(row) ||
    column < 0 ||
    column >= grid.columns ||
    row < 0 ||
    row >= grid.rows
  )
    throw new Error("field coordinate is outside the bounded grid");
  return {
    x:
      grid.bounds.x +
      (column / Math.max(1, grid.columns - 1)) * grid.bounds.width,
    y: grid.bounds.y + (row / Math.max(1, grid.rows - 1)) * grid.bounds.height,
  };
}

export function terrainCameraProjection(
  world: { width: number; height: number },
  view?: TerrainCameraView,
): TerrainCameraProjection {
  const scale = Math.max(0.000_001, view?.rendered_scale ?? 1);
  const viewportWidth = view?.viewport_width ?? world.width;
  const viewportHeight = view?.viewport_height ?? world.height;
  const centerX = world.width / 2 - (view?.pan_x ?? 0) / scale;
  const centerY = world.height / 2 - (view?.pan_y ?? 0) / scale;
  return Object.freeze({
    bottom: -viewportHeight / scale / 2,
    center_x: centerX,
    center_y: centerY,
    far: Math.max(world.width, world.height) * 4,
    left: -viewportWidth / scale / 2,
    right: viewportWidth / scale / 2,
    top: viewportHeight / scale / 2,
    position: Object.freeze([
      centerX,
      Math.max(world.width, world.height) * 1.75,
      centerY,
    ] as const),
    rotation: Object.freeze([-Math.PI / 2, 0, 0] as const),
  });
}

export function createContinuousReliefMaterial(): MeshStandardNodeMaterial {
  const material = new MeshStandardNodeMaterial();
  material.name = CONTINUOUS_RELIEF_MATERIAL_REVISION;
  material.metalness = 0;
  const tint = attribute<"vec3">("reyTint", "vec3");
  const occlusion = attribute<"float">("reyOcclusion", "float");
  const roughness = attribute<"float">("reyRoughness", "float");
  const curvature = attribute<"float">("reyCurvature", "float");
  const northwest = normalWorld
    .dot(vec3(-0.42, 0.84, -0.34).normalize())
    .max(0);
  const southeast = normalWorld.dot(vec3(0.5, 0.72, 0.48).normalize()).max(0);
  const multidirectionalHillshade = northwest
    .mul(0.62)
    .add(southeast.mul(0.2))
    .add(float(0.3))
    .clamp(0.32, 1.08);
  const ridge = mul(max(negate(curvature), 0), 0.18);
  const valley = mul(max(curvature, 0), 0.12);
  material.colorNode = clamp(
    sub(
      add(mul(mul(tint, multidirectionalHillshade), occlusion), ridge),
      valley,
    ),
    0,
    1,
  );
  material.roughnessNode = clamp(roughness, 0.62, 1);
  return material;
}
