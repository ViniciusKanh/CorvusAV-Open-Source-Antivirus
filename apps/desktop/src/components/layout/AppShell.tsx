import type { PropsWithChildren, ReactNode } from 'react'

interface AppShellProps extends PropsWithChildren {
  sidebar: ReactNode
  topBar: ReactNode
}

export function AppShell({ sidebar, topBar, children }: AppShellProps) {
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-brand-bg font-sans text-brand-text">
      <aside className="flex w-64 shrink-0 flex-col border-r border-white/5 bg-brand-bg-secondary">
        {sidebar}
      </aside>
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center border-b border-white/5 px-5" data-tauri-drag-region>
          {topBar}
        </header>
        <main className="flex-1 overflow-y-auto p-6">{children}</main>
      </div>
    </div>
  )
}
