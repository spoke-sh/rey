import {
  AmbientLight,
  CircleGeometry,
  DirectionalLight,
  Group,
  InstancedMesh,
  Matrix4,
  Mesh,
  MeshBasicNodeMaterial,
  MeshStandardNodeMaterial,
  OrthographicCamera,
  Quaternion,
  RingGeometry,
  Scene,
  SphereGeometry,
  Vector3,
  type BufferGeometry,
  type Material,
} from "three/webgpu";
import type { GlobeCameraView } from "../engine/camera";
import type { TopologyGlobe } from "../../topology";
import type { ThreeTerrainBundle } from "./three-terrain";
import { contextGlobeSamples } from "./globe-samples";

export const SEMANTIC_GLOBE_MATERIAL_REVISION =
  "rey.semantic-globe.tsl-stippled-atmosphere@2";

const GLOBE_RADIUS = 1.72;
const SAMPLE_RADIUS = 0.0082;
const SAMPLE_COUNT = 26_000;
const SURFACE_NORMAL = new Vector3(0, 0, 1);
const X_AXIS = new Vector3(1, 0, 0);
const Y_AXIS = new Vector3(0, 1, 0);

export function createContextGlobeBundle(
  globe: TopologyGlobe,
  world: { width: number; height: number },
  initialView: GlobeCameraView = { yaw_degrees: 0, pitch_degrees: 0 },
): ThreeTerrainBundle {
  const scene = new Scene();
  const globeGroup = new Group();
  globeGroup.name = `context-globe:${globe.source_revision}`;
  applyGlobeView(globeGroup, initialView);
  scene.add(globeGroup);

  const geometries: BufferGeometry[] = [];
  const materials: Material[] = [];
  const sphereGeometry = new SphereGeometry(GLOBE_RADIUS, 160, 96);
  const sphereMaterial = new MeshStandardNodeMaterial();
  sphereMaterial.name = SEMANTIC_GLOBE_MATERIAL_REVISION;
  sphereMaterial.color.set(0xe8e9df);
  sphereMaterial.roughness = 0.98;
  sphereMaterial.metalness = 0;
  geometries.push(sphereGeometry);
  materials.push(sphereMaterial);
  globeGroup.add(new Mesh(sphereGeometry, sphereMaterial));

  addAtmosphere(globeGroup, geometries, materials);

  const samples = contextGlobeSamples(
    globe.source_revision,
    SAMPLE_COUNT,
    globe.regions,
  );
  const sampleTriangleCount = addSampleField(
    globeGroup,
    samples,
    geometries,
    materials,
  );

  let markerTriangles = 0;
  for (const region of globe.regions) {
    const radius =
      0.026 + Math.min(0.056, region.angular_radius_degrees / 2_200);
    markerTriangles += addSurfaceMarker(
      globeGroup,
      `semantic-region:${region.id}`,
      region.longitude_degrees,
      region.latitude_degrees,
      radius,
      region.tone === "frontier"
        ? 0xd6a94d
        : region.tone === "omitted"
          ? 0xa87862
          : 0x446c61,
      geometries,
      materials,
    );
  }

  for (const beacon of globe.beacons) {
    const radius = beacon.mapping_role === "survey" ? 0.065 : 0.048;
    const color =
      beacon.state === "admitted"
        ? 0x3b7458
        : beacon.state === "index"
          ? 0xb28a25
          : beacon.state === "request"
            ? 0x658593
            : 0xd57824;
    markerTriangles += addSurfaceMarker(
      globeGroup,
      `workload-beacon:${beacon.workload_id}`,
      beacon.longitude_degrees,
      beacon.latitude_degrees,
      radius,
      color,
      geometries,
      materials,
      true,
    );
  }

  scene.add(new AmbientLight(0xf4f0df, 1.72));
  const keyLight = new DirectionalLight(0xfff4d2, 3.4);
  keyLight.position.set(-3.8, 4.8, 6.2);
  scene.add(keyLight);
  const rimLight = new DirectionalLight(0x8fb6ac, 1.55);
  rimLight.position.set(4.8, 1.4, -3.8);
  scene.add(rimLight);

  const aspect = world.width / Math.max(1, world.height);
  const halfHeight = 2.12;
  const camera = new OrthographicCamera(
    -halfHeight * aspect,
    halfHeight * aspect,
    halfHeight,
    -halfHeight,
    0.1,
    100,
  );
  camera.position.set(0, 0, 6);
  camera.lookAt(0, 0, 0);
  camera.updateProjectionMatrix();

  const sphereTriangles = 160 * 96 * 2;
  return {
    scene,
    camera,
    material_revision: SEMANTIC_GLOBE_MATERIAL_REVISION,
    statistics: Object.freeze({
      field_sets: 1,
      vertices: sphereGeometry.getAttribute("position").count + samples.length,
      triangles: sphereTriangles + sampleTriangleCount + markerTriangles,
      field_bytes: samples.length * 16,
      gpu_bytes: samples.length * 16,
      gpu_budget_bytes: samples.length * 16,
    }),
    updateGlobeView(view) {
      applyGlobeView(globeGroup, view);
    },
    dispose() {
      for (const geometry of geometries) geometry.dispose();
      for (const material of materials) material.dispose();
      scene.clear();
    },
  };
}

function applyGlobeView(group: Group, view: GlobeCameraView) {
  const pitch = new Quaternion().setFromAxisAngle(
    X_AXIS,
    (view.pitch_degrees * Math.PI) / 180,
  );
  const yaw = new Quaternion().setFromAxisAngle(
    Y_AXIS,
    (view.yaw_degrees * Math.PI) / 180,
  );
  group.quaternion.copy(yaw).multiply(pitch);
}

function addAtmosphere(
  group: Group,
  geometries: BufferGeometry[],
  materials: Material[],
) {
  const layers = [
    { radius: GLOBE_RADIUS * 1.018, color: 0xf6ecd4, opacity: 0.12 },
    { radius: GLOBE_RADIUS * 1.045, color: 0xcbd8c9, opacity: 0.055 },
    { radius: GLOBE_RADIUS * 1.082, color: 0x6f9188, opacity: 0.022 },
  ];
  for (const [index, layer] of layers.entries()) {
    const geometry = new SphereGeometry(layer.radius, 112, 64);
    const material = new MeshBasicNodeMaterial({
      color: layer.color,
      opacity: layer.opacity,
      transparent: true,
      depthWrite: false,
    });
    const shell = new Mesh(geometry, material);
    shell.name = `context-globe-atmosphere:${index}`;
    geometries.push(geometry);
    materials.push(material);
    group.add(shell);
  }
}

function addSampleField(
  group: Group,
  samples: ReturnType<typeof contextGlobeSamples>,
  geometries: BufferGeometry[],
  materials: Material[],
) {
  const buckets = [
    { minimum: 0, color: 0x708079, opacity: 0.48 },
    { minimum: 0.58, color: 0x3e504c, opacity: 0.7 },
    { minimum: 0.76, color: 0x243b38, opacity: 0.88 },
  ];
  let triangles = 0;
  for (let bucketIndex = 0; bucketIndex < buckets.length; bucketIndex += 1) {
    const bucket = buckets[bucketIndex]!;
    const maximum =
      buckets[bucketIndex + 1]?.minimum ?? Number.POSITIVE_INFINITY;
    const members = samples.filter(
      (sample) =>
        sample.brightness >= bucket.minimum && sample.brightness < maximum,
    );
    if (members.length === 0) continue;
    const geometry = new CircleGeometry(SAMPLE_RADIUS, 5);
    const material = new MeshBasicNodeMaterial({
      color: bucket.color,
      opacity: bucket.opacity,
      transparent: bucket.opacity < 1,
    });
    const mesh = new InstancedMesh(geometry, material, members.length);
    mesh.name = `context-globe-samples:${bucketIndex}`;
    const matrix = new Matrix4();
    const quaternion = new Quaternion();
    const scale = new Vector3(1, 1, 1);
    for (const [index, sample] of members.entries()) {
      const position = sphericalVector(
        sample.longitude_degrees,
        sample.latitude_degrees,
        GLOBE_RADIUS * 1.005,
      );
      quaternion.setFromUnitVectors(
        SURFACE_NORMAL,
        position.clone().normalize(),
      );
      matrix.compose(position, quaternion, scale);
      mesh.setMatrixAt(index, matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
    geometries.push(geometry);
    materials.push(material);
    group.add(mesh);
    triangles += members.length * 5;
  }
  return triangles;
}

function addSurfaceMarker(
  group: Group,
  name: string,
  longitude: number,
  latitude: number,
  radius: number,
  color: number,
  geometries: BufferGeometry[],
  materials: Material[],
  halo = false,
) {
  const markerGroup = new Group();
  markerGroup.name = name;
  const position = sphericalVector(longitude, latitude, GLOBE_RADIUS * 1.013);
  markerGroup.position.copy(position);
  markerGroup.quaternion.setFromUnitVectors(
    SURFACE_NORMAL,
    position.clone().normalize(),
  );

  const pointGeometry = new CircleGeometry(radius, 24);
  const pointMaterial = new MeshBasicNodeMaterial({ color });
  geometries.push(pointGeometry);
  materials.push(pointMaterial);
  markerGroup.add(new Mesh(pointGeometry, pointMaterial));

  let triangles = 24;
  if (halo) {
    const haloGeometry = new RingGeometry(radius * 1.5, radius * 2.25, 36);
    const haloMaterial = new MeshBasicNodeMaterial({
      color,
      depthWrite: false,
      opacity: 0.42,
      transparent: true,
    });
    const haloMesh = new Mesh(haloGeometry, haloMaterial);
    haloMesh.position.z = -0.001;
    geometries.push(haloGeometry);
    materials.push(haloMaterial);
    markerGroup.add(haloMesh);
    triangles += 72;
  }
  group.add(markerGroup);
  return triangles;
}

function sphericalVector(
  longitudeDegrees: number,
  latitudeDegrees: number,
  radius: number,
) {
  const longitude = (longitudeDegrees * Math.PI) / 180;
  const latitude = (latitudeDegrees * Math.PI) / 180;
  const latitudeRadius = Math.cos(latitude) * radius;
  return new Vector3(
    Math.sin(longitude) * latitudeRadius,
    Math.sin(latitude) * radius,
    Math.cos(longitude) * latitudeRadius,
  );
}
