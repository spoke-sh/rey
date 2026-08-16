import type {
  AdmittedRegionalScene,
  ContractIdentity,
  SemanticAtlas,
  SemanticAtlasRegionalRegion,
  SceneAdmissionResult,
  WorkloadList,
  WorkloadSummary,
} from "../../domain";
import {
  compileCountyFootprint,
  compileCountyFrame,
  type CountyFootprint,
  type CountyFrame,
} from "./county-frame";
import {
  regionalTerrainGridCellAt,
  regionalTerrainGridValueColumns,
  validRegionalTerrainGridTransport,
} from "../terrain/regional-grid-transport";

export interface AdmittedRegionalProjection {
  workload: WorkloadSummary;
  result: SceneAdmissionResult;
  scene: AdmittedRegionalScene;
  atlas_region: SemanticAtlasRegionalRegion;
  atlas_sector: SemanticAtlas["sectors"][number];
  county_frame: CountyFrame;
  county_footprint: CountyFootprint | null;
}

const coordinateSpaces = [
  "native_crs84",
  "synthetic_semantic",
  "semantic_mercator",
  "county_local",
  "camera",
] as const;

export function admittedRegionalScenes(
  portfolio: WorkloadList,
): AdmittedRegionalProjection[] {
  return portfolio.workloads.flatMap((workload) => {
    const results =
      workload.scene_admissions ??
      (workload.latest_scene_admission
        ? [workload.latest_scene_admission]
        : []);
    return results.flatMap((result) => {
      const scene = result?.scene;
      const atlas = portfolio.semantic_atlas;
      const retainedAtlas = portfolio.semantic_atlas_history.at(-1);
      const retainedDelta = portfolio.semantic_atlas_deltas.at(-1);
      const admittedAtlas = portfolio.semantic_atlas_history.find(
        (candidate) =>
          candidate.atlas_revision === scene?.artifacts.admitted_atlas_revision,
      );
      const semanticPlacement = scene?.projection.transforms.find(
        (transform) =>
          transform.source_space === "native_crs84" &&
          transform.target_space === "synthetic_semantic",
      );
      const atlasSource = atlas?.regional_sources.find(
        (source) =>
          source.workload_id === workload.workload.id &&
          source.scene_region_id === scene?.region_id &&
          source.source_scene_id === scene?.scene_id &&
          source.source_admission_id === scene?.admission.admission_id &&
          source.source_package_id === scene?.admission.package_id &&
          source.source_package_revision ===
            scene?.admission.package_snapshot_revision &&
          source.projection_packet_id === scene?.projection.packet_id,
      );
      const admittedAtlasSource = admittedAtlas?.regional_sources.find(
        (source) =>
          source.workload_id === workload.workload.id &&
          source.scene_region_id === scene?.region_id &&
          source.source_scene_id === scene?.scene_id &&
          source.source_admission_id === scene?.admission.admission_id &&
          source.source_package_id === scene?.admission.package_id &&
          source.source_package_revision ===
            scene?.admission.package_snapshot_revision &&
          source.projection_packet_id === scene?.projection.packet_id,
      );
      const atlasRegion = atlas?.regional_regions.find(
        (region) =>
          region.region_id === atlasSource?.region_id &&
          region.workload_id === atlasSource.workload_id &&
          region.scene_region_id === atlasSource.scene_region_id &&
          region.source_scene_id === atlasSource.source_scene_id &&
          region.source_admission_id === atlasSource.source_admission_id &&
          region.source_package_id === atlasSource.source_package_id &&
          region.source_package_revision ===
            atlasSource.source_package_revision &&
          region.projection_packet_id === atlasSource.projection_packet_id,
      );
      const atlasSector = atlas?.sectors.find(
        (sector) =>
          sector.sector_id === atlasRegion?.sector_id &&
          sector.member_region_ids.includes(atlasRegion.region_id),
      );
      const countyFrame = scene ? safeCountyFrame(scene) : null;
      const countyFootprint = scene ? safeCountyFootprint(scene) : undefined;
      const terrainValid = scene ? validRegionalTerrain(scene) : false;
      if (
        !result ||
        result.schema !== "rey.scene-admission-result.v1" ||
        result.status !== "accepted" ||
        result.scenario !== null ||
        !scene ||
        scene.schema !== "rey.admitted-regional-scene.v1" ||
        scene.projection.schema !== "rey.regional-projection-packet.v1" ||
        !sameContract(result.workload, workload.workload) ||
        !sameContract(result.graph, workload.candidate_graph) ||
        !sameContract(scene.admission.workload, result.workload) ||
        !sameContract(scene.admission.graph, result.graph) ||
        scene.admission.capability_snapshot_id !==
          result.capability_snapshot_id ||
        scene.admission.package_id !== scene.projection.source_package_id ||
        scene.admission.package_snapshot_revision !==
          scene.projection.source_snapshot_revision ||
        scene.artifacts.projection_packet_id !== scene.projection.packet_id ||
        !admittedAtlasSource ||
        scene.artifacts.terrain_program_id !==
          scene.projection.terrain_program_id ||
        !terrainValid ||
        scene.projection.coordinate_bindings.length !==
          coordinateSpaces.length ||
        scene.projection.coordinate_bindings.some(
          (binding, index) => binding.space !== coordinateSpaces[index],
        ) ||
        !semanticPlacement ||
        semanticPlacement.target_origin.length !== 2 ||
        !semanticPlacement.target_origin.every(Number.isFinite) ||
        !atlas ||
        retainedAtlas?.atlas_revision !== atlas.atlas_revision ||
        retainedDelta?.target_revision !== atlas.atlas_revision ||
        portfolio.semantic_atlas_history.length !==
          portfolio.semantic_atlas_deltas.length ||
        !atlasSource ||
        !atlasRegion ||
        !atlasSector ||
        !countyFrame ||
        countyFootprint === undefined ||
        atlasRegion.semantic_longitude_microdegrees !==
          semanticPlacement.target_origin[0] ||
        atlasRegion.semantic_latitude_microdegrees !==
          semanticPlacement.target_origin[1]
      )
        return [];
      return [
        {
          workload,
          result,
          scene,
          atlas_region: atlasRegion,
          atlas_sector: atlasSector,
          county_frame: countyFrame,
          county_footprint: countyFootprint,
        },
      ];
    });
  });
}

function validRegionalTerrain(scene: AdmittedRegionalScene): boolean {
  const terrain = scene.projection.terrain;
  if (!terrain)
    return (
      scene.projection.terrain_program_id === null &&
      scene.artifacts.terrain_program_id === null
    );
  if (
    ![
      "rey.regional-terrain-program.v1",
      "rey.regional-terrain-program.v2",
    ].includes(terrain.schema) ||
    terrain.program_id !== scene.projection.terrain_program_id ||
    terrain.program_id !== scene.artifacts.terrain_program_id ||
    terrain.authority !== scene.artifacts.terrain_authority ||
    terrain.height_unit !== "micrometer" ||
    terrain.material_semantics !==
      "source-declared bounded material identifier; no inferred physical properties"
  )
    return false;
  const objectsById = new Map(
    scene.projection.objects.map((object) => [object.object_id, object]),
  );
  if (objectsById.size !== scene.projection.objects.length) return false;
  const samplesValid = terrain.samples.every((sample) => {
    const object = objectsById.get(sample.source_object_id);
    return (
      object?.layer === "terrain" &&
      object.geometry_kind === "Point" &&
      object.source_artifact_id === sample.source_artifact_id &&
      object.object_revision === sample.source_object_revision &&
      object.native_bounds.west_microdegrees === sample.position[0] &&
      object.native_bounds.east_microdegrees === sample.position[0] &&
      object.native_bounds.south_microdegrees === sample.position[1] &&
      object.native_bounds.north_microdegrees === sample.position[1]
    );
  });
  if (!samplesValid) return false;
  if (terrain.grid) {
    const packed =
      terrain.grid.schema !== "rey.regional-terrain-grid.v1" &&
      terrain.grid.cell_source_encoding === "geojson_packed_grid_v1";
    return (
      terrain.samples.length === 0 &&
      terrain.schema === "rey.regional-terrain-program.v2" &&
      terrain.evaluator.id === "rey.regional-terrain.rectilinear-grid" &&
      terrain.evaluator.revision === (packed ? 2 : 1) &&
      terrain.interpolation ===
        "piecewise linear only within triangles whose three admitted source vertices are valid" &&
      terrain.authority ===
        (packed
          ? "qualified packed rectilinear height/material grid; validity ends at supported source triangles"
          : "qualified rectilinear height/material grid; validity ends at supported source triangles") &&
      validRegionalTerrainGrid(terrain.grid, objectsById)
    );
  }
  return (
    terrain.samples.length > 0 &&
    terrain.schema === "rey.regional-terrain-program.v1" &&
    terrain.evaluator.id === "rey.regional-terrain.exact-samples" &&
    terrain.evaluator.revision === 1 &&
    terrain.interpolation === "none; exact admitted samples only" &&
    terrain.authority ===
      "qualified exact height/material samples; no interpolated terrain coverage"
  );
}

function validRegionalTerrainGrid(
  grid: NonNullable<AdmittedRegionalScene["projection"]["terrain"]>["grid"],
  objectsById: ReadonlyMap<
    string,
    AdmittedRegionalScene["projection"]["objects"][number]
  >,
): boolean {
  if (!grid) return false;
  const expectedCells = grid.columns * grid.rows;
  const packed =
    grid.schema !== "rey.regional-terrain-grid.v1" &&
    grid.cell_source_encoding === "geojson_packed_grid_v1";
  if (
    !validRegionalTerrainGridTransport(grid) ||
    !Number.isSafeInteger(grid.columns) ||
    !Number.isSafeInteger(grid.rows) ||
    grid.columns < 2 ||
    grid.rows < 2 ||
    grid.dataset_id.length === 0 ||
    grid.source_dataset_id.length === 0 ||
    grid.native_bounds.crosses_antimeridian ||
    grid.native_bounds.west_microdegrees >=
      grid.native_bounds.east_microdegrees ||
    grid.native_bounds.south_microdegrees >=
      grid.native_bounds.north_microdegrees ||
    grid.validity_semantics !==
      "row-major source vertices are explicitly valid or no_data; no_data cuts triangle support" ||
    grid.interpolation !==
      "piecewise linear only within triangles whose three admitted source vertices are valid" ||
    grid.authority !==
      (packed
        ? "qualified packed rectilinear height/material grid; validity ends at supported source triangles"
        : "qualified rectilinear height/material grid; validity ends at supported source triangles")
  )
    return false;
  const longitudeStep =
    (grid.native_bounds.east_microdegrees -
      grid.native_bounds.west_microdegrees) /
    (grid.columns - 1);
  const latitudeStep =
    (grid.native_bounds.north_microdegrees -
      grid.native_bounds.south_microdegrees) /
    (grid.rows - 1);
  if (!Number.isInteger(longitudeStep) || !Number.isInteger(latitudeStep))
    return false;
  if (
    grid.schema === "rey.regional-terrain-grid.transport.v2" ||
    grid.schema === "rey.regional-terrain-grid.transport.v3"
  ) {
    const values = regionalTerrainGridValueColumns(grid);
    for (let index = 0; index < expectedCells; index += 1) {
      const valid = values.validity[index] === 1;
      const elevation = values.elevation_micrometers[index];
      const material = values.material_indices[index]!;
      if (
        !Number.isSafeInteger(elevation) ||
        (valid &&
          (elevation! < -12_000_000_000 ||
            elevation! > 100_000_000_000 ||
            material >= values.material_palette.length ||
            !/^[A-Za-z0-9._-]{1,64}$/.test(
              values.material_palette[material]!,
            ))) ||
        (!valid && material !== 255)
      )
        return false;
    }
    return hasSupportedTerrainTriangle(
      grid.columns,
      grid.rows,
      values.validity,
    );
  }
  const identities = new Set<string>();
  const objects = new Set<string>();
  let cellsValid = true;
  for (let index = 0; index < expectedCells; index += 1) {
    const cell = regionalTerrainGridCellAt(grid, index);
    if (!cell) return false;
    const column = index % grid.columns;
    const row = Math.floor(index / grid.columns);
    const object = objectsById.get(cell.source_object_id);
    const validValue =
      cell.validity === "valid"
        ? Number.isSafeInteger(cell.elevation_micrometers) &&
          cell.elevation_micrometers! >= -12_000_000_000 &&
          cell.elevation_micrometers! <= 100_000_000_000 &&
          typeof cell.material === "string" &&
          /^[A-Za-z0-9._-]{1,64}$/.test(cell.material) &&
          cell.authority ===
            (packed
              ? "exact packed source altitude and material at one valid grid vertex"
              : "exact admitted Point altitude and material at one valid grid vertex")
        : cell.validity === "no_data" &&
          cell.elevation_micrometers === null &&
          cell.material === null &&
          cell.authority ===
            (packed
              ? "explicit packed source no-data vertex; grid position locates the hole but supplies no height or material"
              : "explicit source no-data vertex; geometry locates the hole but supplies no height or material");
    const sourceValid =
      grid.schema === "rey.regional-terrain-grid.transport.v1"
        ? cell.source_artifact_id === grid.source_artifact_id &&
          cell.source_object_id === grid.source_object_ids[index] &&
          cell.source_object_revision === grid.source_object_revisions[index]
        : object?.layer === "terrain" &&
          object.geometry_kind === "Point" &&
          object.source_artifact_id === cell.source_artifact_id &&
          object.object_revision === cell.source_object_revision &&
          object.native_bounds.west_microdegrees === cell.native_position[0] &&
          object.native_bounds.east_microdegrees === cell.native_position[0] &&
          object.native_bounds.south_microdegrees === cell.native_position[1] &&
          object.native_bounds.north_microdegrees === cell.native_position[1];
    cellsValid =
      cellsValid &&
      cell.cell_id.length > 0 &&
      !identities.has(cell.cell_id) &&
      !objects.has(cell.source_object_id) &&
      Boolean(identities.add(cell.cell_id)) &&
      Boolean(objects.add(cell.source_object_id)) &&
      cell.grid_position[0] === column &&
      cell.grid_position[1] === row &&
      cell.native_position[0] ===
        grid.native_bounds.west_microdegrees + column * longitudeStep &&
      cell.native_position[1] ===
        grid.native_bounds.north_microdegrees - row * latitudeStep &&
      validValue &&
      sourceValid;
  }
  if (!cellsValid) return false;
  for (let row = 0; row < grid.rows - 1; row += 1) {
    for (let column = 0; column < grid.columns - 1; column += 1) {
      const topLeft = row * grid.columns + column;
      const topRight = topLeft + 1;
      const bottomLeft = topLeft + grid.columns;
      const bottomRight = bottomLeft + 1;
      const valid = (index: number) =>
        regionalTerrainGridCellAt(grid, index)?.validity === "valid";
      if (
        (valid(topLeft) && valid(bottomLeft) && valid(bottomRight)) ||
        (valid(topLeft) && valid(bottomRight) && valid(topRight)) ||
        (valid(topLeft) && valid(bottomLeft) && valid(topRight)) ||
        (valid(topRight) && valid(bottomLeft) && valid(bottomRight))
      )
        return true;
    }
  }
  return false;
}

function hasSupportedTerrainTriangle(
  columns: number,
  rows: number,
  validity: Uint8Array,
): boolean {
  const valid = (index: number) => validity[index] === 1;
  for (let row = 0; row < rows - 1; row += 1) {
    for (let column = 0; column < columns - 1; column += 1) {
      const topLeft = row * columns + column;
      const topRight = topLeft + 1;
      const bottomLeft = topLeft + columns;
      const bottomRight = bottomLeft + 1;
      if (
        (valid(topLeft) && valid(bottomLeft) && valid(bottomRight)) ||
        (valid(topLeft) && valid(bottomRight) && valid(topRight)) ||
        (valid(topLeft) && valid(bottomLeft) && valid(topRight)) ||
        (valid(topRight) && valid(bottomLeft) && valid(bottomRight))
      )
        return true;
    }
  }
  return false;
}

function safeCountyFrame(scene: AdmittedRegionalScene) {
  try {
    return compileCountyFrame(scene);
  } catch {
    return null;
  }
}

function safeCountyFootprint(scene: AdmittedRegionalScene) {
  try {
    return compileCountyFootprint(scene);
  } catch {
    return undefined;
  }
}

function sameContract(left: ContractIdentity, right: ContractIdentity) {
  return (
    left.id === right.id &&
    left.revision === right.revision &&
    left.semantic_digest === right.semantic_digest
  );
}
