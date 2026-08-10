export type GrammarStatus = 'active' | 'experimental' | 'planned'

export interface GrammarTheme<Name extends string = string> {
  readonly name: Name
  readonly label: string
  readonly description: string
}

export interface GrammarDefinition<
  Name extends string = string,
  Theme extends GrammarTheme = GrammarTheme,
> {
  readonly name: Name
  readonly label: string
  readonly description: string
  readonly status: GrammarStatus
  readonly themes: readonly [Theme, ...Theme[]]
}

export function defineGrammar<const Definition extends GrammarDefinition>(
  definition: Definition,
): Definition {
  return definition
}

export function getGrammarTheme<const Definition extends GrammarDefinition>(
  grammar: Definition,
  name: string | undefined,
): Definition['themes'][number] {
  return (grammar.themes.find((theme) => theme.name === name) ??
    grammar.themes[0]) as Definition['themes'][number]
}

export {
  isFiniteMaterialNumber,
  isMaterialRecord,
  type MaterialRecord,
  type ProgrammableMaterial,
  parseMaterialEnvelope,
  serializeMaterial,
} from './material.js'
