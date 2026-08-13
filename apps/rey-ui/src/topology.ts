import type {
  AgentSummary,
  AttentionRow,
  ProjectionPacket,
  SemanticAtlas,
  SemanticAtlasDelta,
  WorkloadDraft,
  WorkloadList,
  WorkloadSummary,
} from "./domain";
import { deriveAgentIndex } from "./domain";
import {
  DEFAULT_LENS_ZOOM,
  lensRegimeForZoom,
  type LensRegime,
} from "./explore/engine/camera";
import { admittedTopographies } from "./explore/projection/topography-projector";
import {
  admittedRegionalScenes,
  type AdmittedRegionalProjection,
} from "./explore/projection/regional-scene-projector";
import {
  projectSemanticMercator,
  projectSemanticMercatorBounds,
  SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
  SEMANTIC_MERCATOR_PROJECTION_REVISION,
} from "./explore/projection/semantic-mercator";
import {
  countyFrameView,
  nativeBoundsToCountyLocal,
  projectCountyFootprint,
  projectCountyLocal,
  type CountyFrame,
  type ProjectedCountyFootprint,
} from "./explore/projection/county-frame";
import {
  type TerrainFieldSet,
  type TerrainProgram,
} from "./explore/terrain/compile";
import { buildSurveyScene } from "./explore/projection/survey-scene";
import {
  buildAgentObjectScene,
  buildPortfolioLandscape,
  buildPortfolioNeighborhoods,
  buildWorkloadObjectScene,
} from "./explore/projection/portfolio-scene";
import { regionalObjectEvidenceRoute } from "./regional-object-route";

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

export type TopologyGlobeBeaconState =
  "request" | "working" | "index" | "admitted";

export interface TopologyGlobeBeacon {
  id: string;
  focus_id: string;
  workload_id: string;
  label: string;
  detail: string;
  source: string;
  source_revision: string;
  producer: string;
  state: TopologyGlobeBeaconState;
  mapping_role: "survey" | "workload";
  next_step: string;
  longitude_degrees: number;
  latitude_degrees: number;
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
  schema:
    | "rey.explore-orientation-globe.v1"
    | "rey.semantic-globe-scene.v1"
    | "rey.regional-world-scene.v1";
  posture: "orientation" | "semantic_atlas" | "regional_scenes";
  globe_id: string;
  source_revision: string;
  compiler_revision: string;
  coordinate_authority: string;
  regions: TopologyGlobeRegion[];
  clusters: TopologyGlobeCluster[];
  beacons: TopologyGlobeBeacon[];
}

export interface TopologyRegion {
  id: string;
  fragment_id?: string;
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
  kind: "stream" | "river" | "weather_front";
  label: string;
  detail: string;
  intensity: number;
  workload_id: string;
}

export interface TopologyBearing {
  status:
    "world" | "charted" | "probe_required" | "consent_required" | "isolated";
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
  evidence_uri?: string;
  semantic_identity?: string;
  semantic_coordinate?: {
    longitude_microdegrees: number;
    latitude_microdegrees: number;
  };
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
  terrain_programs: TerrainProgram[];
  globe: TopologyGlobe | null;
  world_atlas_transition: TopologyWorldAtlasTransition | null;
  county_frame: CountyFrame | null;
  county_footprint: ProjectedCountyFootprint | null;
}

export interface TopologyWorldAtlasPoint {
  identity: string;
  focus_id: string;
  label: string;
  longitude_microdegrees: number;
  latitude_microdegrees: number;
  tone: TopologyTone;
}

export interface TopologyWorldAtlasSector {
  identity: string;
  label: string;
  west_microdegrees: number;
  south_microdegrees: number;
  east_microdegrees: number;
  north_microdegrees: number;
  crosses_antimeridian: boolean;
  tone: TopologyTone;
}

export interface TopologyWorldAtlasTransition {
  schema: "rey.world-atlas-transition.v1";
  atlas_revision: string;
  projection_revision: string;
  atlas_frame: { x: number; y: number; width: number; height: number };
  points: TopologyWorldAtlasPoint[];
  sectors: TopologyWorldAtlasSector[];
  authority: string;
}

export const TOPOLOGY_WORLD = { width: 1200, height: 720 } as const;

export function buildTopologyScene(
  portfolio: WorkloadList,
  zoom: number,
  focusId = "cluster:portfolio",
  retainedRegime?: LensRegime,
): TopologyScene {
  const regime = retainedRegime ?? lensRegimeForZoom(zoom);
  const regionalScenes = admittedRegionalScenes(portfolio);
  const surveyFocus =
    focusId.startsWith("topography:") ||
    focusId.startsWith("seed:") ||
    focusId.startsWith("anchor:") ||
    focusId.startsWith("frontier:");
  const regionalFocus = regionalScenes.some(
    ({ scene, county_footprint }) =>
      Boolean(county_footprint) && regionalSceneMatchesFocus(scene, focusId),
  );
  let projection: TopologyProjection;
  if (isFreshProjectOrientation(portfolio))
    projection = buildOrientationWorld(portfolio, focusId);
  else if (regionalScenes.length > 0 && regime === "world")
    projection = buildRegionalWorld(
      regionalScenes,
      portfolio.semantic_atlas ?? null,
      focusId,
    );
  else if (regionalScenes.length > 0 && regime === "atlas" && !surveyFocus)
    projection = buildRegionalAtlas(regionalScenes, focusId);
  else if (regionalScenes.length > 0 && !surveyFocus && !regionalFocus)
    projection = buildRegionalAtlas(regionalScenes, focusId);
  else if (
    regionalScenes.length > 0 &&
    regionalFocus &&
    !surveyFocus &&
    !focusId.startsWith("agent:")
  )
    projection = buildRegionalCounty(regionalScenes, focusId, regime);
  else if (regime === "world") projection = buildWorld(portfolio, focusId);
  else if (regime === "atlas") projection = buildAtlas(portfolio, focusId);
  else if (regime === "landscape")
    projection = buildPortfolioLandscape(portfolio, focusId);
  else if (regime === "neighborhoods")
    projection = buildPortfolioNeighborhoods(portfolio, focusId);
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
    terrain_programs: projection.terrain_programs ?? [],
    globe: projection.globe ?? null,
    world_atlas_transition:
      regionalScenes.length > 0 &&
      !surveyFocus &&
      (regime === "world" || regime === "atlas")
        ? buildWorldAtlasTransition(regionalScenes)
        : null,
    county_frame: projection.county_frame ?? null,
    county_footprint: projection.county_footprint ?? null,
    world: projection.world ?? topologyWorld(projection),
    fit_world:
      projection.fit_world ?? projection.world ?? topologyWorld(projection),
  };
}

function isFreshProjectOrientation(portfolio: WorkloadList): boolean {
  return (
    portfolio.revision !== undefined &&
    admittedTopographies(portfolio).length === 0 &&
    admittedRegionalScenes(portfolio).length === 0 &&
    ((portfolio.catalog.admitted_count === 0 &&
      portfolio.workloads.length === 0) ||
      portfolio.workloads.some(
        (workload) => workload.workload.id === "context-anchor-survey",
      ))
  );
}

export type TopologyProjection = Omit<
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
  | "terrain_programs"
  | "world"
  | "world_atlas_transition"
  | "county_frame"
  | "county_footprint"
> & {
  bearing?: TopologyBearing;
  contours?: TopologyContour[];
  landforms?: TopologyLandform[];
  natural_features?: TopologyNaturalFeature[];
  points?: TopologyPointOfInterest[];
  terrain?: boolean;
  terrain_fields?: TerrainFieldSet[];
  terrain_programs?: TerrainProgram[];
  world?: TopologyWorld;
  fit_world?: TopologyWorld;
  globe?: TopologyGlobe | null;
  world_atlas_transition?: TopologyWorldAtlasTransition | null;
  county_frame?: CountyFrame | null;
  county_footprint?: ProjectedCountyFootprint | null;
};

function buildWorldAtlasTransition(
  regionalScenes: AdmittedRegionalProjection[],
): TopologyWorldAtlasTransition {
  const sectors = new Map(
    regionalScenes.map(({ atlas_sector: sector }) => [
      sector.sector_id,
      sector,
    ]),
  );
  const atlasRevision =
    regionalScenes[0]!.scene.artifacts.admitted_atlas_revision!;
  return {
    schema: "rey.world-atlas-transition.v1",
    atlas_revision: atlasRevision,
    projection_revision: SEMANTIC_MERCATOR_PROJECTION_REVISION,
    atlas_frame: semanticMercatorFrame(),
    points: regionalScenes
      .map(({ scene, atlas_region: region }) => ({
        identity: region.region_id,
        focus_id: `regional:${scene.scene_id}`,
        label: scene.region_id,
        longitude_microdegrees: region.semantic_longitude_microdegrees,
        latitude_microdegrees: region.semantic_latitude_microdegrees,
        tone: scene.complete ? ("healthy" as const) : ("omitted" as const),
      }))
      .sort((left, right) => left.identity.localeCompare(right.identity)),
    sectors: [...sectors.values()]
      .map((sector) => ({
        identity: sector.sector_id,
        label: `SECTOR ${sector.longitude_band + 1}.${sector.latitude_band + 1}`,
        west_microdegrees: sector.west_microdegrees,
        south_microdegrees: sector.south_microdegrees,
        east_microdegrees: sector.east_microdegrees,
        north_microdegrees: sector.north_microdegrees,
        crosses_antimeridian: false,
        tone: "neutral" as const,
      }))
      .sort((left, right) => left.identity.localeCompare(right.identity)),
    authority:
      "same retained atlas identities through presentation-only World-to-Atlas geometry; no locate, survey, admission, or distance authority",
  };
}

function currentSemanticAtlasDelta(
  portfolio: WorkloadList,
): SemanticAtlasDelta | null {
  const atlas = portfolio.semantic_atlas;
  const retained = portfolio.semantic_atlas_history.at(-1);
  const delta = portfolio.semantic_atlas_deltas.at(-1);
  if (
    !atlas ||
    !retained ||
    !delta ||
    retained.atlas_revision !== atlas.atlas_revision ||
    delta.target_revision !== atlas.atlas_revision ||
    portfolio.semantic_atlas_history.length !==
      portfolio.semantic_atlas_deltas.length
  )
    return null;
  return delta;
}

function buildWorld(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length > 0)
    return buildSurveyScene(
      topographies,
      focusId,
      "world",
      portfolio.semantic_atlas ?? null,
      currentSemanticAtlasDelta(portfolio),
    );
  return buildOrientationWorld(portfolio, focusId);
}

export function workloadOrientationBeacons(
  portfolio: WorkloadList,
): TopologyGlobeBeacon[] {
  const beacons = new Map<string, TopologyGlobeBeacon>();
  const revision = portfolio.revision;
  const headPackages = new Map(
    (revision?.head?.snapshot.packages ?? []).map((item) => [
      item.workload_id,
      item,
    ]),
  );
  const indexPackages = new Map(
    (revision?.index?.packages ?? []).map((item) => [item.workload_id, item]),
  );
  for (const candidate of revision?.working.packages ?? []) {
    if (
      headPackages.get(candidate.workload_id)?.source_digest ===
      candidate.source_digest
    )
      continue;
    const indexed =
      indexPackages.get(candidate.workload_id)?.source_digest ===
      candidate.source_digest;
    beacons.set(
      candidate.workload_id,
      workloadBeacon(
        candidate.workload_id,
        candidate.title,
        candidate.source,
        candidate.source_digest,
        `${candidate.generation.kind.replaceAll("_", " ")} / ${candidate.generation.producer}@${candidate.generation.producer_revision}`,
        indexed ? "index" : "working",
      ),
    );
  }
  for (const workload of portfolio.workloads) {
    if (beacons.has(workload.workload.id) || workload.topography_patch)
      continue;
    const source = workload.provenance?.source ?? "admitted workload HEAD";
    const sourceRevision =
      workload.provenance?.source_digest ?? workload.workload.semantic_digest;
    const generation = workload.provenance?.generation;
    beacons.set(
      workload.workload.id,
      workloadBeacon(
        workload.workload.id,
        workload.title,
        source,
        sourceRevision,
        generation
          ? `${generation.kind.replaceAll("_", " ")} / ${generation.producer}@${generation.producer_revision}`
          : "admitted workload",
        "admitted",
      ),
    );
  }
  for (const draft of portfolio.drafts) {
    if (beacons.has(draft.request.workload_id)) continue;
    beacons.set(
      draft.request.workload_id,
      workloadBeacon(
        draft.request.workload_id,
        draft.request.title,
        draft.source,
        draft.source_digest,
        "coding harness request",
        "request",
      ),
    );
  }
  return [...beacons.values()].sort(
    (left, right) =>
      Number(right.mapping_role === "survey") -
        Number(left.mapping_role === "survey") ||
      left.workload_id.localeCompare(right.workload_id),
  );
}

function workloadBeacon(
  workloadId: string,
  title: string,
  source: string,
  sourceRevision: string,
  producer: string,
  state: TopologyGlobeBeaconState,
): TopologyGlobeBeacon {
  const mappingRole =
    workloadId === "context-anchor-survey" ? "survey" : "workload";
  const hash = stableHash(workloadId);
  const longitude = -58 + (hash % 117);
  const latitude = -38 + (Math.floor(hash / 117) % 77);
  const nextStep =
    state === "request"
      ? "an agent must materialize and fine-tune the requested workload package"
      : state === "working"
        ? "review the exact file and consent to qualification and admission"
        : state === "index"
          ? "review retained qualification and approve the frozen index"
          : mappingRole === "survey"
            ? "an agent may now run a bounded survey over explicitly chosen project seeds"
            : "an agent may now use this admitted workload through its declared interface";
  return {
    id: `workload-beacon:${workloadId}`,
    focus_id: `beacon:${workloadId}`,
    workload_id: workloadId,
    label: title,
    detail: `${state.toUpperCase()} / ${source} / ${shortCoordinate(sourceRevision)}`,
    source,
    source_revision: sourceRevision,
    producer,
    state,
    mapping_role: mappingRole,
    next_step: nextStep,
    longitude_degrees: longitude,
    latitude_degrees: latitude,
    tone:
      state === "admitted"
        ? "healthy"
        : state === "index"
          ? "accent"
          : state === "request"
            ? "frontier"
            : "attention",
  };
}

function buildOrientationWorld(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const beacons = workloadOrientationBeacons(portfolio);
  const incoming = beacons.filter((beacon) => beacon.state !== "admitted");
  const survey = beacons.find((beacon) => beacon.mapping_role === "survey");
  const orientationRevision =
    portfolio.revision?.working.snapshot_revision ??
    portfolio.attention.source_snapshot_id;
  return {
    regime: "world",
    label: incoming.length > 0 ? "PROJECT ORIENTATION" : "UNMAPPED PROJECT",
    detail:
      incoming.length > 0
        ? `${incoming.length} exact workload ${incoming.length === 1 ? "beacon" : "beacons"} awaiting review · no project survey has been admitted`
        : survey?.state === "admitted"
          ? "the survey workload is admitted and awaits an explicit bounded run"
          : "no survey workload candidate is available for review",
    focus_id: focusId,
    regions: [],
    landforms: [],
    contours: [],
    natural_features: [],
    points: [],
    nodes: [],
    edges: [],
    omissions: [
      "the orientation globe is presentation geometry and makes no semantic-distance claim",
      "project terrain remains unexplored until a consented survey produces admitted topography",
    ],
    bearing: {
      status: beacons.length > 0 ? "consent_required" : "isolated",
      label:
        incoming.length > 0
          ? "SURVEY CONSENT REQUIRED"
          : survey?.state === "admitted"
            ? "SURVEY RUN REQUIRED"
            : "SURVEY PROPOSAL REQUIRED",
      detail:
        survey?.next_step ??
        "an agent must create a file-backed context survey workload before Rey can request consent",
      sampled_conditions: 0,
      unresolved_boundaries: beacons.length,
    },
    world: TOPOLOGY_WORLD,
    fit_world: TOPOLOGY_WORLD,
    terrain: true,
    terrain_fields: [],
    terrain_programs: [],
    globe: {
      schema: "rey.explore-orientation-globe.v1",
      posture: "orientation",
      globe_id: `orientation:${orientationRevision}`,
      source_revision: orientationRevision,
      compiler_revision: "rey.explore.orientation-globe@1",
      coordinate_authority:
        "stable presentation-only workload placement; not an admitted semantic atlas or distance claim",
      regions: [],
      clusters: [],
      beacons,
    },
  };
}

function buildAtlas(
  portfolio: WorkloadList,
  focusId: string,
): TopologyProjection {
  const topographies = admittedTopographies(portfolio);
  if (topographies.length === 0) {
    const fallback = buildPortfolioLandscape(portfolio, focusId);
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
  return buildSurveyScene(
    topographies,
    focusId,
    "atlas",
    portfolio.semantic_atlas ?? null,
    currentSemanticAtlasDelta(portfolio),
  );
}

function buildRegionalWorld(
  regionalScenes: AdmittedRegionalProjection[],
  atlas: SemanticAtlas | null,
  focusId: string,
): TopologyProjection {
  const regionalRegions = regionalScenes.map(
    ({ workload, result, scene, atlas_region: atlasRegion }) => {
      const longitude = atlasRegion.semantic_longitude_microdegrees;
      const latitude = atlasRegion.semantic_latitude_microdegrees;
      return {
        id: atlasRegion.region_id,
        cluster_id: atlasRegion.cluster_id,
        focus_id: `regional:${scene.scene_id}`,
        workload_id: workload.workload.id,
        label: scene.region_id,
        detail: `SCENE@${scene.admission.editor_sequence} · sector ${shortCoordinate(atlasRegion.sector_id)} · exact admitted point placement · footprint scale withheld · ${shortCoordinate(result.result_id)}`,
        longitude_degrees: longitude / 1_000_000,
        latitude_degrees: latitude / 1_000_000,
        angular_radius_degrees: 0,
        tone: scene.complete ? ("healthy" as const) : ("omitted" as const),
      };
    },
  );
  const surveyRegions: TopologyGlobeRegion[] = (atlas?.regions ?? []).map(
    (region) => ({
      id: region.region_id,
      cluster_id: region.cluster_id,
      focus_id: `topography:${region.workload_id}`,
      workload_id: region.workload_id,
      label: region.workload_id,
      detail: `${region.anchor_count} surveyed anchors · ${region.frontier_rows} frontier rows · retained atlas placement`,
      longitude_degrees: region.semantic_longitude_microdegrees / 1_000_000,
      latitude_degrees: region.semantic_latitude_microdegrees / 1_000_000,
      angular_radius_degrees: region.angular_radius_microdegrees / 1_000_000,
      tone: region.complete
        ? region.frontier_rows > 0
          ? "frontier"
          : "healthy"
        : "omitted",
    }),
  );
  const packetRevisions = regionalScenes
    .map(({ scene }) => scene.projection.packet_id)
    .sort((left, right) => left.localeCompare(right));
  const sourceRevision = [
    ...packetRevisions,
    ...(atlas ? [atlas.atlas_revision] : []),
  ].join("+");
  return {
    regime: "world",
    label: "REGIONAL EVIDENCE WORLD",
    detail: `${regionalRegions.length} admitted regional ${regionalRegions.length === 1 ? "scene" : "scenes"} · ${surveyRegions.length} retained survey ${surveyRegions.length === 1 ? "region" : "regions"}`,
    focus_id: focusId,
    regions: [],
    nodes: [],
    edges: [],
    omissions: [
      "regional atlas members retain exact admitted synthetic placement points; sector membership grants no footprint radius",
      ...regionalScenes.flatMap(({ scene }) =>
        scene.omissions.map((omission) => omission.reason),
      ),
    ],
    bearing: {
      status: "world",
      label: "ADMITTED REGIONAL BEARING",
      detail:
        "select a regional marker to enter its revision-bound County frame; survey markers retain their separate atlas path",
      sampled_conditions: regionalScenes.reduce(
        (count, { scene }) => count + scene.projection.objects.length,
        0,
      ),
      unresolved_boundaries: regionalScenes.reduce(
        (count, { scene }) => count + scene.omissions.length,
        0,
      ),
    },
    world: TOPOLOGY_WORLD,
    fit_world: TOPOLOGY_WORLD,
    terrain: false,
    globe: {
      schema: "rey.regional-world-scene.v1",
      posture: "regional_scenes",
      globe_id: regionalScenes
        .map(({ scene }) => scene.scene_id)
        .sort((left, right) => left.localeCompare(right))
        .join("+"),
      source_revision: sourceRevision,
      compiler_revision: regionalScenes
        .map(({ scene }) => scene.projection.grammar_id)
        .sort((left, right) => left.localeCompare(right))
        .join("+"),
      coordinate_authority:
        "revision-bound synthetic scene placement and sector membership only; native coordinates, County footprints, physical distance, and footprint scale remain separate",
      regions: [...surveyRegions, ...regionalRegions],
      clusters: (atlas?.clusters ?? []).map((cluster) => ({
        id: cluster.cluster_id,
        longitude_degrees: cluster.semantic_longitude_microdegrees / 1_000_000,
        latitude_degrees: cluster.semantic_latitude_microdegrees / 1_000_000,
        angular_radius_degrees: cluster.angular_radius_microdegrees / 1_000_000,
        member_count: cluster.member_region_ids.length,
        dominant_feature: cluster.dominant_feature,
      })),
      beacons: [],
    },
  };
}

function buildRegionalAtlas(
  regionalScenes: AdmittedRegionalProjection[],
  focusId: string,
): TopologyProjection {
  const nodes = regionalScenes.map(
    ({ workload, result, scene, atlas_region: atlasRegion }) => {
      const longitude = atlasRegion.semantic_longitude_microdegrees;
      const latitude = atlasRegion.semantic_latitude_microdegrees;
      const mercator = projectSemanticMercator(
        {
          longitude_microdegrees: longitude,
          latitude_microdegrees: latitude,
        },
        semanticMercatorFrame(),
      );
      return {
        ...node(
          `regional-atlas:${atlasRegion.region_id}`,
          `regional:${scene.scene_id}`,
          "COUNTY",
          scene.region_id,
          `SCENE@${scene.admission.editor_sequence} · sector ${shortCoordinate(atlasRegion.sector_id)} · point placement only${mercator.polar_disclosure ? ` · ${mercator.polar_disclosure.replace("_", " ")} clipped at ${SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES}µ°` : ""} · ${scene.projection.objects.length} exact native objects · ${shortCoordinate(result.result_id)}`,
          mercator.x,
          mercator.y,
          230,
          scene.complete ? "healthy" : "omitted",
          workload.workload.id,
        ),
        semantic_identity: atlasRegion.region_id,
        semantic_coordinate: {
          longitude_microdegrees: longitude,
          latitude_microdegrees: latitude,
        },
      };
    },
  );
  const sectors = new Map(
    regionalScenes.map(({ atlas_sector: sector }) => [
      sector.sector_id,
      sector,
    ]),
  );
  const regions = [...sectors.values()].flatMap((sector) =>
    projectSemanticMercatorBounds(
      `atlas-sector:${sector.sector_id}`,
      { ...sector, crosses_antimeridian: false },
      semanticMercatorFrame(),
    ).map((fragment) => ({
      id: fragment.identity,
      fragment_id: fragment.fragment_id,
      label: `SECTOR ${sector.longitude_band + 1}.${sector.latitude_band + 1}`,
      detail: `${sector.member_region_ids.length} admitted ${sector.member_region_ids.length === 1 ? "member" : "members"} · synthetic partition only · not a County footprint${fragment.polar_disclosures.length > 0 ? ` · ${fragment.polar_disclosures.map((cap) => cap.replace("_", " ")).join(" + ")} clipped at ±${SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES}µ°` : ""}`,
      x: fragment.x,
      y: fragment.y,
      width: Math.max(1, fragment.width),
      height: Math.max(1, fragment.height),
      tone: "neutral" as const,
      variant: "map-zone" as const,
    })),
  );
  return {
    regime: "atlas",
    label: "SEMANTIC MERCATOR ATLAS",
    detail: `${regionalScenes.length} admitted regional point ${regionalScenes.length === 1 ? "placement" : "placements"} · ${regions.length} stable occupied ${regions.length === 1 ? "sector" : "sectors"} · three bounded chart copies share one semantic identity`,
    focus_id: focusId,
    regions,
    nodes,
    edges: [],
    omissions: [
      "synthetic sector polygons express membership only; they are not surveyed coverage or native County footprints",
      "semantic Mercator positions are not Earth CRS84, EPSG:3857, physical distance, or geographic area",
      "semantic Mercator clips at ±85051129µ°; retained polar-cap membership is disclosed rather than silently dropped",
      ...regionalScenes.flatMap(({ scene }) =>
        scene.omissions.map((omission) => omission.reason),
      ),
    ],
    bearing: {
      status: "charted",
      label: "REGIONAL PLACEMENTS CHARTED",
      detail:
        "select one admitted County point through any chart copy; inverse picking returns its canonical synthetic coordinate and unchanged identity",
      sampled_conditions: regionalScenes.length,
      unresolved_boundaries: regionalScenes.reduce(
        (count, { scene }) => count + scene.omissions.length,
        0,
      ),
    },
    world: TOPOLOGY_WORLD,
    fit_world: TOPOLOGY_WORLD,
    terrain: false,
  };
}

function buildRegionalCounty(
  regionalScenes: AdmittedRegionalProjection[],
  focusId: string,
  regime: LensRegime,
): TopologyProjection {
  const selected = selectRegionalScene(regionalScenes, focusId);
  const {
    workload,
    result,
    scene,
    county_frame: countyFrame,
    county_footprint: countyFootprint,
  } = selected;
  if (!countyFootprint)
    throw new Error("County projection requires an admitted footprint");
  const world = TOPOLOGY_WORLD;
  const bounds = scene.native_bounds;
  const objects = scene.projection.objects;
  const frameView = countyFrameView(countyFrame, world);
  const projectedFootprint = projectCountyFootprint(
    countyFrame,
    countyFootprint,
    frameView,
  );
  const nodes = objects.map((object) => {
    const local = nativeBoundsToCountyLocal(countyFrame, object.native_bounds);
    const screen = projectCountyLocal(countyFrame, local, frameView);
    const width = countyObjectWidth(bounds, object.native_bounds, world);
    const terrainSample = scene.projection.terrain?.samples.find(
      (sample) => sample.source_object_id === object.object_id,
    );
    const exactDetail = `${object.layer.replaceAll("_", " ")} · ${object.geometry_kind} · ${object.source_path} · ${shortCoordinate(object.object_revision)}${terrainSample ? ` · ${terrainSample.position[2]}µm · ${terrainSample.material}` : ""}`;
    return {
      ...node(
        `regional-object:${object.object_id}`,
        `regional-object:${object.object_id}`,
        object.layer.replaceAll("_", " ").toUpperCase(),
        object.object_id,
        regime === "evidence"
          ? `${exactDetail} · source artifact ${shortCoordinate(object.source_artifact_id)}`
          : exactDetail,
        screen.x,
        screen.y,
        width,
        regionalLayerTone(object.layer),
        workload.workload.id,
      ),
      evidence_uri: regionalObjectEvidenceRoute(
        workload.workload.id,
        scene.scene_id,
        object.object_revision,
      ),
    };
  });
  const selectedObject = objects.find(
    (object) => `regional-object:${object.object_id}` === focusId,
  );
  const copyByRegime: Record<LensRegime, readonly [string, string]> = {
    landscape: [
      "ADMITTED COUNTY",
      `${scene.region_id} · exact admitted footprint · ${scene.projection.terrain ? `${scene.projection.terrain.samples.length} exact terrain samples; no interpolation` : "terrain height unsupported"}`,
    ],
    neighborhoods: [
      "COUNTY NEIGHBORHOODS",
      `${scene.projection.layers.length} typed layers · ${objects.length} exact native objects`,
    ],
    objects: [
      "COUNTY OBJECTS",
      selectedObject
        ? `${selectedObject.object_id} · ${selectedObject.source_path}`
        : `${objects.length} bounded objects`,
    ],
    evidence: [
      "COUNTY EVIDENCE",
      selectedObject
        ? `${selectedObject.object_id} · ${selectedObject.object_revision}`
        : `admission ${scene.admission.admission_id}`,
    ],
    world: ["REGIONAL EVIDENCE WORLD", scene.region_id],
    atlas: ["SEMANTIC MERCATOR ATLAS", scene.region_id],
  };
  const copy = copyByRegime[regime];
  const validityBoundaries = scene.projection.validity.filter(
    (validity) => validity.class !== "valid",
  );
  return {
    regime,
    label: copy[0],
    detail: copy[1],
    focus_id: focusId,
    regions: [],
    nodes,
    edges: [],
    omissions: [
      ...scene.omissions.map((omission) => omission.reason),
      ...validityBoundaries.map(
        (validity) => `${validity.class}: ${validity.scope} · ${validity.rule}`,
      ),
      `County fabric and validity end at exact footprint ${shortCoordinate(countyFootprint.footprint_id)} from ${countyFootprint.source_object_id}; holes remain holes`,
      countyFrame.authority,
    ],
    bearing: {
      status: "charted",
      label: "EXACT COUNTY FOOTPRINT",
      detail: `result ${shortCoordinate(result.result_id)} · packet ${shortCoordinate(scene.projection.packet_id)} · footprint ${shortCoordinate(countyFootprint.footprint_id)} · frame ${shortCoordinate(countyFrame.transform_digest)} · ${countyFrame.pitch_degrees}° pitch / ${countyFrame.yaw_degrees}° yaw · County-local presentation retains native CRS84 source identity`,
      sampled_conditions: objects.length,
      unresolved_boundaries: scene.omissions.length + validityBoundaries.length,
    },
    world,
    fit_world: world,
    terrain: false,
    county_frame: countyFrame,
    county_footprint: projectedFootprint,
  };
}

function selectRegionalScene(
  regionalScenes: AdmittedRegionalProjection[],
  focusId: string,
): AdmittedRegionalProjection {
  const selected = regionalScenes.find(({ scene }) =>
    regionalSceneMatchesFocus(scene, focusId),
  );
  if (!selected)
    throw new Error("County projection requires an admitted focus");
  return selected;
}

function regionalSceneMatchesFocus(
  scene: AdmittedRegionalProjection["scene"],
  focusId: string,
) {
  return (
    focusId === `regional:${scene.scene_id}` ||
    scene.projection.objects.some(
      (object) => focusId === `regional-object:${object.object_id}`,
    )
  );
}

function semanticMercatorFrame() {
  return {
    x: 70,
    y: 55,
    width: TOPOLOGY_WORLD.width - 140,
    height: TOPOLOGY_WORLD.height - 110,
  };
}

function countyObjectWidth(
  sceneBounds: AdmittedRegionalProjection["scene"]["native_bounds"],
  objectBounds: AdmittedRegionalProjection["scene"]["native_bounds"],
  world: TopologyWorld,
) {
  const longitudeSpan = regionalLongitudeSpan(sceneBounds);
  const latitudeSpan = Math.max(
    1,
    sceneBounds.north_microdegrees - sceneBounds.south_microdegrees,
  );
  const west = regionalLongitudeOffset(
    sceneBounds,
    objectBounds.west_microdegrees,
  );
  const east = regionalLongitudeOffset(
    sceneBounds,
    objectBounds.east_microdegrees,
  );
  const usableWidth = world.width - 220;
  return Math.max(
    120,
    Math.min(260, ((east - west) / longitudeSpan) * usableWidth),
  );
}

function regionalLongitudeSpan(
  bounds: AdmittedRegionalProjection["scene"]["native_bounds"],
) {
  const east = bounds.crosses_antimeridian
    ? bounds.east_microdegrees + 360_000_000
    : bounds.east_microdegrees;
  return Math.max(1, east - bounds.west_microdegrees);
}

function regionalLongitudeOffset(
  bounds: AdmittedRegionalProjection["scene"]["native_bounds"],
  longitude: number,
) {
  const adjusted =
    bounds.crosses_antimeridian && longitude < bounds.west_microdegrees
      ? longitude + 360_000_000
      : longitude;
  return Math.max(
    0,
    Math.min(
      regionalLongitudeSpan(bounds),
      adjusted - bounds.west_microdegrees,
    ),
  );
}

function regionalLayerTone(
  layer: AdmittedRegionalProjection["scene"]["projection"]["objects"][number]["layer"],
): TopologyTone {
  if (layer === "terrain") return "healthy";
  if (["hydrology", "highway", "road", "connector"].includes(layer))
    return "accent";
  if (["boundary", "district", "lot"].includes(layer)) return "neutral";
  if (["poi", "label", "beacon", "construction"].includes(layer))
    return "attention";
  if (layer === "terrain_control") return "unsupported";
  return "healthy";
}

function formatRegionalBounds(
  bounds: AdmittedRegionalProjection["scene"]["native_bounds"],
) {
  return `${bounds.west_microdegrees}µ°, ${bounds.south_microdegrees}µ° → ${bounds.east_microdegrees}µ°, ${bounds.north_microdegrees}µ°${bounds.crosses_antimeridian ? " · crosses antimeridian" : ""}`;
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
    return buildSurveyScene(topographies, requestedFocusId, "objects");
  const focusId = resolveObjectFocus(portfolio, requestedFocusId);
  if (focusId.startsWith("workload:")) {
    const workloadId = focusId.slice("workload:".length);
    const workload = portfolio.workloads.find(
      (candidate) => candidate.workload.id === workloadId,
    );
    const draft = portfolio.drafts.find(
      (candidate) => candidate.request.workload_id === workloadId,
    );
    if (workload) return buildWorkloadObjectScene(portfolio, workload, focusId);
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
    if (agent) return buildAgentObjectScene(portfolio, agent, focusId);
  }
  return portfolioObjectScene(portfolio, focusId);
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
  return buildSurveyScene(topographies, focusId, "evidence");
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
