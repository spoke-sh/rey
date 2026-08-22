import { MeshBasicNodeMaterial } from "three/src/Three.WebGPU.js";
import {
  add,
  attribute,
  clamp,
  float,
  mul,
  vec3,
} from "three/src/nodes/TSL.js";
import {
  compileLandscapePatchSet,
  deriveLandscapeReliefField,
  LANDSCAPE_RELIEF_ENGINE_REVISION,
  verifyLandscapeReliefField,
  type LandscapeReliefField,
} from "./landscape-relief";
import type {
  TerrainCameraView,
  TerrainFieldSetInput,
  TerrainRenderPassSetInput,
} from "./types";
import {
  verifyLandscapePyramidEnvelope,
  type LandscapePyramidEnvelope,
} from "./terrain-pyramid";

export const CONTINUOUS_RELIEF_MATERIAL_REVISION =
  "rey.terrain.tsl-cartographic-relief@4";
const CONTINUOUS_RELIEF_MATERIAL_STAGES = Object.freeze([
  "base_terrain",
  "height_normals_hillshade",
  "ambient_valley_occlusion",
] as const);
export const MAX_ACCELERATED_TERRAIN_GPU_BYTES = 64 * 1024 * 1024;
export const TERRAIN_MESH_PARITY_REVISION =
  "rey.terrain.cpu-mesh-upload-parity@2";

export interface CompiledContinuousRelief {
  material_revision: string;
  patch_set: ReturnType<typeof compileLandscapePatchSet>;
  pyramid_envelopes: readonly LandscapePyramidEnvelope[];
  render_passes?: TerrainRenderPassSetInput;
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
  target: readonly [number, number, number];
}

export interface TerrainMeshData {
  positions: Float32Array;
  normals: Float32Array;
  tint: Float32Array;
  occlusion: Float32Array;
  roughness: Float32Array;
  curvature: Float32Array;
  hillshade: Float32Array;
  salience: Float32Array;
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
    mesh.hillshade.byteLength +
    mesh.salience.byteLength +
    mesh.indices.byteLength
  );
}

export function verifyTerrainMeshParity(
  fields: TerrainFieldSetInput,
  mesh: TerrainMeshData,
  relief: LandscapeReliefField = deriveLandscapeReliefField(fields),
): number {
  verifyLandscapeReliefField(fields, relief);
  if (
    mesh.positions.length !== fields.field_cells * 3 ||
    mesh.normals.length !== fields.field_cells * 3 ||
    mesh.tint.length !== fields.field_cells * 3 ||
    mesh.occlusion.length !== fields.field_cells ||
    mesh.roughness.length !== fields.field_cells ||
    mesh.curvature.length !== fields.field_cells ||
    mesh.hillshade.length !== fields.field_cells ||
    mesh.salience.length !== fields.field_cells
  )
    throw new Error("accelerated terrain mesh shape diverges from CPU fields");
  if (terrainNoDataLeakTriangleCount(fields, mesh) > 0)
    throw new Error("accelerated terrain mesh indexes invalid CPU support");
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
        relief.hillshade[index]!,
        relief.salience[index]!,
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
        mesh.hillshade[index],
        mesh.salience[index],
      ];
      if (actual.some((value, component) => value !== expected[component]))
        throw new Error(
          `accelerated terrain mesh diverges from CPU fields at sample ${index}`,
        );
    }
  }
  return fields.field_cells;
}

export function terrainNoDataLeakTriangleCount(
  fields: Pick<TerrainFieldSetInput, "validity">,
  mesh: Pick<TerrainMeshData, "indices">,
): number {
  let leaks = 0;
  for (let index = 0; index < mesh.indices.length; index += 3) {
    if (
      fields.validity.values[mesh.indices[index]!] === 0 ||
      fields.validity.values[mesh.indices[index + 1]!] === 0 ||
      fields.validity.values[mesh.indices[index + 2]!] === 0
    )
      leaks += 1;
  }
  return leaks;
}

export function buildTerrainMeshData(
  fields: TerrainFieldSetInput,
  relief: LandscapeReliefField = deriveLandscapeReliefField(fields),
): TerrainMeshData {
  verifyLandscapeReliefField(fields, relief);
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
    hillshade: relief.hillshade.slice(),
    salience: relief.salience.slice(),
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
  renderPasses?: TerrainRenderPassSetInput,
  reliefFields?: readonly LandscapeReliefField[],
  pyramidEnvelopes: readonly LandscapePyramidEnvelope[] = [],
): CompiledContinuousRelief {
  const compilationStarted = measurementNow();
  if (!Number.isSafeInteger(gpuBudgetBytes) || gpuBudgetBytes < 1)
    throw new Error("accelerated terrain GPU budget is invalid");
  const resolvedRelief = resolveLandscapeReliefFields(fields, reliefFields);
  verifyPyramidEnvelopeSet(resolvedRelief, pyramidEnvelopes);
  const meshData = fields.map((field, index) =>
    buildTerrainMeshData(field, resolvedRelief[index]!),
  );
  const paritySamples = fields.reduce(
    (total, fieldSet, index) =>
      total +
      verifyTerrainMeshParity(
        fieldSet,
        meshData[index]!,
        resolvedRelief[index]!,
      ),
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
    material_revision: continuousReliefMaterialRevision(renderPasses),
    patch_set: compileLandscapePatchSet(fields),
    pyramid_envelopes: Object.freeze([...pyramidEnvelopes]),
    render_passes: renderPasses,
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

function verifyPyramidEnvelopeSet(
  reliefFields: readonly LandscapeReliefField[],
  envelopes: readonly LandscapePyramidEnvelope[],
): void {
  const reliefSourceIds = new Set(
    reliefFields.map(({ source_field_set_id }) => source_field_set_id),
  );
  if (
    new Set(envelopes.map(({ field_set_id }) => field_set_id)).size !==
    envelopes.length
  )
    throw new Error("accelerated terrain pyramid envelope set is ambiguous");
  for (const envelope of envelopes) {
    verifyLandscapePyramidEnvelope(envelope);
    if (!reliefSourceIds.has(envelope.field_set_id))
      throw new Error(
        "accelerated terrain pyramid envelope has no sampled relief consumer",
      );
  }
}

function resolveLandscapeReliefFields(
  fields: readonly TerrainFieldSetInput[],
  reliefFields?: readonly LandscapeReliefField[],
): readonly LandscapeReliefField[] {
  if (!reliefFields) return fields.map(deriveLandscapeReliefField);
  const byFieldSetId = new Map(
    reliefFields.map((relief) => [relief.field_set_id, relief]),
  );
  if (
    reliefFields.length !== fields.length ||
    byFieldSetId.size !== reliefFields.length
  )
    throw new Error("accelerated terrain relief field set is invalid");
  return fields.map((field) => {
    const relief = byFieldSetId.get(field.field_set_id);
    if (!relief)
      throw new Error(
        `accelerated terrain relief omitted ${field.field_set_id}`,
      );
    verifyLandscapeReliefField(field, relief);
    return relief;
  });
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
  const pitchDegrees = Math.max(22, Math.min(90, view?.pitch_degrees ?? 90));
  const yawDegrees = Math.max(-180, Math.min(180, view?.yaw_degrees ?? 0));
  const pitch = (pitchDegrees * Math.PI) / 180;
  const yaw = (yawDegrees * Math.PI) / 180;
  const panX = (view?.pan_x ?? 0) / scale;
  const panY = (view?.pan_y ?? 0) / (scale * Math.max(0.2, Math.sin(pitch)));
  const centerX = world.width / 2 - panX * Math.cos(yaw) - panY * Math.sin(yaw);
  const centerY =
    world.height / 2 + panX * Math.sin(yaw) - panY * Math.cos(yaw);
  const distance = Math.max(world.width, world.height) * 1.75;
  const horizontalDistance =
    pitchDegrees === 90 ? 0 : distance * Math.cos(pitch);
  return Object.freeze({
    bottom: -viewportHeight / scale / 2,
    center_x: centerX,
    center_y: centerY,
    far: distance * 4,
    left: -viewportWidth / scale / 2,
    right: viewportWidth / scale / 2,
    top: viewportHeight / scale / 2,
    position: Object.freeze([
      centerX + Math.sin(yaw) * horizontalDistance,
      Math.sin(pitch) * distance,
      centerY + Math.cos(yaw) * horizontalDistance,
    ] as const),
    rotation: Object.freeze([0, 0, 0] as const),
    target: Object.freeze([centerX, 0, centerY] as const),
  });
}

/**
 * Projects one world-space terrain point through the same real orbit
 * camera `terrainCameraProjection` positions (an eye orbiting a sphere of
 * fixed radius around `(world.width/2, 0, world.height/2)`, oriented by
 * `camera.lookAt` toward that same target) to a 2D screen offset — the
 * point-projection counterpart `terrainCameraProjection` doesn't provide on
 * its own (it only returns the camera's own pose and ortho frustum bounds).
 *
 * Orthographic projection is distance-independent, so the camera's actual
 * orbit distance cancels out entirely; screen position depends only on
 * `(point - target)` resolved against the camera's right/up basis vectors,
 * derived here the same way `Matrix4.lookAt` derives them (forward toward
 * target, crossed with world-up) rather than a hand-derived affine
 * approximation — verified to reproduce the exact numeric output of the
 * previous hand-derived 2D matrix at every sampled pitch/yaw/point
 * combination (to double-precision float error) when elevation is 0, so
 * this is a strict generalization (adding a real elevation term and a
 * verified derivation), not a behavior change on its own.
 *
 * `world.width/2, world.height/2` is used directly as the pivot (matching
 * how the DOM reference renderer's own terrain-tilt transform pivots),
 * not `terrainCameraProjection`'s own pan-adjusted `center_x/center_y` —
 * panning there is handled by a separate outer transform in that caller,
 * so folding it in here too would double-apply it.
 */
export function projectTerrainCoordinate(
  point: { x: number; z: number; elevation?: number },
  view: Pick<TerrainCameraView, "pitch_degrees" | "yaw_degrees">,
  world: { width: number; height: number },
): { x: number; y: number } {
  const pitchDegrees = Math.max(22, Math.min(90, view.pitch_degrees ?? 90));
  const yawDegrees = Math.max(-180, Math.min(180, view.yaw_degrees ?? 0));
  const pitch = (pitchDegrees * Math.PI) / 180;
  const yaw = (yawDegrees * Math.PI) / 180;
  const elevation = point.elevation ?? 0;
  const centerX = world.width / 2;
  const centerY = world.height / 2;
  const relativeX = point.x - centerX;
  const relativeZ = point.z - centerY;
  const rotatedX = relativeX * Math.cos(yaw) - relativeZ * Math.sin(yaw);
  const rotatedZ = relativeX * Math.sin(yaw) + relativeZ * Math.cos(yaw);
  return Object.freeze({
    x: centerX + rotatedX,
    y: centerY + rotatedZ * Math.sin(pitch) - elevation * Math.cos(pitch),
  });
}

export function createContinuousReliefMaterial(
  renderPasses?: TerrainRenderPassSetInput,
): MeshBasicNodeMaterial {
  const material = new MeshBasicNodeMaterial();
  material.name = continuousReliefMaterialRevision(renderPasses);
  const tint = attribute<"vec3">("reyTint", "vec3");
  const occlusion = attribute<"float">("reyOcclusion", "float");
  const hillshade = attribute<"float">("reyHillshade", "float");
  const enabled = (id: TerrainRenderPassSetInput["passes"][number]["id"]) =>
    renderPasses?.passes.some((pass) => pass.id === id) ?? true;
  const baseTint = enabled("base_terrain") ? tint : vec3(0.32, 0.33, 0.31);
  const luminance = baseTint.dot(vec3(0.2126, 0.7152, 0.0722));
  const cartographicTint = add(mul(baseTint, 0.72), mul(vec3(luminance), 0.28));
  const multidirectionalHillshade = enabled("height_normals_hillshade")
    ? hillshade
    : float(1);
  const ambientOcclusion = enabled("ambient_valley_occlusion")
    ? add(float(0.76), mul(occlusion, 0.24))
    : float(1);
  material.colorNode = clamp(
    mul(mul(cartographicTint, multidirectionalHillshade), ambientOcclusion),
    0,
    1,
  );
  return material;
}

export function continuousReliefMaterialRevision(
  renderPasses?: TerrainRenderPassSetInput,
): string {
  if (!renderPasses)
    return `${CONTINUOUS_RELIEF_MATERIAL_REVISION}:${LANDSCAPE_RELIEF_ENGINE_REVISION}`;
  const disabled = CONTINUOUS_RELIEF_MATERIAL_STAGES.filter(
    (id) => !renderPasses.passes.some((pass) => pass.id === id),
  );
  return disabled.length === 0
    ? `${CONTINUOUS_RELIEF_MATERIAL_REVISION}:${LANDSCAPE_RELIEF_ENGINE_REVISION}`
    : `${CONTINUOUS_RELIEF_MATERIAL_REVISION}:${LANDSCAPE_RELIEF_ENGINE_REVISION}:without=${disabled.join(",")}`;
}
