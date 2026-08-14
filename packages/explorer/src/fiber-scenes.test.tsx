import { create } from "@react-three/test-renderer";
import {
  Color,
  DirectionalLight,
  type InstancedBufferAttribute,
  type InstancedMesh,
  Matrix4,
  type Mesh,
  type MeshBasicNodeMaterial,
  type MeshStandardNodeMaterial,
  type SphereGeometry,
} from "three/src/Three.WebGPU.js";
import { describe, expect, it } from "vitest";
import { ContextGlobeScene, ContinuousReliefScene } from "./fiber-scenes";
import {
  globeAtlasRepeatOpacity,
  globeAtlasRepeatOffset,
  globeAtlasWidth,
  projectGlobeAtlasRepeatCoordinate,
} from "./globe-projection";
import { globeFixture, terrainFieldFixture } from "./test-fixtures";
import { compileContextGlobe, GLOBE_RADIUS } from "./three-globe";
import { compileContinuousRelief } from "./three-terrain";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

describe("declarative React Three Fiber scenes", () => {
  it("materializes valid terrain buffers, lighting, and the bounded camera", async () => {
    const fields = terrainFieldFixture();
    const compiled = compileContinuousRelief([fields]);
    const renderer = await create(
      <ContinuousReliefScene
        compiled={compiled}
        view={{
          world_width: 1500,
          world_height: 1000,
          viewport_width: 900,
          viewport_height: 600,
          rendered_scale: 2,
          pan_x: 100,
          pan_y: -40,
        }}
        world={{ width: 1500, height: 1000 }}
      />,
    );

    const terrain = renderer.scene.findByProps({
      name: "rey-continuous-relief",
    });
    const mesh = renderer.scene.findByProps({ name: fields.field_set_id });
    const meshInstance = mesh.instance as Mesh;
    expect(terrain.children).toHaveLength(1);
    expect(meshInstance.type).toBe("Mesh");
    expect(meshInstance.geometry.getAttribute("reyTint").count).toBe(
      fields.field_cells,
    );
    expect(meshInstance.geometry.index?.count).toBeGreaterThan(0);
    const lights = renderer.scene.findAllByType("DirectionalLight");
    expect(lights).toHaveLength(2);
    const keyLight = lights[0]?.instance as DirectionalLight;
    expect(keyLight).toBeInstanceOf(DirectionalLight);
    expect(keyLight.color).toBeInstanceOf(Color);
    expect(keyLight.color.getHex()).toBe(0xfff4d4);
    const camera = renderer.scene.findByType("OrthographicCamera").instance;
    expect(camera.position.toArray()).toEqual([700, 2625, 520]);
    expect(camera.rotation.x).toBeCloseTo(-Math.PI / 2);

    await renderer.unmount();
  });

  it("keeps globe evidence identities in named declarative objects", async () => {
    const globe = globeFixture();
    const renderer = await create(
      <ContextGlobeScene
        compiled={compileContextGlobe(globe)}
        view={{ yaw_degrees: 24, pitch_degrees: -8 }}
        world={{ width: 1200, height: 720 }}
      />,
    );

    const root = renderer.scene.findByProps({
      name: "context-globe:working:fixture",
    });
    expect(root.instance.quaternion.y).toBe(0);
    const surface = renderer.scene.findByProps({
      name: "context-globe-surface",
    }).instance as Mesh;
    expect(surface.geometry.getAttribute("position").count).toBe(161 * 97);
    expect(
      renderer.scene.findByProps({
        name: "context-globe-sector:sector:fixture:0",
      }),
    ).toBeDefined();
    expect(
      renderer.scene.findByProps({ name: "workload-beacon:survey" }),
    ).toBeDefined();
    expect(
      renderer.scene.findByProps({ name: "context-globe-atmosphere:2" }),
    ).toBeDefined();
    const northPole = renderer.scene.findByProps({
      name: "context-globe-pole-pattern:north",
    });
    const northPoleMaterial = (northPole.instance as InstancedMesh)
      .material as MeshBasicNodeMaterial;
    expect(northPoleMaterial.color.getHex()).toBe(0x243b38);
    expect(northPoleMaterial.opacity).toBe(0.88);
    expect(
      renderer.scene.findByProps({ name: "context-globe-pole-pattern:south" }),
    ).toBeDefined();
    expect(
      renderer.scene.findAll(
        ({ props }) =>
          typeof props.name === "string" &&
          props.name.startsWith("context-globe-samples:"),
      ).length,
    ).toBeGreaterThan(0);
    const globeLights = renderer.scene.findAllByType("DirectionalLight");
    expect(globeLights).toHaveLength(2);
    const globeKeyLight = globeLights[0]?.instance as DirectionalLight;
    expect(globeKeyLight.color).toBeInstanceOf(Color);
    expect(globeKeyLight.color.getHex()).toBe(0xfff4d2);
    const sampleField = renderer.scene.findByProps({
      name: "context-globe-samples:0",
    }).instance as InstancedMesh;
    expect(sampleField.count).toBeGreaterThan(0);
    expect((sampleField.material as MeshBasicNodeMaterial).color.getHex()).toBe(
      0x708079,
    );

    await renderer.unmount();
  });

  it("subdues spherical fabric at the Mercator endpoint", async () => {
    const world = { width: 1200, height: 720 };
    const renderer = await create(
      <ContextGlobeScene
        compiled={compileContextGlobe(globeFixture())}
        view={{
          yaw_degrees: 24,
          pitch_degrees: -8,
          projection_morph_progress: 1,
        }}
        world={world}
      />,
    );

    const sampleField = renderer.scene.findByProps({
      name: "context-globe-samples:0",
    }).instance as InstancedMesh;
    const poleField = renderer.scene.findByProps({
      name: "context-globe-pole-pattern:north",
    }).instance as InstancedMesh;
    expect((sampleField.material as MeshBasicNodeMaterial).opacity).toBeCloseTo(
      0.48 * 0.36,
    );
    expect((poleField.material as MeshBasicNodeMaterial).opacity).toBe(0);
    const wraps = renderer.scene.findAll(
      ({ props }) =>
        typeof props.name === "string" &&
        props.name.startsWith("context-globe-atlas-wrap:"),
    );
    expect(wraps).toHaveLength(3);
    expect(wraps.map(({ instance }) => instance.position.x)).toEqual([
      -globeAtlasWidth(world),
      0,
      globeAtlasWidth(world),
    ]);
    expect(
      renderer.scene.findByProps({
        name: "context-globe-samples:0:wrap:-1",
      }),
    ).toBeDefined();
    expect(
      renderer.scene.findByProps({
        name: "context-globe-samples:0:wrap:1",
      }),
    ).toBeDefined();
    expect(
      renderer.scene.findAll(
        ({ props }) => props.name === "context-globe-surface",
      ),
    ).toHaveLength(0);
    expect(
      renderer.scene.findAll(
        ({ props }) =>
          typeof props.name === "string" &&
          props.name.startsWith("context-globe-atmosphere:"),
      ),
    ).toHaveLength(0);

    await renderer.unmount();
  });

  it("dissolves repeated Atlas fabric through intermediate morph frames", async () => {
    const progress = 0.79;
    const world = { width: 1200, height: 720 };
    const view = {
      yaw_degrees: 24,
      pitch_degrees: -8,
      projection_morph_progress: progress,
    };
    const compiled = compileContextGlobe(globeFixture());
    const renderer = await create(
      <ContextGlobeScene compiled={compiled} view={view} world={world} />,
    );

    const canonical = renderer.scene.findByProps({
      name: "context-globe-samples:0",
    }).instance as InstancedMesh;
    const repeated = renderer.scene.findByProps({
      name: "context-globe-samples:0:wrap:1",
    }).instance as InstancedMesh;
    const repeatedLeft = renderer.scene.findByProps({
      name: "context-globe-samples:0:wrap:-1",
    }).instance as InstancedMesh;
    const repeatedAtlas = renderer.scene.findByProps({
      name: "context-globe-atlas-wrap:1",
    }).instance;
    const postureOpacity = 0.48 * (1 - progress * (1 - 0.36));
    expect((canonical.material as MeshBasicNodeMaterial).opacity).toBeCloseTo(
      postureOpacity,
    );
    expect((repeated.material as MeshBasicNodeMaterial).opacity).toBeCloseTo(
      postureOpacity,
    );
    expect(
      (repeated.material as MeshBasicNodeMaterial).opacityNode,
    ).not.toBeNull();
    const rightGradient = repeated.geometry.getAttribute(
      "reyRepeatSeamWeight",
    ) as InstancedBufferAttribute;
    const leftGradient = repeatedLeft.geometry.getAttribute(
      "reyRepeatSeamWeight",
    ) as InstancedBufferAttribute;
    expect(rightGradient.count).toBe(repeated.count);
    expect(leftGradient.count).toBe(repeatedLeft.count);
    expect(Math.min(...rightGradient.array)).toBeLessThan(0.01);
    expect(Math.max(...rightGradient.array)).toBeGreaterThan(0.99);
    for (const index of [0, 17, 101, rightGradient.count - 1]) {
      expect(rightGradient.getX(index) + leftGradient.getX(index)).toBeCloseTo(
        1,
      );
    }
    let outerIndex = 0;
    let seamIndex = 0;
    for (let index = 1; index < rightGradient.count; index += 1) {
      if (rightGradient.getX(index) < rightGradient.getX(outerIndex))
        outerIndex = index;
      if (rightGradient.getX(index) > rightGradient.getX(seamIndex))
        seamIndex = index;
    }
    const canonicalMatrix = new Matrix4();
    const repeatedMatrix = new Matrix4();
    canonical.getMatrixAt(outerIndex, canonicalMatrix);
    repeated.getMatrixAt(outerIndex, repeatedMatrix);
    const outerSample = compiled.sample_buckets[0]!.samples[outerIndex]!;
    const expectedOuter = projectGlobeAtlasRepeatCoordinate(
      outerSample.longitude_degrees,
      outerSample.latitude_degrees,
      view,
      world,
      progress,
      1,
      GLOBE_RADIUS * 1.005,
      0.008,
    );
    expect(repeatedMatrix.elements[12]).toBeCloseTo(
      expectedOuter.position[0],
      5,
    );
    expect(repeatedMatrix.elements[13]).toBeCloseTo(
      expectedOuter.position[1],
      5,
    );
    expect(repeatedMatrix.elements[14]).toBeCloseTo(
      expectedOuter.position[2],
      5,
    );
    canonical.getMatrixAt(seamIndex, canonicalMatrix);
    repeated.getMatrixAt(seamIndex, repeatedMatrix);
    const seamSample = compiled.sample_buckets[0]!.samples[seamIndex]!;
    const expectedSeam = projectGlobeAtlasRepeatCoordinate(
      seamSample.longitude_degrees,
      seamSample.latitude_degrees,
      view,
      world,
      progress,
      1,
      GLOBE_RADIUS * 1.005,
      0.008,
    );
    expect(repeatedMatrix.elements[12]).toBeCloseTo(
      expectedSeam.position[0],
      5,
    );
    expect(repeatedMatrix.elements[13]).toBeCloseTo(
      expectedSeam.position[1],
      5,
    );
    expect(repeatedMatrix.elements[14]).toBeCloseTo(
      expectedSeam.position[2],
      5,
    );
    const surface = renderer.scene.findByProps({
      name: "context-globe-surface",
    }).instance as Mesh;
    expect((surface.material as MeshStandardNodeMaterial).depthWrite).toBe(
      true,
    );
    expect(surface.renderOrder).toBe(-1);
    expect(repeated.parent?.position.x).toBeCloseTo(
      globeAtlasRepeatOffset(world, progress, 1),
    );
    expect(repeatedAtlas.position.x).toBeCloseTo(
      globeAtlasRepeatOffset(world, progress, 1),
    );

    await renderer.unmount();
  });

  it("contracts atmosphere shells while fading them ahead of the globe-to-map morph", async () => {
    const renderer = await create(
      <ContextGlobeScene
        compiled={compileContextGlobe(globeFixture())}
        view={{
          yaw_degrees: 0,
          pitch_degrees: 0,
          projection_morph_progress: 0.5,
        }}
        world={{ width: 1200, height: 720 }}
      />,
    );

    const atmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0",
    }).instance as Mesh;
    const surface = renderer.scene.findByProps({
      name: "context-globe-surface",
    }).instance as Mesh;
    const surfaceMaterial = surface.material as MeshStandardNodeMaterial;
    expect(surfaceMaterial.opacity).toBeCloseTo(0.25);
    expect(surfaceMaterial.transparent).toBe(true);
    expect(surfaceMaterial.depthWrite).toBe(false);
    expect(
      (atmosphere.geometry as SphereGeometry).parameters.radius,
    ).toBeCloseTo(GLOBE_RADIUS * 1.018 * 0.5);
    expect((atmosphere.material as MeshBasicNodeMaterial).opacity).toBeCloseTo(
      0.12 * 0.03125,
    );

    await renderer.unmount();
  });
});
