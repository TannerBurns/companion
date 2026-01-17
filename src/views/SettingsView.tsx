import { clsx } from 'clsx'
import {
  ChevronLeft,
  Slack,
  FileText,
  Link2,
  Circle,
  Bell,
  Clock,
  Palette,
} from 'lucide-react'
import { useAppStore } from '../store'
import type { SettingsSection } from '../store'
import { SourceCard } from '../components'

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
    console.log('Connect:', sourceId)
  }

  const handleDisconnect = (sourceId: string) => {
    console.log('Disconnect:', sourceId)
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
              will be implemented in Phase 2.
            </p>
          </div>
        </div>
      </div>
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
        <div className="flex items-center justify-between p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3">
            <Circle className="h-5 w-5 text-muted-foreground" />
            <div>
              <h4 className="font-medium text-foreground">
                Daily Digest Notification
              </h4>
              <p className="text-sm text-muted-foreground">
                Get notified when your daily digest is ready
              </p>
            </div>
          </div>
          <button className="text-sm text-muted-foreground">Coming soon</button>
        </div>
      </div>
    </div>
  )
}

function SyncSettings() {
  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Sync Settings</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Configure how often data is synced from your connected sources.
        </p>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3">
            <Circle className="h-5 w-5 text-muted-foreground" />
            <div>
              <h4 className="font-medium text-foreground">Sync Interval</h4>
              <p className="text-sm text-muted-foreground">
                How often to sync data from sources
              </p>
            </div>
          </div>
          <button className="text-sm text-muted-foreground">Coming soon</button>
        </div>
      </div>
    </div>
  )
}

function AppearanceSettings() {
  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Appearance</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Customize how Companion looks.
        </p>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-3">
            <Circle className="h-5 w-5 text-muted-foreground" />
            <div>
              <h4 className="font-medium text-foreground">Theme</h4>
              <p className="text-sm text-muted-foreground">
                Choose between light and dark mode
              </p>
            </div>
          </div>
          <button className="text-sm text-muted-foreground">Coming soon</button>
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
