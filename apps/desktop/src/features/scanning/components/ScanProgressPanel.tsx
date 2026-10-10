import { XCircle } from 'lucide-react'
import { useEffect, useState } from 'react'
import scanInProgress from '../../../assets/animations/scan-in-progress.gif'
import logoMark from '../../../assets/corvusav-mark.png'
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
    if (Number.isNaN(start)) {
      setElapsed(0)
      return
    }
    const tick = () => setElapsed(Date.now() - start)
    tick()
    const id = window.setInterval(tick, 1000)
    return () => window.clearInterval(id)
  }, [startedAt])

  return elapsed
}

/** Arquivos por segundo, calculado só a partir de dados reais (arquivos já
 * examinados e tempo decorrido). Nunca extrapola um total ou um tempo
 * restante que o backend não conhece. */
function formatScanSpeed(filesScanned: number, elapsedMs: number): string | null {
  if (elapsedMs < 1000 || filesScanned === 0) return null
  const perSecond = filesScanned / (elapsedMs / 1000)
  if (perSecond >= 10) return `${Math.round(perSecond)} arq/s`
  return `${perSecond.toFixed(1)} arq/s`
}

const CANCELLABLE_STATUSES = new Set(['preparing', 'running'])

export function ScanProgressPanel({ job }: ScanProgressPanelProps) {
  const elapsed = useElapsedMs(job?.started_at ?? null)

  if (!job) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 rounded-2xl border border-white/5 bg-brand-surface/30 p-10 text-center">
        <img src={logoMark} alt="" className="h-14 w-14" />
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
  const speed = formatScanSpeed(job.files_scanned, elapsed)

  return (
    <div className="flex flex-col gap-4 rounded-2xl border border-white/5 bg-brand-surface/30 p-6">
      <div className="flex items-center gap-4">
        {/* A animação só aparece enquanto `job` existe, e `job` só existe
         * enquanto o status é não-terminal (ver useScanEventBridge) — ou
         * seja, ela reflete uma verificação real em andamento, nunca um
         * progresso ou resultado simulado, e some sozinha assim que a
         * verificação termina, falha ou é cancelada. */}
        <img
          src={scanInProgress}
          alt=""
          className="h-14 w-14 shrink-0 rounded-xl"
          aria-hidden="true"
        />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <p className="font-display text-base font-semibold text-brand-text">
              {scanStatusLabel(job.status)}
            </p>
            {canCancel && (
              <button
                type="button"
                onClick={() => void cancelScan(job.id)}
                className="inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-brand-danger hover:bg-white/5"
              >
                <XCircle className="h-3.5 w-3.5" />
                Cancelar
              </button>
            )}
          </div>
          {job.current_file && (
            <p className="mt-1 truncate text-sm text-brand-text-secondary" title={job.current_file}>
              {truncatePath(job.current_file)}
            </p>
          )}
          {job.error_message && (
            <p className="mt-1 text-sm text-brand-danger">{job.error_message}</p>
          )}
        </div>
      </div>

      {percent !== null && (
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
          <div className="h-full bg-brand-cyan transition-all" style={{ width: `${percent}%` }} />
        </div>
      )}

      <div className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
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
        <div>
          <p className="text-brand-text-secondary">Velocidade</p>
          <p className="font-semibold text-brand-text">{speed ?? '—'}</p>
        </div>
      </div>
    </div>
  )
}
