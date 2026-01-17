import { clsx } from 'clsx'
import { Calendar, Settings, RefreshCw } from 'lucide-react'
import { useAppStore } from '../store'
import { NavItem } from './NavItem'

export function Header() {
  const { currentView, setView } = useAppStore()

  return (
    <header className="border-b border-border bg-card">
      <div className="flex items-center justify-between px-6 py-4">
        <div className="flex items-center gap-3">
          <div className="h-8 w-8 rounded-lg bg-primary-500 flex items-center justify-center">
            <span className="text-white font-bold text-sm">C</span>
          </div>
          <h1 className="text-xl font-semibold text-foreground">Companion</h1>
        </div>
        <div className="flex items-center gap-2">
          <button className="p-2 rounded-lg hover:bg-muted transition-colors">
            <RefreshCw className="h-5 w-5 text-muted-foreground" />
          </button>
          <button
            onClick={() => setView('settings')}
            className={clsx(
              'p-2 rounded-lg transition-colors',
              currentView === 'settings'
                ? 'bg-primary-50 text-primary-600'
                : 'hover:bg-muted text-muted-foreground'
            )}
          >
            <Settings className="h-5 w-5" />
          </button>
        </div>
      </div>
    </header>
  )
}

export function Sidebar() {
  const { currentView, setView } = useAppStore()

  return (
    <aside className="w-64 border-r border-border bg-card min-h-[calc(100vh-65px)]">
      <nav className="p-4 space-y-2">
        <NavItem
          icon={Calendar}
          label="Daily Digest"
          active={currentView === 'daily-digest'}
          onClick={() => setView('daily-digest')}
        />
        <NavItem
          icon={Calendar}
          label="Weekly Summary"
          active={currentView === 'weekly-summary'}
          onClick={() => setView('weekly-summary')}
        />
      </nav>
    </aside>
  )
}

interface LayoutProps {
  children: React.ReactNode
}

export function Layout({ children }: LayoutProps) {
  const { currentView } = useAppStore()
  const showSidebar = currentView !== 'settings'

  return (
    <div className="min-h-screen bg-background">
      <Header />
      <div className="flex">
        {showSidebar && <Sidebar />}
        <main className={clsx('flex-1 p-6', !showSidebar && 'max-w-none')}>
          {children}
        </main>
      </div>
    </div>
  )
}
