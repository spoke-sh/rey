import { Link } from "@tanstack/react-router";
import type { CSSProperties } from "react";
import { OBJECT_LENS_ZOOM, type LensRegime } from "../engine/camera";
import { exploreStyles as styles } from "../../stylex/explore.stylex";
import { className as sx } from "../../stylex/shared.stylex";
import type {
  TopologyEdge,
  TopologyNode,
  TopologyPointOfInterest,
  TopologyScene,
  TopologyTone,
} from "../../topology";

export type FocusableTopologyObject = Pick<
  TopologyNode | TopologyPointOfInterest,
  "focus_id" | "x" | "y"
>;

export interface ReferenceLayerVisibility {
  relief: boolean;
  water: boolean;
  weather: boolean;
  probes: boolean;
}

export function ReferenceRenderer({
  accelerated = false,
  layers,
  onFocus,
  scene,
}: {
  accelerated?: boolean;
  layers: ReferenceLayerVisibility;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
}) {
  const globeWorld = scene.regime === "world" && scene.globe !== null;
  return (
    <div
      className={sx(
        styles.projection,
        scene.terrain && styles.terrainProjection,
        scene.terrain && accelerated && styles.acceleratedTerrainProjection,
        scene.terrain &&
          scene.regime === "world" &&
          styles.worldTerrainProjection,
      )}
      data-lens-regime={scene.regime}
      data-renderer={accelerated ? "reference-overlays" : "reference"}
    >
      {!globeWorld &&
        scene.regions.map((region) => (
          <div
            className={sx(
              styles.region,
              region.variant === "map-boundary" && styles.mapBoundary,
              region.variant === "map-zone" && styles.mapZone,
              toneStyle(region.tone, "region"),
              scene.regime === "world" &&
                region.variant === "map-zone" &&
                region.tone === "unknown" &&
                styles.worldUnexploredZone,
            )}
            key={region.id}
            style={{
              height: region.height,
              left: region.x,
              top: region.y,
              width: region.width,
            }}
          >
            <span>{region.label}</span>
            <small>{region.detail}</small>
          </div>
        ))}
      <WorldGeometryLayer
        accelerated={accelerated}
        onFocus={onFocus}
        scene={scene}
      />
      {!globeWorld && layers.relief ? <ReliefLayer scene={scene} /> : null}
      {!globeWorld && (layers.water || layers.weather) ? (
        <NaturalFeatureLayer
          scene={scene}
          showWater={layers.water}
          showWeather={layers.weather}
        />
      ) : null}
      {!scene.terrain && scene.edges.length > 0 ? (
        <EdgeLayer
          edges={scene.edges}
          nodes={scene.nodes}
          points={scene.points}
          terrain={scene.terrain}
          world={scene.world}
        />
      ) : null}
      {!globeWorld &&
        scene.points
          .filter((point) => layers.probes || point.kind !== "frontier")
          .map((point) => (
            <PointOfInterest
              key={point.id}
              onFocus={onFocus}
              point={point}
              regime={scene.regime}
            />
          ))}
      {!globeWorld &&
        scene.nodes.map((node) => (
          <TopologyObject
            counterScale={scene.terrain}
            key={node.id}
            linkToWorkload={scene.regime === "objects" && !scene.terrain}
            node={node}
            onFocus={onFocus}
          />
        ))}
    </div>
  );
}

function WorldGeometryLayer({
  accelerated,
  onFocus,
  scene,
}: {
  accelerated: boolean;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
}) {
  if (scene.regime === "world" && scene.globe)
    return (
      <SemanticGlobeLayer
        accelerated={accelerated}
        onFocus={onFocus}
        scene={scene}
      />
    );
  if (scene.landforms.length === 0) return null;
  const meridians = Array.from({ length: 11 }, (_, index) =>
    Math.round((scene.world.width * index) / 10),
  );
  const parallels = Array.from({ length: 7 }, (_, index) =>
    Math.round((scene.world.height * index) / 6),
  );
  return (
    <svg
      aria-label={`${scene.landforms.length} admitted world boundary geometries`}
      className={sx(styles.worldGeometryLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <title>
        Charted land is derived from admitted anchor extents. Dashed horizons
        include unresolved frontier probes and do not claim observed terrain.
      </title>
      <g className={sx(styles.worldGraticule)} aria-hidden="true">
        {meridians.map((x) => (
          <line
            key={`meridian:${x}`}
            x1={x}
            x2={x}
            y1={0}
            y2={scene.world.height}
          />
        ))}
        {parallels.map((y) => (
          <line
            key={`parallel:${y}`}
            x1={0}
            x2={scene.world.width}
            y1={y}
            y2={y}
          />
        ))}
      </g>
      {scene.landforms
        .filter((landform) => landform.kind === "horizon")
        .map((landform) => (
          <path
            className={sx(styles.worldHorizon)}
            d={landform.path}
            data-world-geometry={landform.kind}
            key={landform.id}
          >
            <title>{`${landform.label} / ${landform.detail}`}</title>
          </path>
        ))}
      {scene.landforms
        .filter((landform) => landform.kind === "charted")
        .map((landform) => (
          <path
            className={sx(
              styles.chartedLand,
              accelerated && styles.chartedLandAccelerated,
            )}
            data-accelerated-surface={accelerated || undefined}
            d={landform.path}
            data-world-geometry={landform.kind}
            key={landform.id}
          >
            <title>{`${landform.label} / ${landform.detail}`}</title>
          </path>
        ))}
    </svg>
  );
}

function SemanticGlobeLayer({
  accelerated,
  onFocus,
  scene,
}: {
  accelerated: boolean;
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
}) {
  const globe = scene.globe!;
  const center = { x: scene.world.width / 2, y: scene.world.height / 2 };
  const radius = Math.min(scene.world.width, scene.world.height) * 0.41;
  const projectedRegions = globe.regions
    .map((region) => ({ region, ...projectGlobe(region, center, radius) }))
    .filter(({ visible }) => visible)
    .sort((left, right) => left.depth - right.depth);
  const projectedClusters = globe.clusters
    .map((cluster) => ({ cluster, ...projectGlobe(cluster, center, radius) }))
    .filter(({ visible }) => visible);
  const focus = (focus_id: string, x: number, y: number) =>
    onFocus({ focus_id, x, y });
  return (
    <svg
      aria-label={`${globe.regions.length} admitted semantic world regions on a synthetic globe`}
      className={sx(styles.worldGeometryLayer, styles.semanticGlobeLayer)}
      data-atlas-revision={globe.atlas_revision}
      role="group"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <title>
        Synthetic semantic longitude and latitude place admitted survey regions
        on a spherical world. They are not Earth coordinates, and zoom never
        reclusters this atlas revision.
      </title>
      <defs>
        <radialGradient id="rey-semantic-globe-fill" cx="34%" cy="26%" r="76%">
          <stop offset="0%" stopColor="#eef1dc" />
          <stop offset="48%" stopColor="#aebd9a" />
          <stop offset="82%" stopColor="#708779" />
          <stop offset="100%" stopColor="#41585a" />
        </radialGradient>
        <clipPath id="rey-semantic-globe-clip">
          <circle cx={center.x} cy={center.y} r={radius} />
        </clipPath>
      </defs>
      <circle
        className={sx(styles.semanticGlobeSphere)}
        cx={center.x}
        cy={center.y}
        data-accelerated-surface={accelerated || undefined}
        fill={accelerated ? "transparent" : "url(#rey-semantic-globe-fill)"}
        r={radius}
      />
      <g
        aria-hidden="true"
        className={sx(styles.semanticGlobeGraticule)}
        clipPath="url(#rey-semantic-globe-clip)"
      >
        {[-60, -30, 0, 30, 60].map((latitude) => {
          const latitudeRadians = (latitude * Math.PI) / 180;
          return (
            <ellipse
              cx={center.x}
              cy={center.y - radius * Math.sin(latitudeRadians)}
              key={`latitude:${latitude}`}
              rx={radius * Math.cos(latitudeRadians)}
              ry={radius * 0.075 * Math.cos(latitudeRadians)}
            />
          );
        })}
        {[-60, -30, 0, 30, 60].map((longitude) => (
          <ellipse
            cx={center.x}
            cy={center.y}
            key={`longitude:${longitude}`}
            rx={Math.max(
              4,
              radius * Math.abs(Math.sin((longitude * Math.PI) / 180)),
            )}
            ry={radius}
          />
        ))}
      </g>
      <g aria-label={`${globe.clusters.length} world clusters`}>
        {projectedClusters.map(({ cluster, x, y, depth }) => (
          <circle
            className={sx(styles.semanticGlobeCluster)}
            cx={x}
            cy={y}
            key={cluster.id}
            r={Math.max(
              18,
              (cluster.angular_radius_degrees / 90) *
                radius *
                (0.72 + depth * 0.28),
            )}
          >
            <title>{`${cluster.member_count} admitted regions / ${cluster.dominant_feature.replaceAll("_", " ")} structure`}</title>
          </circle>
        ))}
      </g>
      <g aria-label={`${projectedRegions.length} visible semantic regions`}>
        {projectedRegions.map(({ region, x, y, depth }) => (
          <g
            aria-label={`${region.label}: ${region.detail}`}
            className={sx(
              styles.semanticGlobeRegion,
              toneStyle(region.tone, "node"),
            )}
            data-semantic-region={region.id}
            key={region.id}
            onClick={() => focus(region.focus_id, x, y)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                focus(region.focus_id, x, y);
              }
            }}
            role="button"
            tabIndex={0}
          >
            <circle
              className={sx(styles.semanticGlobeRegionPoint)}
              cx={x}
              cy={y}
              r={7 + depth * 7}
            />
            <text
              className={sx(styles.semanticGlobeRegionLabel)}
              x={x + 14}
              y={y - 10}
            >
              {region.label}
            </text>
            <title>{region.detail}</title>
          </g>
        ))}
      </g>
      <text
        className={sx(styles.semanticGlobeCaption)}
        x={center.x - radius}
        y={center.y + radius + 34}
      >
        SEMANTIC SPHERE / {globe.regions.length} ADMITTED REGIONS / REV{" "}
        {globe.atlas_revision.slice(0, 12)}
      </text>
    </svg>
  );
}

function projectGlobe(
  coordinate: { longitude_degrees: number; latitude_degrees: number },
  center: { x: number; y: number },
  radius: number,
) {
  const longitude = (coordinate.longitude_degrees * Math.PI) / 180;
  const latitude = (coordinate.latitude_degrees * Math.PI) / 180;
  const depth = Math.cos(latitude) * Math.cos(longitude);
  return {
    x: center.x + radius * Math.cos(latitude) * Math.sin(longitude),
    y: center.y - radius * Math.sin(latitude),
    depth,
    visible: depth >= -0.02,
  };
}

function NaturalFeatureLayer({
  scene,
  showWater,
  showWeather,
}: {
  scene: TopologyScene;
  showWater: boolean;
  showWeather: boolean;
}) {
  const features = scene.natural_features.filter((feature) =>
    feature.kind === "weather_front" ? showWeather : showWater,
  );
  if (features.length === 0) return null;
  const showLabels =
    scene.regime === "world" ||
    scene.regime === "atlas" ||
    scene.regime === "landscape";
  return (
    <svg
      aria-label={`${features.length} emergent natural features`}
      className={sx(styles.naturalFeatureLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <title>
        Weather fronts derive from unresolved admitted survey conditions.
        Streams and rivers derive from rainfall accumulation over anchor-only
        relief and visibly erode that projected field. None are source edges,
        discovered paths, or retained natural facts.
      </title>
      {features.map((feature) => {
        const pathId = `natural-feature-${feature.id.replaceAll(/[^a-zA-Z0-9_-]/g, "-")}`;
        return (
          <g className={sx(styles.naturalFeatureGroup)} key={feature.id}>
            <path
              className={sx(
                styles.naturalFeature,
                feature.kind === "stream" && styles.streamFeature,
                feature.kind === "river" && styles.riverFeature,
                feature.kind === "weather_front" && styles.weatherFront,
              )}
              d={feature.path}
              data-natural-feature={feature.kind}
              id={pathId}
              style={{
                strokeWidth: `calc(${0.7 + feature.intensity * (feature.kind === "river" ? 0.48 : 0.22)}px * var(--rey-terrain-counter-scale))`,
              }}
            >
              <title>{`${feature.label} / ${feature.detail}`}</title>
            </path>
            {showLabels &&
            (feature.kind === "river" || feature.kind === "weather_front") ? (
              <text className={sx(styles.naturalFeatureLabel)}>
                <textPath
                  href={`#${pathId}`}
                  startOffset="50%"
                  textAnchor="middle"
                >
                  {feature.label}
                </textPath>
              </text>
            ) : null}
          </g>
        );
      })}
    </svg>
  );
}

function ReliefLayer({ scene }: { scene: TopologyScene }) {
  if (scene.contours.length === 0) return null;
  return (
    <svg
      aria-label={`${scene.contours.length} anchor-derived terrain contour levels`}
      className={sx(styles.reliefLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <title>
        Relief height is derived only from admitted anchor samples, then eroded
        by the deterministic runoff projection. It does not assert semantic
        similarity or a discovered path.
      </title>
      {scene.contours.map((contour) => (
        <path
          className={sx(
            styles.reliefContour,
            contour.level >= 6 && styles.reliefContourPeak,
            contour.level <= 2 && styles.reliefContourLow,
          )}
          d={contour.path}
          data-anchor-count={contour.anchor_count}
          data-relief-level={contour.level}
          key={contour.id}
          style={{
            strokeDasharray:
              contour.level <= 2
                ? "calc(3px * var(--rey-terrain-counter-scale)) calc(3px * var(--rey-terrain-counter-scale))"
                : undefined,
            strokeWidth: `calc(${contour.level >= 6 ? 1.55 : 1.15}px * var(--rey-terrain-counter-scale))`,
          }}
        />
      ))}
    </svg>
  );
}

function EdgeLayer({
  edges,
  nodes,
  points,
  terrain,
  world,
}: {
  edges: TopologyEdge[];
  nodes: TopologyNode[];
  points: TopologyPointOfInterest[];
  terrain: boolean;
  world: TopologyScene["world"];
}) {
  const byId = new Map(
    [...nodes, ...points].map((candidate) => [candidate.id, candidate]),
  );
  return (
    <svg
      aria-hidden="true"
      className={sx(styles.edgeLayer)}
      viewBox={`0 0 ${world.width} ${world.height}`}
    >
      <defs>
        <marker
          id="topology-arrow"
          markerHeight="7"
          markerWidth="7"
          orient="auto-start-reverse"
          refX="6"
          refY="3.5"
        >
          <path d="M0,0 L7,3.5 L0,7 Z" />
        </marker>
      </defs>
      {edges.map((edge) => {
        const from = byId.get(edge.from);
        const to = byId.get(edge.to);
        if (!from || !to) return null;
        const midpoint = {
          x: (from.x + to.x) / 2,
          y: (from.y + to.y) / 2,
        };
        return (
          <g className={sx(styles.edgeGroup)} key={edge.id}>
            <line
              className={sx(
                styles.edge,
                edge.kind === "directs" && styles.edgeDirects,
                edge.kind === "contains" && styles.edgeContains,
              )}
              markerEnd={terrain ? undefined : "url(#topology-arrow)"}
              style={
                terrain
                  ? {
                      strokeDasharray:
                        edge.kind === "contains"
                          ? "calc(6px * var(--rey-terrain-counter-scale)) calc(5px * var(--rey-terrain-counter-scale))"
                          : undefined,
                      strokeWidth: `calc(${edge.kind === "directs" ? 2.4 : 1.5}px * var(--rey-terrain-counter-scale))`,
                    }
                  : undefined
              }
              x1={from.x}
              x2={to.x}
              y1={from.y}
              y2={to.y}
            />
            <text
              className={sx(styles.edgeLabel)}
              style={
                terrain
                  ? {
                      fontSize: "calc(9px * var(--rey-terrain-counter-scale))",
                      strokeWidth:
                        "calc(5px * var(--rey-terrain-counter-scale))",
                    }
                  : undefined
              }
              textAnchor="middle"
              x={midpoint.x}
              y={midpoint.y - 7}
            >
              {edge.label.toUpperCase()}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

function PointOfInterest({
  onFocus,
  point,
  regime,
}: {
  onFocus: (point: TopologyPointOfInterest) => void;
  point: TopologyPointOfInterest;
  regime: LensRegime;
}) {
  const showLabel =
    regime === "world"
      ? point.kind === "frontier" || point.prominence >= 4
      : regime === "atlas"
        ? point.prominence >= 3
        : regime === "landscape"
          ? point.prominence >= 2
          : true;
  const showDetail =
    regime === "neighborhoods" || regime === "objects" || regime === "evidence";
  return (
    <button
      aria-label={`${point.family} point of interest: ${point.label}`}
      className={sx(
        styles.pointOfInterest,
        point.kind === "frontier" && styles.frontierPoint,
        point.prominence >= 3 && styles.prominentPoint,
      )}
      data-prominence={point.prominence}
      onClick={() => onFocus(point)}
      style={{ left: point.x, top: point.y }}
      type="button"
    >
      <i
        className={sx(
          styles.pointMarker,
          point.kind === "frontier" && styles.frontierMarker,
        )}
        aria-hidden="true"
      />
      {showLabel ? (
        <span className={sx(styles.pointLabel)}>
          <small className={sx(styles.pointFamily)}>{point.family}</small>
          <strong className={sx(styles.pointName)}>{point.label}</strong>
          {showDetail ? (
            <>
              <em className={sx(styles.pointSignal)}>{point.signal}</em>
              <em className={sx(styles.pointDetail)}>{point.detail}</em>
              <em className={sx(styles.pointAction)}>{point.action}</em>
            </>
          ) : null}
        </span>
      ) : null}
    </button>
  );
}

function TopologyObject({
  counterScale,
  linkToWorkload,
  node,
  onFocus,
}: {
  counterScale: boolean;
  linkToWorkload: boolean;
  node: TopologyNode;
  onFocus: (node: FocusableTopologyObject) => void;
}) {
  const className = sx(styles.topologyObject, toneStyle(node.tone, "node"));
  const style = {
    left: node.x,
    top: node.y,
    ...(counterScale
      ? {
          transform:
            "translate(-50%, -50%) scale(var(--rey-terrain-counter-scale))",
        }
      : {}),
    width: node.width,
  } as CSSProperties;
  const content = (
    <>
      <span className={sx(styles.objectFamily)}>{node.family}</span>
      <strong className={sx(styles.objectLabel)}>{node.label}</strong>
      <small className={sx(styles.objectDetail)}>{node.detail}</small>
      <span className={sx(styles.objectAction)}>
        {node.coordinate_uri
          ? "OPEN COORDINATE ↗"
          : linkToWorkload && node.workload_id
            ? "OPEN RECORD ↗"
            : "FOCUS / ZOOM →"}
      </span>
    </>
  );

  if (node.coordinate_uri) {
    return (
      <a
        className={className}
        href={`/explore?coordinate=${encodeURIComponent(node.coordinate_uri)}&scale=${OBJECT_LENS_ZOOM}`}
        style={style}
      >
        {content}
      </a>
    );
  }

  if (linkToWorkload && node.workload_id) {
    return (
      <Link
        className={className}
        params={{ workloadId: node.workload_id }}
        style={style}
        to="/workloads/$workloadId"
      >
        {content}
      </Link>
    );
  }

  return (
    <button
      className={className}
      onClick={() => onFocus(node)}
      style={style}
      type="button"
    >
      {content}
    </button>
  );
}

export function ReferenceMapReading({ scene }: { scene: TopologyScene }) {
  if (!scene.terrain) return null;
  const waterSystems = scene.natural_features.filter(
    (feature) => feature.kind === "stream" || feature.kind === "river",
  );
  const weatherFronts = scene.natural_features.filter(
    (feature) => feature.kind === "weather_front",
  );
  const probes = scene.points.filter((point) => point.kind === "frontier");
  return (
    <aside className={sx(styles.mapReading)} aria-label="Map evidence legend">
      <div className={sx(styles.bearingCard)}>
        <span className={sx(styles.bearingEyebrow)}>
          BEARING / {scene.bearing.status.replaceAll("_", " ")}
        </span>
        <strong className={sx(styles.bearingTitle)}>
          {scene.bearing.label}
        </strong>
        <small className={sx(styles.bearingDetail)}>
          {scene.bearing.detail}
        </small>
      </div>
      <div className={sx(styles.mapKey)} aria-hidden="true">
        <span>
          <i className={sx(styles.keyContour)} /> ANCHORS / RELIEF
        </span>
        <span>
          <i className={sx(styles.keyStream)} /> RUNOFF / STREAM
        </span>
        <span>
          <i className={sx(styles.keyRiver)} /> ACCUMULATION / RIVER
        </span>
        <span>
          <i className={sx(styles.keyWeather)} /> UNRESOLVED / WEATHER
        </span>
      </div>
      <div className={sx(styles.mapScale)} aria-hidden="true">
        <i className={sx(styles.mapScaleBar)} />
        <span>
          {waterSystems.length} PROJECTED WATER SYSTEMS · {weatherFronts.length}{" "}
          WEATHER FRONTS · {probes.length} PROBES · NO PATH CLAIM · LOD{" "}
          {scene.regime.toUpperCase()}
        </span>
      </div>
    </aside>
  );
}

function toneStyle(tone: TopologyTone, kind: "node" | "region") {
  if (kind === "region") {
    if (tone === "accent") return styles.regionAccent;
    if (tone === "attention" || tone === "blocked")
      return styles.regionAttention;
    if (tone === "healthy") return styles.regionHealthy;
    if (tone === "unknown") return styles.regionUnknown;
    if (tone === "omitted") return styles.regionOmitted;
    if (tone === "stale") return styles.regionStale;
    if (tone === "unsupported") return styles.regionUnsupported;
    if (tone === "frontier") return styles.regionFrontier;
    return styles.regionNeutral;
  }
  if (tone === "accent") return styles.objectAccent;
  if (tone === "attention") return styles.objectAttention;
  if (tone === "blocked") return styles.objectBlocked;
  if (tone === "healthy") return styles.objectHealthy;
  if (tone === "unknown") return styles.objectUnknown;
  if (tone === "omitted") return styles.objectOmitted;
  if (tone === "stale") return styles.objectStale;
  if (tone === "unsupported") return styles.objectUnsupported;
  if (tone === "frontier") return styles.objectFrontier;
  return styles.objectNeutral;
}
