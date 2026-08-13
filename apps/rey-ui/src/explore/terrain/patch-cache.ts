import {
  materializeTerrainWorkingSet,
  type TerrainFieldSet,
  type TerrainProgram,
  type TerrainWorkingSetRequest,
} from "./compile";

export interface TerrainPatchCacheStats {
  entries: number;
  field_cells: number;
  field_bytes: number;
  hits: number;
  misses: number;
  evictions: number;
}

export class TerrainPatchCache {
  readonly #maximumCells: number;
  readonly #maximumBytes: number;
  readonly #entries = new Map<
    string,
    { fields: TerrainFieldSet; cells: number; bytes: number }
  >();
  #cells = 0;
  #bytes = 0;
  #hits = 0;
  #misses = 0;
  #evictions = 0;

  constructor(maximumCells: number, maximumBytes: number) {
    if (
      !Number.isSafeInteger(maximumCells) ||
      !Number.isSafeInteger(maximumBytes) ||
      maximumCells < 1 ||
      maximumBytes < 1
    )
      throw new Error("terrain patch cache limits are invalid");
    this.#maximumCells = maximumCells;
    this.#maximumBytes = maximumBytes;
  }

  materialize(
    program: TerrainProgram,
    request: TerrainWorkingSetRequest,
  ): TerrainFieldSet {
    const key = `${program.program_id}|${request.working_set_id}`;
    const retained = this.#entries.get(key);
    if (retained) {
      this.#entries.delete(key);
      this.#entries.set(key, retained);
      this.#hits += 1;
      return retained.fields;
    }
    this.#misses += 1;
    const fields = materializeTerrainWorkingSet(program, request);
    if (
      fields.field_cells > this.#maximumCells ||
      fields.field_bytes > this.#maximumBytes
    )
      throw new Error("terrain patch exceeds its retained cache budget");
    while (
      this.#entries.size > 0 &&
      (this.#cells + fields.field_cells > this.#maximumCells ||
        this.#bytes + fields.field_bytes > this.#maximumBytes)
    ) {
      const oldestKey = this.#entries.keys().next().value;
      if (oldestKey === undefined) break;
      const oldest = this.#entries.get(oldestKey)!;
      this.#entries.delete(oldestKey);
      this.#cells -= oldest.cells;
      this.#bytes -= oldest.bytes;
      this.#evictions += 1;
    }
    this.#entries.set(key, {
      fields,
      cells: fields.field_cells,
      bytes: fields.field_bytes,
    });
    this.#cells += fields.field_cells;
    this.#bytes += fields.field_bytes;
    return fields;
  }

  stats(): TerrainPatchCacheStats {
    return Object.freeze({
      entries: this.#entries.size,
      field_cells: this.#cells,
      field_bytes: this.#bytes,
      hits: this.#hits,
      misses: this.#misses,
      evictions: this.#evictions,
    });
  }
}
