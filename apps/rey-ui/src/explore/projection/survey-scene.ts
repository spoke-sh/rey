import type {
  SemanticAtlas,
  SemanticAtlasDelta,
  TopographyPatch,
} from "../../domain";
import type {
  TopologyBearing,
  TopologyEdge,
  TopologyGlobe,
  TopologyNode,
  TopologyPointOfInterest,
  TopologyProjection,
  TopologyTone,
} from "../../topology";
import type { LensRegime } from "../engine/camera";
import type { AdmittedTopography } from "./topography-projector";
import { layoutSurveyTerrain } from "./survey-scene-layout";

export const SURVEY_SCENE_PROJECTION_REVISION =
  "rey.explorer.survey-scene-projection@1";

export function buildSurveyScene(
  topographies: AdmittedTopography[],
  focusId: string,
  regime: LensRegime,
  semanticAtlas: SemanticAtlas | null = null,
  atlasDelta: SemanticAtlasDelta | null = null,
): TopologyProjection {
  const layout = layoutSurveyTerrain(topographies, regime);
  const selected = selectTopography(topographies, focusId);
  const selectedPoints = layout.points.filter(
    (point) => point.workload_id === selected.workload.workload.id,
  );
  const requestedFocusPoint = selectedPoints.find(
    (point) => point.focus_id === focusId,
  );
  const focusPoint =
    requestedFocusPoint ??
    selectedPoints.find((point) => point.family === "WORKSPACE") ??
    selectedPoints[0];
  const baseBearing = buildSurveyBearing(
    selectedPoints,
    requestedFocusPoint,
    selected.patch,
  );
  const bearing = atlasDelta
    ? {
        ...baseBearing,
        detail: `${baseBearing.detail} · atlas ${shortCoordinate(atlasDelta.source_revision)} → ${shortCoordinate(atlasDelta.target_revision)} · +${atlasDelta.inserted} −${atlasDelta.removed} · ${atlasDelta.moved} moved · ${atlasDelta.interest_changed} interest changed · ${atlasDelta.merged} merged · ${atlasDelta.split} split`,
      }
    : baseBearing;
  const detail = buildSurveySceneDetails(selected, focusPoint, focusId, regime);
  const visibleRegions =
    regime === "atlas" || regime === "landscape"
      ? layout.regions
      : regime === "neighborhoods"
        ? layout.regions.filter((region) => region.variant === "map-boundary")
        : [];
  const anchorCount = topographies.reduce(
    (count, { patch }) => count + patch.anchors.length,
    0,
  );
  const streamCount = layout.natural_features.filter(
    (feature) => feature.kind === "stream" || feature.kind === "river",
  ).length;
  const weatherFrontCount = layout.natural_features.filter(
    (feature) => feature.kind === "weather_front",
  ).length;
  const regimeCopy = {
    world: {
      label: "CONTEXT WORLD",
      detail: `${topographies.length} admitted chart${topographies.length === 1 ? "" : "s"} · overview terrain LOD · ${streamCount} emergent water systems · ${weatherFrontCount} boundary weather fronts`,
    },
    atlas: {
      label: "ANCHOR RELIEF ATLAS",
      detail: `${anchorCount} admitted anchors shape ${layout.contours.length} contour levels across ${topographies.length} regional terrain scene${topographies.length === 1 ? "" : "s"}`,
    },
    landscape: {
      label: "ANCHOR TERRAIN",
      detail: `${anchorCount} anchor stations · regional terrain LOD · anchor-only relief with projected runoff and erosion · surveyed boundaries visible`,
    },
    neighborhoods: {
      label: "ANCHOR NEIGHBORHOOD",
      detail: `${selected.workload.workload.id} · local terrain LOD · survey conditions over persistent relief`,
    },
    objects: {
      label: "ANCHOR OBJECTS",
      detail: focusPoint?.detail ?? selected.patch.topography_revision,
    },
    evidence: {
      label: "ANCHOR EVIDENCE",
      detail: `${selected.workload.workload.id} · patch ${selected.patch.patch_id} · exact retained basis`,
    },
  }[regime];
  const globe =
    regime === "world" && semanticAtlas
      ? buildSemanticGlobe(semanticAtlas, layout.points)
      : null;
  const sceneWorld = globe
    ? {
        width: Math.max(
          ...topographies.map(({ projection }) => projection.extent.width),
        ),
        height: Math.max(
          ...topographies.map(({ projection }) => projection.extent.height),
        ),
      }
    : layout.world;
  return {
    regime,
    ...regimeCopy,
    focus_id: focusId,
    regions: visibleRegions,
    landforms: layout.landforms,
    contours: layout.contours,
    natural_features: layout.natural_features,
    points: layout.points,
    nodes: detail.nodes,
    // Evidence detail associations are not geographic paths. A traversable
    // path requires its own admitted contract.
    edges: [],
    omissions: layout.omissions,
    bearing,
    world: sceneWorld,
    fit_world: globe
      ? sceneWorld
      : {
          width: selected.projection.extent.width,
          height: selected.projection.extent.height,
        },
    terrain: true,
    terrain_fields: layout.terrain_fields,
    terrain_programs: layout.terrain_programs,
    globe,
  };
}

function buildSemanticGlobe(
  atlas: SemanticAtlas,
  points: TopologyPointOfInterest[],
): TopologyGlobe {
  const workspaceFocus = new Map(
    points
      .filter(
        (point) => point.kind === "anchor" && point.family === "WORKSPACE",
      )
      .map((point) => [point.workload_id, point.focus_id]),
  );
  return {
    schema: "rey.semantic-globe-scene.v1",
    posture: "semantic_atlas",
    globe_id: atlas.atlas_id,
    source_revision: atlas.atlas_revision,
    compiler_revision: atlas.compiler.semantic_digest,
    coordinate_authority: atlas.coordinate_system.authority,
    clusters: atlas.clusters.map((cluster) => ({
      id: cluster.cluster_id,
      longitude_degrees: cluster.semantic_longitude_microdegrees / 1_000_000,
      latitude_degrees: cluster.semantic_latitude_microdegrees / 1_000_000,
      angular_radius_degrees: cluster.angular_radius_microdegrees / 1_000_000,
      member_count: cluster.member_region_ids.length,
      dominant_feature: cluster.dominant_feature,
    })),
    regions: atlas.regions.map((region) => ({
      id: region.region_id,
      cluster_id: region.cluster_id,
      focus_id:
        workspaceFocus.get(region.workload_id) ??
        `topography:${region.workload_id}`,
      workload_id: region.workload_id,
      label: region.workload_id,
      detail: `${region.anchor_count} anchors · ${region.frontier_rows} frontier rows · ${region.dominant_feature.replaceAll("_", " ")} terrain · ${shortCoordinate(region.source_topography_revision)}`,
      longitude_degrees: region.semantic_longitude_microdegrees / 1_000_000,
      latitude_degrees: region.semantic_latitude_microdegrees / 1_000_000,
      angular_radius_degrees: region.angular_radius_microdegrees / 1_000_000,
      tone:
        region.frontier_rows > 0
          ? "frontier"
          : region.complete
            ? "healthy"
            : "omitted",
    })),
    beacons: [],
  };
}

function buildSurveySceneDetails(
  selected: AdmittedTopography,
  focusPoint: TopologyPointOfInterest | undefined,
  focusId: string,
  regime: LensRegime,
): { edges: TopologyEdge[]; nodes: TopologyNode[] } {
  if (
    !focusPoint ||
    regime === "world" ||
    regime === "atlas" ||
    regime === "landscape" ||
    regime === "neighborhoods"
  )
    return { edges: [], nodes: [] };
  const { workload, patch } = selected;
  const coordinate = focusPoint.coordinate_uri;
  const exactResolution = patch.resolutions.find(
    (candidate) => candidate.coordinate?.coordinate === coordinate,
  );
  const relatedEdges = coordinate
    ? patch.edges.filter(
        (candidate) =>
          candidate.source_coordinate === coordinate ||
          candidate.target_coordinate === coordinate,
      )
    : [];
  const offset = regime === "objects" ? { x: 68, y: 52 } : { x: 48, y: 42 };
  const nodes: TopologyNode[] = [];
  const edges: TopologyEdge[] = [];
  const addDetail = (
    id: string,
    family: string,
    label: string,
    detail: string,
    xDirection: -1 | 1,
    yDirection: -1 | 1,
    tone: TopologyTone,
    coordinateUri?: string,
  ) => {
    nodes.push({
      id,
      focus_id: focusId,
      family,
      label,
      detail,
      x: focusPoint.x + offset.x * xDirection,
      y: focusPoint.y + offset.y * yDirection,
      width: 220,
      tone,
      workload_id: workload.workload.id,
      coordinate_uri: coordinateUri,
    });
    edges.push({
      id: `${focusPoint.id}:${id}`,
      from: focusPoint.id,
      to: id,
      kind: family.startsWith("LINEAGE") ? "observes" : "produces",
      label: family.toLowerCase(),
    });
  };

  if (regime === "objects") {
    addDetail(
      "terrain-object-patch",
      "ADMITTED PATCH",
      shortCoordinate(patch.patch_id),
      `${patch.complete ? "complete" : "bounded"} · ${patch.operation.id}@${patch.operation.revision}`,
      -1,
      -1,
      patch.complete ? "healthy" : "omitted",
    );
    addDetail(
      "terrain-object-revision",
      "SOURCE REVISION",
      shortCoordinate(
        exactResolution?.source_revision ?? patch.topography_revision,
      ),
      "revision-bound semantic identity",
      1,
      -1,
      "healthy",
    );
    addDetail(
      "terrain-object-resolution",
      "LOCATOR OUTCOME",
      exactResolution?.status ??
        (focusPoint.kind === "frontier" ? "unresolved" : "resolved"),
      exactResolution?.detail ??
        `${relatedEdges.length} retained relationships`,
      -1,
      1,
      exactResolution
        ? resolutionTone(exactResolution.status)
        : focusPoint.tone,
      coordinate,
    );
    addDetail(
      "terrain-object-delta",
      "DIRECTED PATCH",
      `+${patch.delta.inserted} −${patch.delta.deleted} ~${patch.delta.modified}`,
      `${shortCoordinate(patch.delta.source_revision)} → ${shortCoordinate(patch.delta.target_revision)}`,
      1,
      1,
      "frontier",
    );
    return { edges, nodes };
  }

  const evidenceRows = [
    ...patch.resolutions
      .filter(
        (candidate) =>
          candidate.coordinate?.coordinate === coordinate ||
          candidate.candidate === focusPoint.label,
      )
      .slice(0, 2)
      .map((candidate) => ({
        family: "LOCATOR EVIDENCE",
        label: candidate.candidate,
        detail: `${candidate.status} · ${candidate.detail}`,
        tone: resolutionTone(candidate.status),
        coordinate: candidate.coordinate?.coordinate,
      })),
    ...patch.lineage.slice(0, 2).map((lineage) => ({
      family: `LINEAGE / ${lineage.kind}`,
      label: lineage.identity,
      detail: lineage.revision,
      tone: "neutral" as TopologyTone,
      coordinate: undefined,
    })),
  ].slice(0, 4);
  evidenceRows.forEach((candidate, index) =>
    addDetail(
      `terrain-evidence:${index}`,
      candidate.family,
      candidate.label,
      candidate.detail,
      index % 2 === 0 ? -1 : 1,
      index < 2 ? -1 : 1,
      candidate.tone,
      candidate.coordinate,
    ),
  );
  return { edges, nodes };
}

function buildSurveyBearing(
  points: TopologyPointOfInterest[],
  focusPoint: TopologyPointOfInterest | undefined,
  patch: TopographyPatch,
): TopologyBearing {
  const sampledConditions = patch.seeds.reduce(
    (total, seed) => total + seed.candidate_count,
    0,
  );
  const frontierCount = patch.frontier.length;
  if (!focusPoint) {
    return {
      status: "world",
      label: "SURVEY WEATHER",
      detail: `${sampledConditions} admitted local conditions · ${frontierCount} unresolved boundary front${frontierCount === 1 ? "" : "s"} · no path has been discovered or built`,
      sampled_conditions: sampledConditions,
      unresolved_boundaries: frontierCount,
    };
  }
  const origin =
    points.find(
      (point) =>
        point.workload_id === focusPoint.workload_id &&
        point.family === "WORKSPACE",
    ) ?? points[0];
  if (!origin || origin.id === focusPoint.id) {
    return {
      status: "world",
      label: "SURVEY ORIGIN",
      detail: `${frontierCount} retained boundary condition${frontierCount === 1 ? "" : "s"}; selection does not reshape terrain`,
      sampled_conditions: sampledConditions,
      unresolved_boundaries: frontierCount,
    };
  }
  if (focusPoint.kind === "frontier") {
    return {
      status: "probe_required",
      label: "BOUNDARY PROBE REQUIRED",
      detail: `${focusPoint.signal}; its weather front marks conditions only and supplies no route`,
      sampled_conditions: sampledConditions,
      unresolved_boundaries: 1,
    };
  }
  return {
    status: "charted",
    label: "SAMPLED STATION",
    detail: `${focusPoint.signal}; no curation path is implied by source relationships`,
    sampled_conditions: sampledConditions,
    unresolved_boundaries: frontierCount,
  };
}

function selectTopography(
  topographies: AdmittedTopography[],
  focusId: string,
): AdmittedTopography {
  return (
    topographies.find(({ workload }) =>
      ["topography", "seed", "anchor", "frontier"].some((kind) =>
        focusId.startsWith(`${kind}:${workload.workload.id}`),
      ),
    ) ?? topographies[0]!
  );
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

function shortCoordinate(value: string): string {
  const coordinate = value.startsWith("blake3:") ? value.slice(7) : value;
  return coordinate.length > 22
    ? `${coordinate.slice(0, 10)}…${coordinate.slice(-8)}`
    : coordinate;
}
