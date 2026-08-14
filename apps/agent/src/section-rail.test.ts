import { describe, expect, it } from "vitest";
import { activeSectionAt } from "./section-rail";

describe("section-aware coordinate rail", () => {
  const sections = [
    { label: "01 / DIRECTED TEXT", top: -420 },
    { label: "02 / BOUNDED SEARCH", top: 88 },
    { label: "03 / REFERENCE PLANE", top: 780 },
  ];

  it("advances to the last section that crossed the sticky rail", () => {
    expect(activeSectionAt(sections, 105)).toBe("02 / BOUNDED SEARCH");
  });

  it("previews the first section before it reaches the rail", () => {
    expect(
      activeSectionAt(
        sections.map((section) => ({ ...section, top: section.top + 600 })),
        105,
      ),
    ).toBe("01 / DIRECTED TEXT");
  });

  it("has no invented coordinate when a route exposes no sections", () => {
    expect(activeSectionAt([], 105)).toBeNull();
  });
});
