import { DoubleSide, MeshBasicNodeMaterial } from "three/src/Three.WebGPU.js";
import { uniform } from "three/src/nodes/TSL.js";
import { useEffect, useLayoutEffect, useMemo, useRef } from "react";
import type { GlobeCameraView } from "../types";
import {
  buildProjectedBoundsMeshes,
  globeAtlasRepeatSeamWeight,
  globeAtlasRepeatVisibility,
  globeAtlasWidth,
  globeProjectionMorphRemaining,
  interpolateProjectedGlobeMeshes,
  projectGlobeAtlasRepeatCoordinate,
  projectGlobeCoordinate,
  type ProjectedGlobeMeshInterpolationBuffer,
} from "../globe-projection";
import type { CompiledContextGlobe } from "../three-globe";
import { ProjectedMeshGeometry } from "./projected-mesh-geometry";

export function GlobeSector({
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
  // The bent seam-connection math a repeated sector needs is exact and
  // linear in the eased morph progress (verified against buildProjectedBoundsMeshes
  // directly across sampled progress values and both wrap directions), so
  // the sphere/atlas endpoints can be built once per (bounds, view, wrapIndex)
  // and blended every frame with a cheap array lerp instead of re-running
  // buildProjectedBoundsMeshes' full trigonometric projection on every vertex,
  // every frame, for every sector on screen.
  const endpointMeshes = useMemo(() => {
    const bounds = {
      west_degrees: sector.west_degrees,
      south_degrees: sector.south_degrees,
      east_degrees: sector.east_degrees,
      north_degrees: sector.north_degrees,
      crosses_antimeridian: sector.crosses_antimeridian,
    };
    return {
      sphere: buildProjectedBoundsMeshes(
        bounds,
        view,
        world,
        0,
        16,
        10,
        wrapIndex,
      ),
      atlas: buildProjectedBoundsMeshes(
        bounds,
        view,
        world,
        1,
        16,
        10,
        wrapIndex,
      ),
    };
  }, [
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
  ]);
  const interpolationBuffersRef = useRef<
    ProjectedGlobeMeshInterpolationBuffer[]
  >([]);
  const meshes = useMemo(() => {
    const easedProgress = 1 - globeProjectionMorphRemaining(progress);
    const buffers = interpolationBuffersRef.current;
    return endpointMeshes.sphere.map((sphereSpan, index) => {
      const atlasSpan = endpointMeshes.atlas[index]!;
      const retained = buffers[index];
      const buffer =
        retained &&
        retained.positions.length === sphereSpan.positions.length &&
        retained.normals.length === sphereSpan.normals.length
          ? retained
          : (buffers[index] = {
              positions: new Float32Array(sphereSpan.positions.length),
              normals: new Float32Array(sphereSpan.normals.length),
            });
      return interpolateProjectedGlobeMeshes(
        sphereSpan,
        atlasSpan,
        easedProgress,
        buffer,
      );
    });
  }, [endpointMeshes, progress]);
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
