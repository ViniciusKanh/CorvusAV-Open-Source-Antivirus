import { FolderMinus, FolderPlus } from 'lucide-react'
import { useEffect, useState } from 'react'
import { InDevelopment } from '../../components/shared/InDevelopment'
import { getScanExclusions, pickCustomScanTargets, setScanExclusions } from '../../services/corvusApi'

export function SettingsPage() {
  const [exclusions, setExclusions] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    getScanExclusions()
      .then((paths) => {
        if (!cancelled) {
          setExclusions(paths)
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err))
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false)
        }
      })
    return () => {
      cancelled = true
    }
  }, [])

  async function persist(next: string[]) {
    setError(null)
    setSaving(true)
    try {
      await setScanExclusions(next)
      setExclusions(next)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setSaving(false)
    }
  }

  async function handleAddFolders() {
    setError(null)
    try {
      const picked = await pickCustomScanTargets()
      if (picked.length === 0) {
        return
      }
      const merged = Array.from(new Set([...exclusions, ...picked]))
      await persist(merged)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  async function handleRemove(path: string) {
    await persist(exclusions.filter((candidate) => candidate !== path))
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-display text-2xl font-bold text-brand-text">Configurações</h1>
        <p className="mt-1 text-sm text-brand-text-secondary">
          Preferências do CorvusAV neste computador.
        </p>
      </div>

      <section className="rounded-2xl border border-white/5 bg-brand-surface/20 p-5">
        <div className="mb-1 flex items-center justify-between gap-3">
          <h2 className="font-display text-base font-semibold text-brand-text">Exclusões de verificação</h2>
          <button
            type="button"
            onClick={() => void handleAddFolders()}
            disabled={saving}
            className="inline-flex shrink-0 items-center gap-1.5 rounded-md bg-brand-blue-deep/40 px-3 py-1.5 text-xs font-medium text-brand-cyan hover:bg-brand-blue-deep/60 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <FolderPlus className="h-3.5 w-3.5" />
            Adicionar pasta
          </button>
        </div>
        <p className="mb-4 text-sm text-brand-text-secondary">
          Pastas listadas aqui são ignoradas em verificações rápidas, completas e personalizadas. Use
          com cautela: nada dentro delas é examinado.
        </p>

        {error && <p className="mb-3 text-xs text-brand-danger">{error}</p>}

        {loading ? (
          <p className="text-sm text-brand-text-secondary">Carregando…</p>
        ) : exclusions.length === 0 ? (
          <p className="text-sm text-brand-text-secondary">Nenhuma pasta excluída.</p>
        ) : (
          <ul className="space-y-1.5">
            {exclusions.map((path) => (
              <li
                key={path}
                className="flex items-center gap-2 rounded-lg bg-brand-bg-secondary/60 px-3 py-2 text-xs text-brand-text-secondary"
              >
                <span className="truncate" title={path}>
                  {path}
                </span>
                <button
                  type="button"
                  onClick={() => void handleRemove(path)}
                  disabled={saving}
                  className="ml-auto inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 font-medium text-brand-text-secondary hover:bg-white/5 hover:text-brand-danger disabled:cursor-not-allowed disabled:opacity-60"
                >
                  <FolderMinus className="h-3.5 w-3.5" />
                  Remover
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <InDevelopment
        title="Caminho do ClamAV"
        description="A configuração manual do acesso ao ClamAV depende de uma decisão de arquitetura ainda pendente (avaliação de migração de clamscan para clamd) e será liberada após essa definição."
      />
    </div>
  )
}
