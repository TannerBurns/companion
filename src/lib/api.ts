import { invoke } from '@tauri-apps/api/core'

export interface DigestItem {
  id: string
  title: string
  summary: string
  highlights?: string[]
  category: string
  categoryConfidence?: number
  source: 'slack' | 'jira' | 'confluence'
  sourceUrl?: string
  importanceScore: number
  createdAt: number
}

export interface CategorySummary {
  name: string
  count: number
  topItems: DigestItem[]
}

export interface DigestResponse {
  date: string
  items: DigestItem[]
  categories: CategorySummary[]
}

export interface SyncStatus {
  isSyncing: boolean
  lastSyncAt?: number
  sources: SourceStatus[]
}

export interface SourceStatus {
  name: string
  status: string
  itemsSynced: number
  lastError?: string
}

export interface Preferences {
  syncIntervalMinutes: number
  enabledSources: string[]
  enabledCategories: string[]
  notificationsEnabled: boolean
}

export const api = {
  getDailyDigest: (date?: string) =>
    invoke<DigestResponse>('get_daily_digest', { date }),

  getWeeklyDigest: (weekStart?: string) =>
    invoke<DigestResponse>('get_weekly_digest', { weekStart }),

  startSync: (sources?: string[]) =>
    invoke<void>('start_sync', { sources }),

  getSyncStatus: () =>
    invoke<SyncStatus>('get_sync_status'),

  getPreferences: () =>
    invoke<Preferences>('get_preferences'),

  savePreferences: (preferences: Preferences) =>
    invoke<void>('save_preferences', { preferences }),

  saveApiKey: (service: string, apiKey: string) =>
    invoke<void>('save_api_key', { service, apiKey }),
}
