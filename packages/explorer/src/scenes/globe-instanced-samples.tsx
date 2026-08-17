import {
  InstancedBufferAttribute,
  InstancedMesh,
  Matrix4,
  MeshBasicNodeMaterial,
  Quaternion,
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
import type { GlobeCameraView } from "../types";
import {
  globeAtlasProjectionCenter,
  globeAtlasRepeatConnectionProgress,
  globeAtlasRepeatDepthOffset,
  globeAtlasRepeatOffset,
  globeAtlasRepeatSeamWeight,
  globeAtlasWidth,
  globeProjectionMorphRemaining,
  projectGlobeCoordinate,
} from "../globe-projection";
import {
  GLOBE_RADIUS,
  GLOBE_SAMPLE_RADIUS,
  type CompiledContextGlobe,
} from "../three-globe";

const SURFACE_NORMAL = new Vector3(0, 0, 1);
const MERCATOR_STIPPLE_OPACITY_SCALE = 0.36;

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

export function GlobeSampleField({
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
    // Pitch no longer affects any cached position/matrix data here (see
    // globeCameraPose) — only yaw and world dimensions do.
    const projectionRevision = [
      view.yaw_degrees,
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
      // Matches projectGlobeAtlasRepeatCoordinate's own connected-seam
      // reference (globeAtlasProjectionCenter, fixed at pitch 0) — the flat
      // Atlas frame these repeat samples bend toward has no way to
      // represent pitch, so this must agree with it exactly.
      const atlasCenter = globeAtlasProjectionCenter(view);
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
