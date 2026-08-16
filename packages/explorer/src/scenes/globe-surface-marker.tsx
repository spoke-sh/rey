import {
  MeshBasicNodeMaterial,
  Quaternion,
  Vector3,
} from "three/src/Three.WebGPU.js";
import { uniform } from "three/src/nodes/TSL.js";
import { useEffect, useLayoutEffect, useMemo } from "react";
import type { GlobeCameraView } from "../types";
import {
  globeAtlasRepeatSeamWeight,
  globeAtlasRepeatVisibility,
  globeAtlasWidth,
  projectGlobeAtlasRepeatCoordinate,
  projectGlobeCoordinate,
} from "../globe-projection";
import { GLOBE_RADIUS } from "../three-globe";

const SURFACE_NORMAL = new Vector3(0, 0, 1);

export function GlobeSurfaceMarker({
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
  // ProjectedAtmosphereLayer in globe-surface.tsx.
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
