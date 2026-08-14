import { extend, useThree } from "@react-three/fiber";
import {
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DoubleSide,
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
import type { GlobeCameraView, TerrainCameraView } from "./types";
import {
  buildProjectedBoundsMeshes,
  buildProjectedGlobeMesh,
  GLOBE_CAMERA_HALF_HEIGHT,
  projectGlobeCoordinate,
  type ProjectedGlobeMesh,
} from "./globe-projection";
import {
  GLOBE_RADIUS,
  GLOBE_SAMPLE_RADIUS,
  SEMANTIC_GLOBE_MATERIAL_REVISION,
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
  const projectionMorphProgress = Math.max(
    0,
    Math.min(1, view.projection_morph_progress ?? 0),
  );
  const aspect = world.width / Math.max(1, world.height);
  const halfHeight = GLOBE_CAMERA_HALF_HEIGHT;
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
      <group name={`context-globe:${compiled.globe.source_revision}`}>
        <GlobeSurface
          progress={projectionMorphProgress}
          view={view}
          world={world}
        />
        <GlobeAtmosphere progress={projectionMorphProgress} />
        {(compiled.globe.sectors ?? []).map((sector) => (
          <GlobeSector
            key={sector.id}
            progress={projectionMorphProgress}
            sector={sector}
            view={view}
            world={world}
          />
        ))}
        {compiled.sample_buckets.map((bucket) => (
          <GlobeSampleField
            bucket={bucket}
            key={bucket.id}
            progress={projectionMorphProgress}
            view={view}
            world={world}
          />
        ))}
        {compiled.pole_patterns.map((pattern) => (
          <GlobePolePatternField
            key={pattern.id}
            pattern={pattern}
            progress={projectionMorphProgress}
            view={view}
            world={world}
          />
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
            progress={projectionMorphProgress}
            radius={
              0.026 + Math.min(0.056, region.angular_radius_degrees / 2_200)
            }
            view={view}
            world={world}
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
            progress={projectionMorphProgress}
            radius={beacon.mapping_role === "survey" ? 0.065 : 0.048}
            view={view}
            world={world}
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

function GlobeSurface({
  progress,
  view,
  world,
}: {
  progress: number;
  view: GlobeCameraView;
  world: { width: number; height: number };
}) {
  const material = useMemo(() => {
    const next = new MeshStandardNodeMaterial();
    next.name = SEMANTIC_GLOBE_MATERIAL_REVISION;
    next.color.set(0xe8e9df);
    next.roughness = 0.98;
    next.metalness = 0;
    return next;
  }, []);
  useEffect(() => () => material.dispose(), [material]);
  const mesh = useMemo(
    () => buildProjectedGlobeMesh(view, world, progress),
    [progress, view.pitch_degrees, view.yaw_degrees, world.height, world.width],
  );
  return (
    <mesh material={material} name="context-globe-surface">
      <ProjectedMeshGeometry data={mesh} />
    </mesh>
  );
}

function GlobeAtmosphere({ progress }: { progress: number }) {
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
      opacity={layer.opacity * (1 - progress)}
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
  progress,
  view,
  world,
}: {
  bucket: CompiledContextGlobe["sample_buckets"][number];
  progress: number;
  view: GlobeCameraView;
  world: { width: number; height: number };
}) {
  const meshRef = useRef<InstancedMesh>(null);
  const opacity = bucket.opacity * (1 - progress * 0.82);
  const material = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color: bucket.color,
        opacity,
        transparent: opacity < 1,
      }),
    [bucket.color, opacity],
  );
  useEffect(() => () => material.dispose(), [material]);
  useLayoutEffect(() => {
    const mesh = meshRef.current;
    if (!mesh) return;
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const scale = new Vector3(1, 1, 1);
    for (const [index, sample] of bucket.samples.entries()) {
      const projected = projectGlobeCoordinate(
        sample.longitude_degrees,
        sample.latitude_degrees,
        view,
        world,
        progress,
        GLOBE_RADIUS * 1.005,
        0.008,
      );
      const position = new Vector3(...projected.position);
      quaternion.setFromUnitVectors(
        SURFACE_NORMAL,
        new Vector3(...projected.normal),
      );
      matrix.compose(position, quaternion, scale);
      mesh.setMatrixAt(index, matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
  }, [bucket, progress, view, world]);
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

function GlobePolePatternField({
  pattern,
  progress,
  view,
  world,
}: {
  pattern: CompiledContextGlobe["pole_patterns"][number];
  progress: number;
  view: GlobeCameraView;
  world: { width: number; height: number };
}) {
  const meshRef = useRef<InstancedMesh>(null);
  const material = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color: 0x243b38,
        opacity: 0.88 * (1 - progress),
        transparent: true,
      }),
    [progress],
  );
  useEffect(() => () => material.dispose(), [material]);
  useLayoutEffect(() => {
    const mesh = meshRef.current;
    if (!mesh) return;
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const scale = new Vector3(1, 1, 1);
    for (const [index, sample] of pattern.samples.entries()) {
      const projected = projectGlobeCoordinate(
        sample.longitude_degrees,
        sample.latitude_degrees,
        view,
        world,
        progress,
        GLOBE_RADIUS * 1.005,
        0.008,
      );
      const position = new Vector3(...projected.position);
      quaternion.setFromUnitVectors(
        SURFACE_NORMAL,
        new Vector3(...projected.normal),
      );
      matrix.compose(position, quaternion, scale);
      mesh.setMatrixAt(index, matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
  }, [pattern, progress, view, world]);
  return (
    <instancedMesh
      args={[undefined, undefined, pattern.samples.length]}
      material={material}
      name={`context-globe-pole-pattern:${pattern.pole}`}
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
  progress,
  radius,
  view,
  world,
}: {
  color: number;
  halo?: boolean;
  latitude: number;
  longitude: number;
  name: string;
  progress: number;
  radius: number;
  view: GlobeCameraView;
  world: { width: number; height: number };
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
  const projected = projectGlobeCoordinate(
    longitude,
    latitude,
    view,
    world,
    progress,
    GLOBE_RADIUS * 1.013,
    0.02,
  );
  const position = new Vector3(...projected.position);
  const quaternion = new Quaternion().setFromUnitVectors(
    SURFACE_NORMAL,
    new Vector3(...projected.normal),
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

function GlobeSector({
  progress,
  sector,
  view,
  world,
}: {
  progress: number;
  sector: NonNullable<CompiledContextGlobe["globe"]["sectors"]>[number];
  view: GlobeCameraView;
  world: { width: number; height: number };
}) {
  const meshes = useMemo(
    () =>
      buildProjectedBoundsMeshes(
        {
          west_degrees: sector.west_degrees,
          south_degrees: sector.south_degrees,
          east_degrees: sector.east_degrees,
          north_degrees: sector.north_degrees,
          crosses_antimeridian: sector.crosses_antimeridian,
        },
        view,
        world,
        progress,
      ),
    [
      progress,
      sector.crosses_antimeridian,
      sector.east_degrees,
      sector.north_degrees,
      sector.south_degrees,
      sector.west_degrees,
      view.pitch_degrees,
      view.yaw_degrees,
      world.height,
      world.width,
    ],
  );
  const material = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color: 0xd57824,
        depthWrite: false,
        opacity: 0.18,
        side: DoubleSide,
        transparent: true,
      }),
    [],
  );
  useEffect(() => () => material.dispose(), [material]);
  return meshes.map((mesh, index) => (
    <mesh
      material={material}
      name={`context-globe-sector:${sector.id}:${index}`}
      key={index}
    >
      <ProjectedMeshGeometry data={mesh} />
    </mesh>
  ));
}

function ProjectedMeshGeometry({ data }: { data: ProjectedGlobeMesh }) {
  const geometryRef = useRef<BufferGeometry>(null);
  useLayoutEffect(() => geometryRef.current?.computeBoundingSphere(), [data]);
  return (
    <bufferGeometry ref={geometryRef}>
      <bufferAttribute
        args={[data.positions, 3]}
        attach="attributes-position"
      />
      <bufferAttribute args={[data.normals, 3]} attach="attributes-normal" />
      <bufferAttribute args={[data.indices, 1]} attach="index" />
    </bufferGeometry>
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
