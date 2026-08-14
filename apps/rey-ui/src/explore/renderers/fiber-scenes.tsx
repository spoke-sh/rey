import { extend, useThree } from "@react-three/fiber";
import {
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DirectionalLight,
  Group,
  InstancedMesh,
  Matrix4,
  Mesh,
  MeshBasicNodeMaterial,
  MeshStandardNodeMaterial,
  OrthographicCamera,
  Quaternion,
  RingGeometry,
  SphereGeometry,
  Vector3,
} from "three/src/Three.WebGPU.js";
import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import type { GlobeCameraView } from "../engine/camera";
import type { TerrainCameraView } from "../terrain/compile";
import {
  GLOBE_RADIUS,
  GLOBE_SAMPLE_RADIUS,
  type CompiledContextGlobe,
} from "./three-globe";
import {
  createContinuousReliefMaterial,
  terrainCameraProjection,
  type CompiledContinuousRelief,
  type TerrainMeshData,
} from "./three-terrain";

extend({
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DirectionalLight,
  Group,
  InstancedMesh,
  Mesh,
  OrthographicCamera,
  RingGeometry,
  SphereGeometry,
});

const SURFACE_NORMAL = new Vector3(0, 0, 1);
const X_AXIS = new Vector3(1, 0, 0);
const Y_AXIS = new Vector3(0, 1, 0);

export function ContinuousReliefScene({
  compiled,
  view,
  world,
}: {
  compiled: CompiledContinuousRelief;
  view: TerrainCameraView;
  world: { width: number; height: number };
}) {
  const material = useMemo(createContinuousReliefMaterial, [
    compiled.material_revision,
  ]);
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
        top={camera.top}
      />
      <ambientLight color={0xdde4da} intensity={1.32} />
      <directionalLight
        color={0xfff4d4}
        intensity={2.25}
        position={[-world.width * 0.42, world.width, -world.height * 0.36]}
      />
      <directionalLight
        color={0xbad3df}
        intensity={0.72}
        position={[world.width * 0.5, world.width * 0.7, world.height * 0.48]}
      />
      <group name="rey-continuous-relief">
        {compiled.meshes.map((mesh) => (
          <TerrainMesh
            data={mesh.data}
            key={mesh.field_set_id}
            material={material}
            name={mesh.field_set_id}
          />
        ))}
      </group>
    </>
  );
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

export function ContextGlobeScene({
  compiled,
  view,
  world,
}: {
  compiled: CompiledContextGlobe;
  view: GlobeCameraView;
  world: { width: number; height: number };
}) {
  const quaternion = useMemo(
    () => globeQuaternion(view),
    [view.pitch_degrees, view.yaw_degrees],
  );
  const aspect = world.width / Math.max(1, world.height);
  const halfHeight = 2.12;
  return (
    <>
      <ReyOrthographicCamera
        bottom={-halfHeight}
        far={100}
        left={-halfHeight * aspect}
        position={[0, 0, 6]}
        right={halfHeight * aspect}
        rotation={[0, 0, 0]}
        top={halfHeight}
      />
      <group
        name={`context-globe:${compiled.globe.source_revision}`}
        quaternion={quaternion}
      >
        <GlobeSurface />
        <GlobeAtmosphere />
        {compiled.sample_buckets.map((bucket) => (
          <GlobeSampleField bucket={bucket} key={bucket.id} />
        ))}
        {compiled.globe.regions.map((region) => (
          <GlobeSurfaceMarker
            color={
              region.tone === "frontier"
                ? 0xd6a94d
                : region.tone === "omitted"
                  ? 0xa87862
                  : 0x446c61
            }
            key={region.id}
            latitude={region.latitude_degrees}
            longitude={region.longitude_degrees}
            name={`semantic-region:${region.id}`}
            radius={
              0.026 + Math.min(0.056, region.angular_radius_degrees / 2_200)
            }
          />
        ))}
        {compiled.globe.beacons.map((beacon) => (
          <GlobeSurfaceMarker
            color={
              beacon.state === "admitted"
                ? 0x3b7458
                : beacon.state === "index"
                  ? 0xb28a25
                  : beacon.state === "request"
                    ? 0x658593
                    : 0xd57824
            }
            halo
            key={beacon.id}
            latitude={beacon.latitude_degrees}
            longitude={beacon.longitude_degrees}
            name={`workload-beacon:${beacon.workload_id}`}
            radius={beacon.mapping_role === "survey" ? 0.065 : 0.048}
          />
        ))}
      </group>
      <ambientLight color={0xf4f0df} intensity={1.72} />
      <directionalLight
        color={0xfff4d2}
        intensity={3.4}
        position={[-3.8, 4.8, 6.2]}
      />
      <directionalLight
        color={0x8fb6ac}
        intensity={1.55}
        position={[4.8, 1.4, -3.8]}
      />
    </>
  );
}

function GlobeSurface() {
  const material = useMemo(() => {
    const next = new MeshStandardNodeMaterial();
    next.name = "rey.semantic-globe.tsl-stippled-atmosphere@2";
    next.color.set(0xe8e9df);
    next.roughness = 0.98;
    next.metalness = 0;
    return next;
  }, []);
  useEffect(() => () => material.dispose(), [material]);
  return (
    <mesh material={material} name="context-globe-surface">
      <sphereGeometry args={[GLOBE_RADIUS, 160, 96]} />
    </mesh>
  );
}

function GlobeAtmosphere() {
  const layers = [
    { radius: GLOBE_RADIUS * 1.018, color: 0xf6ecd4, opacity: 0.12 },
    { radius: GLOBE_RADIUS * 1.045, color: 0xcbd8c9, opacity: 0.055 },
    { radius: GLOBE_RADIUS * 1.082, color: 0x6f9188, opacity: 0.022 },
  ];
  return layers.map((layer, index) => (
    <NodeMaterialSphere
      color={layer.color}
      key={index}
      name={`context-globe-atmosphere:${index}`}
      opacity={layer.opacity}
      radius={layer.radius}
    />
  ));
}

function NodeMaterialSphere({
  color,
  name,
  opacity,
  radius,
}: {
  color: number;
  name: string;
  opacity: number;
  radius: number;
}) {
  const material = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color,
        depthWrite: false,
        opacity,
        transparent: true,
      }),
    [color, opacity],
  );
  useEffect(() => () => material.dispose(), [material]);
  return (
    <mesh material={material} name={name}>
      <sphereGeometry args={[radius, 112, 64]} />
    </mesh>
  );
}

function GlobeSampleField({
  bucket,
}: {
  bucket: CompiledContextGlobe["sample_buckets"][number];
}) {
  const meshRef = useRef<InstancedMesh>(null);
  const material = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color: bucket.color,
        opacity: bucket.opacity,
        transparent: bucket.opacity < 1,
      }),
    [bucket.color, bucket.opacity],
  );
  useEffect(() => () => material.dispose(), [material]);
  useLayoutEffect(() => {
    const mesh = meshRef.current;
    if (!mesh) return;
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const scale = new Vector3(1, 1, 1);
    for (const [index, sample] of bucket.samples.entries()) {
      const position = sphericalVector(
        sample.longitude_degrees,
        sample.latitude_degrees,
        GLOBE_RADIUS * 1.005,
      );
      quaternion.setFromUnitVectors(
        SURFACE_NORMAL,
        position.clone().normalize(),
      );
      matrix.compose(position, quaternion, scale);
      mesh.setMatrixAt(index, matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
  }, [bucket]);
  return (
    <instancedMesh
      args={[undefined, undefined, bucket.samples.length]}
      material={material}
      name={bucket.id}
      ref={meshRef}
    >
      <circleGeometry args={[GLOBE_SAMPLE_RADIUS, 5]} />
    </instancedMesh>
  );
}

function GlobeSurfaceMarker({
  color,
  halo = false,
  latitude,
  longitude,
  name,
  radius,
}: {
  color: number;
  halo?: boolean;
  latitude: number;
  longitude: number;
  name: string;
  radius: number;
}) {
  const pointMaterial = useMemo(
    () => new MeshBasicNodeMaterial({ color }),
    [color],
  );
  const haloMaterial = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color,
        depthWrite: false,
        opacity: 0.42,
        transparent: true,
      }),
    [color],
  );
  useEffect(
    () => () => {
      pointMaterial.dispose();
      haloMaterial.dispose();
    },
    [haloMaterial, pointMaterial],
  );
  const position = sphericalVector(longitude, latitude, GLOBE_RADIUS * 1.013);
  const quaternion = new Quaternion().setFromUnitVectors(
    SURFACE_NORMAL,
    position.clone().normalize(),
  );
  return (
    <group name={name} position={position} quaternion={quaternion}>
      <mesh material={pointMaterial}>
        <circleGeometry args={[radius, 24]} />
      </mesh>
      {halo ? (
        <mesh material={haloMaterial} position={[0, 0, -0.001]}>
          <ringGeometry args={[radius * 1.5, radius * 2.25, 36]} />
        </mesh>
      ) : null}
    </group>
  );
}

function ReyOrthographicCamera({
  bottom,
  far,
  left,
  position,
  right,
  rotation,
  top,
}: {
  bottom: number;
  far: number;
  left: number;
  position: readonly [number, number, number];
  right: number;
  rotation: readonly [number, number, number];
  top: number;
}) {
  const cameraRef = useRef<OrthographicCamera>(null);
  const get = useThree((state) => state.get);
  const set = useThree((state) => state.set);
  useLayoutEffect(() => {
    const camera = cameraRef.current;
    if (!camera) return;
    const previous = get().camera;
    (camera as OrthographicCamera & { manual?: boolean }).manual = true;
    camera.updateProjectionMatrix();
    set({ camera });
    return () => set({ camera: previous });
  }, [bottom, far, get, left, position, right, rotation, set, top]);
  return (
    <orthographicCamera
      bottom={bottom}
      far={far}
      left={left}
      near={0.1}
      position={position}
      ref={cameraRef}
      right={right}
      rotation={rotation}
      top={top}
    />
  );
}

function globeQuaternion(view: GlobeCameraView) {
  const pitch = new Quaternion().setFromAxisAngle(
    X_AXIS,
    (view.pitch_degrees * Math.PI) / 180,
  );
  const yaw = new Quaternion().setFromAxisAngle(
    Y_AXIS,
    (view.yaw_degrees * Math.PI) / 180,
  );
  return yaw.multiply(pitch);
}

function sphericalVector(
  longitudeDegrees: number,
  latitudeDegrees: number,
  radius: number,
) {
  const longitude = (longitudeDegrees * Math.PI) / 180;
  const latitude = (latitudeDegrees * Math.PI) / 180;
  const latitudeRadius = Math.cos(latitude) * radius;
  return new Vector3(
    Math.sin(longitude) * latitudeRadius,
    Math.sin(latitude) * radius,
    Math.cos(longitude) * latitudeRadius,
  );
}
