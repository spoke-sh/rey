import { extend, useThree } from "@react-three/fiber";
import {
  AdditiveBlending,
  AlwaysStencilFunc,
  AmbientLight,
  BackSide,
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DoubleSide,
  DirectionalLight,
  Group,
  InstancedBufferAttribute,
  InstancedMesh,
  KeepStencilOp,
  LineSegments,
  LineBasicNodeMaterial,
  Matrix4,
  Mesh,
  MeshBasicNodeMaterial,
  MeshStandardNodeMaterial,
  NotEqualStencilFunc,
  OrthographicCamera,
  Quaternion,
  ReplaceStencilOp,
  RingGeometry,
  Vector3,
} from "three/src/Three.WebGPU.js";
import {
  attribute,
  float,
  max,
  mix,
  mul,
  positionGeometry,
  positionLocal,
  smoothstep,
  step,
  sub,
  uniform,
} from "three/src/nodes/TSL.js";
import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import type { GlobeCameraView, TerrainCameraView } from "./types";
import {
  buildProjectedBoundsMeshes,
  buildProjectedGlobeMesh,
  GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES,
  GLOBE_CAMERA_HALF_HEIGHT,
  globeAtlasRepeatConnectionProgress,
  globeAtlasRepeatDepthOffset,
  globeAtlasRepeatOpacity,
  globeAtlasRepeatOffset,
  globeAtlasRepeatSeamWeight,
  globeAtlasRepeatVisibility,
  globeAtlasWidth,
  globeAtlasViewCenter,
  globeAtmosphereOpacity,
  globeAtmosphereRepeatOpacity,
  globeAtmosphereShellScale,
  globeProjectionMorphRemaining,
  globeSurfaceOpacity,
  interpolateProjectedGlobeMeshes,
  projectGlobeAtlasRepeatCoordinate,
  projectGlobeCoordinate,
  type ProjectedGlobeMesh,
  type ProjectedGlobeMeshInterpolationBuffer,
} from "./globe-projection";
import {
  GLOBE_RADIUS,
  GLOBE_SAMPLE_RADIUS,
  SEMANTIC_GLOBE_MATERIAL_REVISION,
  type CompiledContextGlobe,
} from "./three-globe";
import {
  createContinuousReliefMaterial,
  continuousReliefMaterialRevision,
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
  LineSegments,
  LineBasicNodeMaterial,
  Mesh,
  OrthographicCamera,
  RingGeometry,
});

const SURFACE_NORMAL = new Vector3(0, 0, 1);
const MERCATOR_STIPPLE_OPACITY_SCALE = 0.36;
const GLOBE_ATMOSPHERE_LAYERS = [
  { radius: GLOBE_RADIUS * 1.035, color: 0xffd977, opacity: 0.17 },
  { radius: GLOBE_RADIUS * 1.068, color: 0xfff0bd, opacity: 0.15 },
  { radius: GLOBE_RADIUS * 1.11, color: 0xf4dfa5, opacity: 0.09 },
] as const;

interface GlobeRepeatProjectionCache {
  readonly matrices: Float32Array;
  readonly morphOffsets: Float32Array;
  readonly projectionRevision: string;
  readonly seamWeights: Float32Array;
  readonly sourceBucket: CompiledContextGlobe["sample_buckets"][number];
  readonly sourceIndexes: Uint32Array;
}

interface GlobeCanonicalProjectionCache {
  readonly atlasPositions: Float32Array;
  readonly matrices: Float32Array;
  readonly projectionRevision: string;
  readonly sourceBucket: CompiledContextGlobe["sample_buckets"][number];
}

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
  const repeatOpacity = globeAtlasRepeatOpacity(projectionMorphProgress);
  const horizontalLayoutWrapIndexes = GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES;
  const renderedWrapIndexes = GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES;
  const halfHeight = GLOBE_CAMERA_HALF_HEIGHT;
  const morphRemaining = globeProjectionMorphRemaining(projectionMorphProgress);
  const endpointMeshes = useMemo(
    () => ({
      atlas: buildProjectedGlobeMesh(view, world, 1),
      sphere: buildProjectedGlobeMesh(view, world, 0),
    }),
    [view.pitch_degrees, view.yaw_degrees, world.height, world.width],
  );
  // Runs every animation frame while the globe morphs. Reuse one retained
  // buffer across frames instead of letting interpolateProjectedGlobeMeshes
  // allocate two fresh ~47k-float arrays per frame.
  const interpolationBufferRef =
    useRef<ProjectedGlobeMeshInterpolationBuffer | null>(null);
  const projectedMesh = useMemo(() => {
    const positionsLength = endpointMeshes.sphere.positions.length;
    const normalsLength = endpointMeshes.sphere.normals.length;
    const retained = interpolationBufferRef.current;
    const buffer =
      retained &&
      retained.positions.length === positionsLength &&
      retained.normals.length === normalsLength
        ? retained
        : (interpolationBufferRef.current = {
            positions: new Float32Array(positionsLength),
            normals: new Float32Array(normalsLength),
          });
    return interpolateProjectedGlobeMeshes(
      endpointMeshes.sphere,
      endpointMeshes.atlas,
      1 - morphRemaining,
      buffer,
    );
  }, [endpointMeshes, morphRemaining]);
  const projectedGeometry = useProjectedMeshGeometry(
    projectedMesh,
    endpointMeshes.sphere.normals,
  );
  return (
    <>
      <ReyOrthographicCamera
        bottom={-halfHeight}
        far={100}
        left={-halfHeight * aspect * horizontalLayoutWrapIndexes.length}
        position={[0, 0, 6]}
        right={halfHeight * aspect * horizontalLayoutWrapIndexes.length}
        rotation={[0, 0, 0]}
        top={halfHeight}
      />
      <group name={`context-globe:${compiled.globe.source_revision}`}>
        <GlobeSurface
          geometry={projectedGeometry}
          maskRepeats={repeatOpacity > 0}
          progress={projectionMorphProgress}
        />
        <GlobeAtmosphere
          geometry={projectedGeometry}
          progress={projectionMorphProgress}
          world={world}
        />
        {renderedWrapIndexes.map((wrapIndex) => (
          <GlobeAtlasLayers
            compiled={compiled}
            key={wrapIndex}
            progress={projectionMorphProgress}
            view={view}
            world={world}
            wrapIndex={wrapIndex}
          />
        ))}
        {compiled.sample_buckets.map((bucket) => (
          <GlobeSampleField
            bucket={bucket}
            key={bucket.id}
            progress={projectionMorphProgress}
            repeatOpacity={repeatOpacity}
            view={view}
            world={world}
            wrapIndexes={renderedWrapIndexes}
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
      </group>
      <ambientLight color={0xffffff} intensity={0.72} />
      <directionalLight
        color={0xfffef8}
        intensity={1.85}
        position={[-3.8, 4.8, 6.2]}
      />
      <directionalLight
        color={0xa6b8b2}
        intensity={0.62}
        position={[4.8, 1.4, -3.8]}
      />
    </>
  );
}

function GlobeAtlasLayers({
  compiled,
  progress,
  view,
  world,
  wrapIndex,
}: {
  compiled: CompiledContextGlobe;
  progress: number;
  view: GlobeCameraView;
  world: { width: number; height: number };
  wrapIndex: number;
}) {
  const wrappedName = (name: string) =>
    wrapIndex === 0 ? name : `${name}:wrap:${wrapIndex}`;
  return (
    <group
      name={`context-globe-atlas-wrap:${wrapIndex}`}
      position={[globeAtlasRepeatOffset(world, progress, wrapIndex), 0, 0]}
    >
      {(compiled.globe.sectors ?? []).map((sector) => (
        <GlobeSector
          key={sector.id}
          name={wrappedName(`context-globe-sector:${sector.id}`)}
          progress={progress}
          sector={sector}
          view={view}
          world={world}
          wrapIndex={wrapIndex}
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
          name={wrappedName(`semantic-region:${region.id}`)}
          progress={progress}
          radius={
            0.026 + Math.min(0.056, region.angular_radius_degrees / 2_200)
          }
          view={view}
          world={world}
          wrapIndex={wrapIndex}
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
          name={wrappedName(`workload-beacon:${beacon.workload_id}`)}
          progress={progress}
          radius={beacon.mapping_role === "survey" ? 0.065 : 0.048}
          view={view}
          world={world}
          wrapIndex={wrapIndex}
        />
      ))}
    </group>
  );
}

function GlobeSurface({
  geometry,
  maskRepeats,
  progress,
}: {
  geometry: BufferGeometry;
  maskRepeats: boolean;
  progress: number;
}) {
  const morphRemaining = globeProjectionMorphRemaining(progress);
  const opacity = globeSurfaceOpacity(progress);
  const material = useMemo(() => {
    const next = new MeshStandardNodeMaterial();
    next.name = SEMANTIC_GLOBE_MATERIAL_REVISION;
    next.color.set(0xe8e9df);
    next.roughness = 0.98;
    next.metalness = 0;
    return next;
  }, []);
  useEffect(() => () => material.dispose(), [material]);
  useLayoutEffect(() => {
    const wasTransparent = material.transparent;
    material.depthWrite = opacity >= 1 || maskRepeats;
    material.opacity = opacity;
    material.transparent = opacity < 1;
    if (material.transparent !== wasTransparent) material.needsUpdate = true;
  }, [maskRepeats, material, opacity]);
  return (
    <mesh
      geometry={geometry}
      visible={morphRemaining > 0}
      material={material}
      name="context-globe-surface"
      renderOrder={maskRepeats ? -1 : 0}
    />
  );
}

function GlobeAtmosphere({
  geometry,
  progress,
  world,
}: {
  geometry: BufferGeometry;
  progress: number;
  world: { width: number; height: number };
}) {
  const morphRemaining = globeProjectionMorphRemaining(progress);
  const opacity = globeAtmosphereOpacity(progress);
  const repeatGlowOpacity = globeAtmosphereRepeatOpacity(progress);
  const shellScale = globeAtmosphereShellScale(progress);
  return (
    <>
      <AtmosphereStencilMask
        geometry={geometry}
        name="context-globe-atmosphere-mask"
        visible={opacity > 0}
      />
      {GLOBE_ATMOSPHERE_LAYERS.map((layer, index) => (
        <ProjectedAtmosphereLayer
          color={layer.color}
          geometry={geometry}
          key={index}
          name={`context-globe-atmosphere:${index}`}
          opacity={layer.opacity * opacity}
          shellOffset={(layer.radius - GLOBE_RADIUS) * shellScale}
        />
      ))}
      {GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES.filter(
        (wrapIndex) => wrapIndex !== 0,
      ).map((wrapIndex) => (
        <group
          key={wrapIndex}
          name={`context-globe-atmosphere-wrap:${wrapIndex}`}
          position={[globeAtlasRepeatOffset(world, progress, wrapIndex), 0, 0]}
        >
          <AtmosphereStencilMask
            geometry={geometry}
            name={`context-globe-atmosphere-mask:wrap:${wrapIndex}`}
            visible={repeatGlowOpacity > 0}
          />
          {GLOBE_ATMOSPHERE_LAYERS.map((layer, index) => (
            <ProjectedAtmosphereLayer
              color={layer.color}
              geometry={geometry}
              key={index}
              name={`context-globe-atmosphere:${index}:wrap:${wrapIndex}`}
              opacity={layer.opacity * repeatGlowOpacity}
              shellOffset={(layer.radius - GLOBE_RADIUS) * shellScale}
            />
          ))}
        </group>
      ))}
    </>
  );
}

function AtmosphereStencilMask({
  geometry,
  name,
  visible,
}: {
  geometry: BufferGeometry;
  name: string;
  visible: boolean;
}) {
  const material = useMemo(() => {
    const next = new MeshBasicNodeMaterial({
      colorWrite: false,
      depthTest: false,
      depthWrite: false,
      side: DoubleSide,
    });
    next.stencilWrite = true;
    next.stencilRef = 1;
    next.stencilFunc = AlwaysStencilFunc;
    next.stencilFail = ReplaceStencilOp;
    next.stencilZFail = ReplaceStencilOp;
    next.stencilZPass = ReplaceStencilOp;
    return next;
  }, []);
  useEffect(() => () => material.dispose(), [material]);
  return (
    <mesh
      geometry={geometry}
      material={material}
      name={name}
      renderOrder={-4}
      visible={visible}
    />
  );
}

function ProjectedAtmosphereLayer({
  color,
  geometry,
  name,
  opacity,
  shellOffset,
}: {
  color: number;
  geometry: BufferGeometry;
  name: string;
  opacity: number;
  shellOffset: number;
}) {
  const materialState = useMemo(() => {
    const shellOffsetNode = uniform(0);
    const opacityNode = uniform(0);
    const falloffLimitNode = uniform(1);
    const sphereNormal = attribute<"vec3">("reySphereNormal", "vec3");
    const material = new MeshBasicNodeMaterial({
      blending: AdditiveBlending,
      color,
      depthWrite: false,
      opacity: 0,
      side: BackSide,
      transparent: true,
    });
    material.positionNode = positionLocal.add(
      sphereNormal.mul(shellOffsetNode),
    );
    material.opacityNode = mul(
      opacityNode,
      smoothstep(float(0), falloffLimitNode, sphereNormal.z.abs()),
    );
    material.stencilWrite = true;
    material.stencilRef = 1;
    material.stencilFunc = NotEqualStencilFunc;
    material.stencilFail = KeepStencilOp;
    material.stencilZFail = KeepStencilOp;
    material.stencilZPass = KeepStencilOp;
    return { falloffLimitNode, material, opacityNode, shellOffsetNode };
  }, [color]);
  const material = materialState.material;
  useLayoutEffect(() => {
    material.opacity = opacity;
    materialState.opacityNode.value = opacity;
    materialState.shellOffsetNode.value = shellOffset;
    const shellRadius = GLOBE_RADIUS + shellOffset;
    materialState.falloffLimitNode.value = Math.max(
      0.0001,
      Math.sqrt(Math.max(0, 1 - (GLOBE_RADIUS / shellRadius) ** 2)),
    );
  }, [material, materialState, opacity, shellOffset]);
  useEffect(() => () => material.dispose(), [material]);
  return (
    <mesh
      frustumCulled={false}
      geometry={geometry}
      material={material}
      name={name}
      visible={opacity > 0 && shellOffset > 0}
    />
  );
}

function GlobeSampleField({
  bucket,
  progress,
  repeatOpacity,
  view,
  world,
  wrapIndexes,
}: {
  bucket: CompiledContextGlobe["sample_buckets"][number];
  progress: number;
  repeatOpacity: number;
  view: GlobeCameraView;
  world: { width: number; height: number };
  wrapIndexes: readonly number[];
}) {
  const meshRefs = useRef(new Map<number, InstancedMesh>());
  const postureOpacity =
    bucket.opacity * (1 - progress * (1 - MERCATOR_STIPPLE_OPACITY_SCALE));
  const morphRemaining = globeProjectionMorphRemaining(progress);
  const canonicalMaterialState = useMemo(() => {
    const morphProgressNode = uniform(0);
    const atlasPosition = attribute<"vec3">("reyAtlasPosition", "vec3");
    const material = new MeshBasicNodeMaterial({
      color: bucket.color,
      opacity: bucket.opacity,
      transparent: true,
    });
    material.positionNode = mix(
      positionLocal,
      positionGeometry.add(atlasPosition),
      morphProgressNode,
    );
    return { material, morphProgressNode };
  }, [bucket.color, bucket.opacity]);
  const canonicalMaterial = canonicalMaterialState.material;
  const repeatedMaterialState = useMemo(() => {
    const morphRemainingNode = uniform(0);
    const morphOffset = attribute<"vec3">("reyRepeatMorphOffset", "vec3");
    const repeatOpacityNode = uniform(0);
    const postureOpacityNode = uniform(bucket.opacity);
    const material = new MeshBasicNodeMaterial({
      color: bucket.color,
      depthWrite: false,
      opacity: bucket.opacity,
      transparent: true,
    });
    material.opacityNode = mul(
      mul(
        smoothstep(
          sub(float(1), max(repeatOpacityNode, float(0.000_001))),
          float(1),
          attribute<"float">("reyRepeatSeamWeight", "float"),
        ),
        step(float(0.000_001), repeatOpacityNode),
      ),
      postureOpacityNode,
    );
    material.positionNode = positionLocal.add(
      morphOffset.mul(morphRemainingNode),
    );
    return {
      material,
      morphRemainingNode,
      postureOpacityNode,
      repeatOpacityNode,
    };
  }, [bucket.color, bucket.opacity]);
  const repeatedMaterial = repeatedMaterialState.material;
  useLayoutEffect(() => {
    canonicalMaterial.opacity = postureOpacity;
    repeatedMaterial.opacity = postureOpacity;
    canonicalMaterialState.morphProgressNode.value = 1 - morphRemaining;
    repeatedMaterialState.morphRemainingNode.value = morphRemaining;
    repeatedMaterialState.postureOpacityNode.value = postureOpacity;
    repeatedMaterialState.repeatOpacityNode.value = repeatOpacity;
  }, [
    canonicalMaterial,
    canonicalMaterialState,
    morphRemaining,
    postureOpacity,
    repeatOpacity,
    repeatedMaterial,
    repeatedMaterialState,
  ]);
  useEffect(
    () => () => {
      canonicalMaterial.dispose();
      repeatedMaterial.dispose();
    },
    [canonicalMaterial, repeatedMaterial],
  );
  useLayoutEffect(() => {
    const canonicalMesh = meshRefs.current.get(0);
    if (!canonicalMesh) return;
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const scale = new Vector3(1, 1, 1);
    const position = new Vector3();
    const normal = new Vector3();
    const repeatMeshes = new Map<
      number,
      {
        attribute: InstancedBufferAttribute;
        cache: GlobeRepeatProjectionCache | null;
        mesh: InstancedMesh;
        morphAttribute: InstancedBufferAttribute;
      }
    >();
    const projectionRevision = [
      view.yaw_degrees,
      view.pitch_degrees,
      world.width,
      world.height,
      bucket.samples.length,
    ].join(":");
    for (const wrapIndex of wrapIndexes) {
      if (wrapIndex === 0) continue;
      const wrappedMesh = meshRefs.current.get(wrapIndex);
      if (!wrappedMesh) continue;
      const existing = wrappedMesh.geometry.getAttribute("reyRepeatSeamWeight");
      const attribute =
        existing instanceof InstancedBufferAttribute &&
        existing.count === bucket.samples.length
          ? existing
          : new InstancedBufferAttribute(
              new Float32Array(bucket.samples.length),
              1,
            );
      if (attribute !== existing)
        wrappedMesh.geometry.setAttribute("reyRepeatSeamWeight", attribute);
      const existingMorph = wrappedMesh.geometry.getAttribute(
        "reyRepeatMorphOffset",
      );
      const morphAttribute =
        existingMorph instanceof InstancedBufferAttribute &&
        existingMorph.count === bucket.samples.length
          ? existingMorph
          : new InstancedBufferAttribute(
              new Float32Array(bucket.samples.length * 3),
              3,
            );
      if (morphAttribute !== existingMorph)
        wrappedMesh.geometry.setAttribute(
          "reyRepeatMorphOffset",
          morphAttribute,
        );
      const cache = wrappedMesh.userData
        .reyRepeatProjectionCache as GlobeRepeatProjectionCache | null;
      repeatMeshes.set(wrapIndex, {
        attribute,
        cache:
          cache?.projectionRevision === projectionRevision &&
          cache.sourceBucket === bucket
            ? cache
            : null,
        mesh: wrappedMesh,
        morphAttribute,
      });
    }
    const atlasWidth = globeAtlasWidth(world);
    const retainedCanonicalCache = canonicalMesh.userData
      .reyCanonicalProjectionCache as GlobeCanonicalProjectionCache | null;
    let canonicalCache =
      retainedCanonicalCache?.projectionRevision === projectionRevision &&
      retainedCanonicalCache.sourceBucket === bucket
        ? retainedCanonicalCache
        : null;
    if (
      canonicalCache === null ||
      [...repeatMeshes.values()].some(({ cache }) => cache === null)
    ) {
      const atlasCenter = globeAtlasViewCenter(view);
      canonicalCache = {
        atlasPositions: new Float32Array(bucket.samples.length * 3),
        matrices: new Float32Array(bucket.samples.length * 16),
        projectionRevision,
        sourceBucket: bucket,
      };
      const planarMatrices = new Float32Array(bucket.samples.length * 16);
      const planarPositions = new Float32Array(bucket.samples.length * 3);
      const closedSeamPositions = new Float32Array(bucket.samples.length * 3);
      const normalizedChartXs = new Float32Array(bucket.samples.length);
      for (const [index, sample] of bucket.samples.entries()) {
        const spherical = projectGlobeCoordinate(
          sample.longitude_degrees,
          sample.latitude_degrees,
          view,
          world,
          0,
          GLOBE_RADIUS * 1.005,
          0.008,
        );
        const planar = projectGlobeCoordinate(
          sample.longitude_degrees,
          sample.latitude_degrees,
          view,
          world,
          1,
          GLOBE_RADIUS * 1.005,
          0.008,
        );
        const closedSeam = projectGlobeCoordinate(
          atlasCenter.longitude_degrees + 180,
          sample.latitude_degrees,
          view,
          world,
          0,
          GLOBE_RADIUS * 1.005,
          0.008,
        );
        position.set(...spherical.position);
        normal.set(...spherical.normal);
        quaternion.setFromUnitVectors(SURFACE_NORMAL, normal);
        matrix.compose(position, quaternion, scale);
        const matrixOffset = index * 16;
        canonicalCache.matrices.set(matrix.elements, matrixOffset);
        matrix.makeTranslation(...planar.position);
        planarMatrices.set(matrix.elements, matrixOffset);
        const morphOffset = index * 3;
        canonicalCache.atlasPositions.set(planar.position, morphOffset);
        planarPositions.set(planar.position, morphOffset);
        closedSeamPositions.set(closedSeam.position, morphOffset);
        normalizedChartXs[index] = planar.atlas_position[0] / atlasWidth + 0.5;
      }
      const caches = new Map<number, GlobeRepeatProjectionCache>();
      for (const [wrapIndex, repeat] of repeatMeshes) {
        const sourceIndexes = Uint32Array.from(
          bucket.samples.map((_, index) => index),
        );
        sourceIndexes.sort((left, right) => {
          const weightDifference =
            globeAtlasRepeatSeamWeight(normalizedChartXs[right]!, wrapIndex) -
            globeAtlasRepeatSeamWeight(normalizedChartXs[left]!, wrapIndex);
          return weightDifference || left - right;
        });
        const cache: GlobeRepeatProjectionCache = {
          matrices: new Float32Array(bucket.samples.length * 16),
          morphOffsets: new Float32Array(bucket.samples.length * 3),
          projectionRevision,
          seamWeights: new Float32Array(bucket.samples.length),
          sourceBucket: bucket,
          sourceIndexes,
        };
        for (let index = 0; index < sourceIndexes.length; index += 1) {
          const sourceIndex = sourceIndexes[index]!;
          const matrixOffset = index * 16;
          const sourceMatrixOffset = sourceIndex * 16;
          const morphOffset = index * 3;
          const sourceMorphOffset = sourceIndex * 3;
          const seamWeight = globeAtlasRepeatSeamWeight(
            normalizedChartXs[sourceIndex]!,
            wrapIndex,
          );
          const connectionProgress =
            globeAtlasRepeatConnectionProgress(seamWeight);
          repeat.attribute.setX(index, seamWeight);
          cache.seamWeights[index] = seamWeight;
          cache.matrices.set(
            planarMatrices.subarray(
              sourceMatrixOffset,
              sourceMatrixOffset + 16,
            ),
            matrixOffset,
          );
          cache.morphOffsets[morphOffset] =
            closedSeamPositions[sourceMorphOffset]! * connectionProgress;
          cache.morphOffsets[morphOffset + 1] =
            (closedSeamPositions[sourceMorphOffset + 1]! -
              planarPositions[sourceMorphOffset + 1]!) *
            connectionProgress;
          cache.morphOffsets[morphOffset + 2] =
            (closedSeamPositions[sourceMorphOffset + 2]! -
              planarPositions[sourceMorphOffset + 2]!) *
              connectionProgress +
            globeAtlasRepeatDepthOffset(0, seamWeight);
        }
        caches.set(wrapIndex, cache);
      }
      canonicalMesh.userData.reyCanonicalProjectionCache = canonicalCache;
      canonicalMesh.geometry.setAttribute(
        "reyAtlasPosition",
        new InstancedBufferAttribute(canonicalCache.atlasPositions, 3),
      );
      canonicalMesh.instanceMatrix.array.set(canonicalCache.matrices);
      canonicalMesh.instanceMatrix.needsUpdate = true;
      canonicalMesh.userData.reyStippleMorphExecution = "gpu_uniform";
      for (const [wrapIndex, repeat] of repeatMeshes) {
        const cache = caches.get(wrapIndex)!;
        repeat.cache = cache;
        repeat.mesh.userData.reyRepeatProjectionCache = cache;
        repeat.attribute.needsUpdate = true;
        repeat.morphAttribute.array.set(cache.morphOffsets);
        repeat.morphAttribute.needsUpdate = true;
        repeat.mesh.instanceMatrix.array.set(cache.matrices);
        repeat.mesh.instanceMatrix.needsUpdate = true;
        repeat.mesh.userData.reyStippleMorphExecution = "gpu_uniform";
      }
    }
    for (const repeat of repeatMeshes.values()) {
      const cache = repeat.cache!;
      if (repeatOpacity <= 0) repeat.mesh.count = 0;
      else {
        const visibleStart = 1 - repeatOpacity;
        let visibleCount = 0;
        while (
          visibleCount < cache.seamWeights.length &&
          cache.seamWeights[visibleCount]! > visibleStart
        )
          visibleCount += 1;
        repeat.mesh.count = visibleCount;
      }
    }
  }, [bucket, progress, view, world, wrapIndexes]);
  return wrapIndexes.map((wrapIndex) => (
    <group
      key={wrapIndex}
      position={[globeAtlasRepeatOffset(world, progress, wrapIndex), 0, 0]}
    >
      <instancedMesh
        args={[undefined, undefined, bucket.samples.length]}
        material={wrapIndex === 0 ? canonicalMaterial : repeatedMaterial}
        name={wrapIndex === 0 ? bucket.id : `${bucket.id}:wrap:${wrapIndex}`}
        ref={(mesh) => {
          if (mesh) meshRefs.current.set(wrapIndex, mesh);
          else meshRefs.current.delete(wrapIndex);
        }}
      >
        <circleGeometry args={[GLOBE_SAMPLE_RADIUS, 5]} />
      </instancedMesh>
    </group>
  ));
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
  wrapIndex,
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
  wrapIndex: number;
}) {
  const projected =
    wrapIndex === 0
      ? projectGlobeCoordinate(
          longitude,
          latitude,
          view,
          world,
          progress,
          GLOBE_RADIUS * 1.013,
          0.02,
        )
      : projectGlobeAtlasRepeatCoordinate(
          longitude,
          latitude,
          view,
          world,
          progress,
          wrapIndex,
          GLOBE_RADIUS * 1.013,
          0.02,
        );
  const seamWeight = globeAtlasRepeatSeamWeight(
    projected.atlas_position[0] / globeAtlasWidth(world) + 0.5,
    wrapIndex,
  );
  const opacity =
    wrapIndex === 0 ? 1 : globeAtlasRepeatVisibility(progress, seamWeight);
  // Markers re-project every frame while the globe morphs, but rebuilding a
  // MeshBasicNodeMaterial on every opacity tick forces WebGPU pipeline churn
  // for every visible marker each frame. Build the material once per color
  // and drive opacity through a uniform instead, mirroring GlobeSurface and
  // ProjectedAtmosphereLayer below.
  const pointMaterialState = useMemo(() => {
    const opacityNode = uniform(1);
    const material = new MeshBasicNodeMaterial({ color });
    material.opacityNode = opacityNode;
    return { material, opacityNode };
  }, [color]);
  const pointMaterial = pointMaterialState.material;
  const haloMaterialState = useMemo(() => {
    const opacityNode = uniform(1);
    const material = new MeshBasicNodeMaterial({
      color,
      depthWrite: false,
      transparent: true,
    });
    material.opacityNode = opacityNode;
    return { material, opacityNode };
  }, [color]);
  const haloMaterial = haloMaterialState.material;
  useLayoutEffect(() => {
    const wasTransparent = pointMaterial.transparent;
    pointMaterial.depthWrite = opacity >= 1;
    pointMaterial.opacity = opacity;
    pointMaterial.transparent = opacity < 1;
    pointMaterialState.opacityNode.value = opacity;
    if (pointMaterial.transparent !== wasTransparent)
      pointMaterial.needsUpdate = true;
    haloMaterial.opacity = 0.42 * opacity;
    haloMaterialState.opacityNode.value = 0.42 * opacity;
  }, [
    haloMaterial,
    haloMaterialState,
    opacity,
    pointMaterial,
    pointMaterialState,
  ]);
  useEffect(
    () => () => {
      pointMaterial.dispose();
      haloMaterial.dispose();
    },
    [haloMaterial, pointMaterial],
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
  name,
  progress,
  sector,
  view,
  world,
  wrapIndex,
}: {
  name: string;
  progress: number;
  sector: NonNullable<CompiledContextGlobe["globe"]["sectors"]>[number];
  view: GlobeCameraView;
  world: { width: number; height: number };
  wrapIndex: number;
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
        16,
        10,
        wrapIndex,
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
      wrapIndex,
      world.height,
      world.width,
    ],
  );
  const unwrappedEast =
    sector.crosses_antimeridian || sector.east_degrees < sector.west_degrees
      ? sector.east_degrees + 360
      : sector.east_degrees;
  const projectedCenter =
    wrapIndex === 0
      ? projectGlobeCoordinate(
          (sector.west_degrees + unwrappedEast) / 2,
          (sector.south_degrees + sector.north_degrees) / 2,
          view,
          world,
          progress,
        )
      : projectGlobeAtlasRepeatCoordinate(
          (sector.west_degrees + unwrappedEast) / 2,
          (sector.south_degrees + sector.north_degrees) / 2,
          view,
          world,
          progress,
          wrapIndex,
        );
  const seamWeight = globeAtlasRepeatSeamWeight(
    projectedCenter.atlas_position[0] / globeAtlasWidth(world) + 0.5,
    wrapIndex,
  );
  const opacity =
    wrapIndex === 0 ? 1 : globeAtlasRepeatVisibility(progress, seamWeight);
  // Sectors rebuild their geometry every frame while the globe morphs; build
  // the material once instead of on every opacity tick, or every sector on
  // screen forces a WebGPU pipeline rebuild each animation frame.
  const materialState = useMemo(() => {
    const opacityNode = uniform(0.18);
    const material = new MeshBasicNodeMaterial({
      color: 0xd57824,
      depthWrite: false,
      side: DoubleSide,
      transparent: true,
    });
    material.opacityNode = opacityNode;
    return { material, opacityNode };
  }, []);
  const material = materialState.material;
  useLayoutEffect(() => {
    material.opacity = 0.18 * opacity;
    materialState.opacityNode.value = 0.18 * opacity;
  }, [material, materialState, opacity]);
  useEffect(() => () => material.dispose(), [material]);
  return meshes.map((mesh, index) => (
    <mesh material={material} name={`${name}:${index}`} key={index}>
      <ProjectedMeshGeometry data={mesh} />
    </mesh>
  ));
}

function ProjectedMeshGeometry({ data }: { data: ProjectedGlobeMesh }) {
  const geometry = useProjectedMeshGeometry(data);
  return <primitive attach="geometry" object={geometry} />;
}

function useProjectedMeshGeometry(
  data: ProjectedGlobeMesh,
  sphereNormals?: Float32Array,
) {
  const resources = useMemo(() => {
    const positions = new Float32Array(data.positions.length);
    const normals = new Float32Array(data.normals.length);
    const indices = new Uint32Array(data.indices.length);
    const positionAttribute = new BufferAttribute(positions, 3);
    const normalAttribute = new BufferAttribute(normals, 3);
    const indexAttribute = new BufferAttribute(indices, 1);
    const retainedSphereNormals = sphereNormals
      ? new Float32Array(sphereNormals.length)
      : null;
    const sphereNormalAttribute = retainedSphereNormals
      ? new BufferAttribute(retainedSphereNormals, 3)
      : null;
    const geometry = new BufferGeometry();
    geometry.setAttribute("position", positionAttribute);
    geometry.setAttribute("normal", normalAttribute);
    if (sphereNormalAttribute)
      geometry.setAttribute("reySphereNormal", sphereNormalAttribute);
    geometry.setIndex(indexAttribute);
    return {
      geometry,
      indexAttribute,
      indices,
      normalAttribute,
      normals,
      positionAttribute,
      positions,
      retainedSphereNormals,
      sphereNormalAttribute,
    };
  }, [
    data.indices.length,
    data.normals.length,
    data.positions.length,
    sphereNormals?.length,
  ]);
  useLayoutEffect(() => {
    resources.positions.set(data.positions);
    resources.normals.set(data.normals);
    resources.indices.set(data.indices);
    resources.positionAttribute.needsUpdate = true;
    resources.normalAttribute.needsUpdate = true;
    resources.indexAttribute.needsUpdate = true;
    if (
      sphereNormals &&
      resources.retainedSphereNormals &&
      resources.sphereNormalAttribute
    ) {
      resources.retainedSphereNormals.set(sphereNormals);
      resources.sphereNormalAttribute.needsUpdate = true;
    }
    resources.geometry.computeBoundingSphere();
  }, [data, resources, sphereNormals]);
  useEffect(() => () => resources.geometry.dispose(), [resources]);
  return resources.geometry;
}

function ReyOrthographicCamera({
  bottom,
  far,
  left,
  position,
  right,
  rotation,
  target,
  top,
}: {
  bottom: number;
  far: number;
  left: number;
  position: readonly [number, number, number];
  right: number;
  rotation: readonly [number, number, number];
  target?: readonly [number, number, number];
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
    if (target) camera.lookAt(target[0], target[1], target[2]);
    camera.updateProjectionMatrix();
    set({ camera });
    return () => set({ camera: previous });
  }, [bottom, far, get, left, position, right, rotation, set, target, top]);
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
