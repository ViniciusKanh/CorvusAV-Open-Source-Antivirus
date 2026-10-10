import { useEffect } from 'react'
import { fetchEngineHealth, fetchRecentJobs, onScanEvent } from '../services/corvusApi'
import { useEngineStore } from '../stores/engineStore'
import { useScanStore } from '../stores/scanStore'
import { TERMINAL_SCAN_STATUSES, type ScanEvent } from '../types/corvus'

const HISTORY_LIMIT = 20

/** Assina os eventos de verificação emitidos pelo backend e mantém as
 * stores de UI sincronizadas com o estado real (nunca simulado) do scan em
 * andamento, do histórico e da saúde do mecanismo. */
export function useScanEventBridge(): void {
  const setActiveJob = useScanStore((s) => s.setActiveJob)
  const pushResult = useScanStore((s) => s.pushResult)
  const setHistory = useScanStore((s) => s.setHistory)
  const setHealth = useEngineStore((s) => s.setHealth)

  useEffect(() => {
    function handleEvent(event: ScanEvent) {
      if (event.type === 'job_updated') {
        const isTerminal = TERMINAL_SCAN_STATUSES.has(event.status)
        setActiveJob(isTerminal ? null : event)
        if (isTerminal) {
          void fetchRecentJobs(HISTORY_LIMIT).then(setHistory).catch(() => undefined)
          void fetchEngineHealth().then(setHealth).catch(() => undefined)
        }
      } else if (event.type === 'file_result') {
        pushResult(event)
      }
    }

    const unlisten = onScanEvent(handleEvent)
    return () => {
      void unlisten.then((fn) => fn())
    }
  }, [setActiveJob, pushResult, setHistory, setHealth])
}
