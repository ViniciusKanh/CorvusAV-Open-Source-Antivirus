import { RotateCcw, ShieldAlert } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { Badge } from '../../components/shared/Badge'
import { formatDateTime, truncatePath } from '../../lib/format'
import { quarantineStateLabel, quarantineStateTone } from '../../lib/labels'
import { listQuarantineItems, restoreQuarantineItem } from '../../services/corvusApi'
import type { QuarantineItem } from '../../types/corvus'

/** Estados em que um item pode ser restaurado — `restore_failed` é
 * incluído de propósito, para que uma tentativa anterior sem sucesso (por
 * exemplo, porque o destino original já existia) possa ser repetida depois
 * que a causa for resolvida. */
const RESTORABLE_STATES = new Set(['quarantined', 'restore_failed'])

export function QuarantinePage() {
  const [items, setItems] = useState<QuarantineItem[]>([])
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [restoringId, setRestoringId] = useState<string | null>(null)
  const [restoreErrors, setRestoreErrors] = useState<Record<string, string>>({})

  const reload = useCallback(async () => {
    setLoading(true)
    setLoadError(null)
    try {
      const fetched = await listQuarantineItems()
      setItems(fetched)
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : String(error))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void reload()
  }, [reload])

  async function handleRestore(item: QuarantineItem) {
    setRestoringId(item.id)
    setRestoreErrors((prev) => {
      const next = { ...prev }
      delete next[item.id]
      return next
    })
    try {
      await restoreQuarantineItem(item.id)
      await reload()
    } catch (error) {
      setRestoreErrors((prev) => ({
        ...prev,
        [item.id]: error instanceof Error ? error.message : String(error),
      }))
    } finally {
      setRestoringId(null)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-2xl font-bold text-brand-text">Quarentena</h1>
        <p className="mt-1 text-sm text-brand-text-secondary">
          Arquivos isolados manualmente a partir de um resultado de verificação. O CorvusAV nunca
          exclui um arquivo em quarentena automaticamente, e nunca executa ou abre um arquivo
          isolado ou restaurado — a restauração só devolve o arquivo ao caminho original, sob sua
          confirmação.
        </p>
      </div>

      {loadError && (
        <div className="rounded-xl border border-brand-danger/30 bg-brand-danger/10 px-4 py-3 text-sm text-brand-danger">
          {loadError}
        </div>
      )}

      <div className="rounded-2xl border border-white/5 bg-brand-surface/20 p-5">
        {loading && items.length === 0 ? (
          <p className="py-6 text-center text-sm text-brand-text-secondary">Carregando quarentena…</p>
        ) : items.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-2 py-10 text-center">
            <ShieldAlert className="h-8 w-8 text-brand-text-secondary" />
            <p className="text-sm text-brand-text-secondary">
              Nenhum arquivo em quarentena. Isole um arquivo a partir dos detalhes de uma
              verificação no Histórico.
            </p>
          </div>
        ) : (
          <table className="w-full border-separate border-spacing-y-1 text-sm">
            <thead>
              <tr className="text-left text-xs uppercase tracking-wider text-brand-text-secondary">
                <th className="px-3 py-2 font-medium">Arquivo original</th>
                <th className="px-3 py-2 font-medium">Estado</th>
                <th className="px-3 py-2 font-medium">Isolado em</th>
                <th className="px-3 py-2 font-medium">Motivo</th>
                <th className="px-3 py-2 font-medium" aria-label="Ações" />
              </tr>
            </thead>
            <tbody>
              {items.map((item) => (
                <tr key={item.id} className="bg-brand-surface/40 align-top">
                  <td className="rounded-l-lg px-3 py-3 font-medium text-brand-text" title={item.original_path}>
                    {truncatePath(item.original_path)}
                  </td>
                  <td className="px-3 py-3">
                    <Badge tone={quarantineStateTone(item.state)}>{quarantineStateLabel(item.state)}</Badge>
                    {item.state === 'restore_failed' && item.last_error && (
                      <p className="mt-1 max-w-[16rem] text-xs text-brand-danger">{item.last_error}</p>
                    )}
                  </td>
                  <td className="px-3 py-3 text-brand-text-secondary">{formatDateTime(item.quarantined_at)}</td>
                  <td className="max-w-xs px-3 py-3 text-brand-text-secondary" title={item.reason}>
                    <span className="line-clamp-2">{item.reason}</span>
                  </td>
                  <td className="rounded-r-lg px-3 py-3 text-right">
                    {RESTORABLE_STATES.has(item.state) && (
                      <button
                        type="button"
                        onClick={() => void handleRestore(item)}
                        disabled={restoringId === item.id}
                        className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-brand-cyan hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        <RotateCcw className="h-3.5 w-3.5" />
                        {restoringId === item.id ? 'Restaurando…' : 'Restaurar'}
                      </button>
                    )}
                    {restoreErrors[item.id] && (
                      <p className="mt-1 max-w-[14rem] text-xs text-brand-danger">{restoreErrors[item.id]}</p>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
