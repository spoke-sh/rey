import { create } from "@react-three/test-renderer";
import {
  Color,
  DirectionalLight,
  type InstancedMesh,
  type Mesh,
  type MeshBasicNodeMaterial,
} from "three/src/Three.WebGPU.js";
import { describe, expect, it } from "vitest";
import type { TopologyGlobe } from "../../topology";
import {
  compileTerrainProgram,
  materializeTerrainWorkingSet,
} from "../terrain/compile";
import { proceduralProjection } from "../terrain/compile.test-fixture";
import { ContextGlobeScene, ContinuousReliefScene } from "./fiber-scenes";
import { compileContextGlobe } from "./three-globe";
import { compileContinuousRelief } from "./three-terrain";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

function terrainFields() {
  const program = compileTerrainProgram({
    source_id: "survey:fiber",
    source_revision: "topography:one",
    bounds: { x: 100, y: 80, width: 1300, height: 840 },
    anchors: [{ id: "workspace", x: 750, y: 500, prominence: 4 }],
    atmosphere: [],
    unresolved_pressure: 0,
    projection: proceduralProjection,
  });
  return materializeTerrainWorkingSet(program, {
    working_set_id: "fiber:fixture",
    bounds: program.bounds,
    columns: 31,
    rows: 21,
    detail_authority: "React Three Fiber fixture",
  });
}

describe("declarative React Three Fiber scenes", () => {
  it("materializes valid terrain buffers, lighting, and the bounded camera", async () => {
    const fields = terrainFields();
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
    const globe: TopologyGlobe = {
      schema: "rey.explore-orientation-globe.v1",
      posture: "orientation",
      globe_id: "orientation:fiber",
      source_revision: "working:fiber",
      compiler_revision: "orientation@1",
      coordinate_authority: "presentation only",
      clusters: [],
      regions: [],
      beacons: [
        {
          id: "workload-beacon:survey",
          focus_id: "beacon:survey",
          workload_id: "survey",
          label: "Survey context",
          detail: "WORKING",
          source: "sys/survey/workload.yaml",
          source_revision: "blake3:survey",
          producer: "codex@gpt-5",
          state: "working",
          mapping_role: "survey",
          next_step: "review and consent",
          longitude_degrees: 14,
          latitude_degrees: 6,
          tone: "attention",
        },
      ],
    };
    const renderer = await create(
      <ContextGlobeScene
        compiled={compileContextGlobe(globe)}
        view={{ yaw_degrees: 24, pitch_degrees: -8 }}
        world={{ width: 1200, height: 720 }}
      />,
    );

    const root = renderer.scene.findByProps({
      name: "context-globe:working:fiber",
    });
    expect(
      root.instance.quaternion.equals(root.instance.quaternion.clone()),
    ).toBe(true);
    expect(root.instance.quaternion.y).not.toBe(0);
    expect(
      renderer.scene.findByProps({ name: "workload-beacon:survey" }),
    ).toBeDefined();
    expect(
      renderer.scene.findByProps({ name: "context-globe-atmosphere:2" }),
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
});
