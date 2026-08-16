import type {
  ProjectionPacket,
  RegionalLayerKind,
  SemanticAtlas,
  SemanticAtlasDelta,
  WorkloadList,
} from "./domain";
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
  nativePositionToCountyLocal,
  nativeBoundsToCountyLocal,
  projectCountyFootprint,
  projectCountyLocal,
  type CountyFrame,
  type ProjectedCountyFootprint,
} from "./explore/projection/county-frame";
import {
  compileRegionalTerrainField,
  projectRegionalTerrainFootprint,
  projectRegionalTerrainPosition,
} from "./explore/projection/regional-terrain";
import { ATLAS_LANDSCAPE_PROJECTION_REVISION } from "./explore/projection/atlas-landscape";
import {
  type TerrainFieldSet,
  type TerrainProgram,
} from "./explore/terrain/compile";
import { deriveRegionalTerrainContours } from "./explore/terrain/contours";
import { buildSurveyScene } from "./explore/projection/survey-scene";
import {
  buildPortfolioEvidence,
  buildPortfolioLandscape,
  buildPortfolioNeighborhoods,
  buildPortfolioObjects,
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
  spatial_feature?: {
    geometry_kind: string;
    layer: RegionalLayerKind;
    envelope_path: string;
    geometry_path: string;
    geometry_representation: "exact_native" | "bounds_envelope";
    authority: string;
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
  atlas_landscape_transition: TopologyAtlasLandscapeTransition | null;
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

export interface TopologyAtlasLandscapeTransition {
  schema: "rey.atlas-landscape-transition.v1";
  transition_id: string;
  atlas_revision: string;
  scene_id: string;
  terrain_field_id: string;
  projection_revision: typeof ATLAS_LANDSCAPE_PROJECTION_REVISION;
  source_frame: { x: number; y: number; width: number; height: number };
  target_frame: { x: number; y: number; width: number; height: number };
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
    (regionalScene) =>
      regionalSceneTraversable(regionalScene) &&
      regionalSceneMatchesFocus(regionalScene.scene, focusId),
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
  else if (regionalScenes.length > 0 && !surveyFocus && !regionalFocus) {
    const traversable = regionalScenes.filter(regionalSceneTraversable);
    projection =
      traversable.length === 1 && focusId === "cluster:portfolio"
        ? buildRegionalCounty(
            regionalScenes,
            `regional:${traversable[0]!.scene.scene_id}`,
            regime,
          )
        : buildRegionalAtlas(regionalScenes, focusId);
  } else if (
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
  else if (regime === "objects")
    projection = buildPortfolioObjects(portfolio, focusId);
  else projection = buildPortfolioEvidence(portfolio, focusId);
  const atlasLandscape =
    regionalScenes.length > 0 &&
    !surveyFocus &&
    regionalFocus &&
    (regime === "atlas" || regime === "landscape")
      ? buildAtlasLandscapeTransition(regionalScenes, focusId)
      : null;
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
    terrain_fields:
      projection.terrain_fields ??
      (regime === "atlas" && atlasLandscape
        ? [atlasLandscape.terrain_field]
        : []),
    terrain_programs: projection.terrain_programs ?? [],
    globe: projection.globe ?? null,
    world_atlas_transition:
      regionalScenes.length > 0 &&
      !surveyFocus &&
      (regime === "world" ||
        regime === "atlas" ||
        (regime === "landscape" && atlasLandscape !== null))
        ? buildWorldAtlasTransition(regionalScenes)
        : null,
    atlas_landscape_transition: atlasLandscape?.transition ?? null,
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
  | "atlas_landscape_transition"
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
  atlas_landscape_transition?: TopologyAtlasLandscapeTransition | null;
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

function buildAtlasLandscapeTransition(
  regionalScenes: AdmittedRegionalProjection[],
  focusId: string,
): {
  transition: TopologyAtlasLandscapeTransition;
  terrain_field: TerrainFieldSet;
} | null {
  const selected = selectRegionalScene(regionalScenes, focusId);
  const terrainField = compileRegionalTerrainField(
    selected.scene,
    TOPOLOGY_WORLD,
  );
  const atlasRevision = selected.scene.artifacts.admitted_atlas_revision;
  if (!terrainField || !atlasRevision) return null;
  const fragments = projectSemanticMercatorBounds(
    `atlas-landscape:${selected.atlas_sector.sector_id}`,
    {
      ...selected.atlas_sector,
      crosses_antimeridian: false,
    },
    semanticMercatorFrame(),
  );
  const point = projectSemanticMercator(
    {
      longitude_microdegrees:
        selected.atlas_region.semantic_longitude_microdegrees,
      latitude_microdegrees:
        selected.atlas_region.semantic_latitude_microdegrees,
    },
    semanticMercatorFrame(),
  );
  const source =
    fragments.find(
      (fragment) =>
        point.x >= fragment.x &&
        point.x <= fragment.x + fragment.width &&
        point.y >= fragment.y &&
        point.y <= fragment.y + fragment.height,
    ) ?? fragments[0];
  if (!source)
    throw new Error("Atlas-to-Landscape transition has no source fragment");
  const sourceFrame = Object.freeze({
    x: source.x,
    y: source.y,
    width: Math.max(1, source.width),
    height: Math.max(1, source.height),
  });
  const targetFrame = Object.freeze({ ...terrainField.grid.bounds });
  return Object.freeze({
    transition: Object.freeze({
      schema: "rey.atlas-landscape-transition.v1",
      transition_id: [
        "rey.atlas-landscape-transition.v1",
        atlasRevision,
        selected.scene.scene_id,
        terrainField.field_set_id,
        ATLAS_LANDSCAPE_PROJECTION_REVISION,
        `${sourceFrame.x},${sourceFrame.y},${sourceFrame.width},${sourceFrame.height}`,
        `${targetFrame.x},${targetFrame.y},${targetFrame.width},${targetFrame.height}`,
      ].join("|"),
      atlas_revision: atlasRevision,
      scene_id: selected.scene.scene_id,
      terrain_field_id: terrainField.field_set_id,
      projection_revision: ATLAS_LANDSCAPE_PROJECTION_REVISION,
      source_frame: sourceFrame,
      target_frame: targetFrame,
      authority:
        "reversible presentation mapping from one exact admitted synthetic Atlas sector to one exact admitted regional terrain field; it grants no geographic relationship between the coordinate spaces",
    }),
    terrain_field: terrainField,
  });
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
  const world = TOPOLOGY_WORLD;
  const bounds = scene.native_bounds;
  const objects = scene.projection.objects;
  const frameView = countyFrameView(countyFrame, world);
  const terrainField = compileRegionalTerrainField(scene, world);
  if (!countyFootprint && !terrainField)
    throw new Error(
      "County projection requires an admitted footprint or terrain validity grid",
    );
  const projectedFootprint = countyFootprint
    ? terrainField
      ? projectRegionalTerrainFootprint(
          countyFrame,
          scene.native_bounds,
          countyFootprint,
          world,
        )
      : projectCountyFootprint(countyFrame, countyFootprint, frameView)
    : null;
  const terrainSamplesByObjectId = new Map(
    scene.projection.terrain?.samples.map((sample) => [
      sample.source_object_id,
      sample,
    ]) ?? [],
  );
  const terrainGridCellsByObjectId = new Map(
    scene.projection.terrain?.grid?.cells.map((cell) => [
      cell.source_object_id,
      cell,
    ]) ?? [],
  );
  const visibleObjects =
    terrainField && (regime === "landscape" || regime === "neighborhoods")
      ? objects.filter(
          (object) => !terrainGridCellsByObjectId.has(object.object_id),
        )
      : objects;
  const nodes = visibleObjects.map((object) => {
    const local = nativeBoundsToCountyLocal(countyFrame, object.native_bounds);
    const nativeCenter = [
      object.native_bounds.west_microdegrees +
        (object.native_bounds.east_microdegrees -
          object.native_bounds.west_microdegrees) /
          2,
      object.native_bounds.south_microdegrees +
        (object.native_bounds.north_microdegrees -
          object.native_bounds.south_microdegrees) /
          2,
    ] as const;
    const screen = terrainField
      ? projectRegionalTerrainPosition(scene.native_bounds, nativeCenter, world)
      : projectCountyLocal(countyFrame, local, frameView);
    const width = countyObjectWidth(bounds, object.native_bounds, world);
    const envelopePath = projectRegionalObjectEnvelope(
      countyFrame,
      object.native_bounds,
      frameView,
      terrainField ? { scene_bounds: scene.native_bounds, world } : undefined,
    );
    const geometryPath = object.native_geometry
      ? projectRegionalObjectGeometry(
          countyFrame,
          object.native_geometry,
          frameView,
          terrainField
            ? { scene_bounds: scene.native_bounds, world }
            : undefined,
        )
      : null;
    const terrainSample = terrainSamplesByObjectId.get(object.object_id);
    const terrainGridCell = terrainGridCellsByObjectId.get(object.object_id);
    const terrainDetail = terrainSample
      ? ` · ${terrainSample.position[2]}µm · ${terrainSample.material}`
      : terrainGridCell?.validity === "valid"
        ? ` · ${terrainGridCell.elevation_micrometers}µm · ${terrainGridCell.material} · grid ${terrainGridCell.grid_position.join(",")}`
        : terrainGridCell
          ? ` · NO DATA · grid ${terrainGridCell.grid_position.join(",")}`
          : "";
    const exactDetail = `${object.layer.replaceAll("_", " ")} · ${object.geometry_kind} · ${object.source_path} · ${shortCoordinate(object.object_revision)}${terrainDetail}`;
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
      spatial_feature: {
        geometry_kind: object.geometry_kind,
        layer: object.layer,
        envelope_path: envelopePath,
        geometry_path: geometryPath ?? envelopePath,
        geometry_representation: geometryPath
          ? ("exact_native" as const)
          : ("bounds_envelope" as const),
        authority: object.authority,
      },
    };
  });
  const selectedObject = objects.find(
    (object) => `regional-object:${object.object_id}` === focusId,
  );
  const copyByRegime: Record<LensRegime, readonly [string, string]> = {
    landscape: [
      countyFootprint ? "ADMITTED COUNTY" : "ADMITTED TERRAIN",
      `${scene.region_id} · ${countyFootprint ? "exact admitted footprint" : "exact terrain-grid validity mask"} · ${scene.projection.terrain?.grid ? `${scene.projection.terrain.grid.columns}×${scene.projection.terrain.grid.rows} admitted terrain grid; no-data retained` : scene.projection.terrain ? `${scene.projection.terrain.samples.length} exact terrain samples; no interpolation` : "terrain height unsupported"}`,
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
    contours: terrainField
      ? deriveRegionalTerrainContours(terrainField, regime)
      : [],
    nodes,
    edges: [],
    omissions: [
      ...scene.omissions.map((omission) => omission.reason),
      ...validityBoundaries.map(
        (validity) => `${validity.class}: ${validity.scope} · ${validity.rule}`,
      ),
      countyFootprint
        ? `County fabric and validity end at exact footprint ${shortCoordinate(countyFootprint.footprint_id)} from ${countyFootprint.source_object_id}; holes remain holes`
        : `Terrain fabric and validity end at exact grid ${shortCoordinate(scene.projection.terrain!.grid!.dataset_id)}; no-data vertices cut triangle support and remain holes`,
      countyFrame.authority,
    ],
    bearing: {
      status: "charted",
      label: countyFootprint
        ? "EXACT COUNTY FOOTPRINT"
        : "EXACT TERRAIN VALIDITY",
      detail: `result ${shortCoordinate(result.result_id)} · packet ${shortCoordinate(scene.projection.packet_id)} · ${countyFootprint ? `footprint ${shortCoordinate(countyFootprint.footprint_id)}` : `terrain grid ${shortCoordinate(scene.projection.terrain!.grid!.dataset_id)}`} · frame ${shortCoordinate(countyFrame.transform_digest)} · ${countyFrame.pitch_degrees}° pitch / ${countyFrame.yaw_degrees}° yaw · County-local presentation retains native CRS84 source identity`,
      sampled_conditions: objects.length,
      unresolved_boundaries: scene.omissions.length + validityBoundaries.length,
    },
    world,
    fit_world: world,
    terrain: terrainField !== null,
    terrain_fields: terrainField ? [terrainField] : [],
    county_frame: countyFrame,
    county_footprint: projectedFootprint,
  };
}

function projectRegionalObjectGeometry(
  frame: CountyFrame,
  geometry: NonNullable<
    AdmittedRegionalProjection["scene"]["projection"]["objects"][number]["native_geometry"]
  >,
  view: ReturnType<typeof countyFrameView>,
  terrain?: {
    scene_bounds: AdmittedRegionalProjection["scene"]["native_bounds"];
    world: { width: number; height: number };
  },
) {
  const project = (position: readonly [number, number]) =>
    terrain
      ? projectRegionalTerrainPosition(
          terrain.scene_bounds,
          position,
          terrain.world,
        )
      : projectCountyLocal(
          frame,
          nativePositionToCountyLocal(frame, position),
          view,
        );
  const path = (
    positions: ReadonlyArray<readonly [number, number]>,
    close: boolean,
  ) =>
    positions
      .map((position, index) => {
        const point = project(position);
        return `${index === 0 ? "M" : "L"}${point.x.toFixed(2)} ${point.y.toFixed(2)}`;
      })
      .join(" ") + (close ? " Z" : "");
  if (geometry.kind === "point") return path([geometry.position], false);
  if (geometry.kind === "line_string") return path(geometry.positions, false);
  return geometry.rings.map((ring) => path(ring, true)).join(" ");
}

function projectRegionalObjectEnvelope(
  frame: CountyFrame,
  bounds: AdmittedRegionalProjection["scene"]["native_bounds"],
  view: ReturnType<typeof countyFrameView>,
  terrain?: {
    scene_bounds: AdmittedRegionalProjection["scene"]["native_bounds"];
    world: { width: number; height: number };
  },
) {
  const corners = [
    [bounds.west_microdegrees, bounds.south_microdegrees],
    [bounds.east_microdegrees, bounds.south_microdegrees],
    [bounds.east_microdegrees, bounds.north_microdegrees],
    [bounds.west_microdegrees, bounds.north_microdegrees],
  ] as const;
  return (
    corners
      .map((position, index) => {
        const point = terrain
          ? projectRegionalTerrainPosition(
              terrain.scene_bounds,
              position,
              terrain.world,
            )
          : projectCountyLocal(
              frame,
              nativePositionToCountyLocal(frame, position),
              view,
            );
        return `${index === 0 ? "M" : "L"}${point.x.toFixed(2)} ${point.y.toFixed(2)}`;
      })
      .join(" ") + " Z"
  );
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

function regionalSceneTraversable(
  regionalScene: AdmittedRegionalProjection,
): boolean {
  return (
    regionalScene.county_footprint !== null ||
    regionalScene.scene.projection.terrain?.grid !== undefined
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

function stableHash(value: string): number {
  let hash = 2_166_136_261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16_777_619);
  }
  return hash >>> 0;
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
