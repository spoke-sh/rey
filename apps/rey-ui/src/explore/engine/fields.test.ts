import { describe, expect, it } from "vitest";
import {
  createFieldGrid,
  fieldByteLength,
  fieldCellCount,
  fieldIndex,
  fieldPoint,
  scalarField,
} from "./fields";

describe("typed terrain fields", () => {
  it("keeps row-major identity and bounded world coordinates explicit", () => {
    const grid = createFieldGrid(3, 2, {
      x: 10,
      y: 20,
      width: 100,
      height: 50,
    });
    expect(fieldCellCount(grid)).toBe(6);
    expect(fieldIndex(grid, 2, 1)).toBe(5);
    expect(fieldPoint(grid, 2, 1)).toEqual({ x: 110, y: 70 });
    expect(() => fieldIndex(grid, 3, 1)).toThrow("outside");
  });

  it("rejects malformed values and reports exact typed-buffer bytes", () => {
    const grid = createFieldGrid(2, 2, {
      x: 0,
      y: 0,
      width: 1,
      height: 1,
    });
    const field = scalarField(
      "elevation",
      "rey.terrain.elevation@1",
      grid,
      new Float32Array([0, 0.25, 0.5, 1]),
    );
    expect(field).toMatchObject({ minimum: 0, maximum: 1 });
    expect(fieldByteLength(field)).toBe(16);
    expect(() =>
      scalarField("broken", "broken@1", grid, new Float32Array([0])),
    ).toThrow("does not match");
  });
});
