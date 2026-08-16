import {
  AdditiveBlending,
  AlwaysStencilFunc,
  BackSide,
  BufferGeometry,
  DoubleSide,
  KeepStencilOp,
  MeshBasicNodeMaterial,
  MeshStandardNodeMaterial,
  NotEqualStencilFunc,
  ReplaceStencilOp,
} from "three/src/Three.WebGPU.js";
import {
  attribute,
  clamp,
  float,
  max,
  mul,
  positionLocal,
  smoothstep,
  step,
  sub,
  uniform,
} from "three/src/nodes/TSL.js";
import { useEffect, useLayoutEffect, useMemo } from "react";
import {
  GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES,
  globeAtlasRepeatOffset,
  globeAtlasRepeatOpacity,
  globeAtmosphereOpacity,
  globeAtmosphereRepeatOpacity,
  globeAtmosphereShellScale,
  globeProjectionMorphRemaining,
  globeSurfaceOpacity,
} from "../globe-projection";
import { GLOBE_RADIUS, SEMANTIC_GLOBE_MATERIAL_REVISION } from "../three-globe";

const GLOBE_ATMOSPHERE_LAYERS = [
  { radius: GLOBE_RADIUS * 1.035, color: 0xffd977, opacity: 0.17 },
  { radius: GLOBE_RADIUS * 1.068, color: 0xfff0bd, opacity: 0.15 },
  { radius: GLOBE_RADIUS * 1.11, color: 0xf4dfa5, opacity: 0.09 },
] as const;

export function GlobeSurface({
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

export function GlobeAtmosphere({
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
  // Sectors, samples, and markers all reveal a repeated wrap copy with the
  // same expanding-from-the-seam spatial sweep (globeAtlasRepeatOpacity used
  // as the reach parameter into globeAtlasRepeatVisibility). The atmosphere
  // glow previously applied one flat scalar (repeatGlowOpacity) across the
  // whole copy regardless of position, so it read as a uniform wash
  // disconnected from everything else's coordinated reveal. repeatSpatialReach
  // drives that same per-vertex sweep in ProjectedAtmosphereLayer's shader;
  // repeatGlowOpacity is kept as the overall envelope (still 0 at both the
  // World and Atlas endpoints, per the documented transient-glow design) so
  // the two compose as reach × envelope rather than replacing one another.
  const repeatSpatialReach = globeAtlasRepeatOpacity(progress);
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
              repeatSpatialReach={repeatSpatialReach}
              shellOffset={(layer.radius - GLOBE_RADIUS) * shellScale}
              wrapIndex={wrapIndex}
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
  repeatSpatialReach,
  shellOffset,
  wrapIndex = 0,
}: {
  color: number;
  geometry: BufferGeometry;
  name: string;
  opacity: number;
  repeatSpatialReach?: number;
  shellOffset: number;
  wrapIndex?: number;
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
    const rimFalloffNode = smoothstep(
      float(0),
      falloffLimitNode,
      sphereNormal.z.abs(),
    );
    // Unlike sectors/samples/markers, which reveal expanding outward FROM
    // the connected seam (globeAtlasRepeatVisibility in globe-projection.ts),
    // the glow is deliberately inverted: dim at the seam, brightest at the
    // copy's outer edge, so it reads as a directional cue toward the
    // content still ahead if the operator keeps unfurling — rather than
    // mirroring the seam-outward reveal everything else uses. Built from
    // the same expanding-smoothstep shape (see the exact-match check
    // against globeAtlasRepeatVisibility in globe-projection.test.ts), just
    // driven by the seam weight's complement instead of the seam weight
    // itself, so the "expands as repeatOpacity grows" region grows from the
    // outer edge inward rather than from the seam outward. The canonical
    // copy (wrapIndex 0) skips this term entirely, unchanged from before.
    if (wrapIndex !== 0) {
      const normalizedChartXAttribute = attribute<"float">(
        "reyNormalizedChartX",
        "float",
      );
      const boundedChartX = clamp(
        normalizedChartXAttribute,
        float(0),
        float(1),
      );
      const outerEdgeWeightNode =
        wrapIndex < 0 ? sub(float(1), boundedChartX) : boundedChartX;
      const repeatOpacityNode = uniform(0);
      const spatialSweepNode = mul(
        smoothstep(
          sub(float(1), max(repeatOpacityNode, float(0.000_001))),
          float(1),
          outerEdgeWeightNode,
        ),
        step(float(0.000_001), repeatOpacityNode),
      );
      material.opacityNode = mul(
        mul(opacityNode, rimFalloffNode),
        spatialSweepNode,
      );
      material.stencilWrite = true;
      material.stencilRef = 1;
      material.stencilFunc = NotEqualStencilFunc;
      material.stencilFail = KeepStencilOp;
      material.stencilZFail = KeepStencilOp;
      material.stencilZPass = KeepStencilOp;
      return {
        falloffLimitNode,
        material,
        opacityNode,
        repeatOpacityNode,
        shellOffsetNode,
      };
    }
    material.opacityNode = mul(opacityNode, rimFalloffNode);
    material.stencilWrite = true;
    material.stencilRef = 1;
    material.stencilFunc = NotEqualStencilFunc;
    material.stencilFail = KeepStencilOp;
    material.stencilZFail = KeepStencilOp;
    material.stencilZPass = KeepStencilOp;
    return {
      falloffLimitNode,
      material,
      opacityNode,
      repeatOpacityNode: undefined,
      shellOffsetNode,
    };
  }, [color, wrapIndex]);
  const material = materialState.material;
  useLayoutEffect(() => {
    material.opacity = opacity;
    materialState.opacityNode.value = opacity;
    materialState.shellOffsetNode.value = shellOffset;
    if (materialState.repeatOpacityNode)
      materialState.repeatOpacityNode.value = repeatSpatialReach ?? 0;
    const shellRadius = GLOBE_RADIUS + shellOffset;
    materialState.falloffLimitNode.value = Math.max(
      0.0001,
      Math.sqrt(Math.max(0, 1 - (GLOBE_RADIUS / shellRadius) ** 2)),
    );
  }, [material, materialState, opacity, repeatSpatialReach, shellOffset]);
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
