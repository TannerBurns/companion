import { useState } from 'react'
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
import { useAppStore } from '../store'
import { usePreferences } from '../hooks/usePreferences'
import { useSlackChannels } from '../hooks/useSlackChannels'
import {
  type ChannelType,
  getChannelType,
  getChannelTypeLabel,
  getChannelDisplayName,
} from '../lib/slack'

interface SlackChannelSelectorProps {
  isOpen: boolean
  onClose: () => void
  teamId: string
  onSave?: () => void
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

export function SlackChannelSelector({
  isOpen,
  onClose,
  teamId,
  onSave,
}: SlackChannelSelectorProps) {
  const [collapsedSections, setCollapsedSections] = useState<Set<ChannelType>>(new Set())
  
  const setSlackState = useAppStore((s) => s.setSlackState)
  const { preferences, save: savePreferences } = usePreferences()

  const {
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
  } = useSlackChannels({
    enabled: isOpen,
    teamId,
  })

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

  const handleSave = async () => {
    try {
      const selections = await saveSelection()
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
    } catch {
      // Error is already set by the hook
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
                            const channelType = getChannelType(channel)
                            const displayName = getChannelDisplayName(channel, channelType, userMap)
                            
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
