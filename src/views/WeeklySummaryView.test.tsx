import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { format, startOfWeek } from 'date-fns'
import { WeeklySummaryView } from './WeeklySummaryView'
import type { DigestResponse, WeeklyBreakdownResponse } from '../lib/api'

const mockUseWeeklyDigest = vi.fn()
const mockUseGenerateWeeklyBreakdown = vi.fn()
const mockMutateAsync = vi.fn()
const mockSetView = vi.fn()
const mockAddLocalActivity = vi.fn(() => 'activity-1')
const mockUpdateLocalActivity = vi.fn()

vi.mock('../hooks/useDigest', () => ({
  useWeeklyDigest: (...args: unknown[]) => mockUseWeeklyDigest(...args),
  useGenerateWeeklyBreakdown: () => mockUseGenerateWeeklyBreakdown(),
}))

vi.mock('../store', () => ({
  useAppStore: () => ({
    setView: mockSetView,
    addLocalActivity: mockAddLocalActivity,
    updateLocalActivity: mockUpdateLocalActivity,
  }),
}))

vi.mock('../components', () => ({
  ContentCard: ({ item }: { item: { id: string } }) => <div>item-{item.id}</div>,
  ContentDetailModal: () => null,
  ExportMenu: () => <div>Export Menu</div>,
}))

const baseDigest: DigestResponse = {
  date: '2026-02-18',
  items: [
    {
      id: 'item-1',
      title: 'Weekly update',
      summary: 'Summary',
      category: 'overview',
      source: 'slack',
      importanceScore: 8,
      createdAt: Date.now(),
    },
  ],
  categories: [],
}

const breakdownResponse: WeeklyBreakdownResponse = {
  weekStart: '2026-02-16',
  weekEnd: '2026-02-22',
  title: 'Week of Feb 16',
  major: [],
  focus: [],
  obstacles: [],
  informational: [],
  breakdownText: 'Weekly breakdown content',
}

describe('WeeklySummaryView breakdown flow', () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.clearAllMocks()
    mockUseWeeklyDigest.mockReturnValue({
      data: baseDigest,
      isLoading: false,
      error: null,
    })
    mockUseGenerateWeeklyBreakdown.mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
      isError: false,
      error: null,
    })
    mockMutateAsync.mockResolvedValue(breakdownResponse)
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    })
  })

  afterEach(() => {
    consoleErrorSpy.mockRestore()
  })

  it('disables Generate Breakdown when there are no items', () => {
    mockUseWeeklyDigest.mockReturnValue({
      data: { ...baseDigest, items: [] },
      isLoading: false,
      error: null,
    })

    render(<WeeklySummaryView />)

    expect(screen.getByRole('button', { name: 'Generate Breakdown' })).toBeDisabled()
  })

  it('generates breakdown and opens modal with returned text', async () => {
    render(<WeeklySummaryView />)

    fireEvent.click(screen.getByRole('button', { name: 'Generate Breakdown' }))

    const expectedWeekStart = format(startOfWeek(new Date(), { weekStartsOn: 1 }), 'yyyy-MM-dd')
    const expectedTimezoneOffset = startOfWeek(new Date(), { weekStartsOn: 1 }).getTimezoneOffset()
    await waitFor(() =>
      expect(mockMutateAsync).toHaveBeenCalledWith({
        weekStart: expectedWeekStart,
        timezoneOffset: expectedTimezoneOffset,
      })
    )
    expect(await screen.findByText('Weekly Breakdown')).toBeInTheDocument()
    expect(screen.getByDisplayValue('Weekly breakdown content')).toBeInTheDocument()
  })

  it('renders mutation error message when generation fails', () => {
    mockUseGenerateWeeklyBreakdown.mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
      isError: true,
      error: new Error('Generation failed'),
    })

    render(<WeeklySummaryView />)

    expect(screen.getByText('Generation failed')).toBeInTheDocument()
  })

  it('copies breakdown text with clipboard API and shows copied status', async () => {
    render(<WeeklySummaryView />)
    fireEvent.click(screen.getByRole('button', { name: 'Generate Breakdown' }))
    await screen.findByText('Weekly Breakdown')

    fireEvent.click(screen.getByRole('button', { name: 'Copy' }))

    await waitFor(() =>
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Weekly breakdown content')
    )
    expect(screen.getByText('Copied to clipboard.')).toBeInTheDocument()
  })

  it('falls back to execCommand copy when clipboard API is unavailable', async () => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined })
    const execCommand = vi.fn().mockReturnValue(true)
    Object.defineProperty(document, 'execCommand', { configurable: true, value: execCommand })

    render(<WeeklySummaryView />)
    fireEvent.click(screen.getByRole('button', { name: 'Generate Breakdown' }))
    await screen.findByText('Weekly Breakdown')
    fireEvent.click(screen.getByRole('button', { name: 'Copy' }))

    await waitFor(() => expect(execCommand).toHaveBeenCalledWith('copy'))
    expect(screen.getByText('Copied to clipboard.')).toBeInTheDocument()
  })

  it('shows failure message when copy fails', async () => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: vi.fn().mockRejectedValue(new Error('denied')),
      },
    })

    render(<WeeklySummaryView />)
    fireEvent.click(screen.getByRole('button', { name: 'Generate Breakdown' }))
    await screen.findByText('Weekly Breakdown')
    fireEvent.click(screen.getByRole('button', { name: 'Copy' }))

    expect(await screen.findByText('Copy failed. Select and copy manually.')).toBeInTheDocument()
  })
})
