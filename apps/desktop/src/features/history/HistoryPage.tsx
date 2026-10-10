import { ScanHistoryTable } from '../../components/shared/ScanHistoryTable'
import { useScanHistory } from '../../hooks/useScanHistory'

export function HistoryPage() {
  const { history, loading } = useScanHistory(100)

  return (
    <div className="space-y-4">
      <div>
        <h1 className="font-display text-2xl font-bold text-brand-text">Histórico</h1>
        <p className="mt-1 text-sm text-brand-text-secondary">
          Registro real de verificações persistido localmente em SQLite.
        </p>
      </div>
      <div className="rounded-2xl border border-white/5 bg-brand-surface/20 p-5">
        <ScanHistoryTable jobs={history} loading={loading} />
      </div>
    </div>
  )
}
