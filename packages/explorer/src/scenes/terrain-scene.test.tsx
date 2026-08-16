import { create } from "@react-three/test-renderer";
import { Color, DirectionalLight, type Mesh } from "three/src/Three.WebGPU.js";
import { describe, expect, it } from "vitest";
import { ContinuousReliefScene } from "./terrain-scene";
import {
  terrainFieldFixture,
  terrainRenderPassFixture,
} from "../test-fixtures";
import { compileContinuousRelief } from "../three-terrain";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

describe("terrain scene", () => {
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
      terrain.instance.getObjectByName("terrain-pass:validity_background")
        ?.type,
    ).toBe("Group");
    expect(
      terrain.instance.getObjectByName("terrain-pass:contours:contour:fixture"),
    ).toBeDefined();
    expect(
      terrain.instance.getObjectByName("terrain-pass:contours:contour:fixture")
        ?.type,
    ).toBe("LineSegments");
    expect(
      terrain.instance.getObjectByName(
        "terrain-pass:water_weather_boundary:water:fixture",
      ),
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
});
