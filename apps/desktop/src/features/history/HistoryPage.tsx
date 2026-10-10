import { Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { ScanHistoryTable } from '../../components/shared/ScanHistoryTable'
import { useScanHistory } from '../../hooks/useScanHistory'
import { clearScanHistory } from '../../services/corvusApi'
import { scanKindLabel } from '../../lib/labels'
import { TERMINAL_SCAN_STATUSES } from '../../types/corvus'
import type { ScanJob, ScanKind } from '../../types/corvus'

const HISTORY_LIMIT = 200

type StatusFilter = 'all' | 'completed' | 'failed' | 'cancelled' | 'active'
type KindFilter = 'all' | ScanKind
type PeriodFilter = 'all' | '24h' | '7d' | '30d'

const STATUS_FILTER_LABELS: Record<StatusFilter, string> = {
  all: 'Todos os estados',
  completed: 'Concluída',
  failed: 'Falha',
  cancelled: 'Cancelada',
  active: 'Em andamento',
}

const KIND_FILTER_LABELS: Record<KindFilter, string> = {
  all: 'Todos os tipos',
  quick: scanKindLabel('quick'),
  full: scanKindLabel('full'),
  custom: scanKindLabel('custom'),
}

const PERIOD_FILTER_LABELS: Record<PeriodFilter, string> = {
  all: 'Qualquer período',
  '24h': 'Últimas 24 horas',
  '7d': 'Últimos 7 dias',
  '30d': 'Últimos 30 dias',
}

const PERIOD_FILTER_MS: Record<Exclude<PeriodFilter, 'all'>, number> = {
  '24h': 24 * 60 * 60 * 1000,
  '7d': 7 * 24 * 60 * 60 * 1000,
  '30d': 30 * 24 * 60 * 60 * 1000,
}

function matchesStatus(job: ScanJob, filter: StatusFilter): boolean {
  if (filter === 'all') return true
  if (filter === 'active') return !TERMINAL_SCAN_STATUSES.has(job.status)
  return job.status === filter
}

function matchesPeriod(job: ScanJob, filter: PeriodFilter): boolean {
  if (filter === 'all') return true
  const createdAt = new Date(job.created_at).getTime()
  if (Number.isNaN(createdAt)) return true
  return Date.now() - createdAt <= PERIOD_FILTER_MS[filter]
}

function matchesSearch(job: ScanJob, query: string): boolean {
  if (!query.trim()) return true
  const haystack = [...job.target_paths, job.current_file ?? ''].join(' ').toLowerCase()
  return haystack.includes(query.trim().toLowerCase())
}

const SELECT_CLASS =
  'rounded-md border border-white/10 bg-brand-bg/40 px-2.5 py-1.5 text-xs font-medium text-brand-text focus:border-brand-cyan/50 focus:outline-none'

export function HistoryPage() {
  const { history, loading, reload } = useScanHistory(HISTORY_LIMIT)
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all')
  const [kindFilter, setKindFilter] = useState<KindFilter>('all')
  const [periodFilter, setPeriodFilter] = useState<PeriodFilter>('all')
  const [search, setSearch] = useState('')
  const [clearing, setClearing] = useState(false)
  const [clearError, setClearError] = useState<string | null>(null)
  const [clearMessage, setClearMessage] = useState<string | null>(null)

  const filteredJobs = useMemo(
    () =>
      history.filter(
        (job) =>
          matchesStatus(job, statusFilter) &&
          (kindFilter === 'all' || job.kind === kindFilter) &&
          matchesPeriod(job, periodFilter) &&
          matchesSearch(job, search),
      ),
    [history, statusFilter, kindFilter, periodFilter, search],
  )

  const hasClearableHistory = history.some((job) => job.status === 'failed' || job.status === 'cancelled')
  const hasActiveFilters =
    statusFilter !== 'all' || kindFilter !== 'all' || periodFilter !== 'all' || search.trim() !== ''

  async function handleClearHistory() {
    setClearError(null)
    setClearMessage(null)
    setClearing(true)
    try {
      const removed = await clearScanHistory()
      await reload()
      setClearMessage(
        removed > 0
          ? `${removed} ${removed === 1 ? 'verificação removida' : 'verificações removidas'}.`
          : 'Nenhuma verificação com falha ou cancelamento foi encontrada para remover.',
      )
    } catch (error) {
      setClearError(error instanceof Error ? error.message : String(error))
    } finally {
      setClearing(false)
    }
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="font-display text-2xl font-bold text-brand-text">Histórico</h1>
        <p className="mt-1 text-sm text-brand-text-secondary">
          Registro real de verificações persistido localmente em SQLite — últimas {HISTORY_LIMIT}.
        </p>
      </div>

      <div className="rounded-2xl border border-white/5 bg-brand-surface/20 p-5">
        <div className="mb-4 flex flex-wrap items-center gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-brand-text-secondary" />
            <input
              type="text"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Buscar por caminho…"
              className="rounded-md border border-white/10 bg-brand-bg/40 py-1.5 pl-8 pr-3 text-xs text-brand-text placeholder:text-brand-text-secondary focus:border-brand-cyan/50 focus:outline-none"
            />
          </div>

          <select
            value={statusFilter}
            onChange={(event) => setStatusFilter(event.target.value as StatusFilter)}
            className={SELECT_CLASS}
          >
            {(Object.keys(STATUS_FILTER_LABELS) as StatusFilter[]).map((value) => (
              <option key={value} value={value}>
                {STATUS_FILTER_LABELS[value]}
              </option>
            ))}
          </select>

          <select
            value={kindFilter}
            onChange={(event) => setKindFilter(event.target.value as KindFilter)}
            className={SELECT_CLASS}
          >
            {(Object.keys(KIND_FILTER_LABELS) as KindFilter[]).map((value) => (
              <option key={value} value={value}>
                {KIND_FILTER_LABELS[value]}
              </option>
            ))}
          </select>

          <select
            value={periodFilter}
            onChange={(event) => setPeriodFilter(event.target.value as PeriodFilter)}
            className={SELECT_CLASS}
          >
            {(Object.keys(PERIOD_FILTER_LABELS) as PeriodFilter[]).map((value) => (
              <option key={value} value={value}>
                {PERIOD_FILTER_LABELS[value]}
              </option>
            ))}
          </select>

          {hasClearableHistory && (
            <button
              type="button"
              onClick={() => void handleClearHistory()}
              disabled={clearing}
              className="ml-auto shrink-0 rounded-md px-2.5 py-1.5 text-xs font-medium text-brand-text-secondary hover:bg-white/5 hover:text-brand-text disabled:cursor-not-allowed disabled:opacity-60"
            >
              {clearing ? 'Limpando…' : 'Limpar falhas e cancelamentos'}
            </button>
          )}
        </div>

        {clearError && <p className="mb-3 text-xs text-brand-danger">{clearError}</p>}
        {clearMessage && <p className="mb-3 text-xs text-brand-text-secondary">{clearMessage}</p>}

        {hasActiveFilters && !loading && (
          <p className="mb-3 text-xs text-brand-text-secondary">
            {filteredJobs.length} de {history.length} verificações correspondem aos filtros.
          </p>
        )}

        {!loading && filteredJobs.length === 0 && history.length > 0 ? (
          <p className="py-6 text-center text-sm text-brand-text-secondary">
            Nenhuma verificação corresponde aos filtros selecionados.
          </p>
        ) : (
          <ScanHistoryTable jobs={filteredJobs} loading={loading} />
        )}
      </div>
    </div>
  )
}
