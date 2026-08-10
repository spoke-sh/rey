import { type ButtonHTMLAttributes, useState } from 'react'
import type { KineticThemeName } from './grammar.js'
import { type KineticMaterial, kineticThemeMaterials } from './material.js'

export interface KineticButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly material?: KineticMaterial
  readonly theme?: KineticThemeName
}

export function KineticButton({
  children,
  disabled,
  material,
  onBlur,
  onKeyDown,
  onKeyUp,
  onPointerCancel,
  onPointerDown,
  onPointerLeave,
  onPointerUp,
  style,
  theme = 'precision',
  ...props
}: KineticButtonProps) {
  const selected = material ?? kineticThemeMaterials[theme]
  const [pressed, setPressed] = useState(false)
  const duration = Math.round(
    Math.min(420, Math.max(70, 56000 / selected.stiffness + selected.damping * 2.2)),
  )
  const travel = selected.travel * selected.actuation
  const restingShadow = Math.max(2, selected.travel * 0.7)

  return (
    <button
      {...props}
      data-kinetic-response={selected.response}
      data-pressed={pressed}
      disabled={disabled}
      onBlur={(event) => {
        setPressed(false)
        onBlur?.(event)
      }}
      onKeyDown={(event) => {
        if (!disabled && (event.key === ' ' || event.key === 'Enter')) setPressed(true)
        onKeyDown?.(event)
      }}
      onKeyUp={(event) => {
        setPressed(false)
        onKeyUp?.(event)
      }}
      onPointerCancel={(event) => {
        setPressed(false)
        onPointerCancel?.(event)
      }}
      onPointerDown={(event) => {
        if (!disabled) setPressed(true)
        onPointerDown?.(event)
      }}
      onPointerLeave={(event) => {
        setPressed(false)
        onPointerLeave?.(event)
      }}
      onPointerUp={(event) => {
        setPressed(false)
        onPointerUp?.(event)
      }}
      style={{
        background: selected.backgroundColor,
        border: `1px solid color-mix(in srgb, ${selected.foregroundColor} 38%, transparent)`,
        borderRadius: selected.radius,
        boxShadow: pressed
          ? `0 1px 1px color-mix(in srgb, ${selected.foregroundColor} 18%, transparent), inset 0 2px ${Math.max(3, selected.mass * 3)}px color-mix(in srgb, ${selected.foregroundColor} 18%, transparent)`
          : `0 ${restingShadow}px 0 color-mix(in srgb, ${selected.foregroundColor} 34%, transparent), 0 ${restingShadow + 5}px ${Math.max(8, selected.mass * 7)}px color-mix(in srgb, ${selected.foregroundColor} 18%, transparent)`,
        color: selected.foregroundColor,
        cursor: disabled ? 'not-allowed' : 'pointer',
        transform: `translateY(${pressed ? travel : 0}px)`,
        transition: `transform ${duration}ms cubic-bezier(0.2, ${Math.min(1.8, 0.8 + selected.restitution)}, 0.25, 1), box-shadow ${Math.max(70, duration * 0.6)}ms ease`,
        ...style,
      }}
      type={props.type ?? 'button'}
    >
      {children}
    </button>
  )
}
