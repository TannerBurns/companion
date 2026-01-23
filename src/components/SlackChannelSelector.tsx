import { useState, useEffect, useMemo } from 'react'
import { clsx } from 'clsx'
import {
  X,
  Search,
  Hash,
  Lock,
  MessageSquare,
  Users,
  CheckSquare,
  Square,
  RefreshCw,
  Save,
  ChevronDown,
  ChevronRight,
} from 'lucide-react'
import { Button } from './ui/Button'
import { Input } from './ui/Input'
import { api } from '../lib/api'
import type { SlackChannel, SlackChannelSelection, SlackUser } from '../lib/api'
import { useAppStore } from '../store'
import { usePreferences } from '../hooks/usePreferences'

interface SlackChannelSelectorProps {
  isOpen: boolean
  onClose: () => void
  teamId: string
  onSave?: () => void
}

type ChannelType = 'public' | 'private' | 'dm' | 'group'

function getChannelType(channel: SlackChannel): ChannelType {
  // Check isMpim first - group DMs might have both isIm and isMpim set
  if (channel.isMpim) return 'group'
  if (channel.isIm) return 'dm'
  if (channel.isPrivate) return 'private'
  return 'public'
}

function getChannelIcon(type: ChannelType) {
  switch (type) {
    case 'public':
      return Hash
    case 'private':
      return Lock
    case 'dm':
      return MessageSquare
    case 'group':
      return Users
  }
}

function getChannelTypeLabel(type: ChannelType) {
  switch (type) {
    case 'public':
      return 'Public Channels'
    case 'private':
      return 'Private Channels'
    case 'dm':
      return 'Direct Messages'
    case 'group':
      return 'Group Messages'
  }
}

function getChannelDisplayName(
  channel: SlackChannel, 
  type: ChannelType, 
  userMap: Map<string, SlackUser>
): string {
  // For DMs, use the 'user' field to look up the other person's name
  if (type === 'dm') {
    // The API returns a 'user' field with the other person's ID
    if (channel.user && userMap.has(channel.user)) {
      const user = userMap.get(channel.user)!
      return user.displayName || user.realName || user.name
    }
    
    // Fallback: show the user ID if we have it
    if (channel.user) {
      return channel.user
    }
    
    // Last resort: use name or show generic
    if (channel.name && channel.name !== channel.id) {
      return channel.name
    }
    return 'Direct Message'
  }
  
  // For group DMs, the name is often in format "mpdm-user1--user2--user3-1"
  // or comma-separated usernames
  if (type === 'group') {
    if (channel.name) {
      let parts: string[] = []
      
      // Check if it's the mpdm format (mpdm-user1--user2--user3-1)
      if (channel.name.startsWith('mpdm-')) {
        // Remove "mpdm-" prefix and trailing number suffix (e.g., "-1")
        let cleaned = channel.name.slice(5) // Remove "mpdm-"
        cleaned = cleaned.replace(/-\d+$/, '') // Remove trailing "-1", "-2", etc.
        parts = cleaned.split('--')
      } else if (channel.name.includes(',')) {
        // Comma-separated format
        parts = channel.name.split(',').map(p => p.trim())
      } else if (channel.name.includes('--')) {
        // Double-dash separated
        parts = channel.name.split('--')
      } else {
        // Single name or unknown format
        parts = [channel.name]
      }
      
      // Try to resolve each part to a real name
      const resolvedParts = parts.map(part => {
        // Check if this is a user ID we can look up
        if (userMap.has(part)) {
          const user = userMap.get(part)!
          return user.displayName || user.realName || user.name
        }
        // Check if any user has this as their username
        for (const [, user] of userMap) {
          if (user.name === part) {
            return user.displayName || user.realName || user.name
          }
        }
        // Clean up the part (remove extra dashes, etc.)
        return part.replace(/-+/g, ' ').trim()
      }).filter(p => p.length > 0)
      
      if (resolvedParts.length > 0) {
        return resolvedParts.join(', ')
      }
    }
    return 'Group Message'
  }
  
  return channel.name || 'Unnamed Channel'
}

// Check if a display name is a resolved name (not a user ID or generic placeholder)
function isResolvedName(displayName: string): boolean {
  // User IDs typically start with U and are alphanumeric
  if (/^U[A-Z0-9]{8,}$/i.test(displayName)) return false
  // Generic placeholders
  if (displayName === 'Direct Message' || displayName === 'Group Message') return false
  // Empty or whitespace only
  if (!displayName.trim()) return false
  return true
}

export function SlackChannelSelector({
  isOpen,
  onClose,
  teamId,
  onSave,
}: SlackChannelSelectorProps) {
  const [channels, setChannels] = useState<SlackChannel[]>([])
  const [userMap, setUserMap] = useState<Map<string, SlackUser>>(new Map())
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [searchQuery, setSearchQuery] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [collapsedSections, setCollapsedSections] = useState<Set<ChannelType>>(new Set())
  
  const setSlackState = useAppStore((s) => s.setSlackState)
  const { preferences, save: savePreferences } = usePreferences()

  // Load channels and previously selected channels
  useEffect(() => {
    if (isOpen) {
      loadChannels()
    }
  }, [isOpen])

  const loadChannels = async () => {
    setIsLoading(true)
    setError(null)
    try {
      // Fetch channels, saved channels, and users in parallel
      const [allChannels, savedChannels, users] = await Promise.all([
        api.listSlackChannels(),
        api.getSavedSlackChannels(),
        api.listSlackUsers().catch(() => [] as SlackUser[]), // Don't fail if users can't be fetched
      ])
      
      setChannels(allChannels)
      
      // Build user map for quick lookups
      const map = new Map<string, SlackUser>()
      users.forEach((u) => map.set(u.id, u))
      setUserMap(map)
      
      // Pre-select previously saved channels
      const savedIds = new Set(savedChannels.map((c) => c.channelId))
      setSelectedIds(savedIds)
    } catch (e) {
      console.error('[SlackChannelSelector] Error loading channels:', e)
      const errorMsg = e instanceof Error ? e.message : String(e)
      
      if (errorMsg.includes('missing_scope')) {
        setError('Missing required Slack permissions. Please ensure your Slack app has these User Token Scopes: channels:read, groups:read, im:read, mpim:read, users:read. After adding scopes, reinstall the app and get a new token.')
      } else {
        setError(errorMsg || 'Failed to load channels')
      }
    } finally {
      setIsLoading(false)
    }
  }

  // Filter channels based on search query and filter out unresolved DMs/Group DMs
  const filteredChannels = useMemo(() => {
    let result = channels
    
    // Filter out DMs and Group DMs with unresolved names
    result = result.filter((c) => {
      const type = getChannelType(c)
      if (type === 'dm' || type === 'group') {
        const displayName = getChannelDisplayName(c, type, userMap)
        return isResolvedName(displayName)
      }
      return true // Keep all public/private channels
    })
    
    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase()
      result = result.filter((c) => {
        const type = getChannelType(c)
        const displayName = getChannelDisplayName(c, type, userMap)
        return (
          displayName.toLowerCase().includes(query) ||
          c.name.toLowerCase().includes(query) ||
          c.purpose?.toLowerCase().includes(query) ||
          c.topic?.toLowerCase().includes(query)
        )
      })
    }
    
    return result
  }, [channels, searchQuery, userMap])

  // Group channels by type
  const groupedChannels = useMemo(() => {
    const groups: Record<ChannelType, SlackChannel[]> = {
      public: [],
      private: [],
      dm: [],
      group: [],
    }
    filteredChannels.forEach((channel) => {
      const type = getChannelType(channel)
      groups[type].push(channel)
    })
    return groups
  }, [filteredChannels])

  const toggleSection = (type: ChannelType) => {
    setCollapsedSections((prev) => {
      const next = new Set(prev)
      if (next.has(type)) {
        next.delete(type)
      } else {
        next.add(type)
      }
      return next
    })
  }

  const toggleChannel = (channelId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(channelId)) {
        next.delete(channelId)
      } else {
        next.add(channelId)
      }
      return next
    })
  }

  const selectAllInSection = (type: ChannelType) => {
    const typeChannels = groupedChannels[type]
    setSelectedIds((prev) => {
      const next = new Set(prev)
      typeChannels.forEach((c) => next.add(c.id))
      return next
    })
  }

  const deselectAllInSection = (type: ChannelType) => {
    const typeChannels = groupedChannels[type]
    setSelectedIds((prev) => {
      const next = new Set(prev)
      typeChannels.forEach((c) => next.delete(c.id))
      return next
    })
  }

  const selectAll = () => {
    setSelectedIds(new Set(filteredChannels.map((c) => c.id)))
  }

  const deselectAll = () => {
    setSelectedIds(new Set())
  }

  const handleSave = async () => {
    setIsSaving(true)
    setError(null)
    try {
      const selections: SlackChannelSelection[] = channels
        .filter((c) => selectedIds.has(c.id))
        .map((c) => ({
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
      setSlackState({ selectedChannelCount: selections.length })
      
      // Auto-enable Slack sync when channels are saved
      if (!preferences.enabledSources.includes('slack')) {
        savePreferences({
          ...preferences,
          enabledSources: [...preferences.enabledSources, 'slack'],
        })
      }
      
      onSave?.()
      onClose()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to save channels')
    } finally {
      setIsSaving(false)
    }
  }

  if (!isOpen) return null

  const sectionOrder: ChannelType[] = ['public', 'private', 'group', 'dm']

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border border-border rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border">
          <div>
            <h2 className="text-lg font-semibold text-foreground">
              Select Channels to Sync
            </h2>
            <p className="text-sm text-muted-foreground mt-0.5">
              Choose which Slack channels to include in your daily digest
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-muted rounded-lg transition-colors"
          >
            <X className="h-5 w-5 text-muted-foreground" />
          </button>
        </div>

        {/* Search and Actions */}
        <div className="px-6 py-3 border-b border-border flex items-center gap-3">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              type="text"
              placeholder="Search channels..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
          <button
            onClick={selectAll}
            className="text-sm text-primary-500 hover:text-primary-600"
          >
            Select All
          </button>
          <span className="text-muted-foreground">|</span>
          <button
            onClick={deselectAll}
            className="text-sm text-muted-foreground hover:text-foreground"
          >
            Deselect All
          </button>
        </div>

        {/* Channel List */}
        <div className="flex-1 overflow-y-auto px-6 py-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
              <span className="ml-2 text-muted-foreground">Loading channels...</span>
            </div>
          ) : error ? (
            <div className="text-center py-12">
              <p className="text-red-500 max-w-md mx-auto">{error}</p>
              <Button onClick={loadChannels} variant="outline" className="mt-4">
                Retry
              </Button>
            </div>
          ) : (
            <div className="space-y-2">
              {sectionOrder.map((type) => {
                const typeChannels = groupedChannels[type]
                if (typeChannels.length === 0) return null

                const Icon = getChannelIcon(type)
                const isCollapsed = collapsedSections.has(type)
                const selectedInSection = typeChannels.filter((c) => selectedIds.has(c.id)).length

                return (
                  <div key={type} className="border border-border rounded-lg overflow-hidden">
                    {/* Section Header */}
                    <button
                      onClick={() => toggleSection(type)}
                      className="w-full flex items-center gap-2 px-4 py-3 bg-muted/50 hover:bg-muted transition-colors"
                    >
                      {isCollapsed ? (
                        <ChevronRight className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <ChevronDown className="h-4 w-4 text-muted-foreground" />
                      )}
                      <Icon className="h-4 w-4 text-muted-foreground" />
                      <h3 className="text-sm font-medium text-foreground flex-1 text-left">
                        {getChannelTypeLabel(type)}
                      </h3>
                      <span className="text-xs text-muted-foreground">
                        {selectedInSection}/{typeChannels.length} selected
                      </span>
                    </button>

                    {/* Section Content */}
                    {!isCollapsed && (
                      <div className="border-t border-border">
                        {/* Section Actions */}
                        <div className="px-4 py-2 bg-muted/30 flex items-center gap-2 text-xs">
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              selectAllInSection(type)
                            }}
                            className="text-primary-500 hover:text-primary-600"
                          >
                            Select all
                          </button>
                          <span className="text-muted-foreground">|</span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              deselectAllInSection(type)
                            }}
                            className="text-muted-foreground hover:text-foreground"
                          >
                            Deselect all
                          </button>
                        </div>

                        {/* Channel List */}
                        <div className="divide-y divide-border">
                          {typeChannels.map((channel) => {
                            const isSelected = selectedIds.has(channel.id)
                            const displayName = getChannelDisplayName(channel, type, userMap)
                            
                            return (
                              <button
                                key={channel.id}
                                onClick={() => toggleChannel(channel.id)}
                                className={clsx(
                                  'w-full flex items-center gap-3 px-4 py-2.5 transition-colors text-left',
                                  isSelected
                                    ? 'bg-primary-50 dark:bg-primary-900/20'
                                    : 'hover:bg-muted/50'
                                )}
                              >
                                {isSelected ? (
                                  <CheckSquare className="h-4 w-4 text-primary-500 flex-shrink-0" />
                                ) : (
                                  <Square className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                                )}
                                <div className="flex-1 min-w-0">
                                  <div className="flex items-center gap-2">
                                    <span className="text-sm font-medium text-foreground truncate">
                                      {displayName}
                                    </span>
                                    {channel.memberCount != null && channel.memberCount > 0 && (
                                      <span className="text-xs text-muted-foreground flex-shrink-0">
                                        {channel.memberCount} members
                                      </span>
                                    )}
                                  </div>
                                  {channel.purpose && (
                                    <p className="text-xs text-muted-foreground truncate mt-0.5">
                                      {channel.purpose}
                                    </p>
                                  )}
                                </div>
                              </button>
                            )
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                )
              })}

              {filteredChannels.length === 0 && (
                <div className="text-center py-8 text-muted-foreground">
                  No channels found
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-border flex items-center justify-between">
          <span className="text-sm text-muted-foreground">
            {selectedIds.size} channel{selectedIds.size !== 1 ? 's' : ''} selected
          </span>
          <div className="flex items-center gap-3">
            <Button variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button onClick={handleSave} disabled={isSaving || selectedIds.size === 0}>
              {isSaving ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : (
                <Save className="h-4 w-4" />
              )}
              Save Selection
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}
