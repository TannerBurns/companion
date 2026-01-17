import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'

// Mock the useConnectionStatus hook
vi.mock('../hooks/useConnectionStatus', () => ({
  useConnectionStatus: vi.fn(),
}))

import { OfflineIndicator } from './OfflineIndicator'
import { useConnectionStatus } from '../hooks/useConnectionStatus'

const mockUseConnectionStatus = useConnectionStatus as ReturnType<typeof vi.fn>

describe('OfflineIndicator', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null when online', () => {
    mockUseConnectionStatus.mockReturnValue(true)

    const { container } = render(<OfflineIndicator />)
    expect(container.firstChild).toBeNull()
  })

  it('renders offline message when offline', () => {
    mockUseConnectionStatus.mockReturnValue(false)

    render(<OfflineIndicator />)
    expect(screen.getByText('Offline - viewing cached data')).toBeInTheDocument()
  })

  it('has correct styling classes when offline', () => {
    mockUseConnectionStatus.mockReturnValue(false)

    render(<OfflineIndicator />)
    const indicator = screen.getByText('Offline - viewing cached data').parentElement
    expect(indicator).toHaveClass('fixed', 'bottom-4', 'left-4')
  })
})
