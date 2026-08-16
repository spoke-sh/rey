import {
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DoubleSide,
  LineSegments,
  LineBasicNodeMaterial,
  Mesh,
  MeshBasicNodeMaterial,
  MeshStandardNodeMaterial,
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
  const material = useMemo(
    () => createContinuousReliefMaterial(compiled.render_passes),
    [materialRevision],
  );
  useEffect(() => () => material.dispose(), [material]);
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
      <ambientLight color={0xdde4da} intensity={0.78} />
      <directionalLight
        color={0xfff4d4}
        intensity={1.35}
        position={[-world.width * 0.42, world.width, -world.height * 0.36]}
      />
      <directionalLight
        color={0xbad3df}
        intensity={0.34}
        position={[world.width * 0.5, world.width * 0.7, world.height * 0.48]}
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
        {compiled.meshes.map((mesh) => (
          <TerrainMesh
            data={mesh.data}
            key={mesh.field_set_id}
            material={material}
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
      depthWrite: false,
      opacity: area.opacity,
      polygonOffset: true,
      polygonOffsetFactor: -1,
      polygonOffsetUnits: -1,
      side: DoubleSide,
      transparent: area.opacity < 1,
    });
    const mesh = new Mesh(geometry, material);
    mesh.name = `terrain-pass:${area.pass_id}:${area.id}`;
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
        opacity: line.opacity,
        transparent: line.opacity < 1,
      }),
    [line.color, line.opacity],
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
  material: MeshStandardNodeMaterial;
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
        <bufferAttribute args={[data.indices, 1]} attach="index" />
      </bufferGeometry>
    </mesh>
  );
}
