import { describe, expect, it } from "vitest";
import { layoutSemanticLabels, type SemanticLabelCandidate } from "./labels";

const candidate = (
  fragmentId: string,
  x: number,
  priority: number,
  selected = false,
): SemanticLabelCandidate => ({
  fragment_id: fragmentId,
  semantic_identity: fragmentId.split(":")[0]!,
  focus_id: `focus:${fragmentId}`,
  x,
  y: 10,
  width: 80,
  height: 20,
  priority,
  selected,
});

describe("semantic label layout", () => {
  it("is deterministic across input order and keeps markers out of scope", () => {
    const candidates = [
      candidate("one:0", 0, 2),
      candidate("two:0", 40, 1),
      candidate("three:0", 120, 0),
    ];
    const forward = layoutSemanticLabels(candidates, 2);
    const reverse = layoutSemanticLabels([...candidates].reverse(), 2);
    const dispositions = (rows: typeof forward) =>
      new Map(
        rows.map(({ fragment_id, disposition }) => [fragment_id, disposition]),
      );
    expect(dispositions(forward)).toEqual(dispositions(reverse));
    expect(dispositions(forward)).toEqual(
      new Map([
        ["one:0", "placed"],
        ["two:0", "collision"],
        ["three:0", "placed"],
      ]),
    );
  });

  it("always retains the selected identity and keeps limit distinct", () => {
    const placements = layoutSemanticLabels(
      [
        candidate("one:0", 0, 100),
        candidate("selected:0", 10, 0, true),
        candidate("three:0", 200, 1),
      ],
      1,
    );
    expect(
      placements.find(({ fragment_id }) => fragment_id === "selected:0"),
    ).toMatchObject({ visible: true, disposition: "selected" });
    expect(
      placements.find(({ fragment_id }) => fragment_id === "one:0"),
    ).toMatchObject({ visible: false, disposition: "collision" });
    expect(
      placements.find(({ fragment_id }) => fragment_id === "three:0"),
    ).toMatchObject({ visible: false, disposition: "limit" });
  });
});
