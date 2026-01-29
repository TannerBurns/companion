import { clsx } from 'clsx'
import {
  ChevronLeft,
  Link2,
  MessageSquare,
  Bell,
  Clock,
  Palette,
  Database,
  Info,
  Activity,
} from 'lucide-react'
import { useAppStore } from '../store'
import type { SettingsSection } from '../store'
import { GeminiSettings, GoogleIcon } from './GeminiSettings'
import {
  SourcesSection,
  GuidanceSection,
  NotificationsSection,
  SyncSection,
  AppearanceSection,
  DataSection,
  StatusSection,
  AboutSection,
} from './settings'

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

const SETTINGS_SECTIONS: Array<{
  id: SettingsSection
  icon: React.ComponentType<{ className?: string }>
  label: string
}> = [
  { id: 'sources', icon: Link2, label: 'Sources' },
  { id: 'api-keys', icon: GoogleIcon, label: 'Gemini' },
  { id: 'guidance', icon: MessageSquare, label: 'AI Guidance' },
  { id: 'notifications', icon: Bell, label: 'Notifications' },
  { id: 'sync', icon: Clock, label: 'Sync' },
  { id: 'appearance', icon: Palette, label: 'Appearance' },
  { id: 'data', icon: Database, label: 'Data' },
  { id: 'status', icon: Activity, label: 'Status' },
  { id: 'about', icon: Info, label: 'About' },
]

export function SettingsView() {
  const { setView, settingsSection, setSettingsSection } = useAppStore()

  const renderSection = () => {
    switch (settingsSection) {
      case 'sources':
        return <SourcesSection />
      case 'api-keys':
        return <GeminiSettings />
      case 'guidance':
        return <GuidanceSection />
      case 'notifications':
        return <NotificationsSection />
      case 'sync':
        return <SyncSection />
      case 'appearance':
        return <AppearanceSection />
      case 'data':
        return <DataSection />
      case 'status':
        return <StatusSection />
      case 'about':
        return <AboutSection />
      default:
        return <SourcesSection />
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
