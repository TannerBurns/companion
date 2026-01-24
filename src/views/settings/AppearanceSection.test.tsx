import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { AppearanceSection } from './AppearanceSection'

// Mock useTheme hook
const mockSetTheme = vi.fn()
vi.mock('../../lib/useTheme', () => ({
  useTheme: () => ({
    theme: 'system',
    setTheme: mockSetTheme,
  }),
}))

describe('AppearanceSection', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the section title', () => {
    render(<AppearanceSection />)
    expect(screen.getByText('Appearance')).toBeInTheDocument()
  })

  it('renders the description', () => {
    render(<AppearanceSection />)
    expect(screen.getByText('Customize how Companion looks.')).toBeInTheDocument()
  })

  it('renders all theme options', () => {
    render(<AppearanceSection />)
    expect(screen.getByText('Light')).toBeInTheDocument()
    expect(screen.getByText('Dark')).toBeInTheDocument()
    expect(screen.getByText('System')).toBeInTheDocument()
  })

  it('calls setTheme when clicking a theme option', () => {
    render(<AppearanceSection />)
    
    fireEvent.click(screen.getByText('Dark'))
    expect(mockSetTheme).toHaveBeenCalledWith('dark')
    
    fireEvent.click(screen.getByText('Light'))
    expect(mockSetTheme).toHaveBeenCalledWith('light')
  })

  it('renders the Theme heading', () => {
    render(<AppearanceSection />)
    expect(screen.getByText('Theme')).toBeInTheDocument()
    expect(screen.getByText('Choose between light and dark mode')).toBeInTheDocument()
  })
})
