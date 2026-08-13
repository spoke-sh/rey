import { describe, expect, it } from "vitest";
import {
  admissionState,
  environmentApplicationDiff,
  environmentVariableDiff,
  type EnvironmentApplicationObservation,
  type EnvironmentObjectStatus,
  type EnvironmentVariableObservation,
} from "./environment";

describe("environment operator projection", () => {
  it("renders a directed env-shaped variable replacement with admission state", () => {
    const variable: EnvironmentObjectStatus<EnvironmentVariableObservation> = {
      object_id: "endpoint",
      head: observation("REY_MODE", "observe"),
      index: observation("REY_MODE", "observe"),
      working: observation("REY_MODE", "process"),
      changes: {
        head_to_index: "unchanged",
        index_to_working: "modified",
        head_to_working: "modified",
      },
    };

    expect(environmentVariableDiff([variable])).toEqual([
      {
        key: "endpoint:deleted:REY_MODE=observe",
        kind: "deleted",
        text: "REY_MODE=observe",
        admission: "working",
      },
      {
        key: "endpoint:inserted:REY_MODE=process",
        kind: "inserted",
        text: "REY_MODE=process",
        admission: "working",
      },
    ]);
  });

  it("renders application modifications as deleted and inserted observations", () => {
    const before = applicationObservation("/usr/bin/rg", ["retrieval"]);
    const after = applicationObservation("/opt/bin/rg", ["code", "retrieval"]);
    const application: EnvironmentObjectStatus<EnvironmentApplicationObservation> =
      {
        object_id: "rg",
        head: before,
        index: before,
        working: after,
        changes: {
          head_to_index: "unchanged",
          index_to_working: "modified",
          head_to_working: "modified",
        },
      };

    expect(environmentApplicationDiff([application])).toEqual([
      {
        key: "rg:deleted:rg:/usr/bin/rg:retrieval",
        kind: "deleted",
        observation: before,
        admission: "working",
      },
      {
        key: "rg:inserted:rg:/opt/bin/rg:code,retrieval",
        kind: "inserted",
        observation: after,
        admission: "working",
      },
    ]);
    expect(admissionState(application.changes)).toBe("working");
  });

  it("keeps unresolved application observations in the browser projection", () => {
    const resolved = applicationObservation("/usr/bin/rg", ["retrieval"]);
    const unresolved = applicationObservation(
      null,
      ["retrieval"],
      "unavailable",
    );
    const removed: EnvironmentObjectStatus<EnvironmentApplicationObservation> =
      {
        object_id: "rg",
        head: resolved,
        index: resolved,
        working: unresolved,
        changes: {
          head_to_index: "unchanged",
          index_to_working: "modified",
          head_to_working: "modified",
        },
      };
    const missing: EnvironmentObjectStatus<EnvironmentApplicationObservation> =
      {
        object_id: "ag",
        head: null,
        index: null,
        working: {
          ...unresolved,
          name: "ag",
        },
        changes: {
          head_to_index: "unchanged",
          index_to_working: "inserted",
          head_to_working: "inserted",
        },
      };

    expect(environmentApplicationDiff([removed, missing])).toEqual([
      {
        key: "rg:deleted:rg:/usr/bin/rg:retrieval",
        kind: "deleted",
        observation: resolved,
        admission: "working",
      },
      {
        key: "rg:inserted:rg:unresolved:retrieval",
        kind: "inserted",
        observation: unresolved,
        admission: "working",
      },
      {
        key: "ag:inserted:ag:unresolved:retrieval",
        kind: "inserted",
        observation: {
          ...unresolved,
          name: "ag",
        },
        admission: "working",
      },
    ]);
  });
});

function applicationObservation(
  resolvedPath: string | null,
  groups: string[],
  availability: EnvironmentApplicationObservation["availability"] = "available",
): EnvironmentApplicationObservation {
  return {
    name: "rg",
    groups,
    purpose: "Extend bounded source mining with fast text search",
    required: false,
    availability,
    resolved_path: resolvedPath,
    content_digest: null,
    potential_capabilities: ["source.search.literal"],
    searched_path_count: 14,
    error_code: null,
  };
}

function observation(
  name: string,
  value: string,
): EnvironmentVariableObservation {
  return {
    name,
    sensitive: false,
    capture: "value",
    availability: "available",
    value,
    value_digest: "blake3:value",
    error_code: null,
  };
}
