import { clsx } from 'clsx'
import { CalendarDays, CalendarRange, Settings, RefreshCw } from 'lucide-react'
import { useAppStore } from '../store'
import { useSync } from '../hooks/useDigest'
import { PipelineStatus } from './PipelineStatus'

export function Header() {
  const { currentView, setView } = useAppStore()
  const { sync, isSyncing } = useSync()

  const handleSync = () => {
    sync(undefined)
  }

  const isDigestView = currentView === 'daily-digest' || currentView === 'weekly-summary'

  return (
    <header className="border-b border-border bg-card">
      <div className="flex items-center justify-between px-6 py-3">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-3">
            <div className="h-8 w-8 rounded-lg bg-white border border-gray-200 dark:border-gray-700 flex items-center justify-center shadow-sm">
              <span className="text-gray-700 dark:text-gray-300 font-bold text-sm">C</span>
            </div>
            <h1 className="text-xl font-semibold text-foreground">Companion</h1>
          </div>
          
          {/* View Toggle */}
          {isDigestView && (
            <div className="flex items-center bg-muted rounded-lg p-1">
              <button
                onClick={() => setView('daily-digest')}
                className={clsx(
                  'flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium transition-colors',
                  currentView === 'daily-digest'
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground'
                )}
                title="Daily Digest"
              >
                <CalendarDays className="h-4 w-4" />
                <span>Daily</span>
              </button>
              <button
                onClick={() => setView('weekly-summary')}
                className={clsx(
                  'flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium transition-colors',
                  currentView === 'weekly-summary'
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground'
                )}
                title="Weekly Summary"
              >
                <CalendarRange className="h-4 w-4" />
                <span>Weekly</span>
              </button>
            </div>
          )}
        </div>
        <div className="flex items-center gap-2">
          <PipelineStatus />
          <button
            onClick={handleSync}
            disabled={isSyncing}
            className="p-2 rounded-lg hover:bg-muted transition-colors disabled:opacity-50"
            title={isSyncing ? 'Syncing...' : 'Sync now'}
          >
            <RefreshCw className={clsx('h-5 w-5 text-muted-foreground', isSyncing && 'animate-spin')} />
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

interface LayoutProps {
  children: React.ReactNode
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen bg-background">
      <Header />
      <main className="p-6">
        {children}
      </main>
    </div>
  )
}
