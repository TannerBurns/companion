import { create } from 'zustand'

export type View = 'daily-digest' | 'weekly-summary' | 'settings'
export type SettingsSection = 'sources' | 'api-keys' | 'notifications' | 'sync' | 'appearance'

export interface AppStore {
  currentView: View
  settingsSection: SettingsSection
  setView: (view: View) => void
  setSettingsSection: (section: SettingsSection) => void
}

export const useAppStore = create<AppStore>((set) => ({
  currentView: 'daily-digest',
  settingsSection: 'sources',
  setView: (view) => set({ currentView: view }),
  setSettingsSection: (section) => set({ settingsSection: section }),
}))
