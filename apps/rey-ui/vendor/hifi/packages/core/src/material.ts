/** The stable envelope shared by every grammar-specific material document. */
export interface ProgrammableMaterial<
  Grammar extends string = string,
  Version extends number = number,
> {
  readonly grammar: Grammar
  readonly name: string
  readonly version: Version
}

export type MaterialRecord<Grammar extends string, Version extends number> = ProgrammableMaterial<
  Grammar,
  Version
> &
  Record<string, unknown>

/** Produces the canonical, human-readable representation used by material exporters. */
export function serializeMaterial(material: ProgrammableMaterial) {
  return JSON.stringify(material, null, 2)
}

/** Validates the common envelope before a grammar validates its own settings. */
export function parseMaterialEnvelope<const Grammar extends string, const Version extends number>(
  value: unknown,
  grammar: Grammar,
  version: Version,
): MaterialRecord<Grammar, Version> {
  if (!isMaterialRecord(value) || value.grammar !== grammar || value.version !== version) {
    throw new TypeError(`Expected a version ${version} @hifi/${grammar} material`)
  }

  if (typeof value.name !== 'string' || value.name.trim().length === 0) {
    throw new TypeError(`${titleCase(grammar)} material requires a name`)
  }

  return value as MaterialRecord<Grammar, Version>
}

export function isMaterialRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

export function isFiniteMaterialNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function titleCase(value: string) {
  return `${value.charAt(0).toUpperCase()}${value.slice(1)}`
}
