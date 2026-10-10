import { AppShell } from './components/layout/AppShell'
import { Sidebar } from './components/layout/Sidebar'
import { TopBar } from './components/layout/TopBar'
import { DashboardPage } from './features/dashboard/DashboardPage'
import { HistoryPage } from './features/history/HistoryPage'
import { QuarantinePage } from './features/quarantine/QuarantinePage'
import { ScanningPage } from './features/scanning/ScanningPage'
import { SettingsPage } from './features/settings/SettingsPage'
import { UpdatesPage } from './features/updates/UpdatesPage'
import { useEngineHealthPolling } from './hooks/useEngineHealthPolling'
import { useScanEventBridge } from './hooks/useScanEventBridge'
import { useUiStore } from './stores/uiStore'
import type { SectionId } from './types/navigation'

const PAGES: Record<SectionId, JSX.Element> = {
  overview: <DashboardPage />,
  scanning: <ScanningPage />,
  quarantine: <QuarantinePage />,
  history: <HistoryPage />,
  updates: <UpdatesPage />,
  settings: <SettingsPage />,
}

export default function App() {
  const activeSection = useUiStore((s) => s.activeSection)

  useEngineHealthPolling()
  useScanEventBridge()

  return (
    <AppShell sidebar={<Sidebar />} topBar={<TopBar />}>
      {PAGES[activeSection]}
    </AppShell>
  )
}
