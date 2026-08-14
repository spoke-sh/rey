import { create } from "@react-three/test-renderer";
import {
  Color,
  DirectionalLight,
  type InstancedMesh,
  type Mesh,
  type MeshBasicNodeMaterial,
} from "three/src/Three.WebGPU.js";
import { describe, expect, it } from "vitest";
import { ContextGlobeScene, ContinuousReliefScene } from "./fiber-scenes";
import { globeFixture, terrainFieldFixture } from "./test-fixtures";
import { compileContextGlobe } from "./three-globe";
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
    const renderer = await create(
      <ContextGlobeScene
        compiled={compileContextGlobe(globeFixture())}
        view={{
          yaw_degrees: 24,
          pitch_degrees: -8,
          projection_morph_progress: 1,
        }}
        world={{ width: 1200, height: 720 }}
      />,
    );

    const sampleField = renderer.scene.findByProps({
      name: "context-globe-samples:0",
    }).instance as InstancedMesh;
    const poleField = renderer.scene.findByProps({
      name: "context-globe-pole-pattern:north",
    }).instance as InstancedMesh;
    expect((sampleField.material as MeshBasicNodeMaterial).opacity).toBeCloseTo(
      0.48 * 0.18,
    );
    expect((poleField.material as MeshBasicNodeMaterial).opacity).toBe(0);

    await renderer.unmount();
  });
});
