import type { PropsWithChildren } from 'react'
import { cn } from '../../lib/cn'
import type { Tone } from '../../lib/labels'

const TONE_CLASSES: Record<Tone, string> = {
  success: 'bg-brand-success/15 text-brand-success',
  warning: 'bg-brand-warning/15 text-brand-warning',
  danger: 'bg-brand-danger/15 text-brand-danger',
  neutral: 'bg-white/10 text-brand-text-secondary',
}

interface BadgeProps extends PropsWithChildren {
  tone: Tone
  className?: string
}

export function Badge({ tone, className, children }: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full px-2.5 py-1 text-xs font-medium',
        TONE_CLASSES[tone],
        className,
      )}
    >
      {children}
    </span>
  )
}
