import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { SyncSection } from './SyncSection'

// Mock hooks
const mockSync = vi.fn()
const mockSave = vi.fn()

vi.mock('../../hooks/usePreferences', () => ({
  usePreferences: () => ({
    preferences: { syncIntervalMinutes: 15 },
    save: mockSave,
    isSaving: false,
  }),
}))

vi.mock('../../hooks/useDigest', () => ({
  useSync: () => ({
    sync: mockSync,
    isSyncing: false,
    status: { lastSyncAt: Date.now(), nextSyncAt: Date.now() + 900000 },
  }),
}))

describe('SyncSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the section title', () => {
    render(<SyncSection />)
    expect(screen.getByText('Sync Settings')).toBeInTheDocument()
  })

  it('renders the description', () => {
    render(<SyncSection />)
    expect(screen.getByText('Configure how often data is synced from your connected sources.')).toBeInTheDocument()
  })

  it('renders the sync status section', () => {
    render(<SyncSection />)
    expect(screen.getByText('Sync Status')).toBeInTheDocument()
  })

  it('renders the sync now button', () => {
    render(<SyncSection />)
    expect(screen.getByText('Sync Now')).toBeInTheDocument()
  })

  it('calls sync when clicking sync now button', () => {
    render(<SyncSection />)
    fireEvent.click(screen.getByText('Sync Now'))
    expect(mockSync).toHaveBeenCalled()
  })

  it('renders sync interval selector', () => {
    render(<SyncSection />)
    expect(screen.getByText('Sync Interval')).toBeInTheDocument()
    expect(screen.getByText('How often to sync data from sources')).toBeInTheDocument()
  })

  it('renders interval options', () => {
    render(<SyncSection />)
    const select = screen.getByRole('combobox')
    expect(select).toBeInTheDocument()
    expect(screen.getByText('Every 5 minutes')).toBeInTheDocument()
    expect(screen.getByText('Every 15 minutes')).toBeInTheDocument()
    expect(screen.getByText('Every 30 minutes')).toBeInTheDocument()
    expect(screen.getByText('Every hour')).toBeInTheDocument()
  })

  it('calls save when changing interval', () => {
    render(<SyncSection />)
    const select = screen.getByRole('combobox')
    fireEvent.change(select, { target: { value: '30' } })
    expect(mockSave).toHaveBeenCalledWith({ syncIntervalMinutes: 30 })
  })
})
