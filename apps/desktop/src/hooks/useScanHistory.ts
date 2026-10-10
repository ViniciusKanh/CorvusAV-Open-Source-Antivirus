import { useCallback, useEffect } from 'react'
import { fetchRecentJobs } from '../services/corvusApi'
import { useScanStore } from '../stores/scanStore'

/** Carrega (e permite recarregar) o histórico real de verificações. */
export function useScanHistory(limit = 20) {
  const history = useScanStore((s) => s.history)
  const loading = useScanStore((s) => s.historyLoading)
  const setHistory = useScanStore((s) => s.setHistory)
  const setLoading = useScanStore((s) => s.setHistoryLoading)

  const reload = useCallback(async () => {
    setLoading(true)
    try {
      const jobs = await fetchRecentJobs(limit)
      setHistory(jobs)
    } finally {
      setLoading(false)
    }
  }, [limit, setHistory, setLoading])

  useEffect(() => {
    void reload()
  }, [reload])

  return { history, loading, reload }
}
