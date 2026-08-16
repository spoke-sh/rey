import type {
  AdmittedRegionalScene,
  ContractIdentity,
  RegionalBounds,
  SemanticAtlasDelta,
  SemanticAtlasRegionalRegion,
  WorkloadList,
  WorkloadSummary,
} from "./domain";
import { admittedRegionalScenes } from "./explore/projection/regional-scene-projector";
import {
  regionalTerrainGridCellAt,
  regionalTerrainGridCellIndexForRevision,
} from "./explore/terrain/regional-grid-transport";
import { regionalObjectEvidenceRoute } from "./regional-object-route";
import { environmentStyles as chrome } from "./stylex/environment.stylex";
import { className as sx } from "./stylex/shared.stylex";
import { workloadsStyles as styles } from "./stylex/workloads.stylex";

type RegionalObject = AdmittedRegionalScene["projection"]["objects"][number];
type RegionalLayer = AdmittedRegionalScene["projection"]["layers"][number];
type RegionalValidity = AdmittedRegionalScene["projection"]["validity"][number];

export interface RegionalObjectEvidence {
  schema: "rey.ui-regional-object-evidence.v1";
  authority: string;
  route: string;
  workload: WorkloadSummary;
  result: NonNullable<WorkloadSummary["latest_scene_admission"]>;
  scene: AdmittedRegionalScene;
  object: RegionalObject;
  layer: RegionalLayer;
  object_validity: RegionalValidity;
  validity: RegionalValidity[];
  atlas_region: SemanticAtlasRegionalRegion;
  atlas_delta: SemanticAtlasDelta;
  atlas_change: SemanticAtlasDelta["region_changes"][number] | null;
  object_delta: null;
  object_delta_boundary: string;
}

export function resolveRegionalObjectEvidence(
  portfolio: WorkloadList,
  workloadId: string,
  sceneId: string,
  objectRevision: string,
): RegionalObjectEvidence | null {
  const projections = admittedRegionalScenes(portfolio).filter(
    ({ workload, scene }) =>
      workload.workload.id === workloadId && scene.scene_id === sceneId,
  );
  if (projections.length !== 1) return null;
  const projection = projections[0]!;
  const retainedObjects = projection.scene.projection.objects.filter(
    (object) => object.object_revision === objectRevision,
  );
  const transportedObject =
    retainedObjects.length === 0
      ? transportedTerrainObject(projection.scene, objectRevision)
      : null;
  const objects = transportedObject
    ? [transportedObject.object]
    : retainedObjects;
  if (objects.length !== 1) return null;
  const object = objects[0]!;
  const layers = projection.scene.projection.layers.filter(
    (layer) =>
      layer.kind === object.layer &&
      (layer.object_ids.includes(object.object_id) ||
        (transportedObject !== null &&
          layer.kind === "terrain" &&
          layer.object_ids.length === 0 &&
          projection.scene.projection.transport?.schema ===
            "rey.regional-projection-packet.transport.v1")),
  );
  const objectValidity = transportedObject
    ? [transportedObject.validity]
    : projection.scene.projection.validity.filter(
        (validity) =>
          validity.scope === `native_geometry:${object.object_id}` &&
          validity.source_revision === object.object_revision,
      );
  const atlasDelta = portfolio.semantic_atlas_deltas.at(-1);
  if (layers.length !== 1 || objectValidity.length !== 1 || !atlasDelta)
    return null;
  return {
    schema: "rey.ui-regional-object-evidence.v1",
    authority:
      "read-only projection of the exact retained scene admission and atlas delta; no source read, admission, mutation, relationship, terrain, action, or proof authority",
    route: regionalObjectEvidenceRoute(workloadId, sceneId, objectRevision),
    workload: projection.workload,
    result: projection.result,
    scene: projection.scene,
    object,
    layer: layers[0]!,
    object_validity: objectValidity[0]!,
    validity: transportedObject
      ? [...projection.scene.projection.validity, transportedObject.validity]
      : projection.scene.projection.validity,
    atlas_region: projection.atlas_region,
    atlas_delta: atlasDelta,
    atlas_change:
      atlasDelta.region_changes.find(
        (change) => change.region_id === projection.atlas_region.region_id,
      ) ?? null,
    object_delta: null,
    object_delta_boundary:
      "the retained workload document exposes the directed atlas revision delta, not an object-local source diff; no object change is inferred",
  };
}

function transportedTerrainObject(
  scene: AdmittedRegionalScene,
  objectRevision: string,
): { object: RegionalObject; validity: RegionalValidity } | null {
  const grid = scene.projection.terrain?.grid;
  if (
    (grid?.schema !== "rey.regional-terrain-grid.transport.v1" &&
      grid?.schema !== "rey.regional-terrain-grid.transport.v2") ||
    scene.projection.transport?.schema !==
      "rey.regional-projection-packet.transport.v1"
  )
    return null;
  const index = regionalTerrainGridCellIndexForRevision(grid, objectRevision);
  if (index === undefined) return null;
  const cell = regionalTerrainGridCellAt(grid, index);
  if (!cell) return null;
  const nativeBounds = {
    west_microdegrees: cell.native_position[0],
    south_microdegrees: cell.native_position[1],
    east_microdegrees: cell.native_position[0],
    north_microdegrees: cell.native_position[1],
    crosses_antimeridian: false,
  };
  return {
    object: {
      object_id: cell.source_object_id,
      source_id: grid.source_id,
      source_path: grid.source_path,
      source_artifact_id: cell.source_artifact_id,
      object_revision: cell.source_object_revision,
      geometry_kind: "Point",
      native_bounds: nativeBounds,
      native_geometry: {
        kind: "point",
        position: cell.native_position,
      },
      layer: "terrain",
      authority:
        grid.cell_source_encoding === "geojson_packed_grid_v1"
          ? "exact packed source cell reconstructed from the lossless row-major terrain transport; appearance grants no relationship, activity, or action authority"
          : "exact admitted native geometry reconstructed from the lossless row-major terrain transport; appearance grants no relationship, activity, or action authority",
    },
    validity: {
      validity_id: cell.cell_id,
      class: cell.validity,
      scope: `terrain_grid_cell:${cell.source_object_id}`,
      source_revision: cell.source_object_revision,
      rule:
        grid.cell_source_encoding === "geojson_packed_grid_v1"
          ? cell.validity === "valid"
            ? "exact packed terrain cell has height and material only at this source coordinate"
            : "exact packed no-data cell locates a validity hole and supplies no height or material"
          : cell.validity === "valid"
            ? "exact admitted terrain cell has height and material only at this source coordinate"
            : "exact admitted no-data cell locates a validity hole and supplies no height or material",
    },
  };
}

export function RegionalObjectEvidencePage({
  evidence,
}: {
  evidence: RegionalObjectEvidence;
}) {
  const object = evidence.object;
  const sceneOmissions = evidence.scene.omissions.map((omission) => ({
    plane: "scene",
    ...omission,
  }));
  const packetOmissions = evidence.scene.projection.omissions.map(
    (omission) => ({ plane: "projection", ...omission }),
  );
  return (
    <main className={sx(chrome.page, styles.page)}>
      <EvidenceLink
        href={`/workloads/${encodeURIComponent(evidence.workload.workload.id)}`}
      >
        ← WORKLOAD EVIDENCE
      </EvidenceLink>

      <section
        className={sx(styles.section, styles.firstSection)}
        data-rey-section="01 / PLAIN"
      >
        <EvidenceHeading
          detail={`${evidence.scene.region_id} · ${evidence.scene.complete ? "complete" : "partial"}`}
          index="01"
          kicker="PLAIN / SELECTED OBJECT"
          title={object.object_id}
        />
        <div className={sx(styles.evidenceSummary)}>
          <EvidenceMetric label="LAYER" value={object.layer.toUpperCase()} />
          <EvidenceMetric
            label="GEOMETRY"
            value={object.geometry_kind.toUpperCase()}
          />
          <EvidenceMetric
            label="VALIDITY"
            value={evidence.object_validity.class.toUpperCase()}
          />
          <EvidenceMetric
            label="COVERAGE"
            value={evidence.scene.complete ? "COMPLETE" : "PARTIAL"}
          />
        </div>
        <article className={sx(styles.evidenceCard, styles.evidenceList)}>
          <strong>NATIVE SOURCE / {object.source_path}</strong>
          <code className={sx(styles.breakable)}>{object.object_revision}</code>
          <p className={sx(styles.description)}>
            {object.authority}. The browser binds the exact workspace-relative
            source path and artifact identity below; no source-reader provider
            is admitted, so the path is not presented as a fetchable URL.
          </p>
        </article>
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="02 / -V BINDINGS"
      >
        <EvidenceHeading
          detail="native source → admission → atlas revision"
          index="02"
          kicker="-V / BINDINGS"
          title="Source, admission, delta, and validity"
        />
        <ExactRows
          rows={[
            ["object", object.object_id],
            ["object revision", object.object_revision],
            ["source", `${object.source_id} · ${object.source_path}`],
            ["source artifact", object.source_artifact_id],
            ["native bounds", formatBounds(object.native_bounds)],
            ["layer", `${evidence.layer.layer_id} · ${evidence.layer.kind}`],
            ["layer revision", evidence.layer.source_revision],
            ["layer semantics", evidence.layer.semantics],
            ["admission result", evidence.result.result_id],
            ["admission", evidence.scene.admission.admission_id],
            ["candidate", evidence.result.candidate_id],
            ["editor commit", evidence.scene.admission.editor_commit_id],
            ["package", evidence.scene.admission.package_id],
            [
              "package revision",
              evidence.scene.admission.package_snapshot_revision,
            ],
            ["workload", formatContract(evidence.result.workload)],
            ["graph", formatContract(evidence.result.graph)],
            ["capability", evidence.result.capability_snapshot_id],
          ]}
        />
        <article className={sx(styles.evidenceCard, styles.evidenceList)}>
          <div className={sx(styles.evidenceCardHeader)}>
            <strong>DIRECTED ATLAS DELTA</strong>
            <code>{evidence.atlas_delta.delta_id}</code>
          </div>
          <div className={sx(styles.evidenceValues)}>
            <EvidenceValue
              label="SOURCE REVISION"
              value={evidence.atlas_delta.source_revision}
            />
            <EvidenceValue
              label="TARGET REVISION"
              value={evidence.atlas_delta.target_revision}
            />
          </div>
          <p className={sx(styles.description)}>
            REGION / {evidence.atlas_region.region_id} · CHANGE /{" "}
            {evidence.atlas_change?.kind.toUpperCase() ??
              "UNCHANGED IN THIS DELTA"}
          </p>
          <p className={sx(chrome.micro, styles.description)}>
            {evidence.object_delta_boundary}
          </p>
        </article>
        <div className={sx(styles.evidenceList)}>
          {evidence.validity.map((validity) => (
            <article
              className={sx(styles.evidenceCard)}
              key={validity.validity_id}
            >
              <div className={sx(styles.evidenceCardHeader)}>
                <strong>
                  {validity.class.toUpperCase()} · {validity.scope}
                </strong>
                <code>{validity.validity_id}</code>
              </div>
              <p className={sx(styles.description)}>{validity.rule}</p>
              <code className={sx(styles.breakable)}>
                REVISION / {validity.source_revision}
              </code>
            </article>
          ))}
        </div>
      </section>

      <section
        className={sx(styles.section)}
        data-rey-section="03 / -VV EXACT EVIDENCE"
      >
        <EvidenceHeading
          detail="limits, omissions, and lineage"
          index="03"
          kicker="-VV / EXACT EVIDENCE"
          title="Verified retained envelope"
        />
        <ExactRows
          rows={[
            ["route", evidence.route],
            ["scene", evidence.scene.scene_id],
            ["region", evidence.scene.region_id],
            ["projection packet", evidence.scene.projection.packet_id],
            [
              "atlas revision",
              evidence.scene.artifacts.admitted_atlas_revision ?? "absent",
            ],
            ["admission limits", formatLimits(evidence.result.limits)],
            [
              "projection limits",
              formatLimits(evidence.scene.projection.limits),
            ],
            [
              "observed counts",
              `objects=${evidence.scene.projection.objects.length} · layers=${evidence.scene.projection.layers.length} · validity=${evidence.scene.projection.validity.length} · omissions=${sceneOmissions.length + packetOmissions.length}`,
            ],
          ]}
        />
        <div className={sx(styles.evidenceList)}>
          {[...sceneOmissions, ...packetOmissions].map((omission, index) => (
            <article
              className={sx(styles.evidenceCard)}
              key={`${omission.plane}:${omission.kind}:${omission.subject}:${index}`}
            >
              <strong>
                {omission.plane.toUpperCase()} OMISSION · {omission.kind}
              </strong>
              <p className={sx(styles.description)}>{omission.reason}</p>
              <code>
                SUBJECT / {omission.subject} · COUNT / {omission.omitted_count}
              </code>
            </article>
          ))}
          {[
            ...evidence.scene.lineage.map((entry) => ({
              plane: "scene",
              ...entry,
            })),
            ...evidence.scene.projection.lineage.map((entry) => ({
              plane: "projection",
              ...entry,
            })),
          ].map((entry, index) => (
            <article
              className={sx(styles.evidenceCard)}
              key={`${entry.plane}:${entry.kind}:${entry.identity}:${index}`}
            >
              <strong>
                {entry.plane.toUpperCase()} LINEAGE · {entry.kind}
              </strong>
              <code className={sx(styles.breakable)}>{entry.identity}</code>
              <code className={sx(styles.breakable)}>
                REVISION / {entry.revision}
              </code>
            </article>
          ))}
        </div>
        <p className={sx(chrome.micro, styles.description)}>
          {evidence.authority}
        </p>
      </section>
    </main>
  );
}

function EvidenceHeading({
  detail,
  index,
  kicker,
  title,
}: {
  detail: string;
  index: string;
  kicker: string;
  title: string;
}) {
  return (
    <header className={sx(styles.sectionHeading)}>
      <span className={sx(styles.sectionIndex)}>{index}</span>
      <div>
        <p className={sx(chrome.micro, styles.kicker)}>{kicker}</p>
        <h2>{title}</h2>
      </div>
      <small className={sx(chrome.micro)}>{detail}</small>
    </header>
  );
}

function EvidenceMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className={sx(styles.evidenceMetric)}>
      <span className={sx(chrome.micro)}>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function EvidenceValue({ label, value }: { label: string; value: string }) {
  return (
    <pre className={sx(styles.evidenceValue)}>
      <span className={sx(chrome.micro)}>{label}</span>
      {value}
    </pre>
  );
}

function ExactRows({ rows }: { rows: Array<[string, string]> }) {
  return (
    <dl className={sx(styles.exactRows)}>
      {rows.map(([label, value], index) => (
        <div className={sx(styles.exactRow)} key={`${label}:${index}`}>
          <dt className={sx(chrome.micro)}>{label.toUpperCase()}</dt>
          <dd className={sx(styles.exactValue)}>
            <code>{value}</code>
          </dd>
        </div>
      ))}
    </dl>
  );
}

function EvidenceLink({ children, href }: { children: string; href: string }) {
  return (
    <a className={sx(styles.location)} href={href}>
      {children}
    </a>
  );
}

function formatContract(identity: ContractIdentity): string {
  return `${identity.id}@${identity.revision} · ${identity.semantic_digest}`;
}

function formatBounds(bounds: RegionalBounds): string {
  return `${bounds.west_microdegrees},${bounds.south_microdegrees} → ${bounds.east_microdegrees},${bounds.north_microdegrees}µ°${bounds.crosses_antimeridian ? " · crosses antimeridian" : ""}`;
}

function formatLimits(value: Record<string, number>): string {
  return Object.entries(value)
    .map(([key, limit]) => `${key}=${limit}`)
    .join(" · ");
}
