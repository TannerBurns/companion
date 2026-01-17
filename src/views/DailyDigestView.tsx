import { Calendar } from 'lucide-react'
import { useAppStore } from '../store'

export function DailyDigestView() {
  const { setView } = useAppStore()

  return (
    <div className="max-w-4xl mx-auto">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-foreground">Daily Digest</h2>
        <p className="text-muted-foreground mt-1">
          {new Date().toLocaleDateString('en-US', {
            weekday: 'long',
            year: 'numeric',
            month: 'long',
            day: 'numeric',
          })}
        </p>
      </div>

      <div className="bg-card border border-border rounded-xl p-12 text-center">
        <div className="mx-auto w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
          <Calendar className="h-8 w-8 text-muted-foreground" />
        </div>
        <h3 className="text-lg font-semibold text-foreground mb-2">
          No digest available yet
        </h3>
        <p className="text-muted-foreground max-w-sm mx-auto mb-6">
          Connect your Slack and Atlassian accounts to start receiving AI-powered
          summaries of your work.
        </p>
        <button
          onClick={() => setView('settings')}
          className="px-4 py-2 bg-primary-500 text-white rounded-lg hover:bg-primary-600 transition-colors font-medium"
        >
          Connect Accounts
        </button>
      </div>

      <div className="mt-8 grid grid-cols-2 gap-4">
        {['Sales', 'Marketing', 'Product', 'Engineering'].map((category) => (
          <div
            key={category}
            className="bg-card border border-border rounded-lg p-4"
          >
            <h4 className="font-medium text-foreground mb-1">{category}</h4>
            <p className="text-sm text-muted-foreground">No items yet</p>
          </div>
        ))}
      </div>
    </div>
  )
}
