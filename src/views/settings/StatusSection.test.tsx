import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { StatusSection } from './StatusSection'

// Mock useAppStore
const mockSetSettingsSection = vi.fn()
vi.mock('../../store', () => ({
  useAppStore: () => ({
    setSettingsSection: mockSetSettingsSection,
  }),
}))

// Mock useSetupStatus hook
vi.mock('../../hooks/useSetupStatus', () => ({
  useSetupStatus: () => ({
    geminiConfigured: true,
    hasConnectedSource: false,
    isComplete: false,
    isLoading: false,
  }),
}))

describe('StatusSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the section title', () => {
    render(<StatusSection />)
    expect(screen.getByText('Status')).toBeInTheDocument()
  })

  it('renders the description', () => {
    render(<StatusSection />)
    expect(screen.getByText('Overview of your Companion setup and configuration.')).toBeInTheDocument()
  })

  it('shows setup required when not complete', () => {
    render(<StatusSection />)
    expect(screen.getByText('Setup Required')).toBeInTheDocument()
    expect(screen.getByText('Complete the steps below to start using Companion.')).toBeInTheDocument()
  })

  it('renders status items', () => {
    render(<StatusSection />)
    expect(screen.getByText('Gemini AI')).toBeInTheDocument()
    expect(screen.getByText('Data Source')).toBeInTheDocument()
  })

  it('shows correct status for gemini (configured)', () => {
    render(<StatusSection />)
    expect(screen.getByText('API configured and ready')).toBeInTheDocument()
  })

  it('shows correct status for data source (not connected)', () => {
    render(<StatusSection />)
    expect(screen.getByText('Connect Slack or another data source')).toBeInTheDocument()
  })

  it('navigates to api-keys when clicking Gemini item', () => {
    render(<StatusSection />)
    fireEvent.click(screen.getByText('Gemini AI'))
    expect(mockSetSettingsSection).toHaveBeenCalledWith('api-keys')
  })

  it('navigates to sources when clicking Data Source item', () => {
    render(<StatusSection />)
    fireEvent.click(screen.getByText('Data Source'))
    expect(mockSetSettingsSection).toHaveBeenCalledWith('sources')
  })
})
