import { terrainMeshByteLength, type TerrainMeshData } from "@rey/explorer";
import type { TerrainFieldSet } from "./compile";
import type { TerrainTileDescriptor } from "./tiles";

export const TERRAIN_TILE_RESIDENCY_REVISION =
  "rey.terrain.tile-residency@1" as const;
export const MAX_TERRAIN_TILE_CPU_BYTES = 48 * 1024 * 1024;
export const MAX_TERRAIN_TILE_GPU_BYTES = 64 * 1024 * 1024;

export interface CompiledTerrainTile {
  descriptor: TerrainTileDescriptor;
  fields: TerrainFieldSet;
  mesh: TerrainMeshData;
}

export interface TerrainTileResidencyStats {
  revision: typeof TERRAIN_TILE_RESIDENCY_REVISION;
  generation: number;
  entries: number;
  cpu_bytes: number;
  gpu_bytes: number;
  cpu_budget_bytes: number;
  gpu_budget_bytes: number;
  hits: number;
  misses: number;
  evictions: number;
}

interface ResidentEntry extends CompiledTerrainTile {
  last_requested_generation: number;
}

export class TerrainTileResidency {
  readonly #maximumCpuBytes: number;
  readonly #maximumGpuBytes: number;
  readonly #entries = new Map<string, ResidentEntry>();
  #generation = 0;
  #cpuBytes = 0;
  #gpuBytes = 0;
  #hits = 0;
  #misses = 0;
  #evictions = 0;

  constructor(
    maximumCpuBytes = MAX_TERRAIN_TILE_CPU_BYTES,
    maximumGpuBytes = MAX_TERRAIN_TILE_GPU_BYTES,
  ) {
    if (
      !Number.isSafeInteger(maximumCpuBytes) ||
      !Number.isSafeInteger(maximumGpuBytes) ||
      maximumCpuBytes < 1 ||
      maximumGpuBytes < 1
    )
      throw new Error("terrain tile residency budgets are invalid");
    this.#maximumCpuBytes = maximumCpuBytes;
    this.#maximumGpuBytes = maximumGpuBytes;
  }

  admit(
    compiled: readonly CompiledTerrainTile[],
    requestedTileIds: readonly string[],
  ): readonly CompiledTerrainTile[] {
    this.#generation += 1;
    const requested = new Set(requestedTileIds);
    if (requested.size !== requestedTileIds.length)
      throw new Error("terrain residency request contains duplicate tiles");
    const compiledById = new Map(
      compiled.map((tile) => [tile.descriptor.tile_id, tile]),
    );
    const requestedTiles = requestedTileIds.map((tileId) => {
      const tile = this.#entries.get(tileId) ?? compiledById.get(tileId);
      if (!tile)
        throw new Error(`terrain worker omitted requested tile ${tileId}`);
      return tile;
    });
    const activeCpuBytes = requestedTiles.reduce(
      (total, tile) => total + tile.fields.field_bytes,
      0,
    );
    const activeGpuBytes = requestedTiles.reduce(
      (total, tile) => total + terrainMeshByteLength(tile.mesh),
      0,
    );
    if (
      activeCpuBytes > this.#maximumCpuBytes ||
      activeGpuBytes > this.#maximumGpuBytes
    )
      throw new Error("active terrain tiles exceed the retained budgets");
    for (const tileId of requestedTileIds) {
      const resident = this.#entries.get(tileId);
      if (resident) {
        resident.last_requested_generation = this.#generation;
        this.#hits += 1;
        continue;
      }
      const tile = compiledById.get(tileId);
      if (!tile) throw new Error("terrain residency preflight changed");
      const cpuBytes = tile.fields.field_bytes;
      const gpuBytes = terrainMeshByteLength(tile.mesh);
      if (cpuBytes > this.#maximumCpuBytes || gpuBytes > this.#maximumGpuBytes)
        throw new Error("one terrain tile exceeds its residency budgets");
      this.#entries.set(tileId, {
        ...tile,
        last_requested_generation: this.#generation,
      });
      this.#cpuBytes += cpuBytes;
      this.#gpuBytes += gpuBytes;
      this.#misses += 1;
    }
    this.#evictToBudget(requested);
    if (
      this.#cpuBytes > this.#maximumCpuBytes ||
      this.#gpuBytes > this.#maximumGpuBytes
    )
      throw new Error("active terrain tiles exceed the retained budgets");
    return Object.freeze(
      requestedTileIds.map((tileId) => {
        const resident = this.#entries.get(tileId);
        if (!resident)
          throw new Error("active terrain tiles exceed the retained budgets");
        const { last_requested_generation: _, ...tile } = resident;
        return Object.freeze(tile);
      }),
    );
  }

  stats(): TerrainTileResidencyStats {
    return Object.freeze({
      revision: TERRAIN_TILE_RESIDENCY_REVISION,
      generation: this.#generation,
      entries: this.#entries.size,
      cpu_bytes: this.#cpuBytes,
      gpu_bytes: this.#gpuBytes,
      cpu_budget_bytes: this.#maximumCpuBytes,
      gpu_budget_bytes: this.#maximumGpuBytes,
      hits: this.#hits,
      misses: this.#misses,
      evictions: this.#evictions,
    });
  }

  #evictToBudget(requested: ReadonlySet<string>) {
    const candidates = [...this.#entries.entries()]
      .filter(([tileId]) => !requested.has(tileId))
      .sort(
        ([leftId, left], [rightId, right]) =>
          left.last_requested_generation - right.last_requested_generation ||
          leftId.localeCompare(rightId),
      );
    for (const [tileId, tile] of candidates) {
      if (
        this.#cpuBytes <= this.#maximumCpuBytes &&
        this.#gpuBytes <= this.#maximumGpuBytes
      )
        break;
      this.#entries.delete(tileId);
      this.#cpuBytes -= tile.fields.field_bytes;
      this.#gpuBytes -= terrainMeshByteLength(tile.mesh);
      this.#evictions += 1;
    }
  }
}
