import { describe, expect, it } from "vitest";
import {
  admissionState,
  currentApplications,
  environmentVariableDiff,
  type EnvironmentApplicationObservation,
  type EnvironmentObjectStatus,
  type EnvironmentVariableObservation,
} from "./environment";

describe("environment operator projection", () => {
  it("renders a directed env-shaped variable replacement with admission state", () => {
    const variable: EnvironmentObjectStatus<EnvironmentVariableObservation> = {
      object_id: "endpoint",
      head: observation("SPOKE_ENDPOINT", "http://old"),
      index: observation("SPOKE_ENDPOINT", "http://old"),
      working: observation("SPOKE_ENDPOINT", "http://new"),
      changes: {
        head_to_index: "unchanged",
        index_to_working: "modified",
        head_to_working: "modified",
      },
    };

    expect(environmentVariableDiff([variable])).toEqual([
      {
        key: "endpoint:deleted:SPOKE_ENDPOINT=http://old",
        kind: "deleted",
        text: "SPOKE_ENDPOINT=http://old",
        admission: "working",
      },
      {
        key: "endpoint:inserted:SPOKE_ENDPOINT=http://new",
        kind: "inserted",
        text: "SPOKE_ENDPOINT=http://new",
        admission: "working",
      },
    ]);
  });

  it("keeps searched-but-not-found applications visible", () => {
    const missing: EnvironmentObjectStatus<EnvironmentApplicationObservation> =
      {
        object_id: "rg",
        head: null,
        index: null,
        working: {
          name: "rg",
          purpose: "Extend bounded source mining with fast text search",
          required: false,
          availability: "unavailable",
          resolved_path: null,
          content_digest: null,
          potential_capabilities: ["source.search.literal"],
          searched_path_count: 14,
          error_code: null,
        },
        changes: {
          head_to_index: "unchanged",
          index_to_working: "inserted",
          head_to_working: "inserted",
        },
      };

    expect(currentApplications([missing], "unavailable")).toEqual([missing]);
    expect(admissionState(missing.changes)).toBe("working");
  });
});

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
