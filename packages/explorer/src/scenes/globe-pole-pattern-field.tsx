import {
  InstancedMesh,
  Matrix4,
  MeshBasicNodeMaterial,
  Quaternion,
  Vector3,
} from "three/src/Three.WebGPU.js";
import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import type { GlobeCameraView } from "../types";
import { projectGlobeCoordinate } from "../globe-projection";
import {
  GLOBE_RADIUS,
  GLOBE_SAMPLE_RADIUS,
  type CompiledContextGlobe,
} from "../three-globe";

const SURFACE_NORMAL = new Vector3(0, 0, 1);

export function GlobePolePatternField({
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
