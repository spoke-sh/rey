export interface RailSectionPosition {
  label: string;
  top: number;
}

export const SECTION_RAIL_ATTRIBUTE = "data-rey-section";

export function activeSectionAt(
  sections: readonly RailSectionPosition[],
  offset: number,
): string | null {
  const first = sections[0];
  if (!first) return null;

  let active = first.label;
  for (const section of sections) {
    if (section.top > offset) break;
    active = section.label;
  }
  return active;
}
