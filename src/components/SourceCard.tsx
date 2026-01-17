import { clsx } from 'clsx'
import { CheckCircle2 } from 'lucide-react'

interface SourceCardProps {
  icon: React.ComponentType<{ className?: string }>
  name: string
  description: string
  connected: boolean
  onConnect: () => void
  onDisconnect: () => void
}

export function SourceCard({
  icon: Icon,
  name,
  description,
  connected,
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
              connected ? 'bg-green-50 text-green-600' : 'bg-muted text-muted-foreground'
            )}
          >
            <Icon className="h-5 w-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="font-medium text-foreground">{name}</h4>
              {connected && (
                <span className="inline-flex items-center gap-1 text-xs text-green-600 bg-green-50 px-2 py-0.5 rounded-full">
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
          className={clsx(
            'px-3 py-1.5 text-sm font-medium rounded-lg transition-colors',
            connected
              ? 'text-red-600 hover:bg-red-50'
              : 'bg-primary-500 text-white hover:bg-primary-600'
          )}
        >
          {connected ? 'Disconnect' : 'Connect'}
        </button>
      </div>
    </div>
  )
}
