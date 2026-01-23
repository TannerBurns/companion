import { create } from 'zustand'

export type View = 'daily-digest' | 'weekly-summary' | 'settings'
export type SettingsSection = 'sources' | 'api-keys' | 'notifications' | 'sync' | 'appearance' | 'data' | 'status' | 'about'

export interface SlackState {
  connected: boolean
  teamId: string | null
  teamName: string | null
  userId: string | null
  selectedChannelCount: number
}

export type LocalActivityStatus = 'running' | 'completed' | 'failed'

export interface LocalActivity {
  id: string
  type: 'pdf_export' | 'markdown_export'
  message: string
  status: LocalActivityStatus
  startedAt: number
  completedAt: number | null
  error: string | null
}

export interface AppStore {
  currentView: View
  settingsSection: SettingsSection
  setView: (view: View) => void
  setSettingsSection: (section: SettingsSection) => void
  
  // Slack state
  slack: SlackState
  showChannelSelector: boolean
  setSlackState: (state: Partial<SlackState>) => void
  setShowChannelSelector: (show: boolean) => void
  resetSlackState: () => void

  // Local activities (client-side tasks like PDF export)
  localActivities: LocalActivity[]
  hasUnseenActivity: boolean
  addLocalActivity: (activity: Omit<LocalActivity, 'id' | 'startedAt' | 'completedAt' | 'error'> & { status: 'running' }) => string
  updateLocalActivity: (id: string, updates: Partial<Pick<LocalActivity, 'status' | 'message' | 'error'>>) => void
  clearOldLocalActivities: () => void
  markActivitySeen: () => void
}

const initialSlackState: SlackState = {
  connected: false,
  teamId: null,
  teamName: null,
  userId: null,
  selectedChannelCount: 0,
}

let activityIdCounter = 0

export const useAppStore = create<AppStore>((set) => ({
  currentView: 'daily-digest',
  settingsSection: 'sources',
  setView: (view) => set({ currentView: view }),
  setSettingsSection: (section) => set({ settingsSection: section }),
  
  // Slack state
  slack: initialSlackState,
  showChannelSelector: false,
  setSlackState: (state) => set((prev) => ({ 
    slack: { ...prev.slack, ...state } 
  })),
  setShowChannelSelector: (show) => set({ showChannelSelector: show }),
  resetSlackState: () => set({ 
    slack: initialSlackState, 
    showChannelSelector: false 
  }),

  // Local activities
  localActivities: [],
  hasUnseenActivity: false,
  addLocalActivity: (activity) => {
    const id = `local-${++activityIdCounter}-${Date.now()}`
    set((prev) => ({
      localActivities: [
        {
          ...activity,
          id,
          startedAt: Date.now(),
          completedAt: null,
          error: null,
        },
        ...prev.localActivities,
      ],
      hasUnseenActivity: true,
    }))
    return id
  },
  updateLocalActivity: (id, updates) => {
    set((prev) => ({
      localActivities: prev.localActivities.map((activity) =>
        activity.id === id
          ? {
              ...activity,
              ...updates,
              completedAt: updates.status === 'completed' || updates.status === 'failed'
                ? Date.now()
                : activity.completedAt,
            }
          : activity
      ),
      hasUnseenActivity: true,
    }))
  },
  clearOldLocalActivities: () => {
    const fiveMinutesAgo = Date.now() - 5 * 60 * 1000
    set((prev) => ({
      localActivities: prev.localActivities.filter(
        (activity) =>
          activity.status === 'running' ||
          (activity.completedAt && activity.completedAt > fiveMinutesAgo)
      ),
    }))
  },
  markActivitySeen: () => set({ hasUnseenActivity: false }),
}))
