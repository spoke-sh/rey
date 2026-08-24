import {
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DoubleSide,
  LineSegments,
  LineBasicNodeMaterial,
  Mesh,
  MeshBasicNodeMaterial,
} from "three/src/Three.WebGPU.js";
import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import type { TerrainCameraView } from "../types";
import {
  createContinuousReliefMaterial,
  continuousReliefMaterialRevision,
  terrainCameraProjection,
  type CompiledContinuousRelief,
  type TerrainMeshData,
} from "../three-terrain";
import { ReyOrthographicCamera } from "./orthographic-camera";

export const TERRAIN_MATERIAL_BINDING_REVISION =
  "rey.terrain.material-binding@2" as const;

export function ContinuousReliefScene({
  compiled,
  view,
  world,
}: {
  compiled: CompiledContinuousRelief;
  view: TerrainCameraView;
  world: { width: number; height: number };
}) {
  const materialRevision = continuousReliefMaterialRevision(
    compiled.render_passes,
  );
  const materialBindingKeys = terrainMaterialBindingKeys(compiled);
  const materialBindings = useMemo(() => {
    const bindings = new Map<string, MeshBasicNodeMaterial>();
    for (const [index, key] of materialBindingKeys.entries()) {
      if (bindings.has(key)) continue;
      const overlapping = key.startsWith("overlap:");
      const material = createContinuousReliefMaterial(compiled.render_passes);
      if (overlapping) {
        material.polygonOffset = true;
        material.polygonOffsetFactor = -index;
        material.polygonOffsetUnits = -index;
      }
      bindings.set(key, material);
    }
    return bindings;
  }, [compiled.patch_set.patch_set_id, materialRevision]);
  useEffect(
    () => () => materialBindings.forEach((material) => material.dispose()),
    [materialBindings],
  );
  const camera = terrainCameraProjection(world, view);

  return (
    <>
      <ReyOrthographicCamera
        bottom={camera.bottom}
        far={camera.far}
        left={camera.left}
        position={camera.position}
        right={camera.right}
        rotation={camera.rotation}
        target={camera.target}
        top={camera.top}
      />
      <group
        name="rey-continuous-relief"
        position={[
          view.model_transform?.translate_x ?? 0,
          0,
          view.model_transform?.translate_z ?? 0,
        ]}
        scale={[
          view.model_transform?.scale_x ?? 1,
          view.model_transform?.elevation_scale ?? 1,
          view.model_transform?.scale_z ?? 1,
        ]}
      >
        {compiled.meshes.map((mesh, index) => (
          <TerrainMesh
            data={mesh.data}
            key={mesh.field_set_id}
            material={materialBindings.get(materialBindingKeys[index]!)!}
            name={mesh.field_set_id}
          />
        ))}
        {compiled.render_passes ? (
          <TerrainExecutablePasses passes={compiled.render_passes} />
        ) : null}
      </group>
    </>
  );
}

/**
 * Non-overlapping hierarchy tiles use one immutable TSL material graph. A
 * separately constructed node material per tile forces redundant shader
 * compilation during settled refinement. Legacy overlapping patch inputs keep
 * distinct depth-biased materials so their declared ordering is unchanged.
 */
export function terrainMaterialBindingKeys(
  compiled: CompiledContinuousRelief,
): readonly string[] {
  const overlapping = new Set(compiled.patch_set.overlap_pairs.flat());
  return Object.freeze(
    compiled.meshes.map(({ field_set_id }, index) =>
      overlapping.has(field_set_id)
        ? `overlap:${index}:${field_set_id}`
        : `${TERRAIN_MATERIAL_BINDING_REVISION}:shared-non-overlapping-terrain`,
    ),
  );
}

function TerrainExecutablePasses({
  passes,
}: {
  passes: NonNullable<CompiledContinuousRelief["render_passes"]>;
}) {
  const validity = passes.passes.some(
    (pass) => pass.id === "validity_background",
  );
  return (
    <group name={`terrain-passes:${passes.pass_set_id}`}>
      {validity ? <group name="terrain-pass:validity_background" /> : null}
      {passes.areas.map((area) => (
        <TerrainPassArea area={area} key={area.id} />
      ))}
      {passes.lines.map((line) => (
        <TerrainPassLine key={line.id} line={line} />
      ))}
      {passes.points.map((point) => (
        <TerrainPassPoint key={point.id} point={point} />
      ))}
    </group>
  );
}

function TerrainPassArea({
  area,
}: {
  area: NonNullable<CompiledContinuousRelief["render_passes"]>["areas"][number];
}) {
  const object = useMemo(() => {
    const geometry = new BufferGeometry();
    geometry.setAttribute("position", new BufferAttribute(area.positions, 3));
    geometry.computeVertexNormals();
    const material = new MeshBasicNodeMaterial({
      color: area.color,
      depthWrite: true,
      opacity: area.opacity,
      polygonOffset: true,
      polygonOffsetFactor: -4,
      polygonOffsetUnits: -4,
      side: DoubleSide,
      transparent: area.opacity < 1,
    });
    const mesh = new Mesh(geometry, material);
    mesh.name = `terrain-pass:${area.pass_id}:${area.id}`;
    mesh.renderOrder = 10;
    return mesh;
  }, [area.color, area.id, area.opacity, area.pass_id, area.positions]);
  useEffect(
    () => () => {
      object.geometry.dispose();
      object.material.dispose();
    },
    [object],
  );
  return <primitive object={object} />;
}

function TerrainPassPoint({
  point,
}: {
  point: NonNullable<
    CompiledContinuousRelief["render_passes"]
  >["points"][number];
}) {
  const object = useMemo(() => {
    const geometry = new CircleGeometry(point.radius, 20);
    geometry.rotateX(-Math.PI / 2);
    const mesh = new Mesh(
      geometry,
      new MeshBasicNodeMaterial({ color: point.color }),
    );
    mesh.name = `terrain-pass:${point.pass_id}:${point.id}`;
    mesh.position.set(...point.position);
    return mesh;
  }, [point.color, point.id, point.pass_id, point.position, point.radius]);
  useEffect(
    () => () => {
      object.geometry.dispose();
      object.material.dispose();
    },
    [object],
  );
  return <primitive object={object} />;
}

function TerrainPassLine({
  line,
}: {
  line: NonNullable<CompiledContinuousRelief["render_passes"]>["lines"][number];
}) {
  const material = useMemo(
    () =>
      new LineBasicNodeMaterial({
        color: line.color,
        linewidth: line.width ?? 1,
        opacity: line.opacity,
        transparent: line.opacity < 1,
      }),
    [line.color, line.opacity, line.width],
  );
  const object = useMemo(() => {
    const geometry = new BufferGeometry();
    geometry.setAttribute("position", new BufferAttribute(line.positions, 3));
    const object = new LineSegments(geometry, material);
    object.name = `terrain-pass:${line.pass_id}:${line.id}`;
    return object;
  }, [line.id, line.pass_id, line.positions, material]);
  useEffect(
    () => () => {
      object.geometry.dispose();
      material.dispose();
    },
    [material, object],
  );
  return <primitive object={object} />;
}

function TerrainMesh({
  data,
  material,
  name,
}: {
  data: TerrainMeshData;
  material: MeshBasicNodeMaterial;
  name: string;
}) {
  const geometryRef = useRef<BufferGeometry>(null);
  useLayoutEffect(() => geometryRef.current?.computeBoundingSphere(), [data]);
  return (
    <mesh material={material} name={name}>
      <bufferGeometry ref={geometryRef}>
        <bufferAttribute
          args={[data.positions, 3]}
          attach="attributes-position"
        />
        <bufferAttribute args={[data.normals, 3]} attach="attributes-normal" />
        <bufferAttribute args={[data.tint, 3]} attach="attributes-reyTint" />
        <bufferAttribute
          args={[data.cartographic_color, 3]}
          attach="attributes-reyCartographicColor"
        />
        <bufferAttribute
          args={[data.occlusion, 1]}
          attach="attributes-reyOcclusion"
        />
        <bufferAttribute
          args={[data.roughness, 1]}
          attach="attributes-reyRoughness"
        />
        <bufferAttribute
          args={[data.curvature, 1]}
          attach="attributes-reyCurvature"
        />
        <bufferAttribute
          args={[data.hillshade, 1]}
          attach="attributes-reyHillshade"
        />
        <bufferAttribute
          args={[data.salience, 1]}
          attach="attributes-reySalience"
        />
        <bufferAttribute args={[data.indices, 1]} attach="index" />
      </bufferGeometry>
    </mesh>
  );
}
