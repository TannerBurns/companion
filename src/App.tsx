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
  const { setView, setSettingsSection } = useAppStore()

  useEffect(() => {
    let unlistenSettings: (() => void) | undefined
    let unlistenUpdates: (() => void) | undefined
    let mounted = true

    listen('tray:open-settings', () => {
      setView('settings')
    }).then((fn) => {
      if (mounted) {
        unlistenSettings = fn
      } else {
        fn()
      }
    }).catch((err) => {
      console.warn('Failed to set up tray:open-settings listener:', err)
    })

    listen('tray:check-for-updates', () => {
      setView('settings')
      setSettingsSection('about')
    }).then((fn) => {
      if (mounted) {
        unlistenUpdates = fn
      } else {
        fn()
      }
    }).catch((err) => {
      console.warn('Failed to set up tray:check-for-updates listener:', err)
    })

    return () => {
      mounted = false
      unlistenSettings?.()
      unlistenUpdates?.()
    }
  }, [setView, setSettingsSection])

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
