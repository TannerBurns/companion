import { useState, useEffect, useCallback } from 'react'
import { clsx } from 'clsx'
import {
  ChevronLeft,
  Slack,
  FileText,
  Link2,
  Bell,
  Clock,
  Palette,
  Key,
  Save,
  RefreshCw,
  Sun,
  Moon,
  Monitor,
  Check,
  Settings,
} from 'lucide-react'
import { useAppStore } from '../store'
import type { SettingsSection } from '../store'
import { SourceCard, SlackChannelSelector } from '../components'
import { Button } from '../components/ui/Button'
import { Input } from '../components/ui/Input'
import { useTheme } from '../lib/useTheme'
import { usePreferences, useApiKey } from '../hooks/usePreferences'
import { api } from '../lib/api'

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
      id: 'jira',
      icon: FileText,
      name: 'Jira',
      description: 'Sync issues and projects from Atlassian Jira',
      connected: false,
      isConnecting: false,
    },
    {
      id: 'confluence',
      icon: FileText,
      name: 'Confluence',
      description: 'Sync pages and spaces from Atlassian Confluence',
      connected: false,
      isConnecting: false,
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
          <div key={source.id}>
            <SourceCard
              icon={source.icon}
              name={source.name}
              description={source.description}
              connected={source.connected}
              isConnecting={source.isConnecting}
              onConnect={() => handleConnect(source.id)}
              onDisconnect={() => handleDisconnect(source.id)}
            />
            {/* Slack-specific configure channels button */}
            {source.id === 'slack' && slack.connected && (
              <div className="ml-13 mt-2 flex items-center gap-2">
                <button
                  onClick={() => setShowChannelSelector(true)}
                  className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-primary-500 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors"
                >
                  <Settings className="h-4 w-4" />
                  Configure Channels
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Error display */}
      {error && (
        <div className="mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
          {error}
        </div>
      )}

      <div className="mt-6 p-4 bg-muted/50 rounded-lg">
        <div className="flex items-start gap-3">
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
      </div>

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

function ApiKeysSettings() {
  const [geminiKey, setGeminiKey] = useState('')
  const { hasKey, isLoading, saveApiKey, isSaving, isSuccess } = useApiKey('gemini')

  const handleSaveGemini = () => {
    if (geminiKey.trim()) {
      saveApiKey(geminiKey.trim())
      setGeminiKey('')
    }
  }

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">API Keys</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Configure API keys for external services.
        </p>
      </div>

      <div className="space-y-6">
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Key className="h-4 w-4 text-muted-foreground" />
              <h4 className="font-medium text-foreground">Gemini API Key</h4>
            </div>
            {!isLoading && (
              <div className="flex items-center gap-1.5">
                {hasKey ? (
                  <>
                    <div className="h-2 w-2 rounded-full bg-green-500" />
                    <span className="text-sm text-green-600 dark:text-green-400">Configured</span>
                  </>
                ) : (
                  <>
                    <div className="h-2 w-2 rounded-full bg-yellow-500" />
                    <span className="text-sm text-yellow-600 dark:text-yellow-400">Not configured</span>
                  </>
                )}
              </div>
            )}
          </div>
          <p className="text-sm text-muted-foreground mb-3">
            Required for AI-powered summarization and categorization.
          </p>
          <div className="flex gap-2">
            <Input
              type="password"
              value={geminiKey}
              onChange={e => setGeminiKey(e.target.value)}
              placeholder={hasKey ? "Enter new key to replace existing" : "Enter your Gemini API key"}
              className="flex-1"
            />
            <Button
              onClick={handleSaveGemini}
              disabled={isSaving || !geminiKey.trim()}
            >
              {isSaving ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : isSuccess ? (
                <Check className="h-4 w-4" />
              ) : (
                <Save className="h-4 w-4" />
              )}
              {hasKey ? 'Update' : 'Save'}
            </Button>
          </div>
          {isSuccess && (
            <p className="mt-2 text-sm text-green-600 dark:text-green-400">
              API key saved successfully!
            </p>
          )}
        </div>

        <div className="p-4 bg-muted/50 rounded-lg">
          <p className="text-sm text-muted-foreground">
            Get your Gemini API key from the{' '}
            <a
              href="https://aistudio.google.com/app/apikey"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary-500 hover:underline"
            >
              Google AI Studio
            </a>
          </p>
        </div>
      </div>
    </div>
  )
}

function NotificationsSettings() {
  const { preferences, save, isSaving } = usePreferences()

  const handleToggle = () => {
    save({ ...preferences, notificationsEnabled: !preferences.notificationsEnabled })
  }

  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Notifications</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Configure when and how you receive notifications.
        </p>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3">
            <Bell className="h-5 w-5 text-muted-foreground" />
            <div>
              <h4 className="font-medium text-foreground">
                Daily Digest Notification
              </h4>
              <p className="text-sm text-muted-foreground">
                Get notified when your daily digest is ready
              </p>
            </div>
          </div>
          <button
            onClick={handleToggle}
            disabled={isSaving}
            className={clsx(
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
              preferences.notificationsEnabled ? 'bg-primary-500' : 'bg-gray-300 dark:bg-gray-600'
            )}
          >
            <span
              className={clsx(
                'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                preferences.notificationsEnabled ? 'translate-x-6' : 'translate-x-1'
              )}
            />
          </button>
        </div>
      </div>
    </div>
  )
}

function SyncSettings() {
  const { preferences, save, isSaving } = usePreferences()

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
            {['slack', 'jira', 'confluence'].map(source => (
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

const SETTINGS_SECTIONS: Array<{
  id: SettingsSection
  icon: React.ComponentType<{ className?: string }>
  label: string
}> = [
  { id: 'sources', icon: Link2, label: 'Sources' },
  { id: 'api-keys', icon: Key, label: 'API Keys' },
  { id: 'notifications', icon: Bell, label: 'Notifications' },
  { id: 'sync', icon: Clock, label: 'Sync' },
  { id: 'appearance', icon: Palette, label: 'Appearance' },
]

export function SettingsView() {
  const { setView, settingsSection, setSettingsSection } = useAppStore()

  const renderSection = () => {
    switch (settingsSection) {
      case 'sources':
        return <SourcesSettings />
      case 'api-keys':
        return <ApiKeysSettings />
      case 'notifications':
        return <NotificationsSettings />
      case 'sync':
        return <SyncSettings />
      case 'appearance':
        return <AppearanceSettings />
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
