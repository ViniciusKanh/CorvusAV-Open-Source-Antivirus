import { RefreshCw } from 'lucide-react'
import { useState } from 'react'
import { Badge } from '../../components/shared/Badge'
import { formatDateTime } from '../../lib/format'
import { engineAvailabilityLabel, engineAvailabilityTone } from '../../lib/labels'
import { fetchEngineHealth, updateSignatures } from '../../services/corvusApi'
import { useEngineStore } from '../../stores/engineStore'
import type { FreshclamOutcome } from '../../types/corvus'

export function UpdatesPage() {
  const health = useEngineStore((s) => s.health)
  const loading = useEngineStore((s) => s.loading)
  const setHealth = useEngineStore((s) => s.setHealth)

  const [updating, setUpdating] = useState(false)
  const [outcome, setOutcome] = useState<FreshclamOutcome | null>(null)
  const [error, setError] = useState<string | null>(null)

  async function handleUpdate() {
    setError(null)
    setOutcome(null)
    setUpdating(true)
    try {
      const result = await updateSignatures()
      setOutcome(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      // Independente do resultado do freshclam, a saúde do mecanismo pode
      // ter mudado (nova versão de assinaturas, por exemplo) — sondamos de
      // novo para refletir o estado real, nunca o que esperávamos que
      // tivesse acontecido.
      try {
        const refreshed = await fetchEngineHealth()
        setHealth(refreshed)
      } catch {
        // Mantém o estado anterior do motor visível se a nova sondagem falhar.
      }
      setUpdating(false)
    }
  }

  const tone = health ? engineAvailabilityTone(health.availability) : 'neutral'

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-2xl font-bold text-brand-text">Atualizações</h1>
        <p className="mt-1 text-sm text-brand-text-secondary">
          Status real da base de assinaturas do ClamAV, com atualização manual via freshclam.
        </p>
      </div>

      <div className="rounded-2xl border border-white/5 bg-brand-surface/20 p-6">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2">
              <p className="text-sm font-medium text-brand-text-secondary">Mecanismo</p>
              {health && !loading && (
                <Badge tone={tone}>{engineAvailabilityLabel(health.availability)}</Badge>
              )}
            </div>
            <p className="mt-1 font-display text-lg font-semibold text-brand-text">
              {health?.engine_version ?? '—'}
            </p>
          </div>
          <button
            type="button"
            onClick={() => void handleUpdate()}
            disabled={updating}
            className="inline-flex shrink-0 items-center gap-2 rounded-md bg-brand-blue-deep/40 px-4 py-2 text-sm font-medium text-brand-cyan hover:bg-brand-blue-deep/60 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <RefreshCw className={`h-4 w-4 ${updating ? 'animate-spin' : ''}`} />
            {updating ? 'Atualizando…' : 'Atualizar agora'}
          </button>
        </div>

        <div className="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div className="rounded-xl border border-white/5 bg-brand-bg/40 px-4 py-3">
            <p className="text-xs text-brand-text-secondary">Versão das assinaturas</p>
            <p className="mt-1 text-sm font-semibold text-brand-text">
              {health?.signature_version ?? '—'}
            </p>
          </div>
          <div className="rounded-xl border border-white/5 bg-brand-bg/40 px-4 py-3">
            <p className="text-xs text-brand-text-secondary">Última atualização</p>
            <p className="mt-1 text-sm font-semibold text-brand-text">
              {formatDateTime(health?.signatures_updated_at ?? null)}
            </p>
          </div>
        </div>

        {error && (
          <p className="mt-4 text-sm text-brand-danger">Falha ao disparar a atualização: {error}</p>
        )}

        {outcome && (
          <div
            className={`mt-4 rounded-xl border px-4 py-3 text-sm ${
              outcome.success
                ? 'border-brand-success/20 bg-brand-success/10 text-brand-success'
                : 'border-brand-danger/20 bg-brand-danger/10 text-brand-danger'
            }`}
          >
            <p className="font-semibold">
              {outcome.success
                ? 'Assinaturas atualizadas com sucesso.'
                : `O freshclam terminou com falha${
                    outcome.exit_code !== null ? ` (código ${outcome.exit_code})` : ''
                  }.`}
            </p>
            {outcome.output && (
              <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words font-sans text-xs text-brand-text-secondary">
                {outcome.output}
              </pre>
            )}
            {!outcome.success && (
              <p className="mt-2 text-xs text-brand-text-secondary">
                Em instalações do ClamAV dentro de "Program Files", isso costuma acontecer porque o
                CorvusAV roda sem privilégios de administrador. Tente rodar o freshclam manualmente
                em um terminal elevado, como já funcionou antes.
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
