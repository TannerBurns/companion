import { useState, useEffect, useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { getVersion, getName } from '@tauri-apps/api/app'
import { clsx } from 'clsx'
import {
  ChevronLeft,
  Slack,
  FileText,
  Link2,
  Bell,
  Clock,
  Palette,
  RefreshCw,
  Sun,
  Moon,
  Monitor,
  Settings,
  Database,
  Trash2,
  AlertTriangle,
  Info,
  Download,
  CheckCircle,
} from 'lucide-react'
import { useAppStore } from '../store'
import type { SettingsSection } from '../store'
import { SourceCard, SlackChannelSelector } from '../components'
import { Button } from '../components/ui/Button'
import { Input } from '../components/ui/Input'
import { useTheme } from '../lib/useTheme'
import { usePreferences } from '../hooks/usePreferences'
import { useSync } from '../hooks/useDigest'
import { useUpdater } from '../hooks/useUpdater'
import { api } from '../lib/api'
import { formatRelativeTime } from '../lib/formatRelativeTime'
import { GeminiSettings, GoogleIcon } from './GeminiSettings'

interface SettingsNavItemProps {
  icon: React.ComponentType<{ className?: string }>
  label: string
  active: boolean
  onClick: () => void
}

function SettingsNavItem({ icon: Icon, label, active, onClick }: SettingsNavItemProps) {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'w-full flex items-center gap-3 px-3 py-2 rounded-lg transition-colors text-left text-sm',
        active
          ? 'bg-muted text-foreground font-medium'
          : 'text-muted-foreground hover:bg-muted/50'
      )}
    >
      <Icon className="h-4 w-4" />
      {label}
    </button>
  )
}

function SourcesSettings() {
  const { slack, setSlackState, showChannelSelector, setShowChannelSelector } = useAppStore()
  const [isConnecting, setIsConnecting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [showSlackSetup, setShowSlackSetup] = useState(false)
  const [slackToken, setSlackToken] = useState('')

  const loadSlackStatus = useCallback(async () => {
    try {
      const status = await api.getSlackConnectionStatus()
      setSlackState({
        connected: status.connected,
        teamId: status.teamId ?? null,
        teamName: status.teamName ?? null,
        userId: status.userId ?? null,
        selectedChannelCount: status.selectedChannelCount,
      })
    } catch (e) {
      console.error('Failed to load Slack status:', e)
    }
  }, [setSlackState])

  useEffect(() => {
    loadSlackStatus()
  }, [loadSlackStatus])

  const handleConnectSlack = async () => {
    if (!slackToken.trim()) {
      setError('Please enter your Slack token')
      return
    }

    setIsConnecting(true)
    setError(null)
    try {
      const tokens = await api.connectSlack(slackToken.trim())
      setSlackState({
        connected: true,
        teamId: tokens.teamId,
        teamName: tokens.teamName,
        userId: tokens.userId,
        selectedChannelCount: 0,
      })
      setShowSlackSetup(false)
      setSlackToken('')
      // Show channel selector after successful connection
      setShowChannelSelector(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to connect to Slack')
    } finally {
      setIsConnecting(false)
    }
  }

  const handleDisconnectSlack = async () => {
    try {
      await api.disconnectSlack()
      setSlackState({
        connected: false,
        teamId: null,
        teamName: null,
        userId: null,
        selectedChannelCount: 0,
      })
      setError(null)
    } catch (e) {
      console.error('Failed to disconnect Slack:', e)
    }
  }

  const sources = [
    {
      id: 'slack',
      icon: Slack,
      name: 'Slack',
      description: slack.connected && slack.teamName
        ? `Connected to ${slack.teamName} • ${slack.selectedChannelCount} channels`
        : 'Sync messages and channels from your Slack workspace',
      connected: slack.connected,
      isConnecting: isConnecting,
    },
    {
      id: 'confluence',
      icon: FileText,
      name: 'Confluence',
      description: 'Sync pages and spaces from Atlassian Confluence',
      connected: false,
      isConnecting: false,
      comingSoon: true,
    },
  ]

  const handleConnect = (sourceId: string) => {
    if (sourceId === 'slack') {
      setShowSlackSetup(true)
    }
  }

  const handleDisconnect = (sourceId: string) => {
    if (sourceId === 'slack') {
      handleDisconnectSlack()
    }
  }

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Connected Sources</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Connect your accounts to sync data and generate AI-powered summaries.
        </p>
      </div>

      <div className="space-y-3">
        {sources.map((source) => (
          <SourceCard
            key={source.id}
            icon={source.icon}
            name={source.name}
            description={source.description}
            connected={source.connected}
            isConnecting={source.isConnecting}
            comingSoon={source.comingSoon}
            onConnect={() => handleConnect(source.id)}
            onDisconnect={() => handleDisconnect(source.id)}
          >
            {/* Slack-specific content */}
            {source.id === 'slack' && slack.connected && (
              <button
                onClick={() => setShowChannelSelector(true)}
                className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-primary-500 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors"
              >
                <Settings className="h-4 w-4" />
                Configure Channels
              </button>
            )}
            {source.id === 'slack' && !slack.connected && (
              <div className="flex items-start gap-3 p-3 bg-muted/50 rounded-lg">
                <Link2 className="h-5 w-5 text-muted-foreground mt-0.5" />
                <div>
                  <h4 className="text-sm font-medium text-foreground">
                    How It Works
                  </h4>
                  <p className="text-sm text-muted-foreground mt-1">
                    Create a Slack app for your workspace, install it, and paste the
                    User OAuth Token. Your token is stored securely on your device.
                  </p>
                </div>
              </div>
            )}
          </SourceCard>
        ))}
      </div>

      {/* Error display */}
      {error && (
        <div className="mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
          {error}
        </div>
      )}

      {/* Slack Token Setup Modal */}
      {showSlackSetup && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border border-border rounded-xl shadow-xl w-full max-w-lg p-6">
            <h3 className="text-lg font-semibold text-foreground mb-2">
              Connect Slack
            </h3>
            <p className="text-sm text-muted-foreground mb-4">
              To connect Slack, create a Slack app and paste your User OAuth Token.
            </p>
            
            <div className="p-4 bg-muted/50 rounded-lg mb-4">
              <h4 className="text-sm font-medium text-foreground mb-2">Setup Steps:</h4>
              <ol className="text-sm text-muted-foreground space-y-2 list-decimal list-inside">
                <li>
                  Go to{' '}
                  <a
                    href="https://api.slack.com/apps"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-primary-500 hover:underline"
                  >
                    api.slack.com/apps
                  </a>
                  {' '}→ Create New App → From scratch
                </li>
                <li>Name it anything (e.g., "Companion"), select your workspace</li>
                <li>Go to <strong>OAuth & Permissions</strong>, add these <strong>User Token Scopes</strong>:
                  <code className="block mt-1 p-2 bg-background rounded text-xs">
                    channels:history, channels:read, groups:history, groups:read, im:history, im:read, mpim:history, mpim:read, users:read
                  </code>
                </li>
                <li>Click <strong>Install to Workspace</strong> and authorize</li>
                <li>Copy the <strong>User OAuth Token</strong> (starts with <code className="text-xs">xoxp-</code>)</li>
              </ol>
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
                {error}
              </div>
            )}

            <div>
              <label className="block text-sm font-medium text-foreground mb-1">
                User OAuth Token
              </label>
              <Input
                type="password"
                value={slackToken}
                onChange={(e) => setSlackToken(e.target.value)}
                placeholder="xoxp-..."
              />
            </div>

            <div className="flex justify-end gap-3 mt-6">
              <Button
                variant="outline"
                onClick={() => {
                  setShowSlackSetup(false)
                  setSlackToken('')
                  setError(null)
                }}
              >
                Cancel
              </Button>
              <Button
                onClick={handleConnectSlack}
                disabled={isConnecting || !slackToken.trim()}
              >
                {isConnecting ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : (
                  <Slack className="h-4 w-4" />
                )}
                Connect
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Channel Selector Modal */}
      <SlackChannelSelector
        isOpen={showChannelSelector}
        onClose={() => setShowChannelSelector(false)}
        teamId={slack.teamId ?? ''}
        onSave={loadSlackStatus}
      />
    </div>
  )
}

function NotificationsSettings() {
  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Notifications</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Configure when and how you receive notifications.
        </p>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between p-4 bg-card border border-border rounded-lg opacity-75">
          <div className="flex items-center gap-3">
            <Bell className="h-5 w-5 text-muted-foreground" />
            <div>
              <div className="flex items-center gap-2">
                <h4 className="font-medium text-foreground">
                  Daily Digest Notification
                </h4>
                <span className="inline-flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 px-2 py-0.5 rounded-full">
                  <Clock className="h-3 w-3" />
                  Coming Soon
                </span>
              </div>
              <p className="text-sm text-muted-foreground">
                Get notified when your daily digest is ready
              </p>
            </div>
          </div>
          <button
            disabled
            className="relative inline-flex h-6 w-11 items-center rounded-full transition-colors bg-gray-300 dark:bg-gray-600 cursor-not-allowed"
          >
            <span className="inline-block h-4 w-4 transform rounded-full bg-white translate-x-1" />
          </button>
        </div>
      </div>
    </div>
  )
}

function SyncSettings() {
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

        <div className="p-4 bg-card border border-border rounded-lg">
          <h4 className="font-medium text-foreground mb-3">Enabled Sources</h4>
          <div className="flex flex-wrap gap-3">
            {['slack', 'confluence'].map(source => (
              <label key={source} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={preferences.enabledSources.includes(source)}
                  onChange={e => {
                    const sources = e.target.checked
                      ? [...preferences.enabledSources, source]
                      : preferences.enabledSources.filter(s => s !== source)
                    save({ ...preferences, enabledSources: sources })
                  }}
                  className="h-4 w-4 rounded border-border text-primary-500 focus:ring-primary-500"
                />
                <span className="text-sm capitalize text-foreground">{source}</span>
              </label>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

function AppearanceSettings() {
  const { theme, setTheme } = useTheme()

  const themeOptions = [
    { value: 'light' as const, label: 'Light', icon: Sun },
    { value: 'dark' as const, label: 'Dark', icon: Moon },
    { value: 'system' as const, label: 'System', icon: Monitor },
  ]

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Appearance</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Customize how Companion looks.
        </p>
      </div>

      <div className="space-y-4">
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3 mb-4">
            <Palette className="h-5 w-5 text-muted-foreground" />
            <div>
              <h4 className="font-medium text-foreground">Theme</h4>
              <p className="text-sm text-muted-foreground">
                Choose between light and dark mode
              </p>
            </div>
          </div>

          <div className="flex gap-2">
            {themeOptions.map(({ value, label, icon: Icon }) => (
              <button
                key={value}
                onClick={() => setTheme(value)}
                className={clsx(
                  'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors',
                  theme === value
                    ? 'bg-primary-500 text-white'
                    : 'bg-muted text-muted-foreground hover:bg-muted/80'
                )}
              >
                <Icon className="h-4 w-4" />
                {label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

function DataSettings() {
  const { resetSlackState } = useAppStore()
  const queryClient = useQueryClient()
  const [stats, setStats] = useState<{ contentItems: number; aiSummaries: number; slackUsers: number; syncStates: number } | null>(null)
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

function AboutSettings() {
  const [appName, setAppName] = useState<string>('')
  const [appVersion, setAppVersion] = useState<string>('')
  const { 
    state, 
    checkForUpdates, 
    downloadAndInstall, 
    handleRestart 
  } = useUpdater()

  useEffect(() => {
    getName().then(setAppName).catch(() => setAppName('Companion'))
    getVersion().then(setAppVersion).catch(() => setAppVersion('Unknown'))
  }, [])

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">About</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Application information and updates.
        </p>
      </div>

      <div className="space-y-4">
        {/* App Info */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
              <Info className="h-5 w-5 text-primary-500" />
            </div>
            <div>
              <h4 className="font-medium text-foreground">{appName || 'Companion'}</h4>
              <p className="text-sm text-muted-foreground">
                Version {appVersion || 'Loading...'}
              </p>
            </div>
          </div>
        </div>

        {/* Check for Updates */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Download className={clsx(
                'h-5 w-5 text-muted-foreground',
                state.status === 'checking' && 'animate-pulse'
              )} />
              <div>
                <h4 className="font-medium text-foreground">Software Updates</h4>
                <p className="text-sm text-muted-foreground">
                  {state.status === 'idle' && 'Check for available updates'}
                  {state.status === 'checking' && 'Checking for updates...'}
                  {state.status === 'no-update' && 'You are running the latest version'}
                  {state.status === 'available' && `Version ${state.update.version} is available`}
                  {state.status === 'downloading' && `Downloading update... ${state.progress}%`}
                  {state.status === 'ready' && 'Update ready to install'}
                  {state.status === 'error' && state.message}
                </p>
              </div>
            </div>
            <div className="flex-shrink-0">
              {(state.status === 'idle' || state.status === 'no-update' || state.status === 'error') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={checkForUpdates}
                >
                  <RefreshCw className="h-4 w-4" />
                  Check for Updates
                </Button>
              )}
              {state.status === 'checking' && (
                <Button
                  variant="outline"
                  size="sm"
                  disabled
                >
                  <RefreshCw className="h-4 w-4 animate-spin" />
                  Checking...
                </Button>
              )}
              {state.status === 'available' && (
                <Button
                  size="sm"
                  onClick={() => downloadAndInstall()}
                >
                  <Download className="h-4 w-4" />
                  Download & Install
                </Button>
              )}
              {state.status === 'downloading' && (
                <Button
                  variant="outline"
                  size="sm"
                  disabled
                >
                  <RefreshCw className="h-4 w-4 animate-spin" />
                  Downloading...
                </Button>
              )}
              {state.status === 'ready' && (
                <Button
                  size="sm"
                  onClick={handleRestart}
                >
                  <RefreshCw className="h-4 w-4" />
                  Restart Now
                </Button>
              )}
            </div>
          </div>

          {/* Download Progress Bar */}
          {state.status === 'downloading' && (
            <div className="mt-4">
              <div className="w-full bg-muted rounded-full h-2">
                <div 
                  className="bg-primary-500 h-2 rounded-full transition-all duration-300"
                  style={{ width: `${state.progress}%` }}
                />
              </div>
            </div>
          )}

          {/* Update Ready */}
          {state.status === 'ready' && (
            <div className="mt-4 flex items-center gap-2 text-green-600 dark:text-green-400">
              <CheckCircle className="h-4 w-4" />
              <p className="text-sm">Update downloaded and ready to install</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

const SETTINGS_SECTIONS: Array<{
  id: SettingsSection
  icon: React.ComponentType<{ className?: string }>
  label: string
}> = [
  { id: 'sources', icon: Link2, label: 'Sources' },
  { id: 'api-keys', icon: GoogleIcon, label: 'Gemini' },
  { id: 'notifications', icon: Bell, label: 'Notifications' },
  { id: 'sync', icon: Clock, label: 'Sync' },
  { id: 'appearance', icon: Palette, label: 'Appearance' },
  { id: 'data', icon: Database, label: 'Data' },
  { id: 'about', icon: Info, label: 'About' },
]

export function SettingsView() {
  const { setView, settingsSection, setSettingsSection } = useAppStore()

  const renderSection = () => {
    switch (settingsSection) {
      case 'sources':
        return <SourcesSettings />
      case 'api-keys':
        return <GeminiSettings />
      case 'notifications':
        return <NotificationsSettings />
      case 'sync':
        return <SyncSettings />
      case 'appearance':
        return <AppearanceSettings />
      case 'data':
        return <DataSettings />
      case 'about':
        return <AboutSettings />
      default:
        return <SourcesSettings />
    }
  }

  return (
    <div className="max-w-4xl mx-auto">
      <button
        onClick={() => setView('daily-digest')}
        className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors mb-6"
      >
        <ChevronLeft className="h-4 w-4" />
        Back to Digest
      </button>

      <div className="mb-6">
        <h2 className="text-2xl font-bold text-foreground">Settings</h2>
        <p className="text-muted-foreground mt-1">
          Manage your connections and preferences
        </p>
      </div>

      <div className="flex gap-8">
        <div className="w-48 flex-shrink-0">
          <nav className="space-y-1">
            {SETTINGS_SECTIONS.map((section) => (
              <SettingsNavItem
                key={section.id}
                icon={section.icon}
                label={section.label}
                active={settingsSection === section.id}
                onClick={() => setSettingsSection(section.id)}
              />
            ))}
          </nav>
        </div>

        <div className="flex-1">{renderSection()}</div>
      </div>
    </div>
  )
}
