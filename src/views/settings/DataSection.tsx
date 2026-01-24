import { useState, useEffect, useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { clsx } from 'clsx'
import {
  Database,
  Trash2,
  AlertTriangle,
  RefreshCw,
} from 'lucide-react'
import { useAppStore } from '../../store'
import { Button } from '../../components/ui/Button'
import { api } from '../../lib/api'

interface DataStats {
  contentItems: number
  aiSummaries: number
  slackUsers: number
  syncStates: number
}

export function DataSection() {
  const { resetSlackState } = useAppStore()
  const queryClient = useQueryClient()
  const [stats, setStats] = useState<DataStats | null>(null)
  const [isClearing, setIsClearing] = useState(false)
  const [isResetting, setIsResetting] = useState(false)
  const [showResetConfirm, setShowResetConfirm] = useState(false)
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)

  const loadStats = useCallback(async () => {
    try {
      const data = await api.getDataStats()
      setStats(data)
    } catch (e) {
      console.error('Failed to load data stats:', e)
    }
  }, [])

  useEffect(() => {
    loadStats()
  }, [loadStats])

  const invalidateDigestQueries = () => {
    // Invalidate all cached digest queries so views show fresh (empty) data
    queryClient.invalidateQueries({ queryKey: ['daily-digest'] })
    queryClient.invalidateQueries({ queryKey: ['weekly-digest'] })
    queryClient.invalidateQueries({ queryKey: ['sync-status'] })
  }

  const handleClearData = async () => {
    setIsClearing(true)
    setMessage(null)
    try {
      const result = await api.clearSyncedData()
      setMessage({ type: 'success', text: `Cleared ${result.itemsDeleted} items. You can now sync fresh data.` })
      loadStats()
      invalidateDigestQueries()
    } catch (e) {
      setMessage({ type: 'error', text: e instanceof Error ? e.message : 'Failed to clear data' })
    } finally {
      setIsClearing(false)
    }
  }

  const handleFactoryReset = async () => {
    setIsResetting(true)
    setMessage(null)
    try {
      await api.factoryReset()
      setMessage({ type: 'success', text: 'Factory reset complete. All data and settings have been cleared.' })
      resetSlackState()
      loadStats()
      invalidateDigestQueries()
      setShowResetConfirm(false)
    } catch (e) {
      setMessage({ type: 'error', text: e instanceof Error ? e.message : 'Failed to reset' })
    } finally {
      setIsResetting(false)
    }
  }

  const totalItems = stats ? stats.contentItems + stats.aiSummaries : 0

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Data Management</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Manage your synced data and reset options.
        </p>
      </div>

      {message && (
        <div className={clsx(
          'mb-4 p-3 rounded-lg text-sm',
          message.type === 'success' 
            ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 text-green-600 dark:text-green-400'
            : 'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400'
        )}>
          {message.text}
        </div>
      )}

      <div className="space-y-4">
        {/* Data Statistics */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3 mb-4">
            <Database className="h-5 w-5 text-muted-foreground" />
            <div>
              <h4 className="font-medium text-foreground">Storage</h4>
              <p className="text-sm text-muted-foreground">
                {stats ? `${totalItems.toLocaleString()} items stored locally` : 'Loading...'}
              </p>
            </div>
          </div>
          
          {stats && (
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div className="p-2 bg-muted/50 rounded">
                <div className="text-muted-foreground">Messages</div>
                <div className="font-medium text-foreground">{stats.contentItems.toLocaleString()}</div>
              </div>
              <div className="p-2 bg-muted/50 rounded">
                <div className="text-muted-foreground">AI Summaries</div>
                <div className="font-medium text-foreground">{stats.aiSummaries.toLocaleString()}</div>
              </div>
              <div className="p-2 bg-muted/50 rounded">
                <div className="text-muted-foreground">Cached Users</div>
                <div className="font-medium text-foreground">{stats.slackUsers.toLocaleString()}</div>
              </div>
              <div className="p-2 bg-muted/50 rounded">
                <div className="text-muted-foreground">Sync States</div>
                <div className="font-medium text-foreground">{stats.syncStates.toLocaleString()}</div>
              </div>
            </div>
          )}
        </div>

        {/* Clear Synced Data */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Trash2 className="h-5 w-5 text-muted-foreground" />
              <div>
                <h4 className="font-medium text-foreground">Clear Synced Data</h4>
                <p className="text-sm text-muted-foreground">
                  Remove all synced messages and AI summaries. Your connected sources will remain configured.
                </p>
              </div>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={handleClearData}
              disabled={isClearing || totalItems === 0}
            >
              {isClearing ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="h-4 w-4" />
              )}
              Clear Data
            </Button>
          </div>
        </div>

        {/* Factory Reset */}
        <div className="p-4 bg-card border border-red-200 dark:border-red-800 rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <AlertTriangle className="h-5 w-5 text-red-500" />
              <div>
                <h4 className="font-medium text-foreground">Factory Reset</h4>
                <p className="text-sm text-muted-foreground">
                  Remove all data, disconnect all sources, and reset all settings to defaults.
                </p>
              </div>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowResetConfirm(true)}
              className="border-red-300 dark:border-red-700 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
            >
              <AlertTriangle className="h-4 w-4" />
              Reset
            </Button>
          </div>
        </div>
      </div>

      {/* Factory Reset Confirmation Modal */}
      {showResetConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border border-border rounded-xl shadow-xl w-full max-w-md p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-red-100 dark:bg-red-900/30 rounded-full">
                <AlertTriangle className="h-6 w-6 text-red-500" />
              </div>
              <h3 className="text-lg font-semibold text-foreground">
                Confirm Factory Reset
              </h3>
            </div>
            
            <p className="text-sm text-muted-foreground mb-4">
              This will permanently delete:
            </p>
            <ul className="text-sm text-muted-foreground mb-6 space-y-1 list-disc list-inside">
              <li>All synced messages and content</li>
              <li>All AI-generated summaries</li>
              <li>All connected source credentials</li>
              <li>All preferences and settings</li>
            </ul>
            <p className="text-sm font-medium text-red-600 dark:text-red-400 mb-6">
              This action cannot be undone.
            </p>

            <div className="flex justify-end gap-3">
              <Button
                variant="outline"
                onClick={() => setShowResetConfirm(false)}
                disabled={isResetting}
              >
                Cancel
              </Button>
              <Button
                onClick={handleFactoryReset}
                disabled={isResetting}
                className="bg-red-500 hover:bg-red-600 text-white"
              >
                {isResetting ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : (
                  <AlertTriangle className="h-4 w-4" />
                )}
                Reset Everything
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
