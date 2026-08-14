import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { GitCommitLink } from "./git-commit-link";

const repository = "https://github.com/example/rey";
const revision = "02ad6ed24744dbeabb0b8bef5a64d547f424d9a3";

describe("GitCommitLink", () => {
  it("makes the displayed SHA itself an exact GitHub commit link", () => {
    const markup = renderToStaticMarkup(
      <GitCommitLink repository={repository} revision={revision} />,
    );

    expect(markup).toContain(
      `href="https://github.com/example/rey/commit/${revision}"`,
    );
    expect(markup).toContain(`data-git-sha="${revision}"`);
    expect(markup).toContain("02ad6ed2…24d9a3");
  });

  it("does not render an inert or guessed SHA without a repository binding", () => {
    const markup = renderToStaticMarkup(
      <GitCommitLink repository={null} revision={revision} />,
    );

    expect(markup).toBe("<span>GIT COMMIT LINK UNAVAILABLE</span>");
    expect(markup).not.toContain(revision);
  });
});
