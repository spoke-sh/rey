import type {
  AgentSummary,
  AttentionRow,
  ProjectionPacket,
  SceneAdmission,
  SceneGeometry,
  SceneProjectedFeature,
  SemanticAtlas,
  WorkloadDraft,
  WorkloadList,
  WorkloadSummary,
  TopographyPatch,
  TopographyRegionState,
} from "./domain";
import { deriveAgentIndex } from "./domain";
import {
  DEFAULT_LENS_ZOOM,
  lensRegimeForZoom,
  type LensRegime,
} from "./explore/engine/camera";
import {
  admittedTopographies,
  type AdmittedTopography,
} from "./explore/projection/topography-projector";
import { admittedScenes } from "./explore/projection/scene-projector";
import {
  fieldPoint,
  type MaskField2D,
  type ScalarField2D,
} from "./explore/engine/fields";
import {
  compileTerrainFieldPyramid,
  terrainFieldForRegime,
  type TerrainFieldPyramid,
  type TerrainFieldSet,
} from "./explore/terrain/compile";

export {
  DEFAULT_LENS_ZOOM,
  EVIDENCE_LENS_ZOOM,
  LANDSCAPE_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  WORLD_LENS_ZOOM,
  clampLensZoom,
  lensRegimeForZoom,
  stepLensZoom,
} from "./explore/engine/camera";
export type { LensRegime } from "./explore/engine/camera";

export type TopologyTone =
  | "neutral"
  | "accent"
  | "healthy"
  | "attention"
  | "blocked"
  | "unknown"
  | "omitted"
  | "stale"
  | "unsupported"
  | "frontier";

export interface TopologyWorld {
  width: number;
  height: number;
}

export interface TopologyGlobeRegion {
  id: string;
  cluster_id: string;
  focus_id: string;
  workload_id: string;
  label: string;
  detail: string;
  longitude_degrees: number;
  latitude_degrees: number;
  angular_radius_degrees: number;
  tone: TopologyTone;
}

export interface TopologyGlobeCluster {
  id: string;
  longitude_degrees: number;
  latitude_degrees: number;
  angular_radius_degrees: number;
  member_count: number;
  dominant_feature: string;
}

export interface TopologyGlobe {
  schema: "rey.semantic-globe-scene.v1";
  atlas_id: string;
  atlas_revision: string;
  compiler_revision: string;
  coordinate_authority: string;
  regions: TopologyGlobeRegion[];
  clusters: TopologyGlobeCluster[];
}

interface TopologyPosition {
  x: number;
  y: number;
}

export interface TopologyRegion {
  id: string;
  label: string;
  detail: string;
  x: number;
  y: number;
  width: number;
  height: number;
  tone: TopologyTone;
  variant?: "panel" | "map-boundary" | "map-zone";
}

export interface TopologyContour {
  id: string;
  path: string;
  level: number;
  threshold: number;
  anchor_count: number;
}

export interface TopologyLandform {
  id: string;
  path: string;
  kind: "charted" | "horizon";
  label: string;
  detail: string;
  tone: TopologyTone;
}

export interface TopologyNaturalFeature {
  id: string;
  path: string;
  kind: "stream" | "river" | "weather_front" | "authored_line";
  label: string;
  detail: string;
  intensity: number;
  workload_id: string;
}

export interface TopologyBearing {
  status: "world" | "charted" | "probe_required" | "isolated";
  label: string;
  detail: string;
  sampled_conditions: number;
  unresolved_boundaries: number;
}

export interface TopologyPointOfInterest {
  id: string;
  focus_id: string;
  kind: "anchor" | "frontier";
  family: string;
  label: string;
  detail: string;
  x: number;
  y: number;
  prominence: number;
  signal: string;
  action: string;
  tone: TopologyTone;
  workload_id: string;
  coordinate_uri?: string;
}

export interface TopologyNode {
  id: string;
  focus_id: string;
  family: string;
  label: string;
  detail: string;
  x: number;
  y: number;
  width: number;
  tone: TopologyTone;
  workload_id?: string;
  coordinate_uri?: string;
}

export interface TopologyEdge {
  id: string;
  from: string;
  to: string;
  kind: "contains" | "directs" | "produces" | "observes" | "depends";
  label: string;
}

export interface TopologyScene {
  regime: LensRegime;
  label: string;
  detail: string;
  focus_id: string;
  regions: TopologyRegion[];
  landforms: TopologyLandform[];
  contours: TopologyContour[];
  natural_features: TopologyNaturalFeature[];
  points: TopologyPointOfInterest[];
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  omissions: string[];
  bearing: TopologyBearing;
  world: TopologyWorld;
  fit_world: TopologyWorld;
  terrain: boolean;
  terrain_fields: TerrainFieldSet[];
  terrain_pyramids: TerrainFieldPyramid[];
  globe: TopologyGlobe | null;
}

export const TOPOLOGY_WORLD = { width: 1200, height: 720 } as const;

const NEIGHBORHOOD_LIMIT = 8;

export function buildTopologyScene(
  portfolio: WorkloadList,
  zoom: number,
  focusId = "cluster:portfolio",
  retainedRegime?: LensRegime,
): TopologyScene {
  const regime = retainedRegime ?? lensRegimeForZoom(zoom);
  const scenes = admittedScenes(portfolio);
  if (scenes.length > 0)
    return {
      ...buildAdmittedEditorScene(scenes, focusId, regime),
      contours: [],
      terrain_fields: [],
      terrain_pyramids: [],
      globe: null,
    };
  let projection: TopologyProjection;
  if (regime === "world") projection = buildWorld(portfolio, focusId);
  else if (regime === "atlas") projection = buildAtlas(portfolio, focusId);
  else if (regime === "landscape")
    projection = buildLandscape(portfolio, focusId);
  else if (regime === "neighborhoods")
    projection = buildNeighborhoods(portfolio, focusId);
  else if (regime === "objects") projection = buildObjects(portfolio, focusId);
  else projection = buildEvidence(portfolio, focusId);
  return {
    ...projection,
    landforms: projection.landforms ?? [],
    contours: projection.contours ?? [],
    natural_features: projection.natural_features ?? [],
    points: projection.points ?? [],
    bearing: projection.bearing ?? {
      status: "isolated",
      label: "NO CHARTED BEARING",
      detail: "no admitted survey geometry",
      sampled_conditions: 0,
      unresolved_boundaries: 0,
    },
    terrain: projection.terrain ?? false,
    terrain_fields: projection.terrain_fields ?? [],
    terrain_pyramids: projection.terrain_pyramids ?? [],
    globe: projection.globe ?? null,
    world: projection.world ?? topologyWorld(projection),
    fit_world:
      projection.fit_world ?? projection.world ?? topologyWorld(projection),
  };
}

function buildAdmittedEditorScene(
  admissions: SceneAdmission[],
  focusId: string,
  regime: LensRegime,
): TopologyScene {
  const projections = admissions.map((admission) => admission.projection);
  const bounds = projections
    .map((projection) => projection.bounds)
    .filter((value) => value !== null);
  const extent = bounds.reduce(
    (combined, value) => ({
      west: Math.min(combined.west, value.west),
      south: Math.min(combined.south, value.south),
      east: Math.max(combined.east, value.east),
      north: Math.max(combined.north, value.north),
    }),
    bounds[0] ?? { west: -1, south: -1, east: 1, north: 1 },
  );
  const project = sceneCoordinateProjector(extent);
  const features = projections.flatMap((projection) =>
    projection.features.map((feature) => ({
      feature,
      projectId: projection.project_id,
    })),
  );
  const landforms = features.flatMap(({ feature }) => {
    const path = geometryPath(feature.geometry, project);
    return path && geometryHasSurface(feature.geometry)
      ? [
          {
            id: `scene-landform:${feature.feature_id}`,
            path,
            kind: "charted" as const,
            label: feature.label,
            detail:
              feature.detail || `${feature.role} · ${feature.geometry_kind}`,
            tone: sceneRoleTone(feature),
          },
        ]
      : [];
  });
  const naturalFeatures = features.flatMap(({ feature, projectId }) => {
    const path = geometryPath(feature.geometry, project);
    if (!path || !geometryHasLine(feature.geometry)) return [];
    return [
      {
        id: `scene-line:${feature.feature_id}`,
        path,
        kind:
          feature.role === "hydrology"
            ? ("river" as const)
            : ("authored_line" as const),
        label: feature.label,
        detail: feature.detail || `${feature.role} · ${feature.geometry_kind}`,
        intensity: feature.role === "hydrology" ? 0.88 : 0.58,
        workload_id: `scene:${projectId}`,
      },
    ];
  });
  const points = features.flatMap(({ feature, projectId }) => {
    const positions = geometryPositions(feature.geometry);
    if (positions.length === 0) return [];
    const center = positions.reduce(
      (sum, position) => ({
        x: sum.x + position[0],
        y: sum.y + position[1],
      }),
      { x: 0, y: 0 },
    );
    const position = project([
      center.x / positions.length,
      center.y / positions.length,
    ]);
    return [
      {
        id: `scene-point:${feature.feature_id}`,
        focus_id: `scene-feature:${feature.feature_id}`,
        kind: "anchor" as const,
        family: feature.role.toUpperCase(),
        label: feature.label,
        detail: feature.detail || `${feature.role} · ${feature.geometry_kind}`,
        x: position.x,
        y: position.y,
        prominence: feature.marker
          ? Math.max(0.4, feature.marker.collision_priority / 1_000)
          : 0.45,
        signal: feature.category ?? feature.geometry_kind,
        action: "inspect admitted scene evidence",
        tone: sceneRoleTone(feature),
        workload_id: `scene:${projectId}`,
      },
    ];
  });
  const selected = points.find((point) => point.focus_id === focusId);
  const omissions = projections.flatMap((projection) => projection.omissions);
  return {
    regime,
    label:
      selected?.label ??
      projections.map(({ project_id }) => project_id).join(" + "),
    detail:
      selected?.detail ??
      `${admissions.length} validated scene admission${admissions.length === 1 ? "" : "s"} · ${features.length} exact native features`,
    focus_id: focusId,
    regions: [],
    landforms,
    contours: [],
    natural_features: naturalFeatures,
    points,
    nodes: [],
    edges: [],
    omissions,
    bearing: {
      status: "charted",
      label: "VALIDATED EDITOR SCENE",
      detail: `${features.length} features admitted from ${projections.length} projection${projections.length === 1 ? "" : "s"}`,
      sampled_conditions: features.length,
      unresolved_boundaries: omissions.length,
    },
    world: TOPOLOGY_WORLD,
    fit_world: TOPOLOGY_WORLD,
    terrain: true,
    terrain_fields: [],
    terrain_pyramids: [],
    globe: null,
  };
}

type ScenePosition = readonly [number, number];

function sceneCoordinateProjector(bounds: {
  west: number;
  south: number;
  east: number;
  north: number;
}): (position: ScenePosition) => TopologyPosition {
  const margin = 54;
  const width = Math.max(bounds.east - bounds.west, Number.EPSILON);
  const height = Math.max(bounds.north - bounds.south, Number.EPSILON);
  const scale = Math.min(
    (TOPOLOGY_WORLD.width - margin * 2) / width,
    (TOPOLOGY_WORLD.height - margin * 2) / height,
  );
  const drawnWidth = width * scale;
  const drawnHeight = height * scale;
  const offsetX = (TOPOLOGY_WORLD.width - drawnWidth) / 2;
  const offsetY = (TOPOLOGY_WORLD.height - drawnHeight) / 2;
  return ([longitude, latitude]) => ({
    x: offsetX + (longitude - bounds.west) * scale,
    y: offsetY + (bounds.north - latitude) * scale,
  });
}

function geometryPath(
  geometry: SceneGeometry,
  project: (position: ScenePosition) => TopologyPosition,
): string {
  const sequence = (coordinates: unknown, close = false): string => {
    if (!Array.isArray(coordinates)) return "";
    const positions = coordinates.flatMap((value) => {
      const position = scenePosition(value);
      return position ? [project(position)] : [];
    });
    if (positions.length === 0) return "";
    return `${positions
      .map(
        (position, index) =>
          `${index === 0 ? "M" : "L"}${position.x.toFixed(2)},${position.y.toFixed(2)}`,
      )
      .join(" ")}${close ? " Z" : ""}`;
  };
  const coordinates = geometry.coordinates;
  if (geometry.type === "LineString") return sequence(coordinates);
  if (geometry.type === "MultiLineString" && Array.isArray(coordinates))
    return coordinates
      .map((line) => sequence(line))
      .filter(Boolean)
      .join(" ");
  if (geometry.type === "Polygon" && Array.isArray(coordinates))
    return coordinates
      .map((ring) => sequence(ring, true))
      .filter(Boolean)
      .join(" ");
  if (geometry.type === "MultiPolygon" && Array.isArray(coordinates))
    return coordinates
      .flatMap((polygon) =>
        Array.isArray(polygon)
          ? polygon.map((ring) => sequence(ring, true))
          : [],
      )
      .filter(Boolean)
      .join(" ");
  if (geometry.type === "GeometryCollection")
    return (geometry.geometries ?? [])
      .map((nested) => geometryPath(nested, project))
      .filter(Boolean)
      .join(" ");
  return "";
}

function geometryPositions(geometry: SceneGeometry): ScenePosition[] {
  if (geometry.type === "GeometryCollection")
    return (geometry.geometries ?? []).flatMap(geometryPositions);
  const positions: ScenePosition[] = [];
  const visit = (value: unknown) => {
    const position = scenePosition(value);
    if (position) {
      positions.push(position);
      return;
    }
    if (Array.isArray(value)) value.forEach(visit);
  };
  visit(geometry.coordinates);
  return positions;
}

function scenePosition(value: unknown): ScenePosition | null {
  return Array.isArray(value) &&
    value.length >= 2 &&
    typeof value[0] === "number" &&
    typeof value[1] === "number"
    ? [value[0], value[1]]
    : null;
}

function geometryHasSurface(geometry: SceneGeometry): boolean {
  return (
    geometry.type === "Polygon" ||
    geometry.type === "MultiPolygon" ||
    (geometry.type === "GeometryCollection" &&
      (geometry.geometries ?? []).some(geometryHasSurface))
  );
}

function geometryHasLine(geometry: SceneGeometry): boolean {
  return (
    geometry.type === "LineString" ||
    geometry.type === "MultiLineString" ||
    (geometry.type === "GeometryCollection" &&
      (geometry.geometries ?? []).some(geometryHasLine))
  );
}

function sceneRoleTone(feature: SceneProjectedFeature): TopologyTone {
  if (feature.role === "markers") return "attention";
  if (feature.role === "hydrology") return "healthy";
  if (feature.role === "terrain_control") return "accent";
  if (feature.role === "boundary") return "neutral";
  return "unknown";
}

type TopologyProjection = Omit<
  TopologyScene,
  | "bearing"
  | "contours"
  | "fit_world"
  | "globe"
  | "landforms"
  | "natural_features"
  | "points"
  | "terrain"
  | "terrain_fields"
  | "terrain_pyramids"
  | "world"
> & {
  bearing?: TopologyBearing;
  contours?: TopologyContour[];
  landforms?: TopologyLandform[];
  natural_features?: TopologyNaturalFeature[];
  points?: TopologyPointOfInterest[];
  terrain?: boolean;
  terrain_fields?: TerrainFieldSet[];
  terrain_pyramids?: TerrainFieldPyramid[];
  world?: TopologyWorld;
  fit_world?: TopologyWorld;
  globe?: TopologyGlobe | null;
};

function buildWorld(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length > 0)
    return buildSurveyTerrain(
      topographies,
      focusId,
      "world",
      portfolio.semantic_atlas ?? null,
    );
  const fallback = buildAtlas(portfolio, focusId);
  return {
    ...fallback,
    regime: "world",
    label: "UNCHARTED CONTEXT WORLD",
    detail: "no admitted survey boundary or world geometry",
    omissions: [
      "world geometry is unavailable until a survey workload patch is admitted",
      ...fallback.omissions,
    ],
  };
}

function buildAtlas(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length === 0) {
    const fallback = buildLandscape(portfolio, focusId);
    return {
      ...fallback,
      regime: "atlas",
      label: "CONTEXT ATLAS",
      detail: "no admitted survey patch",
      omissions: [
        "topography is unexplored until a survey workload patch is admitted",
        ...fallback.omissions,
      ],
    };
  }
  return buildSurveyTerrain(topographies, focusId, "atlas");
}

function buildLandscape(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length > 0)
    return buildSurveyTerrain(topographies, focusId, "landscape");
  const miningResults = portfolio.workloads.reduce(
    (total, workload) => total + workload.mining_results,
    0,
  );
  const reasoningSurfaces = portfolio.workloads.reduce(
    (total, workload) => total + workload.reasoning_surfaces,
    0,
  );
  const attention = portfolio.attention.rows.length;
  const agents = deriveAgentIndex(portfolio);

  return {
    regime: "landscape",
    label: "CONTEXT LANDSCAPE",
    detail: "bounded portfolio topology",
    focus_id: focusId,
    regions: [],
    nodes: [
      node(
        "context",
        "cluster:context",
        "CONTEXT",
        "Declared surfaces",
        `${portfolio.attention.summary.surfaces} mapped · ${portfolio.attention.summary.unowned_surfaces} unowned`,
        190,
        180,
        245,
        portfolio.attention.summary.unowned_surfaces > 0
          ? "attention"
          : "neutral",
      ),
      node(
        "workloads",
        "cluster:workloads",
        "WORKLOADS",
        "Compute neighborhoods",
        `${portfolio.catalog.admitted_count} admitted · ${portfolio.catalog.draft_count} draft`,
        585,
        155,
        285,
        "accent",
      ),
      node(
        "evidence",
        "cluster:evidence",
        "EVIDENCE",
        "Mined observations",
        `${miningResults} results · ${reasoningSurfaces} reasoning surfaces`,
        980,
        245,
        275,
        "neutral",
      ),
      node(
        "attention",
        "cluster:attention",
        "ATTENTION",
        "Directed frontier",
        `${attention} unresolved row${attention === 1 ? "" : "s"}`,
        865,
        555,
        275,
        attention > 0 ? "attention" : "healthy",
      ),
      node(
        "agents",
        "cluster:agents",
        "AGENTS",
        "Exact producer coordinates",
        `${agents.length} identified · ${portfolio.catalog.draft_count} unassigned`,
        315,
        555,
        265,
        portfolio.catalog.draft_count > 0 ? "blocked" : "neutral",
      ),
      node(
        "portfolio",
        "cluster:portfolio",
        "PORTFOLIO",
        "Current coordinate",
        shortCoordinate(portfolio.attention.source_snapshot_id),
        585,
        355,
        245,
        "healthy",
      ),
    ],
    edges: [
      edge("context-workloads", "context", "workloads", "observes", "binds"),
      edge("workloads-evidence", "workloads", "evidence", "produces", "mines"),
      edge("evidence-attention", "evidence", "attention", "produces", "diffs"),
      edge(
        "attention-portfolio",
        "attention",
        "portfolio",
        "directs",
        "orients",
      ),
      edge(
        "portfolio-workloads",
        "portfolio",
        "workloads",
        "directs",
        "selects",
      ),
      edge("agents-workloads", "agents", "workloads", "produces", "proposes"),
      edge("context-portfolio", "context", "portfolio", "contains", "bounds"),
    ],
    omissions: [],
  };
}

function buildNeighborhoods(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length > 0 && !focusId.startsWith("agent:"))
    return buildSurveyTerrain(topographies, focusId, "neighborhoods");
  const workloadCandidates: Array<WorkloadSummary | WorkloadDraft> = [
    ...portfolio.workloads,
    ...portfolio.drafts,
  ];
  const agentCandidates = deriveAgentIndex(portfolio);
  const showAgents =
    focusId === "cluster:agents" || focusId.startsWith("agent:");
  const candidates = showAgents ? agentCandidates : workloadCandidates;
  const visibleCandidates = candidates.slice(0, NEIGHBORHOOD_LIMIT);
  const visibleAttention = portfolio.attention.rows.slice(
    0,
    NEIGHBORHOOD_LIMIT,
  );
  const nodes: TopologyNode[] = [];

  visibleCandidates.forEach((candidate, index) => {
    if ("producer" in candidate) {
      nodes.push(
        node(
          `agent:${candidate.id}`,
          `agent:${candidate.id}`,
          "AGENT",
          candidate.producer,
          `${candidate.kind.replaceAll("_", " ")} · ${candidate.producer_revision} · ${candidate.workload_ids.length} outputs`,
          205 + (index % 2) * 255,
          150 + Math.floor(index / 2) * 150,
          220,
          candidate.attention_rows > 0 ? "attention" : "healthy",
        ),
      );
      return;
    }
    const isDraft = "request" in candidate;
    const workloadId = isDraft
      ? candidate.request.workload_id
      : candidate.workload.id;
    const qualification = isDraft ? "draft" : candidate.qualification;
    nodes.push(
      node(
        `workload:${workloadId}`,
        `workload:${workloadId}`,
        isDraft ? "REQUEST" : "WORKLOAD",
        workloadId,
        isDraft
          ? "awaiting coding harness"
          : `${candidate.passed}/${candidate.required} scenarios · ${candidate.qualification}`,
        205 + (index % 2) * 255,
        150 + Math.floor(index / 2) * 150,
        220,
        isDraft ? "blocked" : qualificationTone(qualification),
        workloadId,
      ),
    );
  });

  visibleAttention.forEach((row, index) => {
    nodes.push(
      node(
        `attention:${row.row_id}`,
        `attention:${row.row_id}`,
        row.subject_kind === "surface" ? "SURFACE" : "ATTENTION",
        row.subject_id,
        `${row.action} · ${row.reason.replaceAll("_", " ")}`,
        750 + (index % 2) * 255,
        150 + Math.floor(index / 2) * 150,
        220,
        row.readiness === "blocked"
          ? "blocked"
          : row.readiness === "excluded"
            ? "neutral"
            : "attention",
      ),
    );
  });

  const visibleIds = new Set(nodes.map((candidate) => candidate.id));
  const edges = visibleAttention.flatMap((row) => {
    const from = `attention:${row.row_id}`;
    const to = `workload:${row.subject_id}`;
    return visibleIds.has(to)
      ? [edge(`${from}-${to}`, from, to, "directs", row.action)]
      : [];
  });
  const omissions: string[] = [];
  const candidateOmissions = candidates.length - visibleCandidates.length;
  const attentionOmissions =
    portfolio.attention.rows.length - visibleAttention.length;
  if (candidateOmissions > 0)
    omissions.push(
      `${candidateOmissions} ${showAgents ? "agent" : "workload"} neighborhoods omitted`,
    );
  if (attentionOmissions > 0)
    omissions.push(`${attentionOmissions} attention neighborhoods omitted`);

  return {
    regime: "neighborhoods",
    label: "CONTEXT NEIGHBORHOODS",
    detail: `${candidates.length} ${showAgents ? "agent" : "workload"} · ${portfolio.attention.rows.length} attention`,
    focus_id: focusId,
    regions: [
      {
        id: "workload-region",
        label: showAgents ? "AGENT NEIGHBORHOODS" : "WORKLOAD NEIGHBORHOODS",
        detail: `${candidates.length} bounded ${showAgents ? "generator" : "compute"} contexts`,
        x: 55,
        y: 55,
        width: 520,
        height: 610,
        tone: "accent",
      },
      {
        id: "attention-region",
        label: "ATTENTION NEIGHBORHOODS",
        detail: `${portfolio.attention.rows.length} directed deltas`,
        x: 625,
        y: 55,
        width: 520,
        height: 610,
        tone: portfolio.attention.rows.length > 0 ? "attention" : "healthy",
      },
    ],
    nodes,
    edges,
    omissions,
  };
}

function buildObjects(
  portfolio: WorkloadList,
  requestedFocusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (
    topographies.length > 0 &&
    (requestedFocusId.startsWith("topography:") ||
      requestedFocusId.startsWith("seed:") ||
      requestedFocusId.startsWith("anchor:") ||
      requestedFocusId.startsWith("frontier:"))
  )
    return buildSurveyTerrain(topographies, requestedFocusId, "objects");
  const focusId = resolveObjectFocus(portfolio, requestedFocusId);
  if (focusId.startsWith("workload:")) {
    const workloadId = focusId.slice("workload:".length);
    const workload = portfolio.workloads.find(
      (candidate) => candidate.workload.id === workloadId,
    );
    const draft = portfolio.drafts.find(
      (candidate) => candidate.request.workload_id === workloadId,
    );
    if (workload) return workloadObjectScene(portfolio, workload, focusId);
    if (draft) return draftObjectScene(portfolio, draft, focusId);
  }
  if (focusId.startsWith("attention:")) {
    const rowId = focusId.slice("attention:".length);
    const row = portfolio.attention.rows.find(
      (candidate) => candidate.row_id === rowId,
    );
    if (row) return attentionObjectScene(portfolio, row, focusId);
  }
  if (focusId.startsWith("agent:")) {
    const agentId = focusId.slice("agent:".length);
    const agent = deriveAgentIndex(portfolio).find(
      (candidate) => candidate.id === agentId,
    );
    if (agent) return agentObjectScene(portfolio, agent, focusId);
  }
  return portfolioObjectScene(portfolio, focusId);
}

function agentObjectScene(
  portfolio: WorkloadList,
  agent: AgentSummary,
  focusId: string,
): TopologyProjection {
  const outputs = portfolio.workloads.filter((workload) =>
    agent.workload_ids.includes(workload.workload.id),
  );
  const attention = outputs.reduce(
    (total, workload) => total + workload.attention_rows,
    0,
  );
  return {
    regime: "objects",
    label: "AGENT OBJECTS",
    detail: `${agent.producer} / ${agent.producer_revision}`,
    focus_id: focusId,
    regions: [
      {
        id: "agent-boundary",
        label: "GENERATOR PROVENANCE",
        detail: agent.id,
        x: 45,
        y: 55,
        width: 1110,
        height: 610,
        tone: attention > 0 ? "attention" : "healthy",
      },
    ],
    nodes: [
      node(
        "agent-context-object",
        "cluster:context",
        "PROVENANCE",
        agent.kind.replaceAll("_", " "),
        "admitted workload package declarations",
        150,
        355,
        215,
        "neutral",
      ),
      node(
        "agent-object",
        focusId,
        "AGENT",
        agent.producer,
        agent.id,
        405,
        355,
        225,
        "accent",
      ),
      node(
        "agent-revision-object",
        focusId,
        "REVISION",
        agent.producer_revision,
        "exact artifact producer revision",
        675,
        205,
        235,
        "healthy",
      ),
      node(
        "agent-output-object",
        outputs[0] ? `workload:${outputs[0].workload.id}` : "cluster:workloads",
        "ADMITTED OUTPUTS",
        `${outputs.length} workload${outputs.length === 1 ? "" : "s"}`,
        outputs[0]?.workload.id ?? "no retained output",
        675,
        505,
        235,
        outputs.length > 0 ? "healthy" : "blocked",
        outputs[0]?.workload.id,
      ),
      node(
        "agent-scenario-object",
        focusId,
        "FROZEN ORACLES",
        `${agent.scenarios_passed}/${agent.scenarios_required} passing`,
        "qualification remains runtime-owned",
        960,
        205,
        225,
        agent.scenarios_passed === agent.scenarios_required &&
          agent.scenarios_required > 0
          ? "healthy"
          : "attention",
      ),
      node(
        "agent-attention-object",
        "cluster:attention",
        "ATTENTION",
        `${attention} directed row${attention === 1 ? "" : "s"}`,
        "agent provenance cannot resolve runtime deltas",
        960,
        505,
        225,
        attention > 0 ? "attention" : "healthy",
      ),
    ],
    edges: [
      edge(
        "agent-context-agent-object",
        "agent-context-object",
        "agent-object",
        "observes",
        "identifies",
      ),
      edge(
        "agent-revision-agent-object",
        "agent-revision-object",
        "agent-object",
        "contains",
        "binds",
      ),
      edge(
        "agent-agent-output-object",
        "agent-object",
        "agent-output-object",
        "produces",
        "proposes",
      ),
      edge(
        "agent-output-scenario-object",
        "agent-output-object",
        "agent-scenario-object",
        "produces",
        "qualifies",
      ),
      edge(
        "agent-scenario-attention-object",
        "agent-scenario-object",
        "agent-attention-object",
        "produces",
        "diff directs",
      ),
    ],
    omissions:
      outputs.length > 1
        ? [`${outputs.length - 1} additional workload outputs folded`]
        : [],
  };
}

function workloadObjectScene(
  portfolio: WorkloadList,
  workload: WorkloadSummary,
  focusId: string,
): TopologyProjection {
  const attention = portfolio.attention.rows.filter(
    (row) => row.subject_id === workload.workload.id,
  );
  return {
    regime: "objects",
    label: "WORKLOAD OBJECTS",
    detail: workload.workload.id,
    focus_id: focusId,
    regions: [
      {
        id: "object-boundary",
        label: "BOUNDED COMPUTE GRAPH",
        detail: shortCoordinate(workload.workload.semantic_digest),
        x: 45,
        y: 55,
        width: 1110,
        height: 610,
        tone: qualificationTone(workload.qualification),
      },
    ],
    nodes: [
      node(
        "context-object",
        "cluster:context",
        "CONTEXT",
        workload.provenance?.origin ?? "compiled",
        workload.provenance?.source ?? "built-in conformance surface",
        160,
        355,
        205,
        "neutral",
      ),
      node(
        "workload-object",
        focusId,
        "WORKLOAD",
        workload.workload.id,
        `revision ${workload.workload.revision} · ${workload.qualification}`,
        395,
        355,
        230,
        qualificationTone(workload.qualification),
        workload.workload.id,
      ),
      node(
        "graph-object",
        focusId,
        "COMPUTE GRAPH",
        workload.candidate_graph.id,
        `revision ${workload.candidate_graph.revision} · ${shortCoordinate(workload.candidate_graph.semantic_digest)}`,
        650,
        205,
        225,
        "accent",
      ),
      node(
        "scenario-object",
        focusId,
        "SCENARIOS",
        `${workload.passed}/${workload.required} passing`,
        `${workload.failed} failing · ${workload.inconclusive} inconclusive`,
        650,
        505,
        225,
        workload.failed + workload.inconclusive > 0 ? "attention" : "healthy",
      ),
      node(
        "evidence-object",
        "cluster:evidence",
        "EVIDENCE",
        `${workload.mining_results} mined results`,
        `${workload.relation_deltas} deltas · ${workload.reasoning_surfaces} surfaces`,
        905,
        205,
        225,
        workload.incomplete_mining_results > 0 ? "blocked" : "neutral",
      ),
      node(
        "attention-object",
        attention[0] ? `attention:${attention[0].row_id}` : "cluster:attention",
        "ATTENTION",
        attention[0]?.action ?? "no directed row",
        attention[0]?.reason.replaceAll("_", " ") ?? "locally converged",
        905,
        505,
        225,
        attention.length > 0 ? "attention" : "healthy",
      ),
    ],
    edges: [
      edge(
        "context-workload-object",
        "context-object",
        "workload-object",
        "contains",
        "binds",
      ),
      edge(
        "workload-graph-object",
        "workload-object",
        "graph-object",
        "contains",
        "executes",
      ),
      edge(
        "graph-scenario-object",
        "graph-object",
        "scenario-object",
        "produces",
        "evaluated by",
      ),
      edge(
        "graph-evidence-object",
        "graph-object",
        "evidence-object",
        "produces",
        "mines",
      ),
      edge(
        "scenario-attention-object",
        "scenario-object",
        "attention-object",
        "produces",
        "diff directs",
      ),
      edge(
        "evidence-attention-object",
        "evidence-object",
        "attention-object",
        "observes",
        "supports",
      ),
    ],
    omissions:
      attention.length > 1
        ? [`${attention.length - 1} additional attention rows omitted`]
        : [],
  };
}

function draftObjectScene(
  portfolio: WorkloadList,
  draft: WorkloadDraft,
  focusId: string,
): TopologyProjection {
  return {
    regime: "objects",
    label: "REQUEST OBJECTS",
    detail: draft.request.workload_id,
    focus_id: focusId,
    regions: [
      {
        id: "request-boundary",
        label: "AGENTIC HANDOFF",
        detail: shortCoordinate(draft.request.request_id),
        x: 85,
        y: 125,
        width: 1030,
        height: 470,
        tone: "blocked",
      },
    ],
    nodes: [
      node(
        "request-object",
        focusId,
        "REQUEST",
        draft.request.workload_id,
        draft.request.intent ?? draft.request.title,
        235,
        355,
        245,
        "accent",
        draft.request.workload_id,
      ),
      node(
        "harness-object",
        focusId,
        "CODING HARNESS",
        draft.request.proposer,
        "external agentic generation boundary",
        600,
        355,
        245,
        "attention",
      ),
      node(
        "package-object",
        focusId,
        "TARGET PACKAGE",
        draft.request.target_package,
        "graph + frozen scenario oracle missing",
        965,
        355,
        245,
        "blocked",
      ),
    ],
    edges: [
      edge(
        "request-harness-object",
        "request-object",
        "harness-object",
        "directs",
        "requests",
      ),
      edge(
        "harness-package-object",
        "harness-object",
        "package-object",
        "produces",
        "materializes",
      ),
    ],
    omissions: [],
  };
}

function attentionObjectScene(
  portfolio: WorkloadList,
  row: AttentionRow,
  focusId: string,
): TopologyProjection {
  const subjectExists =
    portfolio.workloads.some(
      (workload) => workload.workload.id === row.subject_id,
    ) ||
    portfolio.drafts.some(
      (draft) => draft.request.workload_id === row.subject_id,
    );
  return {
    regime: "objects",
    label: "ATTENTION OBJECTS",
    detail: `${row.action} / ${row.readiness}`,
    focus_id: focusId,
    regions: [
      {
        id: "attention-boundary",
        label: "DIRECTED REASONING SURFACE",
        detail: shortCoordinate(row.row_id),
        x: 55,
        y: 75,
        width: 1090,
        height: 570,
        tone: row.readiness === "blocked" ? "blocked" : "attention",
      },
    ],
    nodes: [
      node(
        "snapshot-object",
        "cluster:portfolio",
        "SOURCE SNAPSHOT",
        shortCoordinate(portfolio.attention.source_snapshot_id),
        "retained portfolio observation",
        155,
        355,
        210,
        "neutral",
      ),
      node(
        "delta-object",
        focusId,
        "ATTENTION DELTA",
        row.reason.replaceAll("_", " "),
        `${row.action} · priority ${row.priority} · cost ${row.estimated_cost_units}`,
        425,
        355,
        245,
        row.readiness === "blocked" ? "blocked" : "attention",
      ),
      node(
        "subject-object",
        subjectExists ? `workload:${row.subject_id}` : focusId,
        row.subject_kind.toUpperCase(),
        row.subject_id,
        `${row.readiness} · ${row.evidence_ids.length} evidence bindings`,
        725,
        225,
        245,
        "accent",
        subjectExists ? row.subject_id : undefined,
      ),
      node(
        "evidence-binding-object",
        "cluster:evidence",
        "EVIDENCE BINDINGS",
        `${row.evidence_ids.length} exact references`,
        row.evidence_ids[0]
          ? shortCoordinate(row.evidence_ids[0])
          : "no retained evidence reference",
        1000,
        225,
        220,
        row.evidence_ids.length > 0 ? "healthy" : "blocked",
      ),
      node(
        "dependency-object",
        focusId,
        "DEPENDENCIES",
        `${row.dependency_ids.length} required coordinates`,
        row.dependency_ids[0]
          ? shortCoordinate(row.dependency_ids[0])
          : "ready without dependency",
        725,
        505,
        245,
        row.dependency_ids.length > 0 ? "attention" : "healthy",
      ),
    ],
    edges: [
      edge(
        "snapshot-delta-object",
        "snapshot-object",
        "delta-object",
        "produces",
        "derives",
      ),
      edge(
        "delta-subject-object",
        "delta-object",
        "subject-object",
        "directs",
        row.action,
      ),
      edge(
        "evidence-delta-object",
        "evidence-binding-object",
        "delta-object",
        "observes",
        "supports",
      ),
      edge(
        "dependency-delta-object",
        "dependency-object",
        "delta-object",
        "depends",
        "bounds",
      ),
    ],
    omissions: [
      ...(row.evidence_ids.length > 1
        ? [`${row.evidence_ids.length - 1} evidence references folded`]
        : []),
      ...(row.dependency_ids.length > 1
        ? [`${row.dependency_ids.length - 1} dependency references folded`]
        : []),
    ],
  };
}

function portfolioObjectScene(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const admitted = portfolio.catalog.admitted_count;
  const qualified = portfolio.workloads.filter(
    (workload) => workload.qualification === "qualified",
  ).length;
  return {
    regime: "objects",
    label: "PORTFOLIO OBJECTS",
    detail: shortCoordinate(portfolio.attention.source_snapshot_id),
    focus_id: focusId,
    regions: [
      {
        id: "portfolio-boundary",
        label: "CURRENT CONTEXT COORDINATE",
        detail: `${portfolio.catalog.workload_count} workload objects`,
        x: 55,
        y: 75,
        width: 1090,
        height: 570,
        tone: "neutral",
      },
    ],
    nodes: [
      node(
        "surface-object",
        "cluster:context",
        "SURFACES",
        `${portfolio.attention.summary.surfaces} declared`,
        `${portfolio.attention.summary.owned_surfaces} owned · ${portfolio.attention.summary.unowned_surfaces} unowned`,
        155,
        355,
        215,
        portfolio.attention.summary.unowned_surfaces > 0
          ? "attention"
          : "healthy",
      ),
      node(
        "catalog-object",
        "cluster:workloads",
        "CATALOG",
        `${admitted} admitted`,
        `${portfolio.catalog.draft_count} creation requests`,
        425,
        225,
        225,
        "accent",
      ),
      node(
        "qualification-object",
        "cluster:workloads",
        "QUALIFICATION",
        `${qualified}/${admitted} qualified`,
        "fresh deterministic scenario results",
        425,
        505,
        225,
        qualified === admitted && admitted > 0 ? "healthy" : "attention",
      ),
      node(
        "snapshot-evidence-object",
        "cluster:evidence",
        "SNAPSHOT",
        shortCoordinate(portfolio.attention.source_snapshot_id),
        "catalog + results + context + coverage",
        725,
        225,
        235,
        "neutral",
      ),
      node(
        "frontier-object",
        "cluster:attention",
        "FRONTIER",
        `${portfolio.attention.rows.length} directed rows`,
        shortCoordinate(portfolio.attention.attention_id),
        725,
        505,
        235,
        portfolio.attention.rows.length > 0 ? "attention" : "healthy",
      ),
      node(
        "bearing-object",
        "cluster:portfolio",
        "NEXT BEARING",
        nextBearing(portfolio),
        "derived from retained attention; no scheduler selection",
        1010,
        355,
        230,
        portfolio.attention.rows.length > 0 ? "accent" : "healthy",
      ),
    ],
    edges: [
      edge(
        "surface-catalog-object",
        "surface-object",
        "catalog-object",
        "contains",
        "bounds",
      ),
      edge(
        "catalog-qualification-object",
        "catalog-object",
        "qualification-object",
        "produces",
        "tests",
      ),
      edge(
        "catalog-snapshot-object",
        "catalog-object",
        "snapshot-evidence-object",
        "observes",
        "records",
      ),
      edge(
        "qualification-snapshot-object",
        "qualification-object",
        "snapshot-evidence-object",
        "produces",
        "retains",
      ),
      edge(
        "snapshot-frontier-object",
        "snapshot-evidence-object",
        "frontier-object",
        "produces",
        "diffs",
      ),
      edge(
        "frontier-bearing-object",
        "frontier-object",
        "bearing-object",
        "directs",
        "orients",
      ),
    ],
    omissions: [],
  };
}

const TERRAIN_LEVELS = [0.12, 0.23, 0.35, 0.48, 0.61, 0.74, 0.86] as const;

interface SurveyTerrainLayout {
  contours: TopologyContour[];
  landforms: TopologyLandform[];
  natural_features: TopologyNaturalFeature[];
  omissions: string[];
  points: TopologyPointOfInterest[];
  regions: TopologyRegion[];
  terrain_fields: TerrainFieldSet[];
  terrain_pyramids: TerrainFieldPyramid[];
  world: TopologyWorld;
}

function buildSurveyTerrain(
  topographies: AdmittedTopography[],
  focusId: string,
  regime: LensRegime,
  semanticAtlas: SemanticAtlas | null = null,
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
  const bearing = buildSurveyBearing(
    selectedPoints,
    requestedFocusPoint,
    selected.patch,
  );
  const detail = buildSurveyTerrainDetails(
    selected,
    focusPoint,
    focusId,
    regime,
  );
  const visibleRegions =
    regime === "atlas" || regime === "landscape"
      ? layout.regions
      : regime === "neighborhoods"
        ? layout.regions.filter((region) => region.variant === "map-boundary")
        : [];
  // Terrain never turns evidence relationships or detail-card associations
  // into geographic lines. Discoverable/built paths require a separate
  // admitted contract.
  const visibleEdges: TopologyEdge[] = [];
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
    edges: visibleEdges,
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
    terrain_pyramids: layout.terrain_pyramids,
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
    atlas_id: atlas.atlas_id,
    atlas_revision: atlas.atlas_revision,
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
  };
}

function layoutSurveyTerrain(
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
  const terrainFields: TerrainFieldSet[] = [];
  const terrainPyramids: TerrainFieldPyramid[] = [];

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
    const terrainField = buildTerrainField(
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
    terrainPyramids.push(terrainField.pyramid);
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
    terrain_pyramids: terrainPyramids,
    world,
  };
}

function buildSurveyTerrainDetails(
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
    nodes.push(
      node(
        id,
        focusId,
        family,
        label,
        detail,
        focusPoint.x + offset.x * xDirection,
        focusPoint.y + offset.y * yDirection,
        220,
        tone,
        workload.workload.id,
        coordinateUri,
      ),
    );
    edges.push(
      edge(
        `${focusPoint.id}:${id}`,
        focusPoint.id,
        id,
        family.startsWith("LINEAGE") ? "observes" : "produces",
        family.toLowerCase(),
      ),
    );
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

interface TerrainFieldResult {
  contours: TopologyContour[];
  fields: TerrainFieldSet;
  pyramid: TerrainFieldPyramid;
  natural_features: TopologyNaturalFeature[];
}

function buildTerrainField(
  id: string,
  points: TopologyPointOfInterest[],
  frontier: TopologyPointOfInterest[],
  patch: TopographyPatch,
  projection: ProjectionPacket,
  regime: LensRegime,
  bounds: { x: number; y: number; width: number; height: number },
): TerrainFieldResult {
  const unresolvedPressure = Math.min(
    1.8,
    patch.frontier.length * 0.08 +
      patch.coverage.unresolved_candidates * 0.04 +
      patch.omissions.reduce(
        (total, omission) => total + omission.omitted_count,
        0,
      ) *
        0.03,
  );
  const pyramid = compileTerrainFieldPyramid({
    source_id: id,
    source_revision: patch.topography_revision,
    bounds,
    anchors: points.map((point) => ({
      id: point.id,
      x: point.x,
      y: point.y,
      prominence: point.prominence,
    })),
    atmosphere: frontier.map((point) => ({ x: point.x, y: point.y })),
    unresolved_pressure: unresolvedPressure,
    projection,
  });
  const fields = terrainFieldForRegime(pyramid, regime);
  const grid = fields.grid;
  const contours = TERRAIN_LEVELS.flatMap((ratio, index) => {
    const threshold = fields.elevation.maximum * ratio;
    const path = marchingSquaresPath(
      fields.elevation,
      fields.validity,
      threshold,
    );
    return path
      ? [
          {
            id: `relief:${id}:${index}`,
            path,
            level: index + 1,
            threshold,
            anchor_count: points.length,
          },
        ]
      : [];
  });

  const maximumAccumulation = fields.flow_accumulation.maximum;
  const streamThreshold = Math.max(3.2, maximumAccumulation * 0.055);
  const riverThreshold = Math.max(
    streamThreshold * 2.4,
    maximumAccumulation * 0.2,
  );
  const segmentPath = (minimum: number, maximum?: number) => {
    const segments: string[] = [];
    for (let row = 0; row < grid.rows; row += 1) {
      for (let column = 0; column < grid.columns; column += 1) {
        const index = row * grid.columns + column;
        const accumulation = fields.flow_accumulation.values[index]!;
        const columnOffset = fields.flow_direction.values[index * 2]!;
        const rowOffset = fields.flow_direction.values[index * 2 + 1]!;
        if (
          (columnOffset === 0 && rowOffset === 0) ||
          accumulation < minimum ||
          (maximum !== undefined && accumulation >= maximum)
        )
          continue;
        const from = fieldPoint(grid, column, row);
        const to = fieldPoint(grid, column + columnOffset, row + rowOffset);
        segments.push(
          `M${from.x.toFixed(1)},${from.y.toFixed(1)}L${to.x.toFixed(1)},${to.y.toFixed(1)}`,
        );
      }
    }
    return segments.join("");
  };
  const streamPath = segmentPath(streamThreshold, riverThreshold);
  const riverPath = segmentPath(riverThreshold);
  const naturalFeatures: TopologyNaturalFeature[] = [];
  if (streamPath)
    naturalFeatures.push({
      id: `stream-system:${id}`,
      path: streamPath,
      kind: "stream",
      label: "PROJECTED HEADWATERS",
      detail:
        "downslope accumulation over anchor-only relief under admitted survey conditions; not a source relationship or discovered path",
      intensity: Math.min(4, Math.max(1, maximumAccumulation / 18)),
      workload_id: id,
    });
  if (riverPath)
    naturalFeatures.push({
      id: `river-system:${id}`,
      path: riverPath,
      kind: "river",
      label: "ACCUMULATED FLOW",
      detail:
        "high-accumulation runoff projected from the same field that erodes the displayed relief; not retained hydrology",
      intensity: Math.min(4, Math.max(2, maximumAccumulation / 28)),
      workload_id: id,
    });
  frontier.forEach((point, index) => {
    const radius = 58 + point.prominence * 14;
    const skew = index % 2 === 0 ? 1 : -1;
    naturalFeatures.push({
      id: `weather-front:${id}:${point.id}`,
      path: `M${(point.x - radius).toFixed(1)},${point.y.toFixed(1)} C${(point.x - radius * 0.45).toFixed(1)},${(point.y - radius * 0.7 * skew).toFixed(1)} ${(point.x + radius * 0.35).toFixed(1)},${(point.y + radius * 0.72 * skew).toFixed(1)} ${(point.x + radius).toFixed(1)},${point.y.toFixed(1)}`,
      kind: "weather_front",
      label: "UNRESOLVED SURVEY FRONT",
      detail: `${point.detail} · ${point.signal}`,
      intensity: Math.min(4, 1 + unresolvedPressure),
      workload_id: id,
    });
  });
  return { contours, fields, pyramid, natural_features: naturalFeatures };
}

function envelopePath(
  points: TopologyPointOfInterest[],
  center: TopologyPosition,
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
  const midpoint = (first: TopologyPosition, second: TopologyPosition) => ({
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

function marchingSquaresPath(
  elevation: ScalarField2D,
  validity: MaskField2D,
  threshold: number,
): string {
  const segments: string[] = [];
  const { grid } = elevation;
  const rowCount = grid.rows - 1;
  const columnCount = grid.columns - 1;
  const point = (column: number, row: number) => fieldPoint(grid, column, row);
  const value = (column: number, row: number) =>
    elevation.values[row * grid.columns + column]!;
  const crossing = (
    first: { x: number; y: number; value: number },
    second: { x: number; y: number; value: number },
  ) => {
    const denominator = second.value - first.value;
    const amount =
      denominator === 0 ? 0.5 : (threshold - first.value) / denominator;
    return {
      x: first.x + (second.x - first.x) * amount,
      y: first.y + (second.y - first.y) * amount,
    };
  };
  const line = (
    first: { x: number; y: number },
    second: { x: number; y: number },
  ) =>
    `M${first.x.toFixed(1)},${first.y.toFixed(1)}L${second.x.toFixed(1)},${second.y.toFixed(1)}`;

  for (let row = 0; row < rowCount; row += 1) {
    for (let column = 0; column < columnCount; column += 1) {
      const cornerIndices = [
        row * grid.columns + column,
        row * grid.columns + column + 1,
        (row + 1) * grid.columns + column + 1,
        (row + 1) * grid.columns + column,
      ];
      if (cornerIndices.some((index) => validity.values[index] === 0)) continue;
      const topLeftPoint = point(column, row);
      const topRightPoint = point(column + 1, row);
      const bottomRightPoint = point(column + 1, row + 1);
      const bottomLeftPoint = point(column, row + 1);
      const topLeft = { ...topLeftPoint, value: value(column, row) };
      const topRight = { ...topRightPoint, value: value(column + 1, row) };
      const bottomRight = {
        ...bottomRightPoint,
        value: value(column + 1, row + 1),
      };
      const bottomLeft = {
        ...bottomLeftPoint,
        value: value(column, row + 1),
      };
      const crossings: Array<{
        edge: "top" | "right" | "bottom" | "left";
        point: { x: number; y: number };
      }> = [];
      const addCrossing = (
        edgeName: "top" | "right" | "bottom" | "left",
        first: typeof topLeft,
        second: typeof topLeft,
      ) => {
        if (first.value >= threshold !== second.value >= threshold)
          crossings.push({ edge: edgeName, point: crossing(first, second) });
      };
      addCrossing("top", topLeft, topRight);
      addCrossing("right", topRight, bottomRight);
      addCrossing("bottom", bottomRight, bottomLeft);
      addCrossing("left", bottomLeft, topLeft);
      if (crossings.length === 2) {
        segments.push(line(crossings[0]!.point, crossings[1]!.point));
      } else if (crossings.length === 4) {
        const byEdge = new Map(
          crossings.map((candidate) => [candidate.edge, candidate.point]),
        );
        const center =
          (topLeft.value +
            topRight.value +
            bottomRight.value +
            bottomLeft.value) /
          4;
        const pairs: Array<
          [
            "top" | "right" | "bottom" | "left",
            "top" | "right" | "bottom" | "left",
          ]
        > =
          center >= threshold
            ? [
                ["top", "left"],
                ["right", "bottom"],
              ]
            : [
                ["top", "right"],
                ["bottom", "left"],
              ];
        pairs.forEach(([first, second]) => {
          const firstPoint = byEdge.get(first!);
          const secondPoint = byEdge.get(second!);
          if (firstPoint && secondPoint)
            segments.push(line(firstPoint, secondPoint));
        });
      }
    }
  }
  return segments.join("");
}

function stableHash(value: string): number {
  let hash = 2_166_136_261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16_777_619);
  }
  return hash >>> 0;
}

function buildEvidence(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length === 0) {
    const fallback = buildObjects(portfolio, focusId);
    return {
      ...fallback,
      regime: "evidence",
      label: "EVIDENCE BOUNDARY",
      detail: "no admitted survey evidence",
      omissions: [
        "exact locator evidence is unavailable until a survey workload patch is admitted",
        ...fallback.omissions,
      ],
    };
  }
  return buildSurveyTerrain(topographies, focusId, "evidence");
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

function topologyWorld(projection: TopologyProjection): TopologyWorld {
  const contentWidth = Math.max(
    ...projection.regions.map((region) => region.x + region.width),
    ...projection.nodes.map((candidate) => candidate.x + candidate.width / 2),
    640,
  );
  const contentHeight = Math.max(
    ...projection.regions.map((region) => region.y + region.height),
    ...projection.nodes.map((candidate) => candidate.y + 90),
    420,
  );
  return {
    width: Math.ceil((contentWidth + 40) / 40) * 40,
    height: Math.ceil((contentHeight + 40) / 40) * 40,
  };
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

function seedTone(
  state: TopographyPatch["seeds"][number]["state"],
): TopologyTone {
  if (state === "surveyed") return "healthy";
  if (state === "surveyed_empty") return "neutral";
  if (state === "omitted") return "omitted";
  if (state === "unsupported") return "unsupported";
  return "unknown";
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

function resolveObjectFocus(portfolio: WorkloadList, focusId: string): string {
  if (
    focusId.startsWith("workload:") ||
    focusId.startsWith("attention:") ||
    focusId.startsWith("agent:")
  )
    return focusId;
  if (focusId === "cluster:workloads") {
    const workloadId =
      portfolio.workloads[0]?.workload.id ??
      portfolio.drafts[0]?.request.workload_id;
    if (workloadId) return `workload:${workloadId}`;
  }
  if (focusId === "cluster:attention" && portfolio.attention.rows[0])
    return `attention:${portfolio.attention.rows[0].row_id}`;
  if (focusId === "cluster:agents") {
    const agent = deriveAgentIndex(portfolio)[0];
    if (agent) return `agent:${agent.id}`;
  }
  return focusId;
}

function nextBearing(portfolio: WorkloadList): string {
  const row = portfolio.attention.rows[0];
  return row
    ? `${row.action.toUpperCase()} ${row.subject_id}`
    : "NO UNRESOLVED ATTENTION";
}

function qualificationTone(qualification: string): TopologyTone {
  if (qualification === "qualified") return "healthy";
  if (qualification === "failing" || qualification === "inconclusive")
    return "blocked";
  if (qualification === "stale" || qualification === "untested")
    return "attention";
  return "neutral";
}

function shortCoordinate(value: string): string {
  const coordinate = value.startsWith("blake3:") ? value.slice(7) : value;
  return coordinate.length > 22
    ? `${coordinate.slice(0, 10)}…${coordinate.slice(-8)}`
    : coordinate;
}

function node(
  id: string,
  focusId: string,
  family: string,
  label: string,
  detail: string,
  x: number,
  y: number,
  width: number,
  tone: TopologyTone,
  workloadId?: string,
  coordinateUri?: string,
): TopologyNode {
  return {
    id,
    focus_id: focusId,
    family,
    label,
    detail,
    x,
    y,
    width,
    tone,
    workload_id: workloadId,
    coordinate_uri: coordinateUri,
  };
}

function edge(
  id: string,
  from: string,
  to: string,
  kind: TopologyEdge["kind"],
  label: string,
): TopologyEdge {
  return { id, from, to, kind, label };
}
