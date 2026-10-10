import { formatDateTime } from '../../../lib/format'
import { engineAvailabilityLabel, scanKindLabel, scanStatusLabel } from '../../../lib/labels'
import { useEngineStore } from '../../../stores/engineStore'
import { useScanStore } from '../../../stores/scanStore'

function IndicatorCard({ label, value, hint }: { label: string; value: string; hint: string }) {
  return (
    <div className="rounded-xl border border-white/5 bg-brand-surface/30 p-4">
      <p className="text-xs text-brand-text-secondary">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-brand-text" title={value}>
        {value}
      </p>
      <p className="mt-0.5 truncate text-xs text-brand-text-secondary" title={hint}>
        {hint}
      </p>
    </div>
  )
}

export function SecondaryIndicators() {
  const history = useScanStore((s) => s.history)
  const recentResults = useScanStore((s) => s.recentResults)
  const health = useEngineStore((s) => s.health)

  const lastJob = history[0] ?? null

  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
      <IndicatorCard
        label="Última verificação"
        value={lastJob ? scanKindLabel(lastJob.kind) : 'Nenhuma ainda'}
        hint={lastJob ? formatDateTime(lastJob.finished_at ?? lastJob.created_at) : '—'}
      />
      <IndicatorCard
        label="Assinaturas"
        value={health?.signature_version ?? '—'}
        hint={health?.engine_version ?? 'ClamAV'}
      />
      <IndicatorCard
        label="Estado do mecanismo"
        value={health ? engineAvailabilityLabel(health.availability) : '—'}
        hint={lastJob ? scanStatusLabel(lastJob.status) : 'Sem verificações recentes'}
      />
      <IndicatorCard
        label="Itens recentemente examinados"
        value={String(recentResults.length)}
        hint="Nesta sessão"
      />
    </div>
  )
}
