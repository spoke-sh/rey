import { extend, useThree } from "@react-three/fiber";
import {
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DirectionalLight,
  Group,
  InstancedMesh,
  LineSegments,
  LineBasicNodeMaterial,
  Mesh,
  OrthographicCamera,
  RingGeometry,
} from "three/src/Three.WebGPU.js";
import { useLayoutEffect, useRef } from "react";

// Rey's Three.js graph comes from the modular WebGPU entry point rather than
// the default `three` package R3F's own catalog is built from, so every
// intrinsic used anywhere in `../scenes` must be re-registered here. This
// module is imported by both `terrain-scene.tsx` and `globe-scene.tsx` for
// the real, consumed `ReyOrthographicCamera` binding, so this registration
// can never be dropped by tree-shaking under the package's
// `"sideEffects": false` the way a standalone side-effect-only module could.
extend({
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  CircleGeometry,
  DirectionalLight,
  Group,
  InstancedMesh,
  LineSegments,
  LineBasicNodeMaterial,
  Mesh,
  OrthographicCamera,
  RingGeometry,
});

export function ReyOrthographicCamera({
  bottom,
  far,
  left,
  position,
  right,
  rotation,
  target,
  top,
}: {
  bottom: number;
  far: number;
  left: number;
  position: readonly [number, number, number];
  right: number;
  rotation: readonly [number, number, number];
  target?: readonly [number, number, number];
  top: number;
}) {
  const cameraRef = useRef<OrthographicCamera>(null);
  const get = useThree((state) => state.get);
  const set = useThree((state) => state.set);
  useLayoutEffect(() => {
    const camera = cameraRef.current;
    if (!camera) return;
    const previous = get().camera;
    (camera as OrthographicCamera & { manual?: boolean }).manual = true;
    if (target) camera.lookAt(target[0], target[1], target[2]);
    camera.updateProjectionMatrix();
    set({ camera });
    return () => set({ camera: previous });
  }, [bottom, far, get, left, position, right, rotation, set, target, top]);
  return (
    <orthographicCamera
      bottom={bottom}
      far={far}
      left={left}
      near={0.1}
      position={position}
      ref={cameraRef}
      right={right}
      rotation={rotation}
      top={top}
    />
  );
}
