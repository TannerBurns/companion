import { useState } from 'react'
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
} from 'lucide-react'
import { useAppStore } from '../store'
import type { SettingsSection } from '../store'
import { SourceCard } from '../components'
import { Button } from '../components/ui/Button'
import { Input } from '../components/ui/Input'
import { useTheme } from '../lib/useTheme'
import { usePreferences, useApiKey } from '../hooks/usePreferences'

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
  const sources = [
    {
      id: 'slack',
      icon: Slack,
      name: 'Slack',
      description: 'Sync messages and channels from your Slack workspace',
      connected: false,
    },
    {
      id: 'jira',
      icon: FileText,
      name: 'Jira',
      description: 'Sync issues and projects from Atlassian Jira',
      connected: false,
    },
    {
      id: 'confluence',
      icon: FileText,
      name: 'Confluence',
      description: 'Sync pages and spaces from Atlassian Confluence',
      connected: false,
    },
  ]

  const handleConnect = (sourceId: string) => {
    void sourceId // TODO: Implement OAuth connection flow
  }

  const handleDisconnect = (sourceId: string) => {
    void sourceId // TODO: Implement disconnect flow
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
            onConnect={() => handleConnect(source.id)}
            onDisconnect={() => handleDisconnect(source.id)}
          />
        ))}
      </div>

      <div className="mt-6 p-4 bg-muted/50 rounded-lg">
        <div className="flex items-start gap-3">
          <Link2 className="h-5 w-5 text-muted-foreground mt-0.5" />
          <div>
            <h4 className="text-sm font-medium text-foreground">
              API Key Configuration
            </h4>
            <p className="text-sm text-muted-foreground mt-1">
              You'll need to provide API credentials for each service. OAuth flows
              will guide you through the connection process.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}

function ApiKeysSettings() {
  const [geminiKey, setGeminiKey] = useState('')
  const { saveApiKey, isSaving, isSuccess } = useApiKey()

  const handleSaveGemini = () => {
    if (geminiKey.trim()) {
      saveApiKey('gemini', geminiKey.trim())
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
          <div className="flex items-center gap-2 mb-3">
            <Key className="h-4 w-4 text-muted-foreground" />
            <h4 className="font-medium text-foreground">Gemini API Key</h4>
          </div>
          <p className="text-sm text-muted-foreground mb-3">
            Required for AI-powered summarization and categorization.
          </p>
          <div className="flex gap-2">
            <Input
              type="password"
              value={geminiKey}
              onChange={e => setGeminiKey(e.target.value)}
              placeholder="Enter your Gemini API key"
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
              Save
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
  const notificationsEnabled = preferences?.notificationsEnabled ?? false

  const handleToggle = () => {
    if (preferences) {
      save({ ...preferences, notificationsEnabled: !notificationsEnabled })
    }
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
              notificationsEnabled ? 'bg-primary-500' : 'bg-gray-300 dark:bg-gray-600'
            )}
          >
            <span
              className={clsx(
                'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                notificationsEnabled ? 'translate-x-6' : 'translate-x-1'
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
  const syncInterval = preferences?.syncIntervalMinutes ?? 15

  const handleIntervalChange = (value: number) => {
    if (preferences) {
      save({ ...preferences, syncIntervalMinutes: value })
    }
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
            value={syncInterval}
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
                  checked={preferences?.enabledSources?.includes(source) ?? false}
                  onChange={e => {
                    if (preferences) {
                      const sources = e.target.checked
                        ? [...(preferences.enabledSources || []), source]
                        : preferences.enabledSources?.filter(s => s !== source) || []
                      save({ ...preferences, enabledSources: sources })
                    }
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
