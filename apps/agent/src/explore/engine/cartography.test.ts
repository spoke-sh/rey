import { describe, expect, it } from "vitest";
import { featureVisibleAtLens } from "./cartography";

describe("Landscape cartographic hierarchy", () => {
  it("keeps base vectors and selection without exposing authoring envelopes", () => {
    expect(
      featureVisibleAtLens(
        { geometry_kind: "LineString", layer: "hydrology" },
        "landscape",
        false,
      ),
    ).toBe(true);
    expect(
      featureVisibleAtLens(
        { geometry_kind: "Polygon", layer: "district" },
        "landscape",
        false,
      ),
    ).toBe(false);
    expect(
      featureVisibleAtLens(
        { geometry_kind: "Polygon", layer: "terrain_control" },
        "landscape",
        false,
      ),
    ).toBe(false);
    expect(
      featureVisibleAtLens(
        { geometry_kind: "Point", layer: "poi" },
        "landscape",
        true,
      ),
    ).toBe(true);
  });

  it("reveals authored objects only at exact-object lenses", () => {
    const control = {
      geometry_kind: "Polygon",
      layer: "terrain_control",
    };
    expect(featureVisibleAtLens(control, "neighborhoods", false)).toBe(false);
    expect(featureVisibleAtLens(control, "objects", false)).toBe(true);
    expect(featureVisibleAtLens(control, "evidence", false)).toBe(true);
  });
});
