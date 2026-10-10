interface InDevelopmentProps {
  title: string
  description: string
}

/** Estado visual padrão para seções ainda sem implementação funcional —
 * nunca apresentamos uma função inexistente como disponível. */
export function InDevelopment({ title, description }: InDevelopmentProps) {
  return (
    <div className="flex h-full flex-col items-center justify-center rounded-2xl border border-dashed border-white/10 bg-brand-surface/40 p-12 text-center">
      <span className="mb-3 rounded-full bg-brand-blue-deep/40 px-3 py-1 text-xs font-semibold uppercase tracking-wider text-brand-cyan">
        Em desenvolvimento
      </span>
      <h2 className="font-display text-xl font-semibold text-brand-text">{title}</h2>
      <p className="mt-2 max-w-md text-sm text-brand-text-secondary">{description}</p>
    </div>
  )
}
