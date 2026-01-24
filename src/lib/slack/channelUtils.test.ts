import { describe, it, expect } from 'vitest'
import {
  getChannelType,
  getChannelTypeLabel,
  getChannelDisplayName,
  isResolvedName,
  groupChannelsByType,
  filterChannels,
} from './channelUtils'
import type { SlackChannel, SlackUser } from '../api'

// Helper to create a mock channel
function createChannel(overrides: Partial<SlackChannel> = {}): SlackChannel {
  return {
    id: 'C123',
    name: 'test-channel',
    isPrivate: false,
    isIm: false,
    isMpim: false,
    ...overrides,
  }
}

// Helper to create a mock user map
function createUserMap(users: Partial<SlackUser>[]): Map<string, SlackUser> {
  const map = new Map<string, SlackUser>()
  users.forEach(u => {
    const user: SlackUser = {
      id: u.id || 'U1',
      name: u.name || 'user1',
      realName: u.realName,
      displayName: u.displayName,
    }
    map.set(user.id, user)
  })
  return map
}

describe('getChannelType', () => {
  it('returns public for regular channels', () => {
    const channel = createChannel()
    expect(getChannelType(channel)).toBe('public')
  })

  it('returns private for private channels', () => {
    const channel = createChannel({ isPrivate: true })
    expect(getChannelType(channel)).toBe('private')
  })

  it('returns dm for direct messages', () => {
    const channel = createChannel({ isIm: true })
    expect(getChannelType(channel)).toBe('dm')
  })

  it('returns group for multi-party DMs', () => {
    const channel = createChannel({ isMpim: true })
    expect(getChannelType(channel)).toBe('group')
  })

  it('prioritizes isMpim over isIm', () => {
    const channel = createChannel({ isIm: true, isMpim: true })
    expect(getChannelType(channel)).toBe('group')
  })
})

describe('getChannelTypeLabel', () => {
  it('returns correct labels', () => {
    expect(getChannelTypeLabel('public')).toBe('Public Channels')
    expect(getChannelTypeLabel('private')).toBe('Private Channels')
    expect(getChannelTypeLabel('dm')).toBe('Direct Messages')
    expect(getChannelTypeLabel('group')).toBe('Group Messages')
  })
})

describe('getChannelDisplayName', () => {
  it('returns channel name for public channels', () => {
    const channel = createChannel({ name: 'engineering' })
    const userMap = new Map<string, SlackUser>()
    expect(getChannelDisplayName(channel, 'public', userMap)).toBe('engineering')
  })

  it('resolves DM user name from user map', () => {
    const channel = createChannel({ isIm: true, user: 'U123', name: 'D123' })
    const userMap = createUserMap([{ id: 'U123', displayName: 'Alice Smith' }])
    expect(getChannelDisplayName(channel, 'dm', userMap)).toBe('Alice Smith')
  })

  it('falls back to user ID for unresolved DMs', () => {
    const channel = createChannel({ isIm: true, user: 'U999', name: 'D123' })
    const userMap = new Map<string, SlackUser>()
    expect(getChannelDisplayName(channel, 'dm', userMap)).toBe('U999')
  })

  it('falls back to Direct Message placeholder', () => {
    const channel = createChannel({ isIm: true, name: 'D123', id: 'D123' })
    const userMap = new Map<string, SlackUser>()
    expect(getChannelDisplayName(channel, 'dm', userMap)).toBe('Direct Message')
  })

  it('resolves group DM names from mpdm format', () => {
    const channel = createChannel({ isMpim: true, name: 'mpdm-alice--bob--charlie-1' })
    const userMap = createUserMap([
      { id: 'U1', name: 'alice', displayName: 'Alice' },
      { id: 'U2', name: 'bob', displayName: 'Bob' },
      { id: 'U3', name: 'charlie', displayName: 'Charlie' },
    ])
    expect(getChannelDisplayName(channel, 'group', userMap)).toBe('Alice, Bob, Charlie')
  })

  it('handles group DMs with comma-separated names', () => {
    const channel = createChannel({ isMpim: true, name: 'alice, bob, charlie' })
    const userMap = createUserMap([
      { id: 'U1', name: 'alice', displayName: 'Alice' },
      { id: 'U2', name: 'bob', displayName: 'Bob' },
    ])
    expect(getChannelDisplayName(channel, 'group', userMap)).toBe('Alice, Bob, charlie')
  })

  it('returns Group Message for unresolvable group DMs', () => {
    // Channel with no name falls back to Group Message
    const channel = createChannel({ isMpim: true, name: '' })
    const userMap = new Map<string, SlackUser>()
    expect(getChannelDisplayName(channel, 'group', userMap)).toBe('Group Message')
  })
})

describe('isResolvedName', () => {
  it('returns false for user IDs', () => {
    expect(isResolvedName('U123ABC456')).toBe(false)
    expect(isResolvedName('UABCD12345')).toBe(false)
  })

  it('returns false for generic placeholders', () => {
    expect(isResolvedName('Direct Message')).toBe(false)
    expect(isResolvedName('Group Message')).toBe(false)
  })

  it('returns false for empty strings', () => {
    expect(isResolvedName('')).toBe(false)
    expect(isResolvedName('   ')).toBe(false)
  })

  it('returns true for real names', () => {
    expect(isResolvedName('Alice Smith')).toBe(true)
    expect(isResolvedName('engineering')).toBe(true)
    expect(isResolvedName('#general')).toBe(true)
  })
})

describe('groupChannelsByType', () => {
  it('groups channels correctly', () => {
    const channels = [
      createChannel({ id: '1', isPrivate: false }),
      createChannel({ id: '2', isPrivate: true }),
      createChannel({ id: '3', isIm: true }),
      createChannel({ id: '4', isMpim: true }),
      createChannel({ id: '5', isPrivate: false }),
    ]

    const groups = groupChannelsByType(channels)
    
    expect(groups.public.length).toBe(2)
    expect(groups.private.length).toBe(1)
    expect(groups.dm.length).toBe(1)
    expect(groups.group.length).toBe(1)
  })

  it('returns empty arrays for missing types', () => {
    const channels = [createChannel({ isPrivate: false })]
    const groups = groupChannelsByType(channels)
    
    expect(groups.public.length).toBe(1)
    expect(groups.private.length).toBe(0)
    expect(groups.dm.length).toBe(0)
    expect(groups.group.length).toBe(0)
  })
})

describe('filterChannels', () => {
  it('filters out unresolved DMs', () => {
    const channels = [
      createChannel({ id: '1', name: 'general' }),
      // This DM has no user in userMap, so displayName falls back to user ID (UABCDEFGH1)
      // which matches the unresolved pattern and gets filtered out
      createChannel({ id: '2', isIm: true, user: 'UABCDEFGH1', name: 'D123' }),
    ]
    const userMap = new Map<string, SlackUser>()
    
    const filtered = filterChannels(channels, '', userMap)
    expect(filtered.length).toBe(1)
    expect(filtered[0].id).toBe('1')
  })

  it('keeps resolved DMs', () => {
    const channels = [
      createChannel({ id: '1', isIm: true, user: 'U123' }),
    ]
    const userMap = createUserMap([{ id: 'U123', displayName: 'Alice' }])
    
    const filtered = filterChannels(channels, '', userMap)
    expect(filtered.length).toBe(1)
  })

  it('filters by search query', () => {
    const channels = [
      createChannel({ id: '1', name: 'engineering' }),
      createChannel({ id: '2', name: 'marketing' }),
      createChannel({ id: '3', name: 'sales', purpose: 'engineering support' }),
    ]
    const userMap = new Map<string, SlackUser>()
    
    const filtered = filterChannels(channels, 'engineering', userMap)
    expect(filtered.length).toBe(2)
    expect(filtered.map(c => c.id)).toContain('1')
    expect(filtered.map(c => c.id)).toContain('3')
  })

  it('is case insensitive', () => {
    const channels = [createChannel({ id: '1', name: 'Engineering' })]
    const userMap = new Map<string, SlackUser>()
    
    const filtered = filterChannels(channels, 'engineering', userMap)
    expect(filtered.length).toBe(1)
  })
})
