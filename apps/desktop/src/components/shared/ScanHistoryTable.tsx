import { ChevronDown, ChevronRight, FileText } from 'lucide-react'
import { Fragment, useState } from 'react'
import { fetchJobResults } from '../../services/corvusApi'
import { formatDateTime, formatDurationBetween } from '../../lib/format'
import { scanKindLabel, scanStatusLabel, scanStatusTone, verdictLabel, verdictTone } from '../../lib/labels'
import type { ScanJob, ScanResult } from '../../types/corvus'
import { Badge } from './Badge'

interface ScanHistoryTableProps {
  jobs: ScanJob[]
  loading: boolean
}

export function ScanHistoryTable({ jobs, loading }: ScanHistoryTableProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [resultsByJob, setResultsByJob] = useState<Record<string, ScanResult[]>>({})
  const [loadingResults, setLoadingResults] = useState<string | null>(null)
  const [resultsError, setResultsError] = useState<string | null>(null)

  async function toggleDetails(jobId: string) {
    if (expandedId === jobId) {
      setExpandedId(null)
      return
    }
    setExpandedId(jobId)
    if (!resultsByJob[jobId]) {
      setLoadingResults(jobId)
      setResultsError(null)
      try {
        const results = await fetchJobResults(jobId)
        setResultsByJob((prev) => ({ ...prev, [jobId]: results }))
      } catch (error) {
        setResultsError(error instanceof Error ? error.message : String(error))
      } finally {
        setLoadingResults(null)
      }
    }
  }

  if (loading && jobs.length === 0) {
    return <p className="py-6 text-center text-sm text-brand-text-secondary">Carregando histórico…</p>
  }

  if (jobs.length === 0) {
    return (
      <p className="py-6 text-center text-sm text-brand-text-secondary">
        Nenhuma verificação foi executada ainda.
      </p>
    )
  }

  return (
    <table className="w-full border-separate border-spacing-y-1 text-sm">
      <thead>
        <tr className="text-left text-xs uppercase tracking-wider text-brand-text-secondary">
          <th className="px-3 py-2 font-medium">Tipo</th>
          <th className="px-3 py-2 font-medium">Início</th>
          <th className="px-3 py-2 font-medium">Estado</th>
          <th className="px-3 py-2 font-medium">Itens verificados</th>
          <th className="px-3 py-2 font-medium">Duração</th>
          <th className="px-3 py-2 font-medium" aria-label="Ações" />
        </tr>
      </thead>
      <tbody>
        {jobs.map((job) => {
          const isExpanded = expandedId === job.id
          return (
            <Fragment key={job.id}>
              <tr className="bg-brand-surface/40">
                <td className="rounded-l-lg px-3 py-3 font-medium text-brand-text">
                  {scanKindLabel(job.kind)}
                </td>
                <td className="px-3 py-3 text-brand-text-secondary">{formatDateTime(job.created_at)}</td>
                <td className="px-3 py-3">
                  <Badge tone={scanStatusTone(job.status)}>{scanStatusLabel(job.status)}</Badge>
                  {job.threats_found > 0 && (
                    <span className="ml-2 text-xs text-brand-danger">
                      {job.threats_found} {job.threats_found === 1 ? 'ameaça' : 'ameaças'}
                    </span>
                  )}
                </td>
                <td className="px-3 py-3 text-brand-text-secondary">{job.files_scanned}</td>
                <td className="px-3 py-3 text-brand-text-secondary">
                  {formatDurationBetween(job.started_at, job.finished_at)}
                </td>
                <td className="rounded-r-lg px-3 py-3 text-right">
                  <button
                    type="button"
                    onClick={() => void toggleDetails(job.id)}
                    className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-brand-cyan hover:bg-white/5"
                  >
                    {isExpanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
                    Detalhes
                  </button>
                </td>
              </tr>
              {isExpanded && (
                <tr key={`${job.id}-details`}>
                  <td colSpan={6} className="rounded-lg bg-brand-bg-secondary/60 px-4 py-3">
                    {loadingResults === job.id && (
                      <p className="text-xs text-brand-text-secondary">Carregando resultados…</p>
                    )}
                    {resultsError && loadingResults !== job.id && (
                      <p className="text-xs text-brand-danger">{resultsError}</p>
                    )}
                    {resultsByJob[job.id]?.length === 0 && (
                      <p className="text-xs text-brand-text-secondary">
                        Nenhum arquivo individual foi registrado para esta tarefa.
                      </p>
                    )}
                    <ul className="space-y-1.5">
                      {resultsByJob[job.id]?.slice(0, 50).map((result) => (
                        <li key={result.id} className="flex items-center gap-2 text-xs">
                          <FileText className="h-3.5 w-3.5 shrink-0 text-brand-text-secondary" />
                          <span className="truncate text-brand-text-secondary" title={result.file_path}>
                            {result.file_path}
                          </span>
                          <Badge tone={verdictTone(result.verdict)} className="ml-auto shrink-0">
                            {verdictLabel(result.verdict)}
                          </Badge>
                        </li>
                      ))}
                    </ul>
                  </td>
                </tr>
              )}
            </Fragment>
          )
        })}
      </tbody>
    </table>
  )
}
