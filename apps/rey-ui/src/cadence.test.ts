import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  CadencePage,
  formatCadenceTime,
  type CadenceProjection,
} from "./cadence";

const revision = "02ad6ed24744dbeabb0b8bef5a64d547f424d9a3";

describe("cadence projection", () => {
  it("renders exact UTC instants while preserving order-only clocks", () => {
    expect(formatCadenceTime(1_786_335_192)).toBe("2026-08-10 04:13:12Z");
    expect(formatCadenceTime(null)).toBe("ORDER ONLY");
  });

  it("renders every visible Git cadence SHA as its exact GitHub link", () => {
    const cadence: CadenceProjection = {
      schema: "rey.ui-cadence.v1",
      ordering: "partial",
      source_repository: "https://github.com/spoke-sh/rey",
      lanes: [
        {
          id: "git-sequence",
          label: "Git commits",
          clock: "reachable_head_history",
          ordering: "newest_first",
          complete: true,
          ticks: [
            {
              id: `git:${revision}`,
              kind: "git_commit",
              state: "observed",
              ordinal: "HEAD",
              title: "Bind every SHA",
              detail: "1 parent · sha1",
              revision,
              parent_revisions: ["12c4df4d22488f84726c7072524b9c52c8cf0b03"],
              occurred_at_unix: 1_786_335_192,
            },
          ],
          omissions: [],
        },
      ],
      schedules: [],
      omissions: [],
    };

    const markup = renderToStaticMarkup(
      createElement(CadencePage, { cadence }),
    );
    expect(markup).toContain(`data-git-sha="${revision}"`);
    expect(markup).toContain(
      `href="https://github.com/spoke-sh/rey/commit/${revision}"`,
    );
    expect(markup).toContain("02ad6ed2…24d9a3");
  });
});
