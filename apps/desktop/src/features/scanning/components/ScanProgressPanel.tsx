import { ShieldCheck, XCircle } from 'lucide-react'
import { useEffect, useState } from 'react'
import { formatDurationMs, truncatePath } from '../../../lib/format'
import { scanStatusLabel } from '../../../lib/labels'
import { cancelScan } from '../../../services/corvusApi'
import type { ScanJob } from '../../../types/corvus'

interface ScanProgressPanelProps {
  job: ScanJob | null
}

function useElapsedMs(startedAt: string | null): number {
  const [elapsed, setElapsed] = useState(0)

  useEffect(() => {
    if (!startedAt) {
      setElapsed(0)
      return
    }
    const start = new Date(startedAt).getTime()
    const tick = () => setElapsed(Date.now() - start)
    tick()
    const id = window.setInterval(tick, 1000)
    return () => window.clearInterval(id)
  }, [startedAt])

  return elapsed
}

const CANCELLABLE_STATUSES = new Set(['preparing', 'running'])

export function ScanProgressPanel({ job }: ScanProgressPanelProps) {
  const elapsed = useElapsedMs(job?.started_at ?? null)

  if (!job) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 rounded-2xl border border-white/5 bg-brand-surface/30 p-10 text-center">
        <ShieldCheck className="h-10 w-10 text-brand-text-secondary" />
        <p className="font-display text-base font-semibold text-brand-text">
          Nenhuma verificação em execução
        </p>
        <p className="max-w-sm text-sm text-brand-text-secondary">
          Selecione um tipo de verificação acima para começar a analisar seu sistema em busca de
          ameaças.
        </p>
      </div>
    )
  }

  const canCancel = CANCELLABLE_STATUSES.has(job.status)
  const percent =
    job.files_total && job.files_total > 0
      ? Math.min(100, Math.round((job.files_scanned / job.files_total) * 100))
      : null

  return (
    <div className="flex flex-col gap-4 rounded-2xl border border-white/5 bg-brand-surface/30 p-6">
      <div>
        <div className="flex items-center justify-between">
          <p className="font-display text-base font-semibold text-brand-text">
            {scanStatusLabel(job.status)}
          </p>
          {canCancel && (
            <button
              type="button"
              onClick={() => void cancelScan(job.id)}
              className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-brand-danger hover:bg-white/5"
            >
              <XCircle className="h-3.5 w-3.5" />
              Cancelar
            </button>
          )}
        </div>
        {job.current_file && (
          <p className="mt-2 truncate text-sm text-brand-text-secondary" title={job.current_file}>
            {truncatePath(job.current_file)}
          </p>
        )}
        {job.error_message && (
          <p className="mt-2 text-sm text-brand-danger">{job.error_message}</p>
        )}
      </div>

      {percent !== null && (
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
          <div className="h-full bg-brand-cyan transition-all" style={{ width: `${percent}%` }} />
        </div>
      )}

      <div className="grid grid-cols-3 gap-4 text-sm">
        <div>
          <p className="text-brand-text-secondary">Arquivos examinados</p>
          <p className="font-semibold text-brand-text">{job.files_scanned}</p>
        </div>
        <div>
          <p className="text-brand-text-secondary">Ameaças</p>
          <p
            className={`font-semibold ${job.threats_found > 0 ? 'text-brand-danger' : 'text-brand-text'}`}
          >
            {job.threats_found}
          </p>
        </div>
        <div>
          <p className="text-brand-text-secondary">Tempo decorrido</p>
          <p className="font-semibold text-brand-text">{formatDurationMs(elapsed)}</p>
        </div>
      </div>
    </div>
  )
}
