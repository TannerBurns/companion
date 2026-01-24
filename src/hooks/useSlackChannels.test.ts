import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'

// Mock the API
const mockListSlackChannels = vi.fn()
const mockGetSavedSlackChannels = vi.fn()
const mockListSlackUsers = vi.fn()
const mockSaveSlackChannels = vi.fn()

vi.mock('../lib/api', () => ({
  api: {
    listSlackChannels: () => mockListSlackChannels(),
    getSavedSlackChannels: () => mockGetSavedSlackChannels(),
    listSlackUsers: () => mockListSlackUsers(),
    saveSlackChannels: (selections: unknown) => mockSaveSlackChannels(selections),
  },
}))

import { useSlackChannels } from './useSlackChannels'

describe('useSlackChannels', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockListSlackChannels.mockResolvedValue([])
    mockGetSavedSlackChannels.mockResolvedValue([])
    mockListSlackUsers.mockResolvedValue([])
    mockSaveSlackChannels.mockResolvedValue(undefined)
  })

  it('starts with empty state', () => {
    const { result } = renderHook(() => useSlackChannels({ enabled: false }))
    
    expect(result.current.channels).toEqual([])
    expect(result.current.selectedIds.size).toBe(0)
    expect(result.current.isLoading).toBe(false)
    expect(result.current.error).toBeNull()
  })

  it('loads channels when enabled', async () => {
    const channels = [
      { id: 'C1', name: 'general', isPrivate: false, isIm: false, isMpim: false },
      { id: 'C2', name: 'random', isPrivate: false, isIm: false, isMpim: false },
    ]
    mockListSlackChannels.mockResolvedValue(channels)
    mockGetSavedSlackChannels.mockResolvedValue([{ channelId: 'C1' }])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })
    
    expect(result.current.channels).toEqual(channels)
    expect(result.current.selectedIds.has('C1')).toBe(true)
    expect(result.current.selectedIds.has('C2')).toBe(false)
  })

  it('handles loading error', async () => {
    mockListSlackChannels.mockRejectedValue(new Error('Network error'))
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })
    
    expect(result.current.error).toBe('Network error')
  })

  it('handles missing_scope error with helpful message', async () => {
    mockListSlackChannels.mockRejectedValue(new Error('missing_scope'))
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })
    
    expect(result.current.error).toContain('Missing required Slack permissions')
  })

  it('toggles channel selection', async () => {
    mockListSlackChannels.mockResolvedValue([
      { id: 'C1', name: 'general', isPrivate: false, isIm: false, isMpim: false },
    ])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    
    act(() => {
      result.current.toggleChannel('C1')
    })
    expect(result.current.selectedIds.has('C1')).toBe(true)
    
    act(() => {
      result.current.toggleChannel('C1')
    })
    expect(result.current.selectedIds.has('C1')).toBe(false)
  })

  it('selects all channels', async () => {
    mockListSlackChannels.mockResolvedValue([
      { id: 'C1', name: 'general', isPrivate: false, isIm: false, isMpim: false },
      { id: 'C2', name: 'random', isPrivate: false, isIm: false, isMpim: false },
    ])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    
    act(() => {
      result.current.selectAll()
    })
    
    expect(result.current.selectedIds.size).toBe(2)
    expect(result.current.selectedIds.has('C1')).toBe(true)
    expect(result.current.selectedIds.has('C2')).toBe(true)
  })

  it('deselects all channels', async () => {
    mockListSlackChannels.mockResolvedValue([
      { id: 'C1', name: 'general', isPrivate: false, isIm: false, isMpim: false },
    ])
    mockGetSavedSlackChannels.mockResolvedValue([{ channelId: 'C1' }])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    expect(result.current.selectedIds.has('C1')).toBe(true)
    
    act(() => {
      result.current.deselectAll()
    })
    
    expect(result.current.selectedIds.size).toBe(0)
  })

  it('filters channels by search query', async () => {
    mockListSlackChannels.mockResolvedValue([
      { id: 'C1', name: 'engineering', isPrivate: false, isIm: false, isMpim: false },
      { id: 'C2', name: 'marketing', isPrivate: false, isIm: false, isMpim: false },
    ])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    
    act(() => {
      result.current.setSearchQuery('eng')
    })
    
    expect(result.current.filteredChannels.length).toBe(1)
    expect(result.current.filteredChannels[0].name).toBe('engineering')
  })

  it('groups channels by type', async () => {
    mockListSlackChannels.mockResolvedValue([
      { id: 'C1', name: 'general', isPrivate: false, isIm: false, isMpim: false },
      { id: 'C2', name: 'private-team', isPrivate: true, isIm: false, isMpim: false },
    ])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    
    expect(result.current.groupedChannels.public.length).toBe(1)
    expect(result.current.groupedChannels.private.length).toBe(1)
    expect(result.current.groupedChannels.dm.length).toBe(0)
    expect(result.current.groupedChannels.group.length).toBe(0)
  })

  it('saves selection', async () => {
    mockListSlackChannels.mockResolvedValue([
      { id: 'C1', name: 'general', isPrivate: false, isIm: false, isMpim: false },
    ])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true, teamId: 'T123' }))
    
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    
    act(() => {
      result.current.toggleChannel('C1')
    })
    
    let savedSelections: unknown
    await act(async () => {
      savedSelections = await result.current.saveSelection()
    })
    
    expect(mockSaveSlackChannels).toHaveBeenCalled()
    expect(savedSelections).toEqual([
      expect.objectContaining({
        channelId: 'C1',
        channelName: 'general',
        teamId: 'T123',
      }),
    ])
  })

  it('builds user map from API response', async () => {
    mockListSlackUsers.mockResolvedValue([
      { id: 'U1', name: 'alice', displayName: 'Alice Smith' },
      { id: 'U2', name: 'bob', realName: 'Bob Jones' },
    ])
    
    const { result } = renderHook(() => useSlackChannels({ enabled: true }))
    
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    
    expect(result.current.userMap.size).toBe(2)
    expect(result.current.userMap.get('U1')?.displayName).toBe('Alice Smith')
    expect(result.current.userMap.get('U2')?.realName).toBe('Bob Jones')
  })
})
