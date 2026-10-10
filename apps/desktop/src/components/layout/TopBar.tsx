import { engineAvailabilityLabel } from '../../lib/labels'
import { useEngineStore } from '../../stores/engineStore'

/** Barra superior minimalista: nesta versão não há campo de pesquisa
 * funcional nem informações de dispositivo verificadas, então ambos são
 * omitidos (ver seção 5.2 da especificação) em vez de simulados. Os
 * controles de janela usam a decoração nativa do sistema operacional. */
export function TopBar() {
  const health = useEngineStore((s) => s.health)
  const available = health?.availability === 'available'

  return (
    <div className="flex w-full items-center justify-end gap-2">
      <span className={`h-2 w-2 rounded-full ${available ? 'bg-brand-success' : 'bg-brand-text-secondary'}`} />
      <span className="text-xs font-medium text-brand-text-secondary">
        {health ? engineAvailabilityLabel(health.availability) : 'Verificando mecanismo…'}
      </span>
    </div>
  )
}
