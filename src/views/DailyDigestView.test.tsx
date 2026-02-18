import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { DailyDigestView } from './DailyDigestView'
import type { DigestResponse } from '../lib/api'

const mockUseDailyDigest = vi.fn()
const mockSetView = vi.fn()
const mockAddLocalActivity = vi.fn(() => 'activity-1')
const mockUpdateLocalActivity = vi.fn()
const mockRefetch = vi.fn().mockResolvedValue(undefined)
const mockResyncHistoricalDay = vi.fn()

vi.mock('../hooks/useDigest', () => ({
  useDailyDigest: (...args: unknown[]) => mockUseDailyDigest(...args),
}))

vi.mock('../store', () => ({
  useAppStore: () => ({
    setView: mockSetView,
    addLocalActivity: mockAddLocalActivity,
    updateLocalActivity: mockUpdateLocalActivity,
  }),
}))

vi.mock('../lib/api', () => ({
  api: {
    resyncHistoricalDay: (...args: unknown[]) => mockResyncHistoricalDay(...args),
  },
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}))

vi.mock('../components', () => ({
  ContentCard: () => null,
  ContentDetailModal: () => null,
  ExportMenu: () => null,
}))

const emptyDigest: DigestResponse = {
  date: '2026-02-18',
  items: [],
  categories: [],
}

describe('DailyDigestView resync handling', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUseDailyDigest.mockReturnValue({
      data: emptyDigest,
      isLoading: false,
      error: null,
      refetch: mockRefetch,
    })
  })

  it('marks activity completed when resync returns warnings but synced items', async () => {
    mockResyncHistoricalDay.mockResolvedValue({
      itemsSynced: 3,
      channelsProcessed: 1,
      errors: ['Slack warning'],
    })

    render(<DailyDigestView />)
    fireEvent.click(screen.getByLabelText('Previous day'))
    fireEvent.click(await screen.findByRole('button', { name: 'Resync This Day' }))

    await waitFor(() =>
      expect(mockUpdateLocalActivity).toHaveBeenCalledWith(
        'activity-1',
        expect.objectContaining({
          status: 'completed',
          message: expect.stringContaining('with warnings'),
        })
      )
    )
    expect(mockRefetch).toHaveBeenCalled()
  })

  it('marks activity failed when resync returns only errors', async () => {
    mockResyncHistoricalDay.mockResolvedValue({
      itemsSynced: 0,
      channelsProcessed: 1,
      errors: ['Sync already in progress'],
    })

    render(<DailyDigestView />)
    fireEvent.click(screen.getByLabelText('Previous day'))
    fireEvent.click(await screen.findByRole('button', { name: 'Resync This Day' }))

    await waitFor(() =>
      expect(mockUpdateLocalActivity).toHaveBeenCalledWith(
        'activity-1',
        expect.objectContaining({
          status: 'failed',
          error: 'Sync already in progress',
        })
      )
    )
  })

  it('marks activity completed when resync succeeds with no errors', async () => {
    mockResyncHistoricalDay.mockResolvedValue({
      itemsSynced: 10,
      channelsProcessed: 2,
      errors: [],
    })

    render(<DailyDigestView />)
    fireEvent.click(screen.getByLabelText('Previous day'))
    fireEvent.click(await screen.findByRole('button', { name: 'Resync This Day' }))

    await waitFor(() =>
      expect(mockUpdateLocalActivity).toHaveBeenCalledWith(
        'activity-1',
        expect.objectContaining({
          status: 'completed',
          message: expect.stringMatching(/Resynced.*10 items/),
        })
      )
    )
    expect(mockRefetch).toHaveBeenCalled()
  })
})
