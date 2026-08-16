import { create } from "@react-three/test-renderer";
import {
  AdditiveBlending,
  BackSide,
  Color,
  DirectionalLight,
  type Group,
  type InstancedBufferAttribute,
  type InstancedMesh,
  type LineSegments,
  Matrix4,
  type Mesh,
  type MeshBasicNodeMaterial,
  type MeshStandardNodeMaterial,
  NotEqualStencilFunc,
} from "three/src/Three.WebGPU.js";
import { describe, expect, it } from "vitest";
import { ContextGlobeScene } from "./globe-scene";
import {
  buildProjectedBoundsMeshes,
  globeAtlasRepeatOffset,
  globeAtlasWidth,
  globeAtmosphereRepeatOpacity,
  globeProjectionMorphRemaining,
  projectGlobeAtlasRepeatCoordinate,
} from "../globe-projection";
import { globeFixture } from "../test-fixtures";
import { compileContextGlobe, GLOBE_RADIUS } from "../three-globe";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

describe("globe scene", () => {
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
    const canonicalAtlasPositions = canonical.geometry.getAttribute(
      "reyAtlasPosition",
    ) as InstancedBufferAttribute;
    const rightMorphOffsets = repeated.geometry.getAttribute(
      "reyRepeatMorphOffset",
    ) as InstancedBufferAttribute;
    expect(canonicalAtlasPositions.count).toBe(canonical.instanceMatrix.count);
    expect(rightMorphOffsets.count).toBe(repeated.instanceMatrix.count);
    expect(
      (canonical.material as MeshBasicNodeMaterial).positionNode,
    ).not.toBeNull();
    expect(
      (repeated.material as MeshBasicNodeMaterial).positionNode,
    ).not.toBeNull();
    expect(canonical.userData.reyStippleMorphExecution).toBe("gpu_uniform");
    expect(repeated.userData.reyStippleMorphExecution).toBe("gpu_uniform");
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
    const effectivePosition = (index: number) => {
      repeated.getMatrixAt(index, repeatedMatrix);
      const remaining = globeProjectionMorphRemaining(progress);
      return [
        repeatedMatrix.elements[12]! +
          rightMorphOffsets.getX(index) * remaining,
        repeatedMatrix.elements[13]! +
          rightMorphOffsets.getY(index) * remaining,
        repeatedMatrix.elements[14]! +
          rightMorphOffsets.getZ(index) * remaining,
      ];
    };
    const outerPosition = effectivePosition(outerIndex);
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
    expect(outerPosition[0]).toBeCloseTo(expectedOuter.position[0], 5);
    expect(outerPosition[1]).toBeCloseTo(expectedOuter.position[1], 5);
    expect(outerPosition[2]).toBeCloseTo(expectedOuter.position[2], 5);
    const seamPosition = effectivePosition(seamIndex);
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
    expect(seamPosition[0]).toBeCloseTo(expectedSeam.position[0], 5);
    expect(seamPosition[1]).toBeCloseTo(expectedSeam.position[1], 5);
    expect(seamPosition[2]).toBeCloseTo(expectedSeam.position[2], 5);
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
    const retainedInstanceMatrices = Float32Array.from(
      repeated.instanceMatrix.array,
    );
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
    expect(updatedRepeat.instanceMatrix.array).toEqual(
      retainedInstanceMatrices,
    );

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
    // The overall envelope amplitude (material.opacity, still the transient
    // bump curve that's 0 at both the World and Atlas endpoints) is
    // unchanged by the new spatial sweep — the sweep multiplies into
    // opacityNode as an additional, independent shader term rather than
    // replacing this scalar.
    expect(
      (closingRepeatedAtmosphere.material as MeshBasicNodeMaterial).opacity,
    ).toBeCloseTo(0.17 * globeAtmosphereRepeatOpacity(0.79));

    await renderer.unmount();
  });

  it("sweeps a repeated wrap copy's atmosphere glow from the seam outward, matching sector/sample/marker reveal", async () => {
    const world = { width: 1200, height: 720 };
    const view = {
      yaw_degrees: 24,
      pitch_degrees: -8,
      projection_morph_progress: 0.79,
    };
    const compiled = compileContextGlobe(globeFixture());
    const renderer = await create(
      <ContextGlobeScene compiled={compiled} view={view} world={world} />,
    );

    const surface = renderer.scene.findByProps({
      name: "context-globe-surface",
    }).instance as Mesh;
    const canonicalAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0",
    }).instance as Mesh;
    const repeatedAtmosphere = renderer.scene.findByProps({
      name: "context-globe-atmosphere:0:wrap:1",
    }).instance as Mesh;

    // The shared surface/atmosphere geometry now carries one normalizedChartX
    // scalar per vertex — the same quantity globeAtlasRepeatSeamWeight and
    // globeAtlasRepeatVisibility consume in globe-projection.ts — so the
    // shader can sweep the wrap copy's glow spatially instead of applying
    // one flat opacity across the whole shell.
    expect(repeatedAtmosphere.geometry).toBe(surface.geometry);
    const chartXAttribute = surface.geometry.getAttribute(
      "reyNormalizedChartX",
    );
    expect(chartXAttribute.count).toBe(
      surface.geometry.getAttribute("position").count,
    );
    for (let index = 0; index < chartXAttribute.count; index += 1) {
      const value = chartXAttribute.getX(index);
      expect(Number.isFinite(value)).toBe(true);
    }

    // Both the canonical and repeated layers build a real, non-null shader
    // opacity graph — the repeated layer's is an independently constructed
    // node object (built under the wrapIndex !== 0 branch), not a shared
    // reference reused from the canonical layer.
    const canonicalOpacityNode = (
      canonicalAtmosphere.material as MeshBasicNodeMaterial
    ).opacityNode;
    const repeatedOpacityNode = (
      repeatedAtmosphere.material as MeshBasicNodeMaterial
    ).opacityNode;
    expect(canonicalOpacityNode).not.toBeNull();
    expect(repeatedOpacityNode).not.toBeNull();
    expect(repeatedOpacityNode).not.toBe(canonicalOpacityNode);

    await renderer.unmount();
  });

  it("reuses sector and marker materials across morph frames instead of rebuilding them", async () => {
    const world = { width: 1200, height: 720 };
    const view = {
      yaw_degrees: 24,
      pitch_degrees: -8,
      projection_morph_progress: 0.79,
    };
    const compiled = compileContextGlobe(globeFixture());
    const renderer = await create(
      <ContextGlobeScene compiled={compiled} view={view} world={world} />,
    );

    const repeatedSector = renderer.scene.findByProps({
      name: "context-globe-sector:sector:fixture:wrap:1:0",
    }).instance as Mesh;
    const repeatedMarkerGroup = renderer.scene.findByProps({
      name: "workload-beacon:survey:wrap:1",
    }).instance as Group;
    const [repeatedPointMesh, repeatedHaloMesh] =
      repeatedMarkerGroup.children as Mesh[];
    const sectorMaterial = repeatedSector.material as MeshBasicNodeMaterial;
    const pointMaterial = repeatedPointMesh!.material as MeshBasicNodeMaterial;
    const haloMaterial = repeatedHaloMesh!.material as MeshBasicNodeMaterial;
    const sectorOpacity = sectorMaterial.opacity;
    const pointOpacity = pointMaterial.opacity;
    const haloOpacity = haloMaterial.opacity;

    await renderer.update(
      <ContextGlobeScene
        compiled={compiled}
        view={{ ...view, projection_morph_progress: 0.9 }}
        world={world}
      />,
    );

    const updatedSector = renderer.scene.findByProps({
      name: "context-globe-sector:sector:fixture:wrap:1:0",
    }).instance as Mesh;
    const updatedMarkerGroup = renderer.scene.findByProps({
      name: "workload-beacon:survey:wrap:1",
    }).instance as Group;
    const [updatedPointMesh, updatedHaloMesh] =
      updatedMarkerGroup.children as Mesh[];

    // The sector geometry legitimately rebuilds every frame as it morphs, but
    // its material — and a repeated marker's materials — must stay the same
    // object across progress ticks. Recreating a MeshBasicNodeMaterial every
    // frame forces WebGPU pipeline churn for every sector and marker on
    // screen, which is what made scrolling through the unfurl sluggish.
    expect(updatedSector.material).toBe(sectorMaterial);
    expect(updatedPointMesh!.material).toBe(pointMaterial);
    expect(updatedHaloMesh!.material).toBe(haloMaterial);
    expect(
      (updatedSector.material as MeshBasicNodeMaterial).opacity,
    ).not.toBeCloseTo(sectorOpacity);
    expect(
      (updatedPointMesh!.material as MeshBasicNodeMaterial).opacity,
    ).not.toBeCloseTo(pointOpacity);
    expect(
      (updatedHaloMesh!.material as MeshBasicNodeMaterial).opacity,
    ).not.toBeCloseTo(haloOpacity);

    await renderer.unmount();
  });

  it("matches direct sector projection exactly across morph frames, canonical and repeated", async () => {
    const world = { width: 1200, height: 720 };
    const baseView = { yaw_degrees: 24, pitch_degrees: -8 };
    const compiled = compileContextGlobe(globeFixture());
    const bounds = {
      west_degrees: -60,
      south_degrees: 10,
      east_degrees: -30,
      north_degrees: 30,
      crosses_antimeridian: false,
    };

    const renderer = await create(
      <ContextGlobeScene
        compiled={compiled}
        view={{ ...baseView, projection_morph_progress: 0.35 }}
        world={world}
      />,
    );

    const expectSectorMatches = (
      name: string,
      wrapIndex: number,
      progress: number,
    ) => {
      const rendered = renderer.scene.findByProps({ name }).instance as Mesh;
      const expected = buildProjectedBoundsMeshes(
        bounds,
        baseView,
        world,
        progress,
        16,
        10,
        wrapIndex,
      )[0]!;
      const renderedPositions =
        rendered.geometry.getAttribute("position").array;
      for (let index = 0; index < expected.positions.length; index += 1)
        expect(renderedPositions[index]).toBeCloseTo(
          expected.positions[index]!,
          4,
        );
    };

    expectSectorMatches("context-globe-sector:sector:fixture:0", 0, 0.35);
    expectSectorMatches(
      "context-globe-sector:sector:fixture:wrap:1:0",
      1,
      0.35,
    );
    expectSectorMatches(
      "context-globe-sector:sector:fixture:wrap:-1:0",
      -1,
      0.35,
    );

    await renderer.update(
      <ContextGlobeScene
        compiled={compiled}
        view={{ ...baseView, projection_morph_progress: 0.83 }}
        world={world}
      />,
    );

    // A later frame at a different progress: the cached sphere/atlas
    // endpoints are reused, but the blended output must still match what
    // buildProjectedBoundsMeshes computes directly for that exact progress —
    // proving the endpoint-cache-and-lerp approach is not an approximation.
    expectSectorMatches("context-globe-sector:sector:fixture:0", 0, 0.83);
    expectSectorMatches(
      "context-globe-sector:sector:fixture:wrap:1:0",
      1,
      0.83,
    );
    expectSectorMatches(
      "context-globe-sector:sector:fixture:wrap:-1:0",
      -1,
      0.83,
    );

    await renderer.unmount();
  });

  it("draws a closed border loop around each sector matching its fill mesh", async () => {
    const world = { width: 1200, height: 720 };
    const view = {
      yaw_degrees: 24,
      pitch_degrees: -8,
      projection_morph_progress: 0.35,
    };
    const compiled = compileContextGlobe(globeFixture());
    const renderer = await create(
      <ContextGlobeScene compiled={compiled} view={view} world={world} />,
    );

    const fill = renderer.scene.findByProps({
      name: "context-globe-sector:sector:fixture:0",
    }).instance as Mesh;
    const border = renderer.scene.findByProps({
      name: "context-globe-sector:sector:fixture:0:border",
    }).instance as LineSegments;

    // A 16x10 grid's outer ring has 2*(16+10) = 52 points, walked as 52
    // closed-loop segments (each sharing an endpoint with its neighbor).
    const borderPositions = border.geometry.getAttribute("position");
    expect(borderPositions.count).toBe(52 * 2);
    // The loop closes: the last segment's second point is the first
    // segment's first point.
    const first = [
      borderPositions.getX(0),
      borderPositions.getY(0),
      borderPositions.getZ(0),
    ];
    const last = [
      borderPositions.getX(borderPositions.count - 1),
      borderPositions.getY(borderPositions.count - 1),
      borderPositions.getZ(borderPositions.count - 1),
    ];
    expect(last[0]).toBeCloseTo(first[0]!, 5);
    expect(last[1]).toBeCloseTo(first[1]!, 5);
    expect(last[2]).toBeCloseTo(first[2]!, 5);

    // Every border vertex sits a small fixed distance from the fill mesh's
    // nearest surface — the depth bias that keeps it from z-fighting the
    // coplanar fill — rather than exactly on top of it or drifting away.
    const fillPositions = fill.geometry.getAttribute("position");
    let closestDistance = Number.POSITIVE_INFINITY;
    for (let fillIndex = 0; fillIndex < fillPositions.count; fillIndex += 1) {
      const dx = fillPositions.getX(fillIndex) - first[0]!;
      const dy = fillPositions.getY(fillIndex) - first[1]!;
      const dz = fillPositions.getZ(fillIndex) - first[2]!;
      const distance = Math.hypot(dx, dy, dz);
      if (distance < closestDistance) closestDistance = distance;
    }
    expect(closestDistance).toBeGreaterThan(0);
    expect(closestDistance).toBeLessThan(0.01);

    const borderMaterial = border.material as MeshBasicNodeMaterial;
    expect(borderMaterial.color.getHex()).toBe(0xd57824);

    await renderer.unmount();
  });
});
