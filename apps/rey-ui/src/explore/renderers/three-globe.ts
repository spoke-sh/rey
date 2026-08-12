import {
  AmbientLight,
  BufferGeometry,
  DirectionalLight,
  Float32BufferAttribute,
  Group,
  Line,
  LineBasicMaterial,
  Mesh,
  MeshStandardNodeMaterial,
  PerspectiveCamera,
  Scene,
  SphereGeometry,
  type Material,
} from "three/webgpu";
import type { TopologyGlobe } from "../../topology";
import type { ThreeTerrainBundle } from "./three-terrain";

export const SEMANTIC_GLOBE_MATERIAL_REVISION =
  "rey.semantic-globe.tsl-material@1";

const GLOBE_RADIUS = 1.72;

export function createContextGlobeBundle(
  globe: TopologyGlobe,
  world: { width: number; height: number },
): ThreeTerrainBundle {
  const scene = new Scene();
  const globeGroup = new Group();
  globeGroup.name = `context-globe:${globe.source_revision}`;
  scene.add(globeGroup);

  const geometries: BufferGeometry[] = [];
  const materials: Material[] = [];
  const sphereGeometry = new SphereGeometry(GLOBE_RADIUS, 128, 64);
  const sphereMaterial = new MeshStandardNodeMaterial();
  sphereMaterial.name = SEMANTIC_GLOBE_MATERIAL_REVISION;
  sphereMaterial.color.set(0x8fa58a);
  sphereMaterial.roughness = 0.94;
  sphereMaterial.metalness = 0;
  geometries.push(sphereGeometry);
  materials.push(sphereMaterial);
  globeGroup.add(new Mesh(sphereGeometry, sphereMaterial));

  const graticuleMaterial = new LineBasicMaterial({
    color: 0xdce4d3,
    opacity: 0.24,
    transparent: true,
  });
  materials.push(graticuleMaterial);
  for (let latitude = -60; latitude <= 60; latitude += 30) {
    const points = Array.from({ length: 97 }, (_, index) =>
      sphericalPoint(-180 + (360 * index) / 96, latitude, GLOBE_RADIUS * 1.002),
    );
    const geometry = lineGeometry(points);
    geometries.push(geometry);
    globeGroup.add(new Line(geometry, graticuleMaterial));
  }
  for (let longitude = -150; longitude <= 180; longitude += 30) {
    const points = Array.from({ length: 65 }, (_, index) =>
      sphericalPoint(longitude, -90 + (180 * index) / 64, GLOBE_RADIUS * 1.002),
    );
    const geometry = lineGeometry(points);
    geometries.push(geometry);
    globeGroup.add(new Line(geometry, graticuleMaterial));
  }

  for (const region of globe.regions) {
    const radius =
      0.035 + Math.min(0.075, region.angular_radius_degrees / 1800);
    const geometry = new SphereGeometry(radius, 20, 12);
    const material = new MeshStandardNodeMaterial();
    material.color.set(
      region.tone === "frontier"
        ? 0xd6a94d
        : region.tone === "omitted"
          ? 0xa87862
          : 0xe6edbd,
    );
    material.roughness = 0.82;
    const marker = new Mesh(geometry, material);
    marker.name = `semantic-region:${region.id}`;
    marker.position.set(
      ...sphericalPoint(
        region.longitude_degrees,
        region.latitude_degrees,
        GLOBE_RADIUS + radius * 0.64,
      ),
    );
    geometries.push(geometry);
    materials.push(material);
    globeGroup.add(marker);
  }

  for (const beacon of globe.beacons) {
    const radius = beacon.mapping_role === "survey" ? 0.072 : 0.052;
    const geometry = new SphereGeometry(radius, 24, 16);
    const material = new MeshStandardNodeMaterial();
    material.color.set(
      beacon.state === "admitted"
        ? 0xb7d7a8
        : beacon.state === "index"
          ? 0xe9d278
          : beacon.state === "request"
            ? 0xb5c8d2
            : 0xf2a94d,
    );
    material.emissive.set(beacon.state === "admitted" ? 0x24472f : 0x6f3d0c);
    material.emissiveIntensity = beacon.mapping_role === "survey" ? 0.72 : 0.4;
    material.roughness = 0.58;
    const marker = new Mesh(geometry, material);
    marker.name = `workload-beacon:${beacon.workload_id}`;
    marker.position.set(
      ...sphericalPoint(
        beacon.longitude_degrees,
        beacon.latitude_degrees,
        GLOBE_RADIUS + radius * 0.72,
      ),
    );
    geometries.push(geometry);
    materials.push(material);
    globeGroup.add(marker);
  }

  scene.add(new AmbientLight(0xdde4da, 1.25));
  const keyLight = new DirectionalLight(0xfff2ce, 3.1);
  keyLight.position.set(-3.8, 4.6, 5.8);
  scene.add(keyLight);
  const rimLight = new DirectionalLight(0x9ec8d0, 1.25);
  rimLight.position.set(4.5, 1.8, -3.4);
  scene.add(rimLight);

  const camera = new PerspectiveCamera(
    37,
    world.width / Math.max(1, world.height),
    0.1,
    100,
  );
  camera.position.set(0, 0.08, 5.25);
  camera.lookAt(0, 0, 0);
  camera.updateProjectionMatrix();

  const sphereTriangles = 128 * 64 * 2;
  const markerTriangles =
    globe.regions.length * 20 * 12 * 2 + globe.beacons.length * 24 * 16 * 2;
  return {
    scene,
    camera,
    material_revision: SEMANTIC_GLOBE_MATERIAL_REVISION,
    statistics: Object.freeze({
      field_sets: 0,
      vertices: sphereGeometry.getAttribute("position").count,
      triangles: sphereTriangles + markerTriangles,
      field_bytes: 0,
    }),
    dispose() {
      for (const geometry of geometries) geometry.dispose();
      for (const material of materials) material.dispose();
      scene.clear();
    },
  };
}

function sphericalPoint(
  longitudeDegrees: number,
  latitudeDegrees: number,
  radius: number,
): [number, number, number] {
  const longitude = (longitudeDegrees * Math.PI) / 180;
  const latitude = (latitudeDegrees * Math.PI) / 180;
  const latitudeRadius = Math.cos(latitude) * radius;
  return [
    Math.sin(longitude) * latitudeRadius,
    Math.sin(latitude) * radius,
    Math.cos(longitude) * latitudeRadius,
  ];
}

function lineGeometry(points: Array<[number, number, number]>): BufferGeometry {
  const geometry = new BufferGeometry();
  geometry.setAttribute(
    "position",
    new Float32BufferAttribute(
      points.flatMap((point) => point),
      3,
    ),
  );
  return geometry;
}
