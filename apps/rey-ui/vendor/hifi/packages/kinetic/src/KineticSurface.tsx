import type { CSSProperties, PropsWithChildren } from 'react'
import type { KineticThemeName } from './grammar.js'
import { type KineticMaterial, kineticThemeMaterials } from './material.js'

export interface KineticSurfaceProps extends PropsWithChildren {
  readonly className?: string
  readonly material?: KineticMaterial
  readonly theme?: KineticThemeName
}

export function KineticSurface({
  children,
  className,
  material,
  theme = 'precision',
}: KineticSurfaceProps) {
  const selected = material ?? kineticThemeMaterials[theme]

  return (
    <section
      className={className}
      data-kinetic-response={selected.response}
      style={{
        ...getKineticMaterialStyle(selected),
        border: `1px solid color-mix(in srgb, ${selected.foregroundColor} 28%, transparent)`,
        borderRadius: selected.radius,
        display: 'grid',
        minHeight: 'var(--kinetic-surface-min-height, 320px)',
        padding: 'var(--kinetic-surface-padding, 48px)',
        placeItems: 'center',
        position: 'relative',
        textAlign: 'center',
      }}
    >
      {children}
    </section>
  )
}

export function getKineticMaterialStyle(material: KineticMaterial): CSSProperties {
  const lift = Math.max(2, material.travel * (0.45 + material.mass * 0.14))
  const shadow = Math.max(5, material.travel * 1.8 + material.mass * 3)

  return {
    '--kinetic-actuation': material.actuation,
    '--kinetic-damping': material.damping,
    '--kinetic-detents': material.detents,
    '--kinetic-friction': material.friction,
    '--kinetic-mass': material.mass,
    '--kinetic-restitution': material.restitution,
    '--kinetic-stiffness': material.stiffness,
    '--kinetic-travel': `${material.travel}px`,
    background: `linear-gradient(145deg, color-mix(in srgb, ${material.backgroundColor} 88%, white), ${material.backgroundColor})`,
    boxShadow: `0 ${lift}px ${shadow}px color-mix(in srgb, ${material.foregroundColor} 22%, transparent), inset 0 1px 0 color-mix(in srgb, white 48%, transparent)`,
    color: material.foregroundColor,
  } as CSSProperties
}
