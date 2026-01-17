import { clsx } from 'clsx'
import { CheckCircle2, Loader2 } from 'lucide-react'

interface SourceCardProps {
  icon: React.ComponentType<{ className?: string }>
  name: string
  description: string
  connected: boolean
  isConnecting?: boolean
  onConnect: () => void
  onDisconnect: () => void
}

export function SourceCard({
  icon: Icon,
  name,
  description,
  connected,
  isConnecting = false,
  onConnect,
  onDisconnect,
}: SourceCardProps) {
  return (
    <div className="bg-card border border-border rounded-lg p-4">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3">
          <div
            className={clsx(
              'h-10 w-10 rounded-lg flex items-center justify-center',
              connected ? 'bg-green-50 text-green-600 dark:bg-green-900/20 dark:text-green-400' : 'bg-muted text-muted-foreground'
            )}
          >
            <Icon className="h-5 w-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="font-medium text-foreground">{name}</h4>
              {connected && (
                <span className="inline-flex items-center gap-1 text-xs text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-900/20 px-2 py-0.5 rounded-full">
                  <CheckCircle2 className="h-3 w-3" />
                  Connected
                </span>
              )}
            </div>
            <p className="text-sm text-muted-foreground mt-0.5">{description}</p>
          </div>
        </div>
        <button
          onClick={connected ? onDisconnect : onConnect}
          disabled={isConnecting}
          className={clsx(
            'px-3 py-1.5 text-sm font-medium rounded-lg transition-colors inline-flex items-center gap-2',
            connected
              ? 'text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20'
              : 'bg-primary-500 text-white hover:bg-primary-600',
            isConnecting && 'opacity-50 cursor-not-allowed'
          )}
        >
          {isConnecting ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Connecting...
            </>
          ) : connected ? (
            'Disconnect'
          ) : (
            'Connect'
          )}
        </button>
      </div>
    </div>
  )
}
