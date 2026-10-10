import { ChevronDown, ChevronRight, FileText, ShieldAlert } from 'lucide-react'
import { Fragment, useEffect, useRef, useState } from 'react'
import { fetchJobResults, fetchVerdictSummaries, quarantineResult } from '../../services/corvusApi'
import { formatDateTime, formatDurationBetween, formatSpeedBetween } from '../../lib/format'
import { scanKindLabel, scanStatusLabel, scanStatusTone, verdictLabel, verdictTone } from '../../lib/labels'
import type { ScanJob, ScanResult, Verdict, VerdictCounts } from '../../types/corvus'
import { Badge } from './Badge'

interface ScanHistoryTableProps {
  jobs: ScanJob[]
  loading: boolean
}

/** Resume o veredito de uma verificação em contagens compactas, destacando
 * primeiro o que exige atenção (maliciosos, suspeitos, erros, indeterminados)
 * e só depois o volume de itens limpos — nunca uma nota única que misture
 * naturezas diferentes de resultado. */
function verdictCountLabel(verdict: Verdict, count: number): string {
  switch (verdict) {
    case 'malicious':
      return count === 1 ? 'ameaça' : 'ameaças'
    case 'suspicious':
      return count === 1 ? 'suspeito' : 'suspeitos'
    case 'error':
      return count === 1 ? 'erro' : 'erros'
    case 'unknown':
      return count === 1 ? 'indeterminado' : 'indeterminados'
    case 'clean':
      return count === 1 ? 'limpo' : 'limpos'
  }
}

function VerdictSummaryCell({ counts }: { counts: VerdictCounts | undefined }) {
  const total = counts
    ? counts.clean + counts.suspicious + counts.malicious + counts.unknown + counts.error
    : 0
  if (!counts || total === 0) {
    return <span className="text-brand-text-secondary">—</span>
  }

  const problemVerdicts: Verdict[] = ['malicious', 'suspicious', 'error', 'unknown']
  const problemEntries = problemVerdicts
    .map((verdict) => ({ verdict, count: counts[verdict] }))
    .filter((entry) => entry.count > 0)

  if (problemEntries.length === 0) {
    return (
      <Badge tone="success">
        {counts.clean} {verdictCountLabel('clean', counts.clean)}
      </Badge>
    )
  }

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {problemEntries.map(({ verdict, count }) => (
        <Badge key={verdict} tone={verdictTone(verdict)}>
          {count} {verdictCountLabel(verdict, count)}
        </Badge>
      ))}
      {counts.clean > 0 && (
        <span className="text-xs text-brand-text-secondary">+{counts.clean} limpos</span>
      )}
    </div>
  )
}

export function ScanHistoryTable({ jobs, loading }: ScanHistoryTableProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [resultsByJob, setResultsByJob] = useState<Record<string, ScanResult[]>>({})
  const [loadingResults, setLoadingResults] = useState<string | null>(null)
  const [resultsError, setResultsError] = useState<string | null>(null)
  const [verdictSummaries, setVerdictSummaries] = useState<Record<string, VerdictCounts>>({})
  const requestedSummaryIds = useRef<Set<string>>(new Set())
  const [quarantiningId, setQuarantiningId] = useState<string | null>(null)
  const [quarantineErrors, setQuarantineErrors] = useState<Record<string, string>>({})
  // Resultados já isolados nesta sessão da tabela. O `ScanResult` em si não
  // carrega essa informação (quarentena é um registro à parte, ver
  // `crates/corvus-quarantine`), então rastreamos localmente para dar
  // feedback imediato sem precisar recarregar os resultados da tarefa.
  const [quarantinedResultIds, setQuarantinedResultIds] = useState<Set<string>>(new Set())

  useEffect(() => {
    const idsToFetch = jobs.map((job) => job.id).filter((id) => !requestedSummaryIds.current.has(id))
    if (idsToFetch.length === 0) {
      return
    }
    idsToFetch.forEach((id) => requestedSummaryIds.current.add(id))
    let cancelled = false
    fetchVerdictSummaries(idsToFetch)
      .then((summaries) => {
        if (!cancelled) {
          setVerdictSummaries((prev) => ({ ...prev, ...summaries }))
        }
      })
      .catch(() => {
        // Falha silenciosa: a coluna de qualidade simplesmente mostra "—"
        // para as verificações cujo resumo não pôde ser carregado.
      })
    return () => {
      cancelled = true
    }
  }, [jobs])

  async function handleQuarantine(result: ScanResult) {
    setQuarantiningId(result.id)
    setQuarantineErrors((prev) => {
      const next = { ...prev }
      delete next[result.id]
      return next
    })
    try {
      await quarantineResult(result.id)
      setQuarantinedResultIds((prev) => new Set(prev).add(result.id))
    } catch (error) {
      setQuarantineErrors((prev) => ({
        ...prev,
        [result.id]: error instanceof Error ? error.message : String(error),
      }))
    } finally {
      setQuarantiningId(null)
    }
  }

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
          <th className="px-3 py-2 font-medium">Velocidade</th>
          <th className="px-3 py-2 font-medium">Qualidade</th>
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
                  {job.status === 'failed' && job.error_message && (
                    <p
                      className="mt-1 max-w-[18rem] truncate text-xs text-brand-danger"
                      title={job.error_message}
                    >
                      {job.error_message}
                    </p>
                  )}
                </td>
                <td className="px-3 py-3 text-brand-text-secondary">{job.files_scanned}</td>
                <td className="px-3 py-3 text-brand-text-secondary">
                  {formatDurationBetween(job.started_at, job.finished_at)}
                </td>
                <td className="px-3 py-3 text-brand-text-secondary">
                  {formatSpeedBetween(job.files_scanned, job.started_at, job.finished_at)}
                </td>
                <td className="px-3 py-3">
                  <VerdictSummaryCell counts={verdictSummaries[job.id]} />
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
                  <td colSpan={8} className="rounded-lg bg-brand-bg-secondary/60 px-4 py-3">
                    {loadingResults === job.id && (
                      <p className="text-xs text-brand-text-secondary">Carregando resultados…</p>
                    )}
                    {resultsError && loadingResults !== job.id && (
                      <p className="text-xs text-brand-danger">{resultsError}</p>
                    )}
                    {job.error_message && (
                      <p className="mb-2 text-xs text-brand-danger">
                        <span className="font-semibold">Motivo da falha: </span>
                        {job.error_message}
                      </p>
                    )}
                    {resultsByJob[job.id]?.length === 0 && (
                      <p className="text-xs text-brand-text-secondary">
                        {job.error_message
                          ? 'A verificação não chegou a examinar nenhum arquivo.'
                          : 'Nenhum arquivo individual foi registrado para esta tarefa.'}
                      </p>
                    )}
                    <ul className="space-y-1.5">
                      {resultsByJob[job.id]?.slice(0, 50).map((result) => {
                        const canQuarantine =
                          (result.verdict === 'malicious' || result.verdict === 'suspicious') &&
                          !quarantinedResultIds.has(result.id)
                        return (
                          <li key={result.id} className="flex flex-col gap-1 text-xs">
                            <div className="flex items-center gap-2">
                              <FileText className="h-3.5 w-3.5 shrink-0 text-brand-text-secondary" />
                              <span className="truncate text-brand-text-secondary" title={result.file_path}>
                                {result.file_path}
                              </span>
                              <Badge tone={verdictTone(result.verdict)} className="ml-auto shrink-0">
                                {verdictLabel(result.verdict)}
                              </Badge>
                              {quarantinedResultIds.has(result.id) && (
                                <Badge tone="neutral" className="shrink-0">
                                  Em quarentena
                                </Badge>
                              )}
                              {canQuarantine && (
                                <button
                                  type="button"
                                  onClick={() => void handleQuarantine(result)}
                                  disabled={quarantiningId === result.id}
                                  className="inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-brand-danger hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-60"
                                >
                                  <ShieldAlert className="h-3.5 w-3.5" />
                                  {quarantiningId === result.id ? 'Isolando…' : 'Colocar em quarentena'}
                                </button>
                              )}
                            </div>
                            {quarantineErrors[result.id] && (
                              <p className="pl-5 text-brand-danger">{quarantineErrors[result.id]}</p>
                            )}
                          </li>
                        )
                      })}
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
