import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { GuidanceSection } from './GuidanceSection'

// Mock hooks
const mockSave = vi.fn()

const mockPreferences = {
  syncIntervalMinutes: 15,
  enabledSources: [],
  enabledCategories: ['engineering'],
  notificationsEnabled: true,
  userGuidance: undefined as string | undefined,
}

vi.mock('../../hooks/usePreferences', () => ({
  usePreferences: () => ({
    preferences: mockPreferences,
    save: mockSave,
    isSaving: false,
  }),
}))

describe('GuidanceSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockPreferences.userGuidance = undefined
  })

  it('renders the section title', () => {
    render(<GuidanceSection />)
    expect(screen.getByText('AI Guidance')).toBeInTheDocument()
  })

  it('renders the description', () => {
    render(<GuidanceSection />)
    expect(
      screen.getByText('Customize how the AI summarizes and prioritizes your information.')
    ).toBeInTheDocument()
  })

  it('renders the Your Preferences heading', () => {
    render(<GuidanceSection />)
    expect(screen.getByText('Your Preferences')).toBeInTheDocument()
  })

  it('renders the guidance explanation text', () => {
    render(<GuidanceSection />)
    expect(
      screen.getByText(/Tell the AI what matters most to you/)
    ).toBeInTheDocument()
  })

  it('renders the textarea with placeholder', () => {
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    expect(textarea).toBeInTheDocument()
    expect(textarea).toHaveAttribute(
      'placeholder',
      expect.stringContaining('Focus on production issues')
    )
  })

  it('renders example text', () => {
    render(<GuidanceSection />)
    expect(
      screen.getByText(/Only summarize production-related discussions/)
    ).toBeInTheDocument()
  })

  it('displays empty textarea when no guidance saved', () => {
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    expect(textarea).toHaveValue('')
  })

  it('displays existing guidance when present', () => {
    mockPreferences.userGuidance = 'Focus on API changes'
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    expect(textarea).toHaveValue('Focus on API changes')
  })

  it('does not show save button when no changes', () => {
    render(<GuidanceSection />)
    expect(screen.queryByText('Save Changes')).not.toBeInTheDocument()
  })

  it('shows save button after typing', () => {
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: 'New guidance' } })
    expect(screen.getByText('Save Changes')).toBeInTheDocument()
  })

  it('shows cancel button after typing', () => {
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: 'New guidance' } })
    expect(screen.getByText('Cancel')).toBeInTheDocument()
  })

  it('calls save with updated preferences when clicking save', () => {
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: 'Focus on incidents' } })
    fireEvent.click(screen.getByText('Save Changes'))
    expect(mockSave).toHaveBeenCalledWith({
      ...mockPreferences,
      userGuidance: 'Focus on incidents',
    })
  })

  it('trims whitespace when saving', () => {
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: '  Focus on incidents  ' } })
    fireEvent.click(screen.getByText('Save Changes'))
    expect(mockSave).toHaveBeenCalledWith({
      ...mockPreferences,
      userGuidance: 'Focus on incidents',
    })
  })

  it('saves undefined when guidance is empty', () => {
    mockPreferences.userGuidance = 'Old guidance'
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: '' } })
    fireEvent.click(screen.getByText('Save Changes'))
    expect(mockSave).toHaveBeenCalledWith({
      ...mockPreferences,
      userGuidance: undefined,
    })
  })

  it('saves undefined when guidance is only whitespace', () => {
    mockPreferences.userGuidance = 'Old guidance'
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: '   ' } })
    fireEvent.click(screen.getByText('Save Changes'))
    expect(mockSave).toHaveBeenCalledWith({
      ...mockPreferences,
      userGuidance: undefined,
    })
  })

  it('resets to preference value when clicking cancel', () => {
    mockPreferences.userGuidance = 'Original guidance'
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: 'New guidance' } })
    fireEvent.click(screen.getByText('Cancel'))
    expect(textarea).toHaveValue('Original guidance')
    expect(screen.queryByText('Save Changes')).not.toBeInTheDocument()
  })

  it('hides buttons after save', () => {
    render(<GuidanceSection />)
    const textarea = screen.getByRole('textbox')
    fireEvent.change(textarea, { target: { value: 'New guidance' } })
    fireEvent.click(screen.getByText('Save Changes'))
    // After save, the edited state is cleared
    expect(screen.queryByText('Save Changes')).not.toBeInTheDocument()
    expect(screen.queryByText('Cancel')).not.toBeInTheDocument()
  })
})
