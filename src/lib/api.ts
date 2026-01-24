import { invoke } from '@tauri-apps/api/core'

export interface DigestItem {
  id: string
  title: string
  summary: string
  highlights?: string[]
  category: string
  categoryConfidence?: number
  source: 'slack' | 'confluence' | 'ai'
  sourceUrl?: string
  importanceScore: number
  createdAt: number
  channels?: string[]
  people?: string[]
  messageCount?: number
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
  nextSyncAt?: number
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

export interface SlackChannel {
  id: string
  name: string
  isPrivate: boolean
  isIm: boolean
  isMpim: boolean
  /** For DMs, this is the user ID of the other person */
  user?: string
  memberCount?: number
  purpose?: string
  topic?: string
}

export interface SlackChannelSelection {
  channelId: string
  channelName: string
  isPrivate: boolean
  isIm: boolean
  isMpim: boolean
  teamId: string
  memberCount?: number
  purpose?: string
  enabled: boolean
}

export interface SlackUser {
  id: string
  name: string
  realName?: string
  displayName?: string
}

export interface SlackConnectionStatus {
  connected: boolean
  teamId?: string
  teamName?: string
  userId?: string
  selectedChannelCount: number
}

export interface SlackTokens {
  accessToken: string
  tokenType: string
  scope: string
  teamId: string
  teamName: string
  userId: string
}

export const api = {
  getDailyDigest: (date?: string, timezoneOffset?: number) =>
    invoke<DigestResponse>('get_daily_digest', { date, timezoneOffset }),

  getWeeklyDigest: (weekStart?: string, timezoneOffset?: number) =>
    invoke<DigestResponse>('get_weekly_digest', { weekStart, timezoneOffset }),

  startSync: (sources?: string[], timezoneOffset?: number) =>
    invoke<{ itemsSynced: number; channelsProcessed: number; errors: string[] }>('start_sync', { sources, timezoneOffset }),

  getSyncStatus: () =>
    invoke<SyncStatus>('get_sync_status'),

  getPreferences: () =>
    invoke<Preferences>('get_preferences'),

  savePreferences: (preferences: Preferences) =>
    invoke<void>('save_preferences', { preferences }),

  saveApiKey: (service: string, apiKey: string) =>
    invoke<void>('save_api_key', { service, apiKey }),

  hasApiKey: (service: string) =>
    invoke<boolean>('has_api_key', { service }),

  // Slack integration
  connectSlack: (token: string) =>
    invoke<SlackTokens>('connect_slack', { token }),

  disconnectSlack: () =>
    invoke<void>('disconnect_slack'),

  listSlackChannels: () =>
    invoke<SlackChannel[]>('list_slack_channels'),

  listSlackUsers: () =>
    invoke<SlackUser[]>('list_slack_users'),


  saveSlackChannels: (channels: SlackChannelSelection[]) =>
    invoke<void>('save_slack_channels', { channels }),

  getSavedSlackChannels: () =>
    invoke<SlackChannelSelection[]>('get_saved_slack_channels'),

  removeSlackChannel: (channelId: string) =>
    invoke<void>('remove_slack_channel', { channelId }),

  getSlackConnectionStatus: () =>
    invoke<SlackConnectionStatus>('get_slack_connection_status'),

  // Gemini authentication
  saveGeminiCredentials: (jsonContent: string, region?: string) =>
    invoke<void>('save_gemini_credentials', { jsonContent, region }),

  verifyGeminiConnection: () =>
    invoke<void>('verify_gemini_connection'),

  getGeminiAuthType: () =>
    invoke<'api_key' | 'service_account' | 'none'>('get_gemini_auth_type'),

  // Data management
  clearSyncedData: () =>
    invoke<{ itemsDeleted: number }>('clear_synced_data'),

  factoryReset: () =>
    invoke<void>('factory_reset'),

  getDataStats: () =>
    invoke<{ contentItems: number; aiSummaries: number; slackUsers: number; syncStates: number }>('get_data_stats'),
}
