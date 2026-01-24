import type { SlackChannel, SlackUser } from '../api'

export type ChannelType = 'public' | 'private' | 'dm' | 'group'

export function getChannelType(channel: SlackChannel): ChannelType {
  // Check isMpim first - group DMs might have both isIm and isMpim set
  if (channel.isMpim) return 'group'
  if (channel.isIm) return 'dm'
  if (channel.isPrivate) return 'private'
  return 'public'
}

export function getChannelTypeLabel(type: ChannelType): string {
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

export function getChannelDisplayName(
  channel: SlackChannel, 
  type: ChannelType, 
  userMap: Map<string, SlackUser>
): string {
  // For DMs, use the 'user' field to look up the other person's name
  if (type === 'dm') {
    if (channel.user && userMap.has(channel.user)) {
      const user = userMap.get(channel.user)!
      return user.displayName || user.realName || user.name
    }
    
    if (channel.user) {
      return channel.user
    }
    
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
        let cleaned = channel.name.slice(5)
        cleaned = cleaned.replace(/-\d+$/, '')
        parts = cleaned.split('--')
      } else if (channel.name.includes(',')) {
        parts = channel.name.split(',').map(p => p.trim())
      } else if (channel.name.includes('--')) {
        parts = channel.name.split('--')
      } else {
        parts = [channel.name]
      }
      
      // Try to resolve each part to a real name
      const resolvedParts = parts.map(part => {
        if (userMap.has(part)) {
          const user = userMap.get(part)!
          return user.displayName || user.realName || user.name
        }
        for (const [, user] of userMap) {
          if (user.name === part) {
            return user.displayName || user.realName || user.name
          }
        }
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

export function isResolvedName(displayName: string): boolean {
  // User IDs typically start with U and are alphanumeric
  if (/^U[A-Z0-9]{8,}$/i.test(displayName)) return false
  // Generic placeholders
  if (displayName === 'Direct Message' || displayName === 'Group Message') return false
  // Empty or whitespace only
  if (!displayName.trim()) return false
  return true
}

export function groupChannelsByType(
  channels: SlackChannel[]
): Record<ChannelType, SlackChannel[]> {
  const groups: Record<ChannelType, SlackChannel[]> = {
    public: [],
    private: [],
    dm: [],
    group: [],
  }
  
  channels.forEach(channel => {
    const type = getChannelType(channel)
    groups[type].push(channel)
  })
  
  return groups
}

export function filterChannels(
  channels: SlackChannel[],
  searchQuery: string,
  userMap: Map<string, SlackUser>
): SlackChannel[] {
  let result = channels
  
  // Filter out DMs and Group DMs with unresolved names
  result = result.filter(c => {
    const type = getChannelType(c)
    if (type === 'dm' || type === 'group') {
      const displayName = getChannelDisplayName(c, type, userMap)
      return isResolvedName(displayName)
    }
    return true
  })
  
  // Apply search filter
  if (searchQuery.trim()) {
    const query = searchQuery.toLowerCase()
    result = result.filter(c => {
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
}
