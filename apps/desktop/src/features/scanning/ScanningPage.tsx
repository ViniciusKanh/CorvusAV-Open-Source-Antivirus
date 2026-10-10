import { useState } from 'react'
import { ScanHistoryTable } from '../../components/shared/ScanHistoryTable'
import { useScanHistory } from '../../hooks/useScanHistory'
import { clearScanHistory, pickCustomScanTargets, startCustomScan, startQuickScan } from '../../services/corvusApi'
import { useEngineStore } from '../../stores/engineStore'
import { useScanStore } from '../../stores/scanStore'
import { EngineStatusPanel } from './components/EngineStatusPanel'
import { ScanActionCards } from './components/ScanActionCards'
import { ScanProgressPanel } from './components/ScanProgressPanel'
import { SecondaryIndicators } from './components/SecondaryIndicators'

export function ScanningPage() {
  const health = useEngineStore((s) => s.health)
  const loadingHealth = useEngineStore((s) => s.loading)
  const healthError = useEngineStore((s) => s.error)
  const activeJob = useScanStore((s) => s.activeJob)
  const { history, loading: historyLoading, reload: reloadHistory } = useScanHistory()

  const [busyKind, setBusyKind] = useState<'quick' | 'custom' | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
  const [clearingHistory, setClearingHistory] = useState(false)
  const [clearError, setClearError] = useState<string | null>(null)
  const [clearMessage, setClearMessage] = useState<string | null>(null)

  const hasClearableHistory = history.some(
    (job) => job.status === 'failed' || job.status === 'cancelled',
  )

  async function handleClearHistory() {
    setClearError(null)
    setClearMessage(null)
    setClearingHistory(true)
    try {
      const removed = await clearScanHistory()
      await reloadHistory()
      // Feedback explícito do resultado real (nunca presumido): se o
      // número de itens visivelmente removidos não bater com o esperado,
      // isso fica claro aqui em vez de parecer que o clique não fez nada.
      setClearMessage(
        removed > 0
          ? `${removed} ${removed === 1 ? 'verificação removida' : 'verificações removidas'}.`
          : 'Nenhuma verificação com falha ou cancelamento foi encontrada para remover.',
      )
    } catch (error) {
      setClearError(error instanceof Error ? error.message : String(error))
    } finally {
      setClearingHistory(false)
    }
  }

  const engineAvailable = health?.availability === 'available'

  async function handleStartQuick() {
    setActionError(null)
    setBusyKind('quick')
    try {
      await startQuickScan()
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusyKind(null)
    }
  }

  async function handleStartCustom() {
    setActionError(null)
    setBusyKind('custom')
    try {
      const targets = await pickCustomScanTargets()
      if (targets.length === 0) {
        return
      }
      await startCustomScan(targets)
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error))
    } finally {
      setBusyKind(null)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-2xl font-bold text-brand-text">Verificação</h1>
        <p className="mt-1 text-sm text-brand-text-secondary">
          Examine arquivos e pastas em busca de ameaças utilizando o mecanismo ClamAV.
        </p>
      </div>

      <EngineStatusPanel health={health} loading={loadingHealth} error={healthError} />

      {actionError && (
        <div className="rounded-xl border border-brand-danger/30 bg-brand-danger/10 px-4 py-3 text-sm text-brand-danger">
          {actionError}
        </div>
      )}

      <ScanActionCards
        engineAvailable={engineAvailable}
        busyKind={busyKind}
        onStartQuick={() => void handleStartQuick()}
        onStartCustom={() => void handleStartCustom()}
      />

      <ScanProgressPanel job={activeJob} />

      <SecondaryIndicators />

      <div className="rounded-2xl border border-white/5 bg-brand-surface/20 p-5">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h2 className="font-display text-base font-semibold text-brand-text">
            Verificações recentes
          </h2>
          {hasClearableHistory && (
            <button
              type="button"
              onClick={() => void handleClearHistory()}
              disabled={clearingHistory}
              className="shrink-0 rounded-md px-2.5 py-1.5 text-xs font-medium text-brand-text-secondary hover:bg-white/5 hover:text-brand-text disabled:cursor-not-allowed disabled:opacity-60"
            >
              {clearingHistory ? 'Limpando…' : 'Limpar falhas e cancelamentos'}
            </button>
          )}
        </div>
        {clearError && <p className="mb-3 text-xs text-brand-danger">{clearError}</p>}
        {clearMessage && <p className="mb-3 text-xs text-brand-text-secondary">{clearMessage}</p>}
        <ScanHistoryTable jobs={history} loading={historyLoading} />
      </div>
    </div>
  )
}
