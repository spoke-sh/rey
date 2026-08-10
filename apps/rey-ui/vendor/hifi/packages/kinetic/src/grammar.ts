import { defineGrammar } from '@hifi/core'

export const kineticGrammar = defineGrammar({
  name: 'kinetic',
  label: 'Kinetic',
  description: 'Controls governed by force, travel, resistance, momentum, and consequence.',
  status: 'experimental',
  themes: [
    {
      name: 'precision',
      label: 'Precision',
      description: 'Short travel, high damping, and fine mechanical detents.',
    },
    {
      name: 'sprung',
      label: 'Sprung',
      description: 'Elastic controls that reveal stored and released energy.',
    },
    {
      name: 'magnetic',
      label: 'Magnetic',
      description: 'Attraction zones with decisive thresholds and snapping.',
    },
    {
      name: 'viscous',
      label: 'Viscous',
      description: 'Heavy movement, high resistance, and slow settling.',
    },
  ],
})

export type KineticThemeName = (typeof kineticGrammar.themes)[number]['name']
