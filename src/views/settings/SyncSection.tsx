import { clsx } from 'clsx'
import { RefreshCw, Clock } from 'lucide-react'
import { Button } from '../../components/ui/Button'
import { usePreferences } from '../../hooks/usePreferences'
import { useSync } from '../../hooks/useDigest'
import { formatRelativeTime } from '../../lib/formatRelativeTime'

export function SyncSection() {
  const { preferences, save, isSaving } = usePreferences()
  const { sync, isSyncing, status } = useSync()

  const handleIntervalChange = (value: number) => {
    save({ ...preferences, syncIntervalMinutes: value })
  }

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Sync Settings</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Configure how often data is synced from your connected sources.
        </p>
      </div>

      <div className="space-y-4">
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <RefreshCw className={clsx('h-5 w-5 text-muted-foreground', isSyncing && 'animate-spin')} />
              <div>
                <h4 className="font-medium text-foreground">Sync Status</h4>
                <p className="text-sm text-muted-foreground">
                  {isSyncing ? (
                    'Syncing now...'
                  ) : (
                    <>
                      Last: {formatRelativeTime(status?.lastSyncAt, false)}
                      {status?.nextSyncAt && (
                        <> · Next: {formatRelativeTime(status.nextSyncAt, true)}</>
                      )}
                    </>
                  )}
                </p>
              </div>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => sync()}
              disabled={isSyncing}
            >
              {isSyncing ? 'Syncing...' : 'Sync Now'}
            </Button>
          </div>
        </div>

        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3 mb-3">
            <Clock className="h-5 w-5 text-muted-foreground" />
            <div>
              <h4 className="font-medium text-foreground">Sync Interval</h4>
              <p className="text-sm text-muted-foreground">
                How often to sync data from sources
              </p>
            </div>
          </div>
          <select
            value={preferences.syncIntervalMinutes}
            onChange={e => handleIntervalChange(Number(e.target.value))}
            disabled={isSaving}
            className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            <option value={5}>Every 5 minutes</option>
            <option value={15}>Every 15 minutes</option>
            <option value={30}>Every 30 minutes</option>
            <option value={60}>Every hour</option>
          </select>
        </div>
      </div>
    </div>
  )
}
