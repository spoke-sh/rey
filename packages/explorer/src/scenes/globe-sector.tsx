import {
  BufferAttribute,
  BufferGeometry,
  DoubleSide,
  LineBasicNodeMaterial,
  MeshBasicNodeMaterial,
} from "three/src/Three.WebGPU.js";
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
  type ProjectedGlobeMesh,
  type ProjectedGlobeMeshInterpolationBuffer,
} from "../globe-projection";
import type { CompiledContextGlobe } from "../three-globe";
import { ProjectedMeshGeometry } from "./projected-mesh-geometry";

const SECTOR_LONGITUDE_SEGMENTS = 16;
const SECTOR_LATITUDE_SEGMENTS = 10;
// Pushes the border outward along each vertex's own surface normal so it
// doesn't z-fight the coplanar fill mesh it shares vertex positions with —
// the same "slightly larger radius per layer" idiom already used to keep
// sectors above the globe surface and markers above sectors.
const SECTOR_BOUNDARY_DEPTH_BIAS = 0.004;
// The (longitudeSegments+1)x(latitudeSegments+1) grid never varies per
// sector, so its perimeter walk is computed once and shared by every sector.
const SECTOR_BOUNDARY_LOOP_INDICES = buildSectorBoundaryLoopIndices(
  SECTOR_LONGITUDE_SEGMENTS,
  SECTOR_LATITUDE_SEGMENTS,
);

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
        SECTOR_LONGITUDE_SEGMENTS,
        SECTOR_LATITUDE_SEGMENTS,
        wrapIndex,
      ),
      atlas: buildProjectedBoundsMeshes(
        bounds,
        view,
        world,
        1,
        SECTOR_LONGITUDE_SEGMENTS,
        SECTOR_LATITUDE_SEGMENTS,
        wrapIndex,
      ),
    };
  }, [
    sector.crosses_antimeridian,
    sector.east_degrees,
    sector.north_degrees,
    sector.south_degrees,
    sector.west_degrees,
    // Pitch no longer affects vertex data at all (see globeCameraPose) —
    // only yaw and world dimensions change the projected grid.
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
  // the materials once instead of on every opacity tick, or every sector on
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
  // The reference (SVG/DOM) atlas renderer draws sectors with a border; the
  // accelerated globe previously drew a plain borderless fill, so the two
  // read as different-looking boxes wherever both were briefly visible. The
  // reference renderer now hides its own sector rect once acceleration is
  // healthy (matching how it already treats every other duplicated vector
  // layer), so this border carries that look through the whole unfurl —
  // including deep in World/globe posture, where the reference renderer
  // never drew a sector at all.
  const borderMaterialState = useMemo(() => {
    const opacityNode = uniform(1);
    const material = new LineBasicNodeMaterial({
      color: 0xd57824,
      depthWrite: false,
      transparent: true,
    });
    material.opacityNode = opacityNode;
    return { material, opacityNode };
  }, []);
  const borderMaterial = borderMaterialState.material;
  useLayoutEffect(() => {
    material.opacity = 0.18 * opacity;
    materialState.opacityNode.value = 0.18 * opacity;
    borderMaterial.opacity = opacity;
    borderMaterialState.opacityNode.value = opacity;
  }, [borderMaterial, borderMaterialState, material, materialState, opacity]);
  useEffect(
    () => () => {
      material.dispose();
      borderMaterial.dispose();
    },
    [borderMaterial, material],
  );
  return meshes.flatMap((mesh, index) => [
    <mesh material={material} name={`${name}:${index}`} key={`${index}:fill`}>
      <ProjectedMeshGeometry data={mesh} />
    </mesh>,
    <SectorBoundary
      key={`${index}:border`}
      material={borderMaterial}
      mesh={mesh}
      name={`${name}:${index}:border`}
    />,
  ]);
}

function SectorBoundary({
  material,
  mesh,
  name,
}: {
  material: LineBasicNodeMaterial;
  mesh: ProjectedGlobeMesh;
  name: string;
}) {
  const positions = useMemo(
    () => extractSectorBoundaryLineSegments(mesh),
    [mesh],
  );
  const geometry = useSectorBoundaryLineGeometry(positions);
  return <lineSegments geometry={geometry} material={material} name={name} />;
}

function useSectorBoundaryLineGeometry(positions: Float32Array) {
  const resources = useMemo(() => {
    const buffer = new Float32Array(positions.length);
    const attribute = new BufferAttribute(buffer, 3);
    const geometry = new BufferGeometry();
    geometry.setAttribute("position", attribute);
    return { attribute, buffer, geometry };
  }, [positions.length]);
  useLayoutEffect(() => {
    resources.buffer.set(positions);
    resources.attribute.needsUpdate = true;
    resources.geometry.computeBoundingSphere();
  }, [positions, resources]);
  useEffect(() => () => resources.geometry.dispose(), [resources]);
  return resources.geometry;
}

/** Walks a (longitudeSegments+1)x(latitudeSegments+1) grid's outer ring. */
function buildSectorBoundaryLoopIndices(
  longitudeSegments: number,
  latitudeSegments: number,
): readonly number[] {
  const columns = longitudeSegments + 1;
  const indices: number[] = [];
  for (let column = 0; column <= longitudeSegments; column += 1)
    indices.push(column);
  for (let row = 1; row <= latitudeSegments; row += 1)
    indices.push(row * columns + longitudeSegments);
  for (let column = longitudeSegments - 1; column >= 0; column -= 1)
    indices.push(latitudeSegments * columns + column);
  for (let row = latitudeSegments - 1; row >= 1; row -= 1)
    indices.push(row * columns);
  return indices;
}

/** Extracts one closed LineSegments loop from an interpolated sector mesh's own boundary vertices. */
function extractSectorBoundaryLineSegments(
  mesh: ProjectedGlobeMesh,
): Float32Array {
  const loop = SECTOR_BOUNDARY_LOOP_INDICES;
  const segments = new Float32Array(loop.length * 2 * 3);
  const writePoint = (vertexIndex: number, offset: number) => {
    const base = vertexIndex * 3;
    segments[offset] =
      mesh.positions[base]! + mesh.normals[base]! * SECTOR_BOUNDARY_DEPTH_BIAS;
    segments[offset + 1] =
      mesh.positions[base + 1]! +
      mesh.normals[base + 1]! * SECTOR_BOUNDARY_DEPTH_BIAS;
    segments[offset + 2] =
      mesh.positions[base + 2]! +
      mesh.normals[base + 2]! * SECTOR_BOUNDARY_DEPTH_BIAS;
  };
  for (let index = 0; index < loop.length; index += 1) {
    writePoint(loop[index]!, index * 6);
    writePoint(loop[(index + 1) % loop.length]!, index * 6 + 3);
  }
  return segments;
}
