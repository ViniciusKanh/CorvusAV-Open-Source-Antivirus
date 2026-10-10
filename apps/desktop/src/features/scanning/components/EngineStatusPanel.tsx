import { ShieldAlert, ShieldQuestion } from 'lucide-react'
import logoMark from '../../../assets/corvusav-mark.png'
import { Badge } from '../../../components/shared/Badge'
import { formatDateTime } from '../../../lib/format'
import { engineAvailabilityLabel, engineAvailabilityTone } from '../../../lib/labels'
import type { Tone } from '../../../lib/labels'
import type { EngineHealth } from '../../../types/corvus'

interface EngineStatusPanelProps {
  health: EngineHealth | null
  loading: boolean
  error: string | null
}

const TONE_STYLES = {
  success: {
    border: 'border-brand-success/20',
    glow: 'from-brand-success/10',
    headline: 'text-brand-success',
  },
  warning: {
    border: 'border-brand-warning/20',
    glow: 'from-brand-warning/10',
    headline: 'text-brand-warning',
  },
  danger: {
    border: 'border-brand-danger/20',
    glow: 'from-brand-danger/10',
    headline: 'text-brand-danger',
  },
  neutral: {
    border: 'border-white/5',
    glow: 'from-brand-blue-deep/20',
    headline: 'text-brand-text',
  },
} as const

export function EngineStatusPanel({ health, loading, error }: EngineStatusPanelProps) {
  const tone: Tone = loading ? 'neutral' : health ? engineAvailabilityTone(health.availability) : 'danger'
  const { border, glow, headline } = TONE_STYLES[tone]
  const available = health?.availability === 'available'
  // O logo só cede lugar a um ícone de alerta quando há algo que realmente
  // exige atenção (aviso ou falha); em estados saudáveis ou neutros a marca
  // da CorvusAV permanece visível.
  const AlertIcon = tone === 'danger' ? ShieldAlert : tone === 'warning' ? ShieldQuestion : null

  const headlineText = loading
    ? 'Sondando mecanismo…'
    : health
      ? available
        ? 'Pronto para verificar'
        : engineAvailabilityLabel(health.availability)
      : 'Mecanismo indisponível'

  const detailText = available
    ? 'O mecanismo ClamAV está disponível e pronto para iniciar uma verificação sob demanda.'
    : (error ?? health?.last_error ?? 'Instale o ClamAV para habilitar as verificações.')

  return (
    <div
      className={`flex flex-col gap-5 rounded-2xl border ${border} bg-gradient-to-br ${glow} to-brand-surface/40 p-6 lg:flex-row lg:items-center lg:justify-between`}
    >
      <div className="flex items-center gap-4">
        {AlertIcon ? (
          <div
            className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-xl ${
              tone === 'danger' ? 'bg-brand-danger/15 text-brand-danger' : 'bg-brand-warning/15 text-brand-warning'
            }`}
          >
            <AlertIcon className="h-7 w-7" />
          </div>
        ) : (
          <img src={logoMark} alt="" className="h-14 w-14 shrink-0" />
        )}
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <p className="text-sm font-medium text-brand-text-secondary">Proteção local</p>
            {health && !loading && <Badge tone={tone}>{engineAvailabilityLabel(health.availability)}</Badge>}
          </div>
          <p className={`font-display text-2xl font-bold ${headline}`}>{headlineText}</p>
          <p className="mt-1 max-w-md text-sm text-brand-text-secondary">{detailText}</p>
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
