import { History, LayoutDashboard, RefreshCw, Search, Settings, ShieldAlert } from 'lucide-react'
import type { ComponentType } from 'react'
import logoMark from '../../assets/corvusav-mark.png'
import { cn } from '../../lib/cn'
import { useEngineStore } from '../../stores/engineStore'
import { useUiStore } from '../../stores/uiStore'
import type { SectionId } from '../../types/navigation'

const APP_VERSION = '0.1.0'

interface NavItem {
  id: SectionId
  label: string
  icon: ComponentType<{ className?: string }>
}

const NAV_ITEMS: NavItem[] = [
  { id: 'overview', label: 'Visão geral', icon: LayoutDashboard },
  { id: 'scanning', label: 'Verificação', icon: Search },
  { id: 'quarantine', label: 'Quarentena', icon: ShieldAlert },
  { id: 'history', label: 'Histórico', icon: History },
  { id: 'updates', label: 'Atualizações', icon: RefreshCw },
  { id: 'settings', label: 'Configurações', icon: Settings },
]

export function Sidebar() {
  const activeSection = useUiStore((s) => s.activeSection)
  const setActiveSection = useUiStore((s) => s.setActiveSection)
  const health = useEngineStore((s) => s.health)

  const statusText = !health
    ? 'Verificando mecanismo…'
    : health.availability === 'available'
      ? 'Pronto para verificar'
      : 'Mecanismo indisponível'

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-3 px-5 py-6">
        <img src={logoMark} alt="CorvusAV" className="h-9 w-9" />
        <div>
          <p className="font-display text-base font-bold leading-tight tracking-wide text-brand-text">
            CORVUS<span className="text-brand-cyan">AV</span>
          </p>
          <p className="text-[10px] font-medium uppercase tracking-wider text-brand-text-secondary">
            Open Source Antivirus
          </p>
        </div>
      </div>

      <nav className="flex-1 space-y-1 px-3">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon
          const isActive = activeSection === item.id
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => setActiveSection(item.id)}
              className={cn(
                'flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm font-medium transition-colors',
                isActive
                  ? 'bg-brand-blue-deep/40 text-brand-cyan'
                  : 'text-brand-text-secondary hover:bg-white/5 hover:text-brand-text',
              )}
            >
              <Icon className="h-4 w-4 shrink-0" />
              {item.label}
            </button>
          )
        })}
      </nav>

      <div className="border-t border-white/5 px-5 py-4">
        <p className="text-xs font-medium text-brand-text">{statusText}</p>
        <p className="mt-1 text-[11px] text-brand-text-secondary">CorvusAV v{APP_VERSION}</p>
      </div>
    </div>
  )
}
