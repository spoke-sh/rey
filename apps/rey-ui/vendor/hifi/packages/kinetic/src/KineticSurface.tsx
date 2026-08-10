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
  const shadowY = Math.max(3, material.travel * 0.68)
  const shadowX = Math.max(1, shadowY * 0.42)
  const softShadowY = shadowY + 6
  const softShadowX = Math.max(2, softShadowY * 0.42)
  const shadowBlur = Math.max(8, material.mass * 7)
  const press = material.travel * material.actuation
  const pressX = Math.max(0.5, press * 0.42)

  return {
    '--kinetic-actuation': material.actuation,
    '--kinetic-control-press': `${press}px`,
    '--kinetic-control-press-x': `${pressX}px`,
    '--kinetic-damping': material.damping,
    '--kinetic-detents': material.detents,
    '--kinetic-edge-shadow': `color-mix(in srgb, ${material.foregroundColor} 20%, transparent)`,
    '--kinetic-friction': material.friction,
    '--kinetic-hard-shadow': `color-mix(in srgb, ${material.foregroundColor} 34%, transparent)`,
    '--kinetic-light-highlight': 'color-mix(in srgb, white 56%, transparent)',
    '--kinetic-mass': material.mass,
    '--kinetic-restitution': material.restitution,
    '--kinetic-shadow-soft-x': `${softShadowX}px`,
    '--kinetic-shadow-soft-y': `${softShadowY}px`,
    '--kinetic-shadow-x': `${shadowX}px`,
    '--kinetic-shadow-y': `${shadowY}px`,
    '--kinetic-soft-shadow': `color-mix(in srgb, ${material.foregroundColor} 16%, transparent)`,
    '--kinetic-stiffness': material.stiffness,
    '--kinetic-travel': `${material.travel}px`,
    background: `linear-gradient(145deg, color-mix(in srgb, ${material.backgroundColor} 88%, white), ${material.backgroundColor})`,
    boxShadow: `inset 1px 1px 0 color-mix(in srgb, white 48%, transparent), inset -1px -1px 0 color-mix(in srgb, ${material.foregroundColor} 16%, transparent), ${shadowX}px ${shadowY}px 0 color-mix(in srgb, ${material.foregroundColor} 24%, transparent), ${softShadowX}px ${softShadowY}px ${shadowBlur}px color-mix(in srgb, ${material.foregroundColor} 16%, transparent)`,
    color: material.foregroundColor,
  } as CSSProperties
}
