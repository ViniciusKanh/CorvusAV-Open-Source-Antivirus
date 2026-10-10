import { Clock, FolderSearch, ScanLine } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '../../../lib/cn'

interface PrimaryActionCardProps {
  icon: ReactNode
  title: string
  description: string
  buttonLabel: string
  onAction: () => void
  disabled: boolean
  disabledReason?: string
  busy?: boolean
}

/** A verificação rápida é o caminho principal desta versão: único card com
 * destaque visual (borda e botão preenchido em ciano) para funcionar como a
 * ação primária da tela. */
function PrimaryActionCard({
  icon,
  title,
  description,
  buttonLabel,
  onAction,
  disabled,
  disabledReason,
  busy,
}: PrimaryActionCardProps) {
  const isInactive = disabled || Boolean(busy)
  return (
    <div className="flex flex-col justify-between gap-4 rounded-2xl border border-brand-cyan/25 bg-gradient-to-br from-brand-blue-deep/30 to-brand-surface/40 p-5">
      <div>
        <div className="mb-3 flex items-center justify-between">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-brand-cyan/15 text-brand-cyan">
            {icon}
          </div>
          <span className="rounded-full bg-brand-cyan/10 px-2.5 py-1 text-xs font-medium text-brand-cyan">
            Mais usada
          </span>
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

interface SecondaryActionCardProps {
  icon: ReactNode
  title: string
  description: string
  buttonLabel: string
  onAction: () => void
  disabled: boolean
  disabledReason?: string
  busy?: boolean
}

/** Verificação personalizada funciona de ponta a ponta, mas é uma ação
 * secundária: botão com contorno em vez de preenchido, para não competir
 * visualmente com a verificação rápida. */
function SecondaryActionCard({
  icon,
  title,
  description,
  buttonLabel,
  onAction,
  disabled,
  disabledReason,
  busy,
}: SecondaryActionCardProps) {
  const isInactive = disabled || Boolean(busy)
  return (
    <div className="flex flex-col justify-between gap-4 rounded-2xl border border-white/5 bg-brand-surface/30 p-5">
      <div>
        <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-white/5 text-brand-blue-accent">
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
            'w-full rounded-lg border px-4 py-2.5 text-sm font-semibold transition-colors',
            isInactive
              ? 'cursor-not-allowed border-white/5 bg-white/5 text-brand-text-secondary'
              : 'border-brand-blue-accent/40 bg-transparent text-brand-blue-accent hover:bg-brand-blue-accent/10',
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

/** Verificação completa ainda não existe nesta versão. Em vez de um botão
 * desabilitado disfarçado de ação real, o card assume visualmente que é uma
 * prévia de algo futuro: borda tracejada, ícone apagado e uma nota de
 * roteiro no lugar do botão. */
function FutureActionCard({
  icon,
  title,
  description,
  note,
}: {
  icon: ReactNode
  title: string
  description: string
  note: string
}) {
  return (
    <div className="flex flex-col justify-between gap-4 rounded-2xl border border-dashed border-white/10 bg-white/[0.02] p-5">
      <div>
        <div className="mb-3 flex items-center justify-between">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-white/5 text-brand-text-secondary">
            {icon}
          </div>
          <span className="rounded-full bg-white/5 px-2.5 py-1 text-xs font-medium text-brand-text-secondary">
            Em breve
          </span>
        </div>
        <h3 className="font-display text-base font-semibold text-brand-text-secondary">{title}</h3>
        <p className="mt-1 text-sm text-brand-text-secondary">{description}</p>
      </div>
      <div className="flex items-start gap-2 rounded-lg bg-white/[0.03] px-3 py-2.5">
        <Clock className="mt-0.5 h-3.5 w-3.5 shrink-0 text-brand-text-secondary" />
        <p className="text-xs text-brand-text-secondary">{note}</p>
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
      <PrimaryActionCard
        icon={<ScanLine className="h-5 w-5" />}
        title="Verificação rápida"
        description="Examina áreas críticas acessíveis ao usuário (downloads, área de trabalho, documentos e arquivos temporários)."
        buttonLabel="Iniciar verificação rápida"
        onAction={onStartQuick}
        disabled={!engineAvailable}
        disabledReason={!engineAvailable ? 'Indisponível até o ClamAV ser detectado.' : undefined}
        busy={busyKind === 'quick'}
      />
      <SecondaryActionCard
        icon={<FolderSearch className="h-5 w-5" />}
        title="Verificação personalizada"
        description="Escolha pastas específicas para verificar através do seletor nativo do Windows."
        buttonLabel="Selecionar local para verificação"
        onAction={onStartCustom}
        disabled={!engineAvailable}
        disabledReason={!engineAvailable ? 'Indisponível até o ClamAV ser detectado.' : undefined}
        busy={busyKind === 'custom'}
      />
      <FutureActionCard
        icon={<Clock className="h-5 w-5" />}
        title="Verificação completa"
        description="Analisará todas as unidades do sistema, respeitando exclusões e limites configurados."
        note="Prevista para uma fase futura do roadmap (ver docs/roadmap)."
      />
    </div>
  )
}
