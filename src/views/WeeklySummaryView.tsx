import { Calendar } from 'lucide-react'
import { useAppStore } from '../store'

export function WeeklySummaryView() {
  const { setView } = useAppStore()

  return (
    <div className="max-w-4xl mx-auto">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-foreground">Weekly Summary</h2>
        <p className="text-muted-foreground mt-1">Overview of the past 7 days</p>
      </div>

      {/* Empty State */}
      <div className="bg-card border border-border rounded-xl p-12 text-center">
        <div className="mx-auto w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
          <Calendar className="h-8 w-8 text-muted-foreground" />
        </div>
        <h3 className="text-lg font-semibold text-foreground mb-2">
          No weekly summary yet
        </h3>
        <p className="text-muted-foreground max-w-sm mx-auto mb-6">
          Connect your accounts and sync data to generate weekly summaries.
        </p>
        <button
          onClick={() => setView('settings')}
          className="px-4 py-2 bg-primary-500 text-white rounded-lg hover:bg-primary-600 transition-colors font-medium"
        >
          Connect Accounts
        </button>
      </div>
    </div>
  )
}
