import { useMemo, useRef } from "react";
import type { GlobeCameraView } from "../types";
import {
  buildProjectedGlobeMesh,
  GLOBE_ATLAS_REGION_MARKER_RADIUS,
  GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES,
  GLOBE_CAMERA_HALF_HEIGHT,
  globeAtlasRepeatOffset,
  globeAtlasRepeatOpacity,
  globeCameraPose,
  globeProjectionMorphRemaining,
  interpolateProjectedGlobeMeshes,
  type ProjectedGlobeMeshInterpolationBuffer,
} from "../globe-projection";
import type { CompiledContextGlobe } from "../three-globe";
import { ReyOrthographicCamera } from "./orthographic-camera";
import { GlobeSurface, GlobeAtmosphere } from "./globe-surface";
import { GlobeSampleField } from "./globe-instanced-samples";
import { GlobePolePatternField } from "./globe-pole-pattern-field";
import { GlobeSurfaceMarker } from "./globe-surface-marker";
import { GlobeSector } from "./globe-sector";
import { useProjectedMeshGeometry } from "./projected-mesh-geometry";

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
    // Pitch no longer affects vertex data at all (see globeCameraPose) —
    // only yaw and world dimensions change the projected grid.
    [view.yaw_degrees, world.height, world.width],
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
    endpointMeshes.sphere.normalizedChartX,
  );
  const cameraPose = globeCameraPose(view, projectionMorphProgress);
  return (
    <>
      <ReyOrthographicCamera
        bottom={-halfHeight}
        far={100}
        left={-halfHeight * aspect * horizontalLayoutWrapIndexes.length}
        position={cameraPose.position}
        right={halfHeight * aspect * horizontalLayoutWrapIndexes.length}
        rotation={cameraPose.rotation}
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
            GLOBE_ATLAS_REGION_MARKER_RADIUS +
            Math.min(0.056, region.angular_radius_degrees / 2_200)
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
