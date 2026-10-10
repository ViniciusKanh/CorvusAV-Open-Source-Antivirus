import { useEffect } from 'react'
import { fetchEngineHealth } from '../services/corvusApi'
import { useEngineStore } from '../stores/engineStore'

/** Busca a saúde do mecanismo uma vez ao montar a aplicação. Atualizações
 * subsequentes chegam via eventos de verificação (ver `useScanEventBridge`),
 * que refazem essa consulta ao final de cada tarefa. */
export function useEngineHealthPolling(): void {
  const setHealth = useEngineStore((s) => s.setHealth)
  const setLoading = useEngineStore((s) => s.setLoading)
  const setError = useEngineStore((s) => s.setError)

  useEffect(() => {
    let cancelled = false

    async function load() {
      setLoading(true)
      try {
        const health = await fetchEngineHealth()
        if (!cancelled) setHealth(health)
      } catch (error) {
        if (!cancelled) {
          setError(error instanceof Error ? error.message : String(error))
        }
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    void load()
    return () => {
      cancelled = true
    }
  }, [setHealth, setLoading, setError])
}
