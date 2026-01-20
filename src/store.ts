import { create } from 'zustand'

export type View = 'daily-digest' | 'weekly-summary' | 'settings'
export type SettingsSection = 'sources' | 'api-keys' | 'notifications' | 'sync' | 'appearance' | 'data'

export interface SlackState {
  connected: boolean
  teamId: string | null
  teamName: string | null
  userId: string | null
  selectedChannelCount: number
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
}

const initialSlackState: SlackState = {
  connected: false,
  teamId: null,
  teamName: null,
  userId: null,
  selectedChannelCount: 0,
}

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
}))
