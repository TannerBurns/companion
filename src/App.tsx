import { useEffect } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { listen } from '@tauri-apps/api/event'
import { ThemeProvider } from './lib/ThemeProvider'
import { useAppStore } from './store'
import { useSyncCompletedListener } from './hooks/useDigest'
import { Layout, OfflineIndicator, UpdateNotification } from './components'
import { DailyDigestView, WeeklySummaryView, SettingsView } from './views'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
})

function MainContent() {
  const { currentView } = useAppStore()

  switch (currentView) {
    case 'daily-digest':
      return <DailyDigestView />
    case 'weekly-summary':
      return <WeeklySummaryView />
    case 'settings':
      return <SettingsView />
    default:
      return <DailyDigestView />
  }
}

function TrayEventHandler() {
  const { setView } = useAppStore()

  useEffect(() => {
    let unlistenFn: (() => void) | undefined
    let mounted = true

    listen('tray:open-settings', () => {
      setView('settings')
    }).then((fn) => {
      if (mounted) {
        unlistenFn = fn
      } else {
        // Component unmounted before promise resolved, clean up immediately
        fn()
      }
    }).catch((err) => {
      // Gracefully handle listener setup failure (e.g., running in browser without Tauri)
      console.warn('Failed to set up tray event listener:', err)
    })

    return () => {
      mounted = false
      unlistenFn?.()
    }
  }, [setView])

  return null
}

function SyncEventHandler() {
  useSyncCompletedListener()
  return null
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <TrayEventHandler />
        <SyncEventHandler />
        <Layout>
          <MainContent />
        </Layout>
        <OfflineIndicator />
        <UpdateNotification />
      </ThemeProvider>
    </QueryClientProvider>
  )
}

export default App
