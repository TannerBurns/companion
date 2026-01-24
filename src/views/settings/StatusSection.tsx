import { clsx } from 'clsx'
import {
  CheckCircle,
  Circle,
  AlertTriangle,
  RefreshCw,
} from 'lucide-react'
import { useAppStore } from '../../store'
import { useSetupStatus } from '../../hooks/useSetupStatus'

interface StatusItemProps {
  completed: boolean
  title: string
  description: string
  onClick: () => void
}

function StatusItem({ completed, title, description, onClick }: StatusItemProps) {
  return (
    <button
      onClick={onClick}
      className="w-full flex items-center gap-3 p-3 bg-card border border-border rounded-lg hover:bg-muted/50 transition-colors text-left"
    >
      {completed ? (
        <CheckCircle className="h-5 w-5 text-green-500 flex-shrink-0" />
      ) : (
        <Circle className="h-5 w-5 text-muted-foreground flex-shrink-0" />
      )}
      <div className="flex-1 min-w-0">
        <div className="font-medium text-foreground">{title}</div>
        <div className="text-sm text-muted-foreground">{description}</div>
      </div>
    </button>
  )
}

export function StatusSection() {
  const { setSettingsSection } = useAppStore()
  const { geminiConfigured, hasConnectedSource, isComplete, isLoading } = useSetupStatus()

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Status</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Overview of your Companion setup and configuration.
        </p>
      </div>

      <div className="space-y-4">
        <div
          className={clsx(
            'p-4 border rounded-lg',
            isComplete
              ? 'bg-green-50 dark:bg-green-900/10 border-green-200 dark:border-green-800'
              : 'bg-amber-50 dark:bg-amber-900/10 border-amber-200 dark:border-amber-800'
          )}
        >
          <div className="flex items-center gap-2 mb-1">
            {isComplete ? (
              <CheckCircle className="h-5 w-5 text-green-500" />
            ) : (
              <AlertTriangle className="h-5 w-5 text-amber-500" />
            )}
            <h4
              className={clsx(
                'font-semibold',
                isComplete
                  ? 'text-green-700 dark:text-green-400'
                  : 'text-amber-700 dark:text-amber-400'
              )}
            >
              {isComplete ? 'Setup Complete' : 'Setup Required'}
            </h4>
          </div>
          <p className="text-sm text-muted-foreground ml-7">
            {isComplete
              ? 'All required configuration is complete. Companion is ready to use.'
              : 'Complete the steps below to start using Companion.'}
          </p>
        </div>

        {isLoading ? (
          <div className="p-4 bg-card border border-border rounded-lg">
            <div className="flex items-center gap-2 text-muted-foreground">
              <RefreshCw className="h-4 w-4 animate-spin" />
              <span className="text-sm">Loading status...</span>
            </div>
          </div>
        ) : (
          <div className="space-y-3">
            <StatusItem
              completed={geminiConfigured}
              title="Gemini AI"
              description={geminiConfigured ? 'API configured and ready' : 'Configure your Gemini API key or service account'}
              onClick={() => setSettingsSection('api-keys')}
            />
            <StatusItem
              completed={hasConnectedSource}
              title="Data Source"
              description={hasConnectedSource ? 'At least one source connected' : 'Connect Slack or another data source'}
              onClick={() => setSettingsSection('sources')}
            />
          </div>
        )}
      </div>
    </div>
  )
}
