import {
  isFiniteMaterialNumber,
  type ProgrammableMaterial,
  parseMaterialEnvelope,
  serializeMaterial,
} from '@hifi/core'
import type { KineticThemeName } from './grammar.js'

export type KineticResponse = 'precision' | 'spring' | 'magnetic' | 'viscous'

export interface KineticMaterial extends ProgrammableMaterial<'kinetic', 1> {
  readonly accentColor: string
  readonly actuation: number
  readonly backgroundColor: string
  readonly damping: number
  readonly detents: number
  readonly feedbackGain: number
  readonly foregroundColor: string
  readonly friction: number
  readonly mass: number
  readonly radius: number
  readonly response: KineticResponse
  readonly restitution: number
  readonly stiffness: number
  readonly travel: number
}

export const kineticThemeMaterials: Readonly<Record<KineticThemeName, KineticMaterial>> = {
  precision: {
    accentColor: '#ff553e',
    actuation: 0.58,
    backgroundColor: '#d8d9d4',
    damping: 42,
    detents: 24,
    feedbackGain: 0.12,
    foregroundColor: '#181b1b',
    friction: 0.34,
    grammar: 'kinetic',
    mass: 0.72,
    name: 'Precision instrument',
    radius: 5,
    response: 'precision',
    restitution: 0.08,
    stiffness: 880,
    travel: 3,
    version: 1,
  },
  sprung: {
    accentColor: '#6558f5',
    actuation: 0.48,
    backgroundColor: '#eee7d9',
    damping: 18,
    detents: 8,
    feedbackGain: 0.16,
    foregroundColor: '#252126',
    friction: 0.16,
    grammar: 'kinetic',
    mass: 0.9,
    name: 'Sprung control',
    radius: 16,
    response: 'spring',
    restitution: 0.74,
    stiffness: 360,
    travel: 11,
    version: 1,
  },
  magnetic: {
    accentColor: '#12a6a6',
    actuation: 0.68,
    backgroundColor: '#e7eceb',
    damping: 28,
    detents: 6,
    feedbackGain: 0.14,
    foregroundColor: '#172827',
    friction: 0.08,
    grammar: 'kinetic',
    mass: 0.55,
    name: 'Magnetic latch',
    radius: 10,
    response: 'magnetic',
    restitution: 0.24,
    stiffness: 620,
    travel: 7,
    version: 1,
  },
  viscous: {
    accentColor: '#cf8d22',
    actuation: 0.76,
    backgroundColor: '#302f2c',
    damping: 72,
    detents: 4,
    feedbackGain: 0.08,
    foregroundColor: '#f2ead9',
    friction: 0.82,
    grammar: 'kinetic',
    mass: 3.2,
    name: 'Viscous mechanism',
    radius: 8,
    response: 'viscous',
    restitution: 0.02,
    stiffness: 180,
    travel: 14,
    version: 1,
  },
}

export function serializeKineticMaterial(material: KineticMaterial) {
  return serializeMaterial(material)
}

export function parseKineticMaterial(value: unknown): KineticMaterial {
  const material = parseMaterialEnvelope(value, 'kinetic', 1)

  if (!isKineticResponse(material.response)) {
    throw new TypeError('Kinetic material has an unsupported response')
  }

  for (const key of ['accentColor', 'backgroundColor', 'foregroundColor'] as const) {
    if (typeof material[key] !== 'string' || material[key].length === 0) {
      throw new TypeError(`Kinetic material requires a ${key}`)
    }
  }

  for (const key of [
    'actuation',
    'damping',
    'detents',
    'feedbackGain',
    'friction',
    'mass',
    'radius',
    'restitution',
    'stiffness',
    'travel',
  ] as const) {
    if (!isFiniteMaterialNumber(material[key])) {
      throw new TypeError(`Kinetic material requires a finite ${key}`)
    }
  }

  return material as unknown as KineticMaterial
}

function isKineticResponse(value: unknown): value is KineticResponse {
  return value === 'precision' || value === 'spring' || value === 'magnetic' || value === 'viscous'
}
