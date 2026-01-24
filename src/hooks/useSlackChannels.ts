import { useState, useEffect, useMemo, useCallback } from 'react'
import { api } from '../lib/api'
import type { SlackChannel, SlackChannelSelection, SlackUser } from '../lib/api'
import {
  type ChannelType,
  filterChannels,
  groupChannelsByType,
} from '../lib/slack'

interface UseSlackChannelsOptions {
  enabled?: boolean
  teamId?: string
}

interface UseSlackChannelsResult {
  channels: SlackChannel[]
  userMap: Map<string, SlackUser>
  selectedIds: Set<string>
  isLoading: boolean
  isSaving: boolean
  error: string | null
  filteredChannels: SlackChannel[]
  groupedChannels: Record<ChannelType, SlackChannel[]>
  searchQuery: string
  setSearchQuery: (query: string) => void
  toggleChannel: (channelId: string) => void
  selectAllInSection: (type: ChannelType) => void
  deselectAllInSection: (type: ChannelType) => void
  selectAll: () => void
  deselectAll: () => void
  loadChannels: () => Promise<void>
  saveSelection: () => Promise<SlackChannelSelection[]>
}

export function useSlackChannels({
  enabled = true,
  teamId = '',
}: UseSlackChannelsOptions = {}): UseSlackChannelsResult {
  const [channels, setChannels] = useState<SlackChannel[]>([])
  const [userMap, setUserMap] = useState<Map<string, SlackUser>>(new Map())
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [searchQuery, setSearchQuery] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadChannels = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const [allChannels, savedChannels, users] = await Promise.all([
        api.listSlackChannels(),
        api.getSavedSlackChannels(),
        api.listSlackUsers().catch(() => [] as SlackUser[]),
      ])
      
      setChannels(allChannels)
      
      const map = new Map<string, SlackUser>()
      users.forEach(u => map.set(u.id, u))
      setUserMap(map)
      
      const savedIds = new Set(savedChannels.map(c => c.channelId))
      setSelectedIds(savedIds)
    } catch (e) {
      console.error('[useSlackChannels] Error loading channels:', e)
      const errorMsg = e instanceof Error ? e.message : String(e)
      
      if (errorMsg.includes('missing_scope')) {
        setError('Missing required Slack permissions. Please ensure your Slack app has these User Token Scopes: channels:read, groups:read, im:read, mpim:read, users:read. After adding scopes, reinstall the app and get a new token.')
      } else {
        setError(errorMsg || 'Failed to load channels')
      }
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    if (enabled) {
      loadChannels()
    }
  }, [enabled, loadChannels])

  const filteredChannels = useMemo(() => {
    return filterChannels(channels, searchQuery, userMap)
  }, [channels, searchQuery, userMap])

  const groupedChannels = useMemo(() => {
    return groupChannelsByType(filteredChannels)
  }, [filteredChannels])

  const toggleChannel = useCallback((channelId: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev)
      if (next.has(channelId)) {
        next.delete(channelId)
      } else {
        next.add(channelId)
      }
      return next
    })
  }, [])

  const selectAllInSection = useCallback((type: ChannelType) => {
    const typeChannels = groupedChannels[type]
    setSelectedIds(prev => {
      const next = new Set(prev)
      typeChannels.forEach(c => next.add(c.id))
      return next
    })
  }, [groupedChannels])

  const deselectAllInSection = useCallback((type: ChannelType) => {
    const typeChannels = groupedChannels[type]
    setSelectedIds(prev => {
      const next = new Set(prev)
      typeChannels.forEach(c => next.delete(c.id))
      return next
    })
  }, [groupedChannels])

  const selectAll = useCallback(() => {
    setSelectedIds(new Set(filteredChannels.map(c => c.id)))
  }, [filteredChannels])

  const deselectAll = useCallback(() => {
    setSelectedIds(new Set())
  }, [])

  const saveSelection = useCallback(async (): Promise<SlackChannelSelection[]> => {
    setIsSaving(true)
    setError(null)
    try {
      const selections: SlackChannelSelection[] = channels
        .filter(c => selectedIds.has(c.id))
        .map(c => ({
          channelId: c.id,
          channelName: c.name,
          isPrivate: c.isPrivate,
          isIm: c.isIm,
          isMpim: c.isMpim,
          teamId,
          memberCount: c.memberCount,
          purpose: c.purpose,
          enabled: true,
        }))

      await api.saveSlackChannels(selections)
      return selections
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to save channels'
      setError(errorMsg)
      throw e
    } finally {
      setIsSaving(false)
    }
  }, [channels, selectedIds, teamId])

  return {
    channels,
    userMap,
    selectedIds,
    isLoading,
    isSaving,
    error,
    filteredChannels,
    groupedChannels,
    searchQuery,
    setSearchQuery,
    toggleChannel,
    selectAllInSection,
    deselectAllInSection,
    selectAll,
    deselectAll,
    loadChannels,
    saveSelection,
  }
}
