import { BufferAttribute, BufferGeometry } from "three/src/Three.WebGPU.js";
import { useEffect, useLayoutEffect, useMemo } from "react";
import type { ProjectedGlobeMesh } from "../globe-projection";

export function ProjectedMeshGeometry({ data }: { data: ProjectedGlobeMesh }) {
  const geometry = useProjectedMeshGeometry(data);
  return <primitive attach="geometry" object={geometry} />;
}

export function useProjectedMeshGeometry(
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
