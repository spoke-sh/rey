import {
  AmbientLight,
  BufferGeometry,
  DirectionalLight,
  Float32BufferAttribute,
  Group,
  Mesh,
  MeshStandardNodeMaterial,
  OrthographicCamera,
  Scene,
  Uint32BufferAttribute,
  type Camera,
  type Object3D,
} from "three/webgpu";
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
} from "three/tsl";
import { fieldPoint } from "../engine/fields";
import type { TerrainFieldSet } from "../terrain/compile";

export const CONTINUOUS_RELIEF_MATERIAL_REVISION =
  "rey.terrain.tsl-continuous-relief@1";

export interface ThreeTerrainBundle {
  scene: Object3D;
  camera: Camera;
  material_revision: string;
  statistics: {
    field_sets: number;
    vertices: number;
    triangles: number;
    field_bytes: number;
  };
  dispose(): void;
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

export function buildTerrainMeshData(fields: TerrainFieldSet): TerrainMeshData {
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
      if ((row + column) % 2 === 0) {
        appendTriangle(topLeft, bottomLeft, bottomRight);
        appendTriangle(topLeft, bottomRight, topRight);
      } else {
        appendTriangle(topLeft, bottomLeft, topRight);
        appendTriangle(topRight, bottomLeft, bottomRight);
      }
    }
  }
  return {
    positions,
    normals,
    tint: fields.material.tint,
    occlusion: fields.material.occlusion,
    roughness: fields.material.roughness,
    curvature: fields.curvature.values,
    indices: Uint32Array.from(indices),
  };
}

export function createContinuousReliefBundle(
  fields: readonly TerrainFieldSet[],
  world: { width: number; height: number },
): ThreeTerrainBundle {
  const scene = new Scene();
  const terrain = new Group();
  terrain.name = "rey-continuous-relief";
  scene.add(terrain);
  const material = createContinuousReliefMaterial();
  const geometries: BufferGeometry[] = [];
  let vertices = 0;
  let triangles = 0;
  let fieldBytes = 0;
  for (const fieldSet of fields) {
    const data = buildTerrainMeshData(fieldSet);
    const geometry = new BufferGeometry();
    geometry.setAttribute(
      "position",
      new Float32BufferAttribute(data.positions, 3),
    );
    geometry.setAttribute(
      "normal",
      new Float32BufferAttribute(data.normals, 3),
    );
    geometry.setAttribute("reyTint", new Float32BufferAttribute(data.tint, 3));
    geometry.setAttribute(
      "reyOcclusion",
      new Float32BufferAttribute(data.occlusion, 1),
    );
    geometry.setAttribute(
      "reyRoughness",
      new Float32BufferAttribute(data.roughness, 1),
    );
    geometry.setAttribute(
      "reyCurvature",
      new Float32BufferAttribute(data.curvature, 1),
    );
    geometry.setIndex(new Uint32BufferAttribute(data.indices, 1));
    geometry.computeBoundingSphere();
    geometries.push(geometry);
    const mesh = new Mesh(geometry, material);
    mesh.name = fieldSet.field_set_id;
    terrain.add(mesh);
    vertices += fieldSet.field_cells;
    triangles += data.indices.length / 3;
    fieldBytes += fieldSet.field_bytes;
  }

  scene.add(new AmbientLight(0xdde4da, 1.32));
  const keyLight = new DirectionalLight(0xfff4d4, 2.25);
  keyLight.position.set(-world.width * 0.42, world.width, -world.height * 0.36);
  scene.add(keyLight);
  const fillLight = new DirectionalLight(0xbad3df, 0.72);
  fillLight.position.set(
    world.width * 0.5,
    world.width * 0.7,
    world.height * 0.48,
  );
  scene.add(fillLight);

  const maximumDimension = Math.max(world.width, world.height);
  const camera = new OrthographicCamera(
    -world.width / 2,
    world.width / 2,
    world.height / 2,
    -world.height / 2,
    0.1,
    maximumDimension * 4,
  );
  camera.position.set(
    world.width / 2,
    maximumDimension * 1.75,
    world.height / 2,
  );
  camera.up.set(0, 0, -1);
  camera.lookAt(world.width / 2, 0, world.height / 2);
  camera.updateProjectionMatrix();

  return {
    scene,
    camera,
    material_revision: CONTINUOUS_RELIEF_MATERIAL_REVISION,
    statistics: Object.freeze({
      field_sets: fields.length,
      vertices,
      triangles,
      field_bytes: fieldBytes,
    }),
    dispose() {
      for (const geometry of geometries) geometry.dispose();
      material.dispose();
      scene.clear();
    },
  };
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
