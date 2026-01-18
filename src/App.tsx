import { useEffect } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { listen } from '@tauri-apps/api/event'
import { ThemeProvider } from './lib/ThemeProvider'
import { useAppStore } from './store'
import { Layout, OfflineIndicator } from './components'
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
    })

    return () => {
      mounted = false
      unlistenFn?.()
    }
  }, [setView])

  return null
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <TrayEventHandler />
        <Layout>
          <MainContent />
        </Layout>
        <OfflineIndicator />
      </ThemeProvider>
    </QueryClientProvider>
  )
}

export default App
