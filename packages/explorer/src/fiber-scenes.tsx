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
  mul,
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
  readonly matrices: Float32Array;
  readonly morphDeltas: Float32Array;
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
        target={camera.target}
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
  const projectedMesh = useMemo(
    () =>
      interpolateProjectedGlobeMeshes(
        endpointMeshes.sphere,
        endpointMeshes.atlas,
        1 - morphRemaining,
      ),
    [endpointMeshes, morphRemaining],
  );
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
  const canonicalMaterial = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color: bucket.color,
        opacity: bucket.opacity,
        transparent: true,
      }),
    [bucket.color, bucket.opacity],
  );
  const repeatedMaterialState = useMemo(() => {
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
    return { material, postureOpacityNode, repeatOpacityNode };
  }, [bucket.color, bucket.opacity]);
  const repeatedMaterial = repeatedMaterialState.material;
  useLayoutEffect(() => {
    canonicalMaterial.opacity = postureOpacity;
    repeatedMaterial.opacity = postureOpacity;
    repeatedMaterialState.postureOpacityNode.value = postureOpacity;
    repeatedMaterialState.repeatOpacityNode.value = repeatOpacity;
  }, [
    canonicalMaterial,
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
        matrices: new Float32Array(bucket.samples.length * 16),
        morphDeltas: new Float32Array(bucket.samples.length * 16),
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
        for (let element = 0; element < 16; element += 1)
          canonicalCache.morphDeltas[matrixOffset + element] =
            matrix.elements[element]! -
            canonicalCache.matrices[matrixOffset + element]!;
        const morphOffset = index * 3;
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
      for (const [wrapIndex, repeat] of repeatMeshes) {
        const cache = caches.get(wrapIndex)!;
        repeat.cache = cache;
        repeat.mesh.userData.reyRepeatProjectionCache = cache;
        repeat.attribute.needsUpdate = true;
      }
    }
    const morphRemaining = globeProjectionMorphRemaining(progress);
    const morphProgress = 1 - morphRemaining;
    const canonicalMatrices = canonicalMesh.instanceMatrix.array;
    canonicalMatrices.set(canonicalCache.matrices);
    for (let element = 0; element < canonicalMatrices.length; element += 1)
      canonicalMatrices[element] =
        canonicalCache.matrices[element]! +
        canonicalCache.morphDeltas[element]! * morphProgress;
    canonicalMesh.instanceMatrix.needsUpdate = true;
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
      const matrices = repeat.mesh.instanceMatrix.array;
      matrices.set(cache.matrices);
      for (let index = 0; index < repeat.mesh.count; index += 1) {
        const matrixOffset = index * 16;
        const morphOffset = index * 3;
        matrices[matrixOffset + 12] =
          (matrices[matrixOffset + 12] ?? 0) +
          cache.morphOffsets[morphOffset]! * morphRemaining;
        matrices[matrixOffset + 13] =
          (matrices[matrixOffset + 13] ?? 0) +
          cache.morphOffsets[morphOffset + 1]! * morphRemaining;
        matrices[matrixOffset + 14] =
          (matrices[matrixOffset + 14] ?? 0) +
          cache.morphOffsets[morphOffset + 2]! * morphRemaining;
      }
      repeat.mesh.instanceMatrix.needsUpdate = true;
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
  const pointMaterial = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color,
        depthWrite: opacity >= 1,
        opacity,
        transparent: opacity < 1,
      }),
    [color, opacity],
  );
  const haloMaterial = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color,
        depthWrite: false,
        opacity: 0.42 * opacity,
        transparent: true,
      }),
    [color, opacity],
  );
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
  const material = useMemo(
    () =>
      new MeshBasicNodeMaterial({
        color: 0xd57824,
        depthWrite: false,
        opacity: 0.18 * opacity,
        side: DoubleSide,
        transparent: true,
      }),
    [opacity],
  );
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
