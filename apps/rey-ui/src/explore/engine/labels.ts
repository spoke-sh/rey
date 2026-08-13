export const SEMANTIC_LABEL_LAYOUT_REVISION = "rey.semantic-label-layout@1";

export interface SemanticLabelCandidate {
  fragment_id: string;
  semantic_identity: string;
  focus_id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  priority: number;
  selected: boolean;
}

export type SemanticLabelDisposition =
  "selected" | "placed" | "collision" | "limit";

export interface SemanticLabelPlacement extends SemanticLabelCandidate {
  visible: boolean;
  disposition: SemanticLabelDisposition;
}

/**
 * Deterministically culls presentation labels while leaving semantic objects
 * and their pick targets untouched. Selection wins; no placement can change
 * identity, focus, evidence, or camera state.
 */
export function layoutSemanticLabels(
  candidates: readonly SemanticLabelCandidate[],
  maxLabels: number,
): readonly SemanticLabelPlacement[] {
  if (!Number.isInteger(maxLabels) || maxLabels < 0)
    throw new Error("semantic label limit must be a non-negative integer");
  const fragmentIds = new Set<string>();
  for (const candidate of candidates) {
    if (
      !candidate.fragment_id ||
      !candidate.semantic_identity ||
      !candidate.focus_id ||
      ![
        candidate.x,
        candidate.y,
        candidate.width,
        candidate.height,
        candidate.priority,
      ].every(Number.isFinite) ||
      candidate.width <= 0 ||
      candidate.height <= 0
    )
      throw new Error("semantic label candidate is malformed");
    if (fragmentIds.has(candidate.fragment_id))
      throw new Error("semantic label fragment identity must be unique");
    fragmentIds.add(candidate.fragment_id);
  }
  const ordered = [...candidates].sort(
    (left, right) =>
      Number(right.selected) - Number(left.selected) ||
      right.priority - left.priority ||
      left.semantic_identity.localeCompare(right.semantic_identity) ||
      left.fragment_id.localeCompare(right.fragment_id),
  );
  const visible: SemanticLabelCandidate[] = [];
  const placements = new Map<string, SemanticLabelPlacement>();
  for (const candidate of ordered) {
    const collides = visible.some((placed) => intersects(candidate, placed));
    const admitted =
      candidate.selected || (!collides && visible.length < maxLabels);
    const disposition: SemanticLabelDisposition = candidate.selected
      ? "selected"
      : collides
        ? "collision"
        : visible.length >= maxLabels
          ? "limit"
          : "placed";
    if (admitted) visible.push(candidate);
    placements.set(
      candidate.fragment_id,
      Object.freeze({ ...candidate, visible: admitted, disposition }),
    );
  }
  return Object.freeze(
    candidates.map((candidate) => placements.get(candidate.fragment_id)!),
  );
}

function intersects(
  left: SemanticLabelCandidate,
  right: SemanticLabelCandidate,
) {
  return !(
    left.x + left.width <= right.x ||
    right.x + right.width <= left.x ||
    left.y + left.height <= right.y ||
    right.y + right.height <= left.y
  );
}
