import type { TopographyPatch, TopographyRegionState } from "../../domain";
import type {
  TopologyContour,
  TopologyLandform,
  TopologyNaturalFeature,
  TopologyPointOfInterest,
  TopologyRegion,
  TopologyTone,
  TopologyWorld,
} from "../../topology";
import type { LensRegime } from "../engine/camera";
import type { AdmittedTopography } from "./topography-projector";
import {
  compileSurveyTerrainField,
  type SurveyTerrainFieldResult,
} from "./survey-terrain";

export const SURVEY_SCENE_LAYOUT_REVISION =
  "rey.explorer.survey-scene-layout@1";

export interface SurveyTerrainLayout {
  contours: TopologyContour[];
  landforms: TopologyLandform[];
  natural_features: TopologyNaturalFeature[];
  omissions: string[];
  points: TopologyPointOfInterest[];
  regions: TopologyRegion[];
  terrain_fields: SurveyTerrainFieldResult["fields"][];
  terrain_programs: SurveyTerrainFieldResult["program"][];
  world: TopologyWorld;
}

export function layoutSurveyTerrain(
  topographies: AdmittedTopography[],
  regime: LensRegime,
): SurveyTerrainLayout {
  const columns = Math.max(1, Math.ceil(Math.sqrt(topographies.length)));
  const rows = Math.max(1, Math.ceil(topographies.length / columns));
  const cellWidth = Math.max(
    1,
    ...topographies.map(({ projection }) => projection.extent.width),
  );
  const cellHeight = Math.max(
    1,
    ...topographies.map(({ projection }) => projection.extent.height),
  );
  const world = {
    width: columns * cellWidth,
    height: rows * cellHeight,
  };
  const contours: TopologyContour[] = [];
  const landforms: TopologyLandform[] = [];
  const naturalFeatures: TopologyNaturalFeature[] = [];
  const omissions: string[] = [];
  const points: TopologyPointOfInterest[] = [];
  const regions: TopologyRegion[] = [];
  const terrainFields: SurveyTerrainLayout["terrain_fields"] = [];
  const terrainPrograms: SurveyTerrainLayout["terrain_programs"] = [];

  const ordered = [...topographies].sort((left, right) =>
    left.workload.workload.id.localeCompare(right.workload.workload.id),
  );
  ordered.forEach(({ workload, patch, projection }, patchIndex) => {
    const column = patchIndex % columns;
    const row = Math.floor(patchIndex / columns);
    const origin = {
      x: column * cellWidth,
      y: row * cellHeight,
    };
    const center = {
      x: origin.x + projection.extent.width / 2,
      y: origin.y + projection.extent.height / 2,
    };
    const states = new Set(projection.validity.map((region) => region.state));
    if (states.has("unexplored")) {
      regions.push({
        id: `terrain-unexplored:${workload.workload.id}`,
        label: "UNEXPLORED BEYOND SURVEY",
        detail: "no admitted terrain claim",
        x: origin.x + 30,
        y: origin.y + 30,
        width: projection.extent.width - 60,
        height: projection.extent.height - 60,
        tone: "unknown",
        variant: "map-zone",
      });
    }
    regions.push({
      id: `terrain-boundary:${workload.workload.id}`,
      label: workload.workload.id,
      detail: `${patch.coverage.surveyed_seeds}/${patch.coverage.requested_seeds} seeds surveyed · ${shortCoordinate(patch.topography_revision)}`,
      x: origin.x + 105,
      y: origin.y + 85,
      width: projection.extent.width - 210,
      height: projection.extent.height - 170,
      tone: states.has("surveyed")
        ? "healthy"
        : patch.complete
          ? "neutral"
          : "omitted",
      variant: "map-boundary",
    });
    const statusStates = [...states]
      .filter((state) => state !== "surveyed" && state !== "unexplored")
      .sort();
    statusStates.forEach((state, index) => {
      const source = projection.validity.find(
        (region) => region.state === state,
      )!;
      regions.push({
        id: `terrain-zone:${workload.workload.id}:${state}`,
        label: state.replaceAll("_", " "),
        detail: source.detail,
        x: origin.x + 150 + index * 245,
        y: origin.y + projection.extent.height - 150,
        width: 215,
        height: 70,
        tone: regionTone(state),
        variant: "map-zone",
      });
    });

    const visibleAnchors = projection.objects.filter(
      (object) =>
        object.kind === "anchor" &&
        object.anchor_kind !== null &&
        object.coordinate !== null,
    );
    const samplesByCoordinate = new Map<string, number>();
    patch.seeds.forEach((seed) => {
      if (!seed.coordinate) return;
      samplesByCoordinate.set(seed.coordinate.coordinate, seed.candidate_count);
    });
    patch.resolutions.forEach((resolution) => {
      const coordinate = resolution.coordinate?.coordinate;
      if (!coordinate) return;
      samplesByCoordinate.set(
        coordinate,
        (samplesByCoordinate.get(coordinate) ?? 0) + 1,
      );
    });
    const nonWorkspace = visibleAnchors.filter(
      (anchor) => anchor.anchor_kind !== "workspace",
    );
    visibleAnchors.forEach((anchor) => {
      const nonWorkspaceIndex = nonWorkspace.findIndex(
        (candidate) => candidate.source_id === anchor.source_id,
      );
      const hash = stableHash(anchor.coordinate!);
      const jitter = ((hash % 1000) / 1000 - 0.5) * 0.42;
      const ring =
        nonWorkspaceIndex < 0 ? 0 : Math.floor(nonWorkspaceIndex / 8);
      const slot = nonWorkspaceIndex < 0 ? 0 : nonWorkspaceIndex % 8;
      const ringCount = Math.min(8, nonWorkspace.length - ring * 8);
      const angle =
        nonWorkspaceIndex < 0
          ? 0
          : (Math.PI * 2 * slot) / Math.max(1, ringCount) + jitter;
      const radiusMultiplier =
        anchor.anchor_kind === "external_resource" ? 1.22 : 1;
      const x =
        nonWorkspaceIndex < 0
          ? center.x
          : center.x + Math.cos(angle) * (190 + ring * 60) * radiusMultiplier;
      const y =
        nonWorkspaceIndex < 0
          ? center.y
          : center.y + Math.sin(angle) * (140 + ring * 40) * radiusMultiplier;
      const sampledConditions =
        anchor.anchor_kind === "workspace"
          ? patch.coverage.surveyed_seeds
          : (samplesByCoordinate.get(anchor.coordinate!) ?? 0);
      const prominence =
        anchor.anchor_kind === "workspace"
          ? 4
          : Math.min(4, 1 + Math.ceil(Math.log2(sampledConditions + 1)));
      points.push({
        id: `anchor-node:${workload.workload.id}:${anchor.source_id}`,
        focus_id: `anchor:${workload.workload.id}:${anchor.source_id}`,
        kind: "anchor",
        family: anchor.anchor_kind!.replaceAll("_", " ").toUpperCase(),
        label: anchor.label,
        detail: `${sampledConditions} admitted survey sample${sampledConditions === 1 ? "" : "s"} · ${shortCoordinate(anchor.source_revision)}`,
        x,
        y,
        prominence,
        signal:
          anchor.anchor_kind === "workspace"
            ? `SURVEY ORIGIN / ${sampledConditions} ADMITTED SEED CONDITION${sampledConditions === 1 ? "" : "S"}`
            : sampledConditions >= 3
              ? `DENSE SAMPLE / ${sampledConditions} ADMITTED OBSERVATIONS`
              : sampledConditions > 0
                ? `SAMPLED / ${sampledConditions} ADMITTED OBSERVATION${sampledConditions === 1 ? "" : "S"}`
                : "SURVEYED STATION / NO LOCAL CANDIDATE OBSERVATION",
        action: "EXACT COORDINATE / MINING STILL REQUIRES AN ADMITTED WORKLOAD",
        tone: "healthy",
        workload_id: workload.workload.id,
        coordinate_uri: anchor.coordinate!,
      });
    });

    const visibleFrontier = projection.objects.filter(
      (object) => object.kind === "frontier" && object.frontier_status !== null,
    );
    visibleFrontier.forEach((frontier, index) => {
      const angle =
        (Math.PI * 2 * index) / Math.max(1, visibleFrontier.length) + 0.38;
      points.push({
        id: `frontier-node:${workload.workload.id}:${frontier.source_id}`,
        focus_id: `frontier:${workload.workload.id}:${frontier.source_id}`,
        kind: "frontier",
        family: "FRONTIER",
        label: frontier.label,
        detail: `${frontier.frontier_status} · ${frontier.detail}`,
        x: center.x + Math.cos(angle) * 585,
        y: center.y + Math.sin(angle) * 365,
        prominence: 1,
        signal: boundaryProbeAction(frontier.frontier_status!),
        action: "PROBE FIRST / THE CANVAS WILL NOT EXECUTE IT",
        tone: resolutionTone(frontier.frontier_status!),
        workload_id: workload.workload.id,
      });
    });

    const patchPoints = points.filter(
      (point) =>
        point.workload_id === workload.workload.id && point.kind === "anchor",
    );
    const patchFrontierPoints = points.filter(
      (point) =>
        point.workload_id === workload.workload.id && point.kind === "frontier",
    );
    landforms.push(
      {
        id: `charted-land:${workload.workload.id}`,
        path: envelopePath(patchPoints, center, 115, 510),
        kind: "charted",
        label: workload.workload.id,
        detail: `${patch.coverage.surveyed_seeds}/${patch.coverage.requested_seeds} admitted seed boundaries`,
        tone: patch.complete ? "healthy" : "omitted",
      },
      {
        id: `probe-horizon:${workload.workload.id}`,
        path: envelopePath(
          [...patchPoints, ...patchFrontierPoints],
          center,
          205,
          660,
        ),
        kind: "horizon",
        label: "SURVEY HORIZON",
        detail:
          patchFrontierPoints.length > 0
            ? `${patchFrontierPoints.length} unresolved probes bound this chart`
            : "no unresolved boundary probe retained",
        tone: patchFrontierPoints.length > 0 ? "frontier" : "unknown",
      },
    );
    const terrainField = compileSurveyTerrainField(
      workload.workload.id,
      patchPoints,
      patchFrontierPoints,
      patch,
      projection,
      regime,
      {
        x: origin.x + 100,
        y: origin.y + 80,
        width: projection.extent.width - 200,
        height: projection.extent.height - 160,
      },
    );
    contours.push(...terrainField.contours);
    terrainFields.push(terrainField.fields);
    terrainPrograms.push(terrainField.program);
    const boundedFeatures = terrainField.natural_features.slice(
      0,
      projection.limits.max_natural_features,
    );
    naturalFeatures.push(...boundedFeatures);
    if (terrainField.natural_features.length > boundedFeatures.length)
      omissions.push(
        `${terrainField.natural_features.length - boundedFeatures.length} natural features folded from ${workload.workload.id}`,
      );

    omissions.push(
      ...projection.omissions.map((omission) => {
        if (omission.kind === "semantic_boundary") return omission.reason;
        if (omission.kind === "anchor_limit")
          return `${omission.omitted_count} anchor POIs folded from ${workload.workload.id}`;
        if (omission.kind === "frontier_limit")
          return `${omission.omitted_count} frontier POIs folded from ${workload.workload.id}`;
        return `${omission.omitted_count} ${omission.kind.replaceAll("_", " ")} omitted: ${omission.reason}`;
      }),
    );
  });
  return {
    contours,
    landforms,
    natural_features: naturalFeatures,
    omissions,
    points,
    regions,
    terrain_fields: terrainFields,
    terrain_programs: terrainPrograms,
    world,
  };
}

function envelopePath(
  points: TopologyPointOfInterest[],
  center: { x: number; y: number },
  padding: number,
  maximumRadius: number,
): string {
  const samples = Array.from({ length: 24 }, (_, index) => {
    const angle = (Math.PI * 2 * index) / 24;
    const direction = { x: Math.cos(angle), y: Math.sin(angle) };
    const extent = points.reduce(
      (maximum, point) =>
        Math.max(
          maximum,
          (point.x - center.x) * direction.x +
            (point.y - center.y) * direction.y,
        ),
      0,
    );
    const radius = Math.min(maximumRadius, Math.max(padding, extent + padding));
    return {
      x: center.x + direction.x * radius,
      y: center.y + direction.y * radius,
    };
  });
  const midpoint = (
    first: { x: number; y: number },
    second: { x: number; y: number },
  ) => ({
    x: (first.x + second.x) / 2,
    y: (first.y + second.y) / 2,
  });
  const first = midpoint(samples.at(-1)!, samples[0]!);
  const segments = samples.map((point, index) => {
    const next = samples[(index + 1) % samples.length]!;
    const end = midpoint(point, next);
    return `Q${point.x.toFixed(1)},${point.y.toFixed(1)} ${end.x.toFixed(1)},${end.y.toFixed(1)}`;
  });
  return `M${first.x.toFixed(1)},${first.y.toFixed(1)} ${segments.join(" ")} Z`;
}

function boundaryProbeAction(
  status: TopographyPatch["frontier"][number]["status"],
): string {
  if (status === "truncated") return "EXPAND DECLARED SURVEY BOUND";
  if (status === "stale") return "REVALIDATE SOURCE REVISION";
  if (status === "unsupported") return "ADMIT A RESOLVER CAPABILITY";
  if (status === "unauthorized") return "OBTAIN EXPLICIT READ AUTHORITY";
  if (status === "malformed") return "CURATE THE LOCATOR";
  if (status === "missing") return "VERIFY ABSENCE OR REPAIR REFERENCE";
  return "RESOLVED COORDINATE / ADMIT MINING SEPARATELY";
}

function regionTone(state: TopographyRegionState): TopologyTone {
  if (state === "surveyed") return "healthy";
  if (state === "surveyed_empty") return "neutral";
  if (state === "unexplored") return "unknown";
  if (state === "omitted") return "omitted";
  if (state === "stale") return "stale";
  if (state === "unsupported") return "unsupported";
  return "frontier";
}

function resolutionTone(
  status: TopographyPatch["resolutions"][number]["status"],
): TopologyTone {
  if (status === "resolved") return "healthy";
  if (status === "stale") return "stale";
  if (status === "unsupported") return "unsupported";
  if (status === "truncated") return "omitted";
  if (status === "missing" || status === "unauthorized") return "blocked";
  return "unknown";
}

function stableHash(value: string): number {
  let hash = 2_166_136_261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16_777_619);
  }
  return hash >>> 0;
}

function shortCoordinate(value: string): string {
  const coordinate = value.startsWith("blake3:") ? value.slice(7) : value;
  return coordinate.length > 22
    ? `${coordinate.slice(0, 10)}…${coordinate.slice(-8)}`
    : coordinate;
}
