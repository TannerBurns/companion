import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ThemeProvider } from './lib/ThemeProvider'
import { useAppStore } from './store'
import { Layout } from './components'
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

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <Layout>
          <MainContent />
        </Layout>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

export default App
