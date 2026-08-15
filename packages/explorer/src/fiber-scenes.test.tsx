import { create } from "@react-three/test-renderer";
import {
  AdditiveBlending,
  BackSide,
  Color,
  DirectionalLight,
  type InstancedBufferAttribute,
  type InstancedMesh,
  Matrix4,
  type Mesh,
  type MeshBasicNodeMaterial,
  type MeshStandardNodeMaterial,
  NotEqualStencilFunc,
} from "three/src/Three.WebGPU.js";
import { describe, expect, it } from "vitest";
import { ContextGlobeScene, ContinuousReliefScene } from "./fiber-scenes";
import {
  globeAtlasRepeatOpacity,
  globeAtlasRepeatOffset,
  globeAtlasWidth,
  globeAtmosphereRepeatOpacity,
  projectGlobeAtlasRepeatCoordinate,
} from "./globe-projection";
import {
  globeFixture,
  terrainFieldFixture,
  terrainRenderPassFixture,
} from "./test-fixtures";
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

  it("applies the bounded terrain model and orbit camera declaratively", async () => {
    const fields = terrainFieldFixture();
    const renderer = await create(
      <ContinuousReliefScene
        compiled={compileContinuousRelief([fields])}
        view={{
          world_width: 1500,
          world_height: 1000,
          viewport_width: 900,
          viewport_height: 600,
          rendered_scale: 2,
          pan_x: 0,
          pan_y: 0,
          pitch_degrees: 35.26439,
          yaw_degrees: 45,
          model_transform: {
            scale_x: 0.25,
            scale_z: 0.2,
            translate_x: 120,
            translate_z: 160,
            elevation_scale: 0.5,
          },
        }}
        world={{ width: 1500, height: 1000 }}
      />,
    );
    const terrain = renderer.scene.findByProps({
      name: "rey-continuous-relief",
    }).instance;
    expect(terrain.position.toArray()).toEqual([120, 0, 160]);
    expect(terrain.scale.toArray()).toEqual([0.25, 0.5, 0.2]);
    const camera = renderer.scene.findByType("OrthographicCamera").instance;
    expect(camera.position.x).not.toBe(750);
    expect(camera.position.z).not.toBe(500);
    expect(camera.rotation.x).not.toBeCloseTo(-Math.PI / 2);
    await renderer.unmount();
  });

  it("attaches validity, draped vectors, and selection to the terrain transform", async () => {
    const fields = terrainFieldFixture();
    const passes = terrainRenderPassFixture();
    const renderer = await create(
      <ContinuousReliefScene
        compiled={compileContinuousRelief([fields], 64 * 1024 * 1024, passes)}
        view={{
          world_width: 1500,
          world_height: 1000,
          viewport_width: 900,
          viewport_height: 600,
          rendered_scale: 2,
          pan_x: 0,
          pan_y: 0,
          model_transform: {
            scale_x: 0.5,
            scale_z: 0.4,
            translate_x: 30,
            translate_z: 40,
            elevation_scale: 0.7,
          },
        }}
        world={{ width: 1500, height: 1000 }}
      />,
    );
    const terrain = renderer.scene.findByProps({
      name: "rey-continuous-relief",
    });
    expect(terrain.instance.position.toArray()).toEqual([30, 0, 40]);
    expect(terrain.instance.scale.toArray()).toEqual([0.5, 0.7, 0.4]);
    expect(
      terrain.instance.getObjectByName("terrain-pass:validity_background"),
    ).toBeDefined();
    expect(
      terrain.instance.getObjectByName("terrain-pass:contours:contour:fixture"),
    ).toBeDefined();
    expect(
      terrain.instance.getObjectByName(
        "terrain-pass:features_labels_selection:selection:fixture",
      ),
    ).toBeDefined();
    expect(
      terrain.instance.getObjectByName(`terrain-passes:${passes.pass_set_id}`)
        ?.parent,
    ).toBe(terrain.instance);
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
    expect(globeKeyLight.color.getHex()).toBe(0xfffef8);
    expect(globeKeyLight.intensity).toBe(1.85);
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
    const retainedSurface = renderer.scene.findByProps({
      name: "context-globe-surface",
    }).instance as Mesh;
    const retainedAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0",
    }).instance as Mesh;
    expect(retainedSurface.visible).toBe(false);
    expect(retainedAtmosphere.visible).toBe(false);
    expect(retainedAtmosphere.geometry).toBe(retainedSurface.geometry);
    expect(
      (retainedAtmosphere.material as MeshBasicNodeMaterial).positionNode,
    ).not.toBeNull();
    expect((retainedAtmosphere.material as MeshBasicNodeMaterial).opacity).toBe(
      0,
    );

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
    const repeatedGlow = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0:wrap:1",
    }).instance as Mesh;
    const repeatedGlowLeft = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0:wrap:-1",
    }).instance as Mesh;
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
    expect(rightGradient.count).toBe(repeated.instanceMatrix.count);
    expect(leftGradient.count).toBe(repeatedLeft.instanceMatrix.count);
    expect(repeated.count).toBeGreaterThan(0);
    expect(repeated.count).toBeLessThan(rightGradient.count);
    expect(repeatedLeft.count).toBeGreaterThan(0);
    expect(repeatedLeft.count).toBeLessThan(leftGradient.count);
    expect(Math.min(...rightGradient.array)).toBeLessThan(0.01);
    expect(Math.max(...rightGradient.array)).toBeGreaterThan(0.99);
    expect(Math.min(...leftGradient.array)).toBeLessThan(0.01);
    expect(Math.max(...leftGradient.array)).toBeGreaterThan(0.99);
    for (let index = 1; index < rightGradient.count; index += 1) {
      expect(rightGradient.getX(index)).toBeLessThanOrEqual(
        rightGradient.getX(index - 1),
      );
      expect(leftGradient.getX(index)).toBeLessThanOrEqual(
        leftGradient.getX(index - 1),
      );
    }
    const repeatedProjectionCache = repeated.userData
      .reyRepeatProjectionCache as {
      sourceIndexes: Uint32Array;
    };
    const seamIndex = 0;
    const outerIndex = repeated.count - 1;
    const repeatedMatrix = new Matrix4();
    repeated.getMatrixAt(outerIndex, repeatedMatrix);
    const outerSample =
      compiled.sample_buckets[0]!.samples[
        repeatedProjectionCache.sourceIndexes[outerIndex]!
      ]!;
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
    repeated.getMatrixAt(seamIndex, repeatedMatrix);
    const seamSample =
      compiled.sample_buckets[0]!.samples[
        repeatedProjectionCache.sourceIndexes[seamIndex]!
      ]!;
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
    expect(repeatedGlow.parent?.position.x).toBeCloseTo(
      globeAtlasRepeatOffset(world, progress, 1),
    );
    expect(repeatedGlowLeft.parent?.position.x).toBeCloseTo(
      globeAtlasRepeatOffset(world, progress, -1),
    );
    expect(repeatedGlow.geometry).toBe(surface.geometry);
    expect((repeatedGlow.material as MeshBasicNodeMaterial).side).toBe(
      BackSide,
    );

    const repeatedMaterial = repeated.material;
    const repeatedCount = repeated.count;
    await renderer.update(
      <ContextGlobeScene
        compiled={compiled}
        view={{ ...view, projection_morph_progress: 0.84 }}
        world={world}
      />,
    );
    const updatedRepeat = renderer.scene.findByProps({
      name: "context-globe-samples:0:wrap:1",
    }).instance as InstancedMesh;
    expect(updatedRepeat.material).toBe(repeatedMaterial);
    expect(updatedRepeat.userData.reyRepeatProjectionCache).toBe(
      repeatedProjectionCache,
    );
    expect(updatedRepeat.count).toBeGreaterThan(repeatedCount);

    await renderer.unmount();
  });

  it("keeps atmosphere presentation stable when traversal reverses", async () => {
    const compiled = compileContextGlobe(globeFixture());
    const renderer = await create(
      <ContextGlobeScene
        compiled={compiled}
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
    const atmosphereMask = renderer.scene.findByProps({
      name: "context-globe-atmosphere-mask",
    }).instance as Mesh;
    const warmAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:1",
    }).instance as Mesh;
    const outerAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:2",
    }).instance as Mesh;
    const atmosphereGeometry = atmosphere.geometry;
    const atmosphereMaterial = atmosphere.material;
    const outerAtmosphereMaterial = outerAtmosphere.material;
    const surface = renderer.scene.findByProps({
      name: "context-globe-surface",
    }).instance as Mesh;
    const surfaceMaterial = surface.material as MeshStandardNodeMaterial;
    expect(surfaceMaterial.opacity).toBeCloseTo(0.25);
    expect(surfaceMaterial.transparent).toBe(true);
    expect(surfaceMaterial.depthWrite).toBe(false);
    expect(atmosphere.geometry).toBe(surface.geometry);
    expect(atmosphere.geometry.getAttribute("reySphereNormal").count).toBe(
      surface.geometry.getAttribute("position").count,
    );
    expect(
      (atmosphere.material as MeshBasicNodeMaterial).positionNode,
    ).not.toBeNull();
    expect(
      (atmosphere.material as MeshBasicNodeMaterial).opacityNode,
    ).not.toBeNull();
    expect((atmosphere.material as MeshBasicNodeMaterial).side).toBe(BackSide);
    expect((atmosphere.material as MeshBasicNodeMaterial).blending).toBe(
      AdditiveBlending,
    );
    expect((atmosphere.material as MeshBasicNodeMaterial).stencilFunc).toBe(
      NotEqualStencilFunc,
    );
    expect((atmosphereMask.material as MeshBasicNodeMaterial).colorWrite).toBe(
      false,
    );
    expect(
      (atmosphereMask.material as MeshBasicNodeMaterial).stencilWrite,
    ).toBe(true);
    expect(
      (warmAtmosphere.material as MeshBasicNodeMaterial).color.getHex(),
    ).toBe(0xfff0bd);
    expect((atmosphere.material as MeshBasicNodeMaterial).color.getHex()).toBe(
      0xffd977,
    );
    expect(
      (outerAtmosphere.material as MeshBasicNodeMaterial).color.getHex(),
    ).toBe(0xf4dfa5);
    expect((atmosphere.material as MeshBasicNodeMaterial).opacity).toBeCloseTo(
      0.17 * 0.25,
    );

    await renderer.update(
      <ContextGlobeScene
        compiled={compiled}
        view={{
          yaw_degrees: 0,
          pitch_degrees: 0,
          projection_morph_progress: 0.55,
        }}
        world={{ width: 1200, height: 720 }}
      />,
    );
    const updatedAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0",
    }).instance as Mesh;
    expect(updatedAtmosphere.geometry).toBe(atmosphereGeometry);
    expect(updatedAtmosphere.material).toBe(atmosphereMaterial);
    expect(
      (updatedAtmosphere.material as MeshBasicNodeMaterial).opacity,
    ).toBeCloseTo(0.17 * 0.42525 ** 2);

    await renderer.update(
      <ContextGlobeScene
        compiled={compiled}
        view={{
          yaw_degrees: 0,
          pitch_degrees: 0,
          projection_morph_progress: 0.5,
        }}
        world={{ width: 1200, height: 720 }}
      />,
    );
    const closingAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0",
    }).instance as Mesh;
    expect(closingAtmosphere.geometry).toBe(atmosphereGeometry);
    expect(closingAtmosphere.material).toBe(atmosphereMaterial);
    expect(
      (closingAtmosphere.material as MeshBasicNodeMaterial).opacity,
    ).toBeCloseTo(0.17 * 0.25);
    const closingOuterAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:2",
    }).instance as Mesh;
    expect(closingOuterAtmosphere.material).toBe(outerAtmosphereMaterial);
    expect(
      (closingOuterAtmosphere.material as MeshBasicNodeMaterial).opacity,
    ).toBeCloseTo(0.09 * 0.25);

    await renderer.update(
      <ContextGlobeScene
        compiled={compiled}
        view={{
          yaw_degrees: 0,
          pitch_degrees: 0,
          projection_morph_progress: 1,
        }}
        world={{ width: 1200, height: 720 }}
      />,
    );
    await renderer.update(
      <ContextGlobeScene
        compiled={compiled}
        view={{
          yaw_degrees: 0,
          pitch_degrees: 0,
          projection_morph_progress: 0.79,
        }}
        world={{ width: 1200, height: 720 }}
      />,
    );
    const closingRepeatedAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0:wrap:1",
    }).instance as Mesh;
    expect(closingRepeatedAtmosphere.geometry).toBe(atmosphereGeometry);
    expect(
      (closingRepeatedAtmosphere.material as MeshBasicNodeMaterial).opacity,
    ).toBeCloseTo(0.17 * globeAtmosphereRepeatOpacity(0.79));

    await renderer.unmount();
  });
});
