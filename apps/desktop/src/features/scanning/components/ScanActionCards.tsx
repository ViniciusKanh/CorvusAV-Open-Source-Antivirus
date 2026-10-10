import { FolderSearch, HardDrive, ScanLine } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '../../../lib/cn'

interface ActionCardProps {
  icon: ReactNode
  title: string
  description: string
  buttonLabel: string
  onAction: () => void
  disabled: boolean
  disabledReason?: string
  busy?: boolean
}

function ActionCard({
  icon,
  title,
  description,
  buttonLabel,
  onAction,
  disabled,
  disabledReason,
  busy,
}: ActionCardProps) {
  const isInactive = disabled || Boolean(busy)
  return (
    <div className="flex flex-col justify-between gap-4 rounded-2xl border border-white/5 bg-brand-surface/40 p-5">
      <div>
        <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-brand-blue-deep/40 text-brand-cyan">
          {icon}
        </div>
        <h3 className="font-display text-base font-semibold text-brand-text">{title}</h3>
        <p className="mt-1 text-sm text-brand-text-secondary">{description}</p>
      </div>
      <div>
        <button
          type="button"
          onClick={onAction}
          disabled={isInactive}
          className={cn(
            'w-full rounded-lg px-4 py-2.5 text-sm font-semibold transition-colors',
            isInactive
              ? 'cursor-not-allowed bg-white/5 text-brand-text-secondary'
              : 'bg-brand-cyan text-brand-bg hover:bg-brand-blue-accent',
          )}
        >
          {busy ? 'Iniciando…' : buttonLabel}
        </button>
        {disabled && disabledReason && (
          <p className="mt-2 text-xs text-brand-text-secondary">{disabledReason}</p>
        )}
      </div>
    </div>
  )
}

interface ScanActionCardsProps {
  engineAvailable: boolean
  busyKind: 'quick' | 'custom' | null
  onStartQuick: () => void
  onStartCustom: () => void
}

export function ScanActionCards({
  engineAvailable,
  busyKind,
  onStartQuick,
  onStartCustom,
}: ScanActionCardsProps) {
  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
      <ActionCard
        icon={<ScanLine className="h-5 w-5" />}
        title="Verificação rápida"
        description="Examina áreas críticas acessíveis ao usuário (downloads, área de trabalho, documentos e arquivos temporários)."
        buttonLabel="Iniciar verificação rápida"
        onAction={onStartQuick}
        disabled={!engineAvailable}
        disabledReason={!engineAvailable ? 'Indisponível até o ClamAV ser detectado.' : undefined}
        busy={busyKind === 'quick'}
      />
      <ActionCard
        icon={<HardDrive className="h-5 w-5" />}
        title="Verificação completa"
        description="Analisa todas as unidades do sistema, respeitando exclusões e limites configurados."
        buttonLabel="Indisponível nesta versão"
        onAction={() => undefined}
        disabled
        disabledReason="Será implementada em uma fase posterior do roadmap (ver docs/roadmap)."
      />
      <ActionCard
        icon={<FolderSearch className="h-5 w-5" />}
        title="Verificação personalizada"
        description="Escolha pastas específicas para verificar através do seletor nativo do Windows."
        buttonLabel="Selecionar local para verificação"
        onAction={onStartCustom}
        disabled={!engineAvailable}
        disabledReason={!engineAvailable ? 'Indisponível até o ClamAV ser detectado.' : undefined}
        busy={busyKind === 'custom'}
      />
    </div>
  )
}
