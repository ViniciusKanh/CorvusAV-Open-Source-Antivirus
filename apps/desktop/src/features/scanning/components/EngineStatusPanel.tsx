import logoMark from '../../../assets/corvusav-mark.svg'
import { formatDateTime } from '../../../lib/format'
import { engineAvailabilityLabel } from '../../../lib/labels'
import type { EngineHealth } from '../../../types/corvus'

interface EngineStatusPanelProps {
  health: EngineHealth | null
  loading: boolean
  error: string | null
}

export function EngineStatusPanel({ health, loading, error }: EngineStatusPanelProps) {
  const available = health?.availability === 'available'
  const headline = loading
    ? 'Sondando mecanismo…'
    : health
      ? available
        ? 'Pronto para verificar'
        : engineAvailabilityLabel(health.availability)
      : 'Mecanismo indisponível'

  return (
    <div className="flex flex-col gap-5 rounded-2xl border border-white/5 bg-gradient-to-br from-brand-blue-deep/30 to-brand-surface/40 p-6 lg:flex-row lg:items-center lg:justify-between">
      <div className="flex items-center gap-4">
        <img src={logoMark} alt="" className="h-14 w-14 shrink-0" />
        <div>
          <p className="text-sm font-medium text-brand-text-secondary">Proteção local</p>
          <p
            className={`font-display text-2xl font-bold ${available ? 'text-brand-cyan' : 'text-brand-warning'}`}
          >
            {headline}
          </p>
          <p className="mt-1 max-w-md text-sm text-brand-text-secondary">
            {available
              ? 'O mecanismo ClamAV está disponível e pronto para iniciar uma verificação sob demanda.'
              : (error ?? health?.last_error ?? 'Instale o ClamAV para habilitar as verificações.')}
          </p>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3 lg:gap-4">
        <div className="rounded-xl border border-white/5 bg-brand-bg/40 px-4 py-3">
          <p className="text-xs text-brand-text-secondary">Mecanismo</p>
          <p className="mt-1 truncate text-sm font-semibold text-brand-text">
            {health?.engine_version ?? '—'}
          </p>
          <p className="text-xs text-brand-text-secondary">
            Assinaturas {health?.signature_version ?? '—'}
          </p>
        </div>
        <div className="rounded-xl border border-white/5 bg-brand-bg/40 px-4 py-3">
          <p className="text-xs text-brand-text-secondary">Última atualização</p>
          <p className="mt-1 text-sm font-semibold text-brand-text">
            {formatDateTime(health?.signatures_updated_at ?? null)}
          </p>
          <p className="text-xs text-brand-text-secondary">
            {available ? 'Disponível para verificar' : 'Indisponível'}
          </p>
        </div>
      </div>
    </div>
  )
}
